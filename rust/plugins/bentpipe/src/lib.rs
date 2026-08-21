mod pcr_manager;

use emane_plugin_api::{
    AntennaPattern, CommonLayerCounters, FfiConfigRequest, FfiControlMessage, FfiFrameworkService,
    FfiPacket, FfiPacketInfo, FfiSlice, MimoRxProperties, MimoTxAntenna, MimoTxFrequencySegment,
    MimoTxProperties, ModelHeader, PluginApi, RxAntennaAdd, RxProperties, TxAntennaProfile,
    TxProperties, CONTROL_MIMO_RX_PROPERTIES, CONTROL_MIMO_TX_PROPERTIES, CONTROL_MODEL_HEADER,
    CONTROL_RX_ANTENNA_ADD, CONTROL_RX_PROPERTIES, CONTROL_TX_PROPERTIES,
    MAC_REGISTRATION_BENTPIPE, PLUGIN_ABI_VERSION,
};
use pcr_manager::PCRManager;
use prost::Message;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::ffi::{c_void, CStr};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const EVENT_TRANSMIT: u32 = 1;
const EVENT_REASSEMBLY_CHECK: u32 = 2;
const DEFAULT_QUEUE_DEPTH: usize = 256;
const BENTPIPE_MESSAGE_TYPE: u8 = 1;
const BROADCAST_NEM: u16 = u16::MAX;

#[derive(Clone, PartialEq, Message)]
struct BentPipeMessage {
    #[prost(uint64, required, tag = "1")]
    start_of_transmission_microseconds: u64,
    #[prost(uint32, required, tag = "2")]
    curve_index: u32,
    #[prost(message, repeated, tag = "3")]
    messages: Vec<BentPipeComponent>,
}

#[derive(Clone, PartialEq, Message)]
struct BentPipeComponent {
    #[prost(uint32, required, tag = "2")]
    destination: u32,
    #[prost(bytes = "vec", required, tag = "4")]
    data: Vec<u8>,
    #[prost(message, optional, tag = "5")]
    fragment: Option<BentPipeFragment>,
}

#[derive(Clone, PartialEq, Message)]
struct BentPipeFragment {
    #[prost(bool, required, tag = "1")]
    more: bool,
    #[prost(uint32, required, tag = "2")]
    index: u32,
    #[prost(uint32, required, tag = "3")]
    offset: u32,
    #[prost(uint64, required, tag = "4")]
    sequence: u64,
}

#[derive(Clone)]
struct OwnedControl {
    msg_type: u32,
    payload: Vec<u8>,
}

#[derive(Clone)]
struct OwnedPacket {
    info: FfiPacketInfo,
    payload: Vec<u8>,
    controls: Vec<OwnedControl>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ReceiveAction {
    Unknown,
    Ubend,
    Process,
}

#[derive(Clone)]
struct Transponder {
    receive_frequency_hz: u64,
    receive_bandwidth_hz: u64,
    receive_antenna_index: u16,
    receive_action: ReceiveAction,
    receive_enable: bool,
    curve_index: u16,
    transmit_frequency_hz: u64,
    transmit_bandwidth_hz: u64,
    transmit_data_rate_bps: u64,
    transmit_mtu_bytes: usize,
    transmit_antenna_index: u16,
    transmit_power_dbm: f64,
    transmit_delay_us: u64,
    transmit_slots_per_frame: u16,
    transmit_slot_size_us: u64,
    transmit_slots: Vec<u16>,
    transmit_enable: bool,
    tos: HashSet<u8>,
    all_tos: bool,
}

impl Default for Transponder {
    fn default() -> Self {
        Self {
            receive_frequency_hz: 0,
            receive_bandwidth_hz: 0,
            receive_antenna_index: 0,
            receive_action: ReceiveAction::Unknown,
            receive_enable: false,
            curve_index: 0,
            transmit_frequency_hz: 0,
            transmit_bandwidth_hz: 0,
            transmit_data_rate_bps: 0,
            transmit_mtu_bytes: 0,
            transmit_antenna_index: 0,
            transmit_power_dbm: 0.0,
            transmit_delay_us: 0,
            transmit_slots_per_frame: 0,
            transmit_slot_size_us: 0,
            transmit_slots: Vec::new(),
            transmit_enable: false,
            tos: HashSet::new(),
            all_tos: false,
        }
    }
}

struct PendingTx {
    packet: OwnedPacket,
    sequence: u64,
    offset: usize,
    fragment_index: u32,
    ready_at: i64,
}

struct TxComponent {
    packet: OwnedPacket,
    sequence: u64,
    offset: usize,
    fragment_index: u32,
    more: bool,
    fragmented: bool,
}

struct Reassembly {
    info: FfiPacketInfo,
    controls: Vec<OwnedControl>,
    destination: u16,
    parts: BTreeMap<usize, (u32, Vec<u8>)>,
    last_fragment_index: Option<u32>,
    last_update: i64,
}

#[derive(Clone, Copy)]
struct AntennaDefinition {
    pattern: AntennaPattern,
    spectral_mask_index: u16,
}

struct State {
    transponders: BTreeMap<u16, Transponder>,
    configured_transponder_fields: HashMap<u16, u32>,
    queues: HashMap<u16, VecDeque<PendingTx>>,
    queue_depth: usize,
    aggregation_enable: bool,
    fragmentation_enable: bool,
    antennas: HashMap<u16, AntennaDefinition>,
    fragment_check_us: i64,
    fragment_timeout_us: i64,
    pcr_uri: String,
    pcr: PCRManager,
    packet_sequence: u64,
    frame_sequence: u64,
    current_eot: HashMap<u16, i64>,
    wakeups: HashMap<u16, i64>,
    timers: HashSet<u64>,
    reassembly: HashMap<(u16, u64), Reassembly>,
    random_state: u64,
    started: bool,
}

struct BentpipeMac {
    id: u16,
    framework: FfiFrameworkService,
    counters: CommonLayerCounters,
    state: Mutex<State>,
}

impl BentpipeMac {
    fn new(id: u16, framework: FfiFrameworkService) -> Self {
        Self {
            id,
            framework,
            counters: CommonLayerCounters::register(framework),
            state: Mutex::new(State {
                transponders: BTreeMap::new(),
                configured_transponder_fields: HashMap::new(),
                queues: HashMap::new(),
                queue_depth: DEFAULT_QUEUE_DEPTH,
                aggregation_enable: true,
                fragmentation_enable: true,
                antennas: HashMap::new(),
                fragment_check_us: 2_000_000,
                fragment_timeout_us: 5_000_000,
                pcr_uri: String::new(),
                pcr: PCRManager::new(),
                packet_sequence: 0,
                frame_sequence: 0,
                current_eot: HashMap::new(),
                wakeups: HashMap::new(),
                timers: HashSet::new(),
                reassembly: HashMap::new(),
                random_state: 0xD1B5_4A32_D192_ED03 ^ u64::from(id),
                started: false,
            }),
        }
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "1" | "on" | "yes" | "true" => Some(true),
        "0" | "off" | "no" | "false" => Some(false),
        _ => None,
    }
}

fn parse_scaled_u64(value: &str) -> Option<u64> {
    let (number, multiplier) = match value.as_bytes().last().copied() {
        Some(b'K' | b'k') => (&value[..value.len() - 1], 1_000.0),
        Some(b'M' | b'm') => (&value[..value.len() - 1], 1_000_000.0),
        Some(b'G' | b'g') => (&value[..value.len() - 1], 1_000_000_000.0),
        _ => (value, 1.0),
    };
    let value = number.parse::<f64>().ok()? * multiplier;
    (value.is_finite() && value >= 0.0 && value <= u64::MAX as f64).then_some(value.round() as u64)
}

fn split_index(value: &str) -> Option<(u16, &str)> {
    let (index, value) = value.split_once(':')?;
    Some((index.parse().ok()?, value))
}

fn transponder_config_bit(name: &str) -> Option<u32> {
    const NAMES: [&str; 18] = [
        "transponder.receive.frequency",
        "transponder.receive.bandwidth",
        "transponder.receive.antenna",
        "transponder.receive.action",
        "transponder.receive.enable",
        "transponder.transmit.pcrcurveindex",
        "transponder.transmit.frequency",
        "transponder.transmit.bandwidth",
        "transponder.transmit.antenna",
        "transponder.transmit.ubend.delay",
        "transponder.transmit.datarate",
        "transponder.transmit.power",
        "transponder.transmit.tosmap",
        "transponder.transmit.slotperframe",
        "transponder.transmit.slotsize",
        "transponder.transmit.txslots",
        "transponder.transmit.mtu",
        "transponder.transmit.enable",
    ];
    NAMES
        .iter()
        .position(|candidate| *candidate == name)
        .map(|index| 1u32 << index)
}

fn parse_antenna(value: &str) -> Option<(u16, AntennaDefinition)> {
    let (index, definition) = split_index(value)?;
    let fields: Vec<_> = definition.split(';').collect();
    let (pattern, spectral_mask_index) = match fields.as_slice() {
        ["omni", gain, mask] => {
            let gain_db = gain.parse::<f64>().ok()?;
            if !gain_db.is_finite() {
                return None;
            }
            (AntennaPattern::IdealOmni { gain_db }, mask.parse().ok()?)
        }
        [profile, azimuth, elevation, mask] => {
            let profile = TxAntennaProfile {
                profile_id: profile.parse().ok()?,
                azimuth_degrees: azimuth.parse().ok()?,
                elevation_degrees: elevation.parse().ok()?,
            };
            profile.encode()?;
            (AntennaPattern::Profile(profile), mask.parse().ok()?)
        }
        _ => return None,
    };
    Some((
        index,
        AntennaDefinition {
            pattern,
            spectral_mask_index,
        },
    ))
}

fn parse_ranges(value: &str) -> Option<HashSet<u8>> {
    let mut result = HashSet::new();
    if value == "na" {
        return Some(result);
    }
    for item in value.split(';') {
        if let Some((start, end)) = item.split_once('-') {
            let (start, end) = (start.parse::<u8>().ok()?, end.parse::<u8>().ok()?);
            if start > end {
                return None;
            }
            result.extend(start..=end);
        } else {
            result.insert(item.parse().ok()?);
        }
    }
    Some(result)
}

fn parse_slots(value: &str) -> Option<Vec<u16>> {
    if value == "na" {
        return Some(Vec::new());
    }
    let mut result = Vec::new();
    for item in value.split(';') {
        if let Some((start, end)) = item.split_once('-') {
            let (start, end) = (start.parse::<u16>().ok()?, end.parse::<u16>().ok()?);
            if start > end {
                return None;
            }
            result.extend(start..=end);
        } else {
            result.push(item.parse().ok()?);
        }
    }
    result.sort_unstable();
    result.dedup();
    Some(result)
}

fn configuration(request: *const c_void) -> Option<Vec<(String, Vec<String>)>> {
    let request = unsafe { (request as *const FfiConfigRequest).as_ref() }?;
    if request.len > 4096 || (request.len != 0 && request.data.is_null()) {
        return None;
    }
    let items = if request.len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(request.data, request.len) }
    };
    let mut result = Vec::with_capacity(items.len());
    for item in items {
        if item.name.is_null()
            || item.values.len > 4096
            || (item.values.len != 0 && item.values.data.is_null())
        {
            return None;
        }
        let name = unsafe { CStr::from_ptr(item.name) }
            .to_str()
            .ok()?
            .to_string();
        let pointers = if item.values.len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(item.values.data, item.values.len) }
        };
        let mut values = Vec::with_capacity(pointers.len());
        for pointer in pointers {
            if pointer.is_null() {
                return None;
            }
            values.push(
                unsafe { CStr::from_ptr(*pointer) }
                    .to_str()
                    .ok()?
                    .to_string(),
            );
        }
        result.push((name, values));
    }
    Some(result)
}

fn controls(ptr: *const FfiControlMessage, len: usize) -> Option<&'static [FfiControlMessage]> {
    if len > 4096 || (len != 0 && ptr.is_null()) {
        None
    } else if len == 0 {
        Some(&[])
    } else {
        Some(unsafe { std::slice::from_raw_parts(ptr, len) })
    }
}

fn own_packet(
    packet: *const FfiPacket,
    messages: *const FfiControlMessage,
    count: usize,
) -> Option<OwnedPacket> {
    let packet = unsafe { packet.as_ref() }?;
    if packet.payload.len > 64 << 20 || (packet.payload.len != 0 && packet.payload.data.is_null()) {
        return None;
    }
    let payload = if packet.payload.len == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(packet.payload.data, packet.payload.len) }.to_vec()
    };
    let controls = controls(messages, count)?
        .iter()
        .map(|message| {
            if message.payload.len > 16 << 20
                || (message.payload.len != 0 && message.payload.data.is_null())
            {
                return None;
            }
            Some(OwnedControl {
                msg_type: message.msg_type,
                payload: if message.payload.len == 0 {
                    Vec::new()
                } else {
                    unsafe { std::slice::from_raw_parts(message.payload.data, message.payload.len) }
                        .to_vec()
                },
            })
        })
        .collect::<Option<_>>()?;
    Some(OwnedPacket {
        info: packet.info,
        payload,
        controls,
    })
}

fn find_control(messages: &[FfiControlMessage], kind: u32) -> Option<&[u8]> {
    messages.iter().find_map(|message| {
        if message.msg_type != kind || (message.payload.len != 0 && message.payload.data.is_null())
        {
            None
        } else if message.payload.len == 0 {
            Some(&[] as &[u8])
        } else {
            Some(unsafe { std::slice::from_raw_parts(message.payload.data, message.payload.len) })
        }
    })
}

fn now_us() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros(),
    )
    .unwrap_or(i64::MAX)
}

fn airtime(bytes: usize, rate: u64) -> u64 {
    (bytes as u64)
        .saturating_mul(8_000_000)
        .saturating_add(rate.saturating_sub(1))
        .checked_div(rate)
        .unwrap_or(0)
}

fn random_unit(state: &mut State) -> f32 {
    let mut value = state.random_state;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    state.random_state = value;
    ((value >> 40) as f32) / ((1u32 << 24) as f32)
}

fn next_slot(transponder: &Transponder, now: i64) -> i64 {
    if transponder.transmit_slots.is_empty()
        || transponder.transmit_slots_per_frame == 0
        || transponder.transmit_slot_size_us == 0
    {
        return now;
    }
    let size = transponder.transmit_slot_size_us as i64;
    let absolute_slot = now.div_euclid(size);
    let current = absolute_slot.rem_euclid(i64::from(transponder.transmit_slots_per_frame)) as u16;
    for slot in &transponder.transmit_slots {
        if *slot > current || (*slot == current && now % size == 0) {
            return (absolute_slot + i64::from(*slot) - i64::from(current)) * size;
        }
    }
    let first = i64::from(transponder.transmit_slots[0]);
    (absolute_slot + i64::from(transponder.transmit_slots_per_frame) - i64::from(current) + first)
        * size
}

fn effective_mtu(transponder: &Transponder) -> usize {
    if transponder.transmit_mtu_bytes != 0 {
        transponder.transmit_mtu_bytes
    } else if transponder.transmit_slot_size_us != 0 {
        ((transponder.transmit_data_rate_bps as u128 * transponder.transmit_slot_size_us as u128)
            / 8_000_000) as usize
    } else {
        0
    }
}

fn enqueue(state: &mut State, index: u16, packet: OwnedPacket, ready_at: i64) -> bool {
    let Some(transponder) = state.transponders.get(&index) else {
        return false;
    };
    if !transponder.transmit_enable {
        return false;
    }
    let mtu = effective_mtu(transponder);
    if mtu == 0 || (packet.payload.len() > mtu && !state.fragmentation_enable) {
        return false;
    }
    let sequence = state.packet_sequence;
    state.packet_sequence = state.packet_sequence.wrapping_add(1);
    let queue = state.queues.entry(index).or_default();
    while queue.len() >= state.queue_depth {
        let position = queue
            .iter()
            .position(|entry| entry.offset == 0)
            .unwrap_or(0);
        queue.remove(position);
    }
    queue.push_back(PendingTx {
        packet,
        sequence,
        offset: 0,
        fragment_index: 0,
        ready_at,
    });
    true
}

fn dequeue_components(state: &mut State, index: u16, mtu: usize, now: i64) -> Vec<TxComponent> {
    let aggregate = state.aggregation_enable;
    let fragment = state.fragmentation_enable;
    let Some(queue) = state.queues.get_mut(&index) else {
        return Vec::new();
    };
    let mut components = Vec::new();
    let mut used = 0usize;

    while used < mtu {
        let Some(front) = queue.front() else { break };
        if front.ready_at > now {
            break;
        }
        let remaining = front.packet.payload.len().saturating_sub(front.offset);
        if remaining == 0 {
            queue.pop_front();
            continue;
        }
        let available = mtu - used;
        if remaining <= available {
            let pending = queue.pop_front().expect("queue front");
            let mut packet = pending.packet;
            packet.payload = packet.payload[pending.offset..].to_vec();
            used += remaining;
            components.push(TxComponent {
                packet,
                sequence: pending.sequence,
                offset: pending.offset,
                fragment_index: pending.fragment_index,
                more: false,
                fragmented: pending.offset != 0,
            });
            if !aggregate {
                break;
            }
        } else if fragment {
            let pending = queue.front_mut().expect("queue front");
            let end = pending.offset + available;
            let mut packet = pending.packet.clone();
            packet.payload = packet.payload[pending.offset..end].to_vec();
            components.push(TxComponent {
                packet,
                sequence: pending.sequence,
                offset: pending.offset,
                fragment_index: pending.fragment_index,
                more: true,
                fragmented: true,
            });
            pending.offset = end;
            pending.fragment_index = pending.fragment_index.saturating_add(1);
            break;
        } else {
            // Legacy strict dequeue drops an oversized head packet so it cannot
            // permanently block smaller packets behind it.
            queue.pop_front();
        }
    }
    components
}

fn schedule_transmit(mac: &BentpipeMac, index: u16, when: i64) {
    let schedule = {
        let mut state = mac.state.lock().unwrap();
        if !state.started || state.wakeups.get(&index).is_some_and(|old| *old <= when) {
            false
        } else {
            state.wakeups.insert(index, when);
            true
        }
    };
    if !schedule {
        return;
    }
    let mut data = Vec::with_capacity(10);
    data.extend_from_slice(&index.to_be_bytes());
    data.extend_from_slice(&when.to_be_bytes());
    let when = when.max(0) as u64;
    let timer = (mac.framework.schedule_timed_event)(
        mac.framework.framework_ctx,
        mac.id,
        when / 1_000_000,
        (when % 1_000_000) as u32,
        EVENT_TRANSMIT,
        data.as_ptr(),
        data.len(),
    );
    if timer != 0 {
        mac.state.lock().unwrap().timers.insert(timer);
    }
}

fn schedule_reassembly_check(mac: &BentpipeMac) {
    let when = {
        let state = mac.state.lock().unwrap();
        if !state.started {
            return;
        }
        now_us().saturating_add(state.fragment_check_us)
    };
    let when_u64 = when.max(0) as u64;
    let timer = (mac.framework.schedule_timed_event)(
        mac.framework.framework_ctx,
        mac.id,
        when_u64 / 1_000_000,
        (when_u64 % 1_000_000) as u32,
        EVENT_REASSEMBLY_CHECK,
        std::ptr::null(),
        0,
    );
    if timer != 0 {
        mac.state.lock().unwrap().timers.insert(timer);
    }
}

fn send_downstream(
    mac: &BentpipeMac,
    components: Vec<TxComponent>,
    frame_sequence: u64,
    transponder: &Transponder,
    antenna: AntennaDefinition,
) {
    let start_of_transmission = now_us();
    let wire = BentPipeMessage {
        start_of_transmission_microseconds: start_of_transmission.max(0) as u64,
        curve_index: u32::from(transponder.curve_index),
        messages: components
            .iter()
            .map(|component| BentPipeComponent {
                destination: u32::from(component.packet.info.destination),
                data: component.packet.payload.clone(),
                fragment: component.fragmented.then_some(BentPipeFragment {
                    more: component.more,
                    index: component.fragment_index,
                    offset: u32::try_from(component.offset).unwrap_or(u32::MAX),
                    sequence: component.sequence,
                }),
            })
            .collect(),
    };
    let payload = wire.encode_to_vec();
    let destination = components
        .first()
        .map(|component| component.packet.info.destination)
        .filter(|destination| {
            components
                .iter()
                .all(|component| component.packet.info.destination == *destination)
        })
        .unwrap_or(BROADCAST_NEM);
    let total_data_bytes = components
        .iter()
        .map(|component| component.packet.payload.len())
        .sum::<usize>();
    let header = ModelHeader {
        registration_id: MAC_REGISTRATION_BENTPIPE,
        sequence: frame_sequence,
        data_rate_bps: transponder.transmit_data_rate_bps,
        category: 0,
        message_type: BENTPIPE_MESSAGE_TYPE,
        flags: transponder.curve_index,
    };
    let header_bytes = header.encode();
    let duration = airtime(total_data_bytes, transponder.transmit_data_rate_bps).max(1);
    let tx_bytes = TxProperties {
        frequency_hz: transponder.transmit_frequency_hz,
        bandwidth_hz: transponder.transmit_bandwidth_hz,
        tx_power_dbm: transponder.transmit_power_dbm
            + match antenna.pattern {
                AntennaPattern::IdealOmni { gain_db } => gain_db,
                _ => 0.0,
            },
        duration_microseconds: duration,
        offset_microseconds: 0,
        tx_time_microseconds: start_of_transmission,
        antenna_index: transponder.transmit_antenna_index,
        spectral_mask_index: antenna.spectral_mask_index,
        sub_id: 0,
    }
    .encode();
    let mimo_bytes = MimoTxProperties {
        frequency_groups: vec![vec![MimoTxFrequencySegment {
            frequency_hz: transponder.transmit_frequency_hz,
            tx_power_dbm: transponder.transmit_power_dbm,
            duration_microseconds: duration,
            offset_microseconds: 0,
        }]],
        transmit_antennas: vec![MimoTxAntenna {
            frequency_group_index: 0,
            antenna_index: transponder.transmit_antenna_index,
            bandwidth_hz: transponder.transmit_bandwidth_hz,
            spectral_mask_index: antenna.spectral_mask_index,
            pattern: antenna.pattern,
        }],
    }
    .encode()
    .expect("validated BentPipe antenna");
    let mut messages: Vec<_> = components
        .first()
        .map(|component| component.packet.controls.as_slice())
        .unwrap_or_default()
        .iter()
        .filter(|control| {
            control.msg_type != CONTROL_MODEL_HEADER
                && control.msg_type != CONTROL_TX_PROPERTIES
                && control.msg_type != CONTROL_MIMO_TX_PROPERTIES
        })
        .map(|control| FfiControlMessage {
            msg_type: control.msg_type,
            payload: FfiSlice {
                data: control.payload.as_ptr(),
                len: control.payload.len(),
            },
        })
        .collect();
    messages.push(FfiControlMessage {
        msg_type: CONTROL_MODEL_HEADER,
        payload: FfiSlice {
            data: header_bytes.as_ptr(),
            len: header_bytes.len(),
        },
    });
    messages.push(FfiControlMessage {
        msg_type: CONTROL_MIMO_TX_PROPERTIES,
        payload: FfiSlice {
            data: mimo_bytes.as_ptr(),
            len: mimo_bytes.len(),
        },
    });
    messages.push(FfiControlMessage {
        msg_type: CONTROL_TX_PROPERTIES,
        payload: FfiSlice {
            data: tx_bytes.as_ptr(),
            len: tx_bytes.len(),
        },
    });
    let packet = FfiPacket {
        info: FfiPacketInfo {
            source: mac.id,
            destination,
            priority: 0,
            creation_time_sec: start_of_transmission.max(0) as u64 / 1_000_000,
            creation_time_usec: start_of_transmission.rem_euclid(1_000_000) as u32,
        },
        payload: FfiSlice {
            data: payload.as_ptr(),
            len: payload.len(),
        },
    };
    (mac.framework.send_downstream_packet)(
        mac.framework.framework_ctx,
        mac.id,
        &packet,
        messages.as_ptr(),
        messages.len(),
    );
    for component in components {
        mac.counters.downstream_tx(
            mac.framework,
            component.packet.info.destination,
            component.packet.payload.len(),
        );
    }
}

fn drive(mac: &BentpipeMac, index: u16, now: i64) {
    let (components, frame_sequence, transponder, antenna, next) = {
        let mut state = mac.state.lock().unwrap();
        let Some(transponder) = state.transponders.get(&index).cloned() else {
            return;
        };
        let Some(antenna) = state
            .antennas
            .get(&transponder.transmit_antenna_index)
            .copied()
        else {
            return;
        };
        let allowed = next_slot(&transponder, now);
        let eot = state.current_eot.get(&index).copied().unwrap_or(0);
        let ready = state
            .queues
            .get(&index)
            .and_then(VecDeque::front)
            .map(|item| item.ready_at);
        let can_send = ready.is_some_and(|ready| ready <= now) && eot <= now && allowed <= now;
        let components = if can_send {
            let mtu = effective_mtu(&transponder);
            let components = dequeue_components(&mut state, index, mtu, now);
            if !components.is_empty() {
                let bytes = components
                    .iter()
                    .map(|component| component.packet.payload.len())
                    .sum();
                state.current_eot.insert(
                    index,
                    now.saturating_add(airtime(bytes, transponder.transmit_data_rate_bps) as i64),
                );
            }
            components
        } else {
            Vec::new()
        };
        let frame_sequence = state.frame_sequence;
        if !components.is_empty() {
            state.frame_sequence = state.frame_sequence.wrapping_add(1);
        }
        let next = state
            .queues
            .get(&index)
            .and_then(VecDeque::front)
            .map(|item| {
                next_slot(
                    &transponder,
                    item.ready_at
                        .max(*state.current_eot.get(&index).unwrap_or(&0))
                        .max(now),
                )
            });
        (components, frame_sequence, transponder, antenna, next)
    };
    if !components.is_empty() {
        send_downstream(mac, components, frame_sequence, &transponder, antenna);
    }
    if let Some(next) = next {
        schedule_transmit(mac, index, next);
    }
}

fn send_upstream(mac: &BentpipeMac, packet: OwnedPacket) {
    let messages: Vec<_> = packet
        .controls
        .iter()
        .map(|control| FfiControlMessage {
            msg_type: control.msg_type,
            payload: FfiSlice {
                data: control.payload.as_ptr(),
                len: control.payload.len(),
            },
        })
        .collect();
    let view = FfiPacket {
        info: packet.info,
        payload: FfiSlice {
            data: packet.payload.as_ptr(),
            len: packet.payload.len(),
        },
    };
    (mac.framework.send_upstream_packet)(
        mac.framework.framework_ctx,
        mac.id,
        &view,
        messages.as_ptr(),
        messages.len(),
    );
    mac.counters
        .upstream_tx(mac.framework, packet.info.destination, packet.payload.len());
}

extern "C" fn init(id: u16, framework: *const FfiFrameworkService) -> *mut c_void {
    let Some(framework) = (unsafe { framework.as_ref() }) else {
        return std::ptr::null_mut();
    };
    Box::into_raw(Box::new(BentpipeMac::new(id, *framework))).cast()
}

extern "C" fn configure(plugin: *mut c_void, request: *const c_void) -> bool {
    let Some(mac) = (unsafe { (plugin as *mut BentpipeMac).as_ref() }) else {
        return false;
    };
    let Some(items) = configuration(request) else {
        return false;
    };
    let mut state = mac.state.lock().unwrap();
    for (name, values) in items {
        match name.as_str() {
            "pcrcurveuri" => {
                let Some(value) = values.first() else {
                    return false;
                };
                state.pcr_uri.clone_from(value);
                continue;
            }
            "queue.depth" => {
                let Some(Ok(value)) = values.first().map(|value| value.parse::<usize>()) else {
                    return false;
                };
                if value == 0 {
                    return false;
                }
                state.queue_depth = value;
                continue;
            }
            "queue.fragmentationenable" => {
                let Some(value) = values.first().and_then(|value| parse_bool(value)) else {
                    return false;
                };
                state.fragmentation_enable = value;
                continue;
            }
            "queue.aggregationenable" => {
                let Some(value) = values.first().and_then(|value| parse_bool(value)) else {
                    return false;
                };
                state.aggregation_enable = value;
                continue;
            }
            "antenna.defines" => {
                let mut antennas = HashMap::new();
                for value in values {
                    let Some((index, antenna)) = parse_antenna(&value) else {
                        return false;
                    };
                    if antennas.insert(index, antenna).is_some() {
                        return false;
                    }
                }
                state.antennas = antennas;
                continue;
            }
            "reassembly.fragmentcheckthreshold" => {
                let Some(Ok(value)) = values.first().map(|value| value.parse::<u16>()) else {
                    return false;
                };
                if value == 0 {
                    return false;
                }
                state.fragment_check_us = i64::from(value) * 1_000_000;
                continue;
            }
            "reassembly.fragmenttimeoutthreshold" => {
                let Some(Ok(value)) = values.first().map(|value| value.parse::<u16>()) else {
                    return false;
                };
                if value == 0 {
                    return false;
                }
                state.fragment_timeout_us = i64::from(value) * 1_000_000;
                continue;
            }
            _ => {}
        }
        for entry in values {
            let Some((index, value)) = split_index(&entry) else {
                return false;
            };
            let Some(field) = transponder_config_bit(&name) else {
                return false;
            };
            *state
                .configured_transponder_fields
                .entry(index)
                .or_default() |= field;
            let transponder = state.transponders.entry(index).or_default();
            match name.as_str() {
                "transponder.receive.frequency" => {
                    transponder.receive_frequency_hz = match parse_scaled_u64(value) {
                        Some(value) => value,
                        None => return false,
                    }
                }
                "transponder.receive.bandwidth" => {
                    transponder.receive_bandwidth_hz = match parse_scaled_u64(value) {
                        Some(value) => value,
                        None => return false,
                    }
                }
                "transponder.receive.antenna" => {
                    transponder.receive_antenna_index = match value.parse() {
                        Ok(value) => value,
                        Err(_) => return false,
                    }
                }
                "transponder.receive.action" => {
                    transponder.receive_action = match value {
                        "ubend" => ReceiveAction::Ubend,
                        "process" => ReceiveAction::Process,
                        _ => return false,
                    }
                }
                "transponder.receive.enable" => {
                    transponder.receive_enable = match parse_bool(value) {
                        Some(value) => value,
                        None => return false,
                    }
                }
                "transponder.transmit.pcrcurveindex" => {
                    transponder.curve_index = match value.parse() {
                        Ok(value) => value,
                        Err(_) => return false,
                    }
                }
                "transponder.transmit.frequency" => {
                    transponder.transmit_frequency_hz = match parse_scaled_u64(value) {
                        Some(value) => value,
                        None => return false,
                    }
                }
                "transponder.transmit.bandwidth" => {
                    transponder.transmit_bandwidth_hz = match parse_scaled_u64(value) {
                        Some(value) => value,
                        None => return false,
                    }
                }
                "transponder.transmit.datarate" => {
                    transponder.transmit_data_rate_bps = match parse_scaled_u64(value) {
                        Some(value) => value,
                        None => return false,
                    }
                }
                "transponder.transmit.mtu" => {
                    if value != "na" {
                        transponder.transmit_mtu_bytes = match value.parse() {
                            Ok(value) => value,
                            Err(_) => return false,
                        }
                    }
                }
                "transponder.transmit.antenna" => {
                    transponder.transmit_antenna_index = match value.parse() {
                        Ok(value) => value,
                        Err(_) => return false,
                    }
                }
                "transponder.transmit.power" => {
                    transponder.transmit_power_dbm = match value.parse::<f64>() {
                        Ok(value) if value.is_finite() => value,
                        _ => return false,
                    }
                }
                "transponder.transmit.ubend.delay" => {
                    if value != "na" {
                        transponder.transmit_delay_us = match value.parse() {
                            Ok(value) => value,
                            Err(_) => return false,
                        }
                    }
                }
                "transponder.transmit.slotperframe" => {
                    if value != "na" {
                        transponder.transmit_slots_per_frame = match value.parse() {
                            Ok(value) => value,
                            Err(_) => return false,
                        }
                    }
                }
                "transponder.transmit.slotsize" => {
                    if value != "na" {
                        transponder.transmit_slot_size_us = match value.parse() {
                            Ok(value) => value,
                            Err(_) => return false,
                        }
                    }
                }
                "transponder.transmit.txslots" => {
                    transponder.transmit_slots = match parse_slots(value) {
                        Some(value) => value,
                        None => return false,
                    }
                }
                "transponder.transmit.tosmap" => {
                    if value == "all" {
                        transponder.all_tos = true
                    } else {
                        transponder.tos = match parse_ranges(value) {
                            Some(value) => value,
                            None => return false,
                        }
                    }
                }
                "transponder.transmit.enable" => {
                    transponder.transmit_enable = match parse_bool(value) {
                        Some(value) => value,
                        None => return false,
                    }
                }
                _ => return false,
            }
        }
    }
    true
}

extern "C" fn start(plugin: *mut c_void) -> bool {
    let Some(mac) = (unsafe { (plugin as *mut BentpipeMac).as_ref() }) else {
        return false;
    };
    let mut state = mac.state.lock().unwrap();
    if state.antennas.is_empty() || state.transponders.is_empty() {
        return false;
    }
    if state.pcr_uri.is_empty() {
        return false;
    }
    let uri = state.pcr_uri.clone();
    if state.pcr.load(&uri).is_err() {
        return false;
    }
    const REQUIRED_FIELDS: u32 = (1 << 18) - 1;
    let mut receive_channels = HashSet::new();
    let mut receive_antennas: HashMap<u16, (u64, Vec<u64>)> = HashMap::new();
    for (index, transponder) in &state.transponders {
        let slotted = transponder.transmit_slots_per_frame != 0
            || transponder.transmit_slot_size_us != 0
            || !transponder.transmit_slots.is_empty();
        if transponder.receive_frequency_hz == 0
            || transponder.receive_bandwidth_hz == 0
            || transponder.receive_action == ReceiveAction::Unknown
            || transponder.transmit_frequency_hz == 0
            || transponder.transmit_bandwidth_hz == 0
            || transponder.transmit_data_rate_bps == 0
            || effective_mtu(transponder) == 0
            || !state
                .antennas
                .contains_key(&transponder.receive_antenna_index)
            || !state
                .antennas
                .contains_key(&transponder.transmit_antenna_index)
            || state.configured_transponder_fields.get(index).copied() != Some(REQUIRED_FIELDS)
            || !state.pcr.contains_curve(transponder.curve_index)
            || !receive_channels.insert((
                transponder.receive_antenna_index,
                transponder.receive_frequency_hz,
            ))
            || (slotted
                && (transponder.transmit_slots_per_frame == 0
                    || transponder.transmit_slot_size_us == 0
                    || transponder.transmit_slots.is_empty()))
            || transponder
                .transmit_slots
                .iter()
                .any(|slot| *slot >= transponder.transmit_slots_per_frame)
        {
            return false;
        }
        let receive = receive_antennas
            .entry(transponder.receive_antenna_index)
            .or_insert_with(|| (transponder.receive_bandwidth_hz, Vec::new()));
        if receive.0 != transponder.receive_bandwidth_hz {
            return false;
        }
        receive.1.push(transponder.receive_frequency_hz);
    }
    state.started = true;
    let definitions = receive_antennas
        .into_iter()
        .map(|(antenna_index, (bandwidth_hz, mut frequencies_hz))| {
            frequencies_hz.sort_unstable();
            frequencies_hz.dedup();
            let definition = *state.antennas.get(&antenna_index)?;
            Some(RxAntennaAdd {
                antenna: MimoTxAntenna {
                    frequency_group_index: 0,
                    antenna_index,
                    bandwidth_hz,
                    spectral_mask_index: definition.spectral_mask_index,
                    pattern: definition.pattern,
                },
                frequencies_hz,
            })
        })
        .collect::<Option<Vec<_>>>();
    let Some(definitions) = definitions else {
        state.started = false;
        return false;
    };
    drop(state);
    let payloads = definitions
        .iter()
        .map(RxAntennaAdd::encode)
        .collect::<Option<Vec<_>>>();
    let Some(payloads) = payloads else {
        return false;
    };
    let messages = payloads
        .iter()
        .map(|payload| FfiControlMessage {
            msg_type: CONTROL_RX_ANTENNA_ADD,
            payload: FfiSlice {
                data: payload.as_ptr(),
                len: payload.len(),
            },
        })
        .collect::<Vec<_>>();
    (mac.framework.send_downstream_control)(
        mac.framework.framework_ctx,
        mac.id,
        messages.as_ptr(),
        messages.len(),
    );
    schedule_reassembly_check(mac);
    true
}

extern "C" fn post_start(_: *mut c_void) {}

extern "C" fn stop(plugin: *mut c_void) {
    let Some(mac) = (unsafe { (plugin as *mut BentpipeMac).as_ref() }) else {
        return;
    };
    let timers = {
        let mut state = mac.state.lock().unwrap();
        state.started = false;
        state.queues.clear();
        state.reassembly.clear();
        state.wakeups.clear();
        state.timers.drain().collect::<Vec<_>>()
    };
    for timer in timers {
        (mac.framework.cancel_timed_event)(mac.framework.framework_ctx, mac.id, timer);
    }
}

extern "C" fn destroy(plugin: *mut c_void) {
    if !plugin.is_null() {
        unsafe { drop(Box::from_raw(plugin as *mut BentpipeMac)) };
    }
}

extern "C" fn upstream(
    plugin: *mut c_void,
    packet: *const FfiPacket,
    messages: *const FfiControlMessage,
    count: usize,
) {
    let Some(mac) = (unsafe { (plugin as *mut BentpipeMac).as_ref() }) else {
        return;
    };
    if packet.is_null() {
        (mac.framework.send_upstream_control)(mac.framework.framework_ctx, mac.id, messages, count);
        return;
    }
    let Some(message_views) = controls(messages, count) else {
        return;
    };
    let Some(header_data) = find_control(message_views, CONTROL_MODEL_HEADER) else {
        return;
    };
    let Some(header) = ModelHeader::decode(header_data) else {
        return;
    };
    if header.registration_id != MAC_REGISTRATION_BENTPIPE
        || header.message_type != BENTPIPE_MESSAGE_TYPE
    {
        return;
    }
    let packet_ref = unsafe { &*packet };
    if packet_ref.payload.len > 64 << 20
        || (packet_ref.payload.len != 0 && packet_ref.payload.data.is_null())
    {
        return;
    }
    let payload = if packet_ref.payload.len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(packet_ref.payload.data, packet_ref.payload.len) }
    };
    let Ok(wire) = BentPipeMessage::decode(payload) else {
        return;
    };
    let Ok(curve_index) = u16::try_from(wire.curve_index) else {
        return;
    };
    mac.counters.upstream_rx(
        mac.framework,
        packet_ref.info.destination,
        packet_ref.payload.len,
    );
    let mimo_rx =
        find_control(message_views, CONTROL_MIMO_RX_PROPERTIES).and_then(MimoRxProperties::decode);
    let legacy_rx =
        find_control(message_views, CONTROL_RX_PROPERTIES).and_then(RxProperties::decode);
    let decision = {
        let mut state = mac.state.lock().unwrap();
        let observation = mimo_rx
            .as_ref()
            .and_then(|mimo| {
                mimo.antenna_infos.iter().find_map(|info| {
                    info.segments.iter().find_map(|segment| {
                        state
                            .transponders
                            .iter()
                            .find(|(_, transponder)| {
                                transponder.receive_enable
                                    && transponder.receive_antenna_index
                                        == info.receive_antenna_index
                                    && transponder.receive_frequency_hz == segment.frequency_hz
                            })
                            .map(|(index, transponder)| {
                                (
                                    *index,
                                    transponder.receive_action,
                                    transponder.transmit_delay_us,
                                    segment.rx_power_dbm,
                                    info.noise_floor_dbm,
                                )
                            })
                    })
                })
            })
            .or_else(|| {
                let rx = legacy_rx?;
                state
                    .transponders
                    .iter()
                    .find(|(_, transponder)| {
                        transponder.receive_enable
                            && transponder.receive_frequency_hz == rx.frequency_hz
                            && transponder.receive_antenna_index == rx.antenna_index
                    })
                    .map(|(index, transponder)| {
                        (
                            *index,
                            transponder.receive_action,
                            transponder.transmit_delay_us,
                            rx.rx_power_dbm
                                + state
                                    .antennas
                                    .get(&transponder.receive_antenna_index)
                                    .map_or(0.0, |antenna| match antenna.pattern {
                                        AntennaPattern::IdealOmni { gain_db } => gain_db,
                                        _ => 0.0,
                                    }),
                            rx.noise_floor_dbm,
                        )
                    })
            });
        let Some((index, receive_action, transmit_delay_us, rx_power_dbm, noise_floor_dbm)) =
            observation
        else {
            return;
        };
        let Some(probability) = state.pcr.get_por(
            curve_index,
            (rx_power_dbm - noise_floor_dbm) as f32,
            packet_ref.payload.len,
        ) else {
            return;
        };
        if probability < random_unit(&mut state) {
            return;
        }
        (index, receive_action, transmit_delay_us)
    };
    let controls = message_views
        .iter()
        .filter_map(|message| {
            let bytes = if message.payload.len == 0 {
                Vec::new()
            } else if message.payload.data.is_null() || message.payload.len > 16 << 20 {
                return None;
            } else {
                unsafe { std::slice::from_raw_parts(message.payload.data, message.payload.len) }
                    .to_vec()
            };
            Some(OwnedControl {
                msg_type: message.msg_type,
                payload: bytes,
            })
        })
        .collect::<Vec<_>>();
    for component in wire.messages {
        let Ok(destination) = u16::try_from(component.destination) else {
            continue;
        };
        let accepted = match decision.1 {
            ReceiveAction::Process => destination == mac.id || destination == BROADCAST_NEM,
            ReceiveAction::Ubend => destination != mac.id,
            ReceiveAction::Unknown => false,
        };
        if !accepted {
            continue;
        }
        let info = FfiPacketInfo {
            destination,
            ..packet_ref.info
        };
        let completed = if let Some(fragment) = component.fragment {
            let mut state = mac.state.lock().unwrap();
            let now = now_us();
            let timeout = state.fragment_timeout_us;
            state
                .reassembly
                .retain(|_, entry| now.saturating_sub(entry.last_update) < timeout);
            let key = (packet_ref.info.source, fragment.sequence);
            let entry = state.reassembly.entry(key).or_insert_with(|| Reassembly {
                info,
                controls: controls.clone(),
                destination,
                parts: BTreeMap::new(),
                last_fragment_index: None,
                last_update: now,
            });
            if entry.destination != destination
                || entry
                    .parts
                    .values()
                    .any(|(index, _)| *index == fragment.index)
            {
                continue;
            }
            entry
                .parts
                .insert(fragment.offset as usize, (fragment.index, component.data));
            if !fragment.more {
                entry.last_fragment_index = Some(fragment.index);
            }
            entry.last_update = now;
            let complete = entry.last_fragment_index.is_some_and(|last| {
                entry.parts.len() == last as usize + 1
                    && (0..=last).all(|index| {
                        entry
                            .parts
                            .values()
                            .any(|(fragment_index, _)| *fragment_index == index)
                    })
                    && {
                        let mut offset = 0usize;
                        entry.parts.iter().all(|(part_offset, (_, data))| {
                            if *part_offset != offset {
                                false
                            } else {
                                offset = offset.saturating_add(data.len());
                                true
                            }
                        })
                    }
            });
            complete.then(|| {
                let entry = state.reassembly.remove(&key).expect("reassembly entry");
                let payload = entry
                    .parts
                    .into_values()
                    .flat_map(|(_, data)| data)
                    .collect();
                OwnedPacket {
                    info: entry.info,
                    payload,
                    controls: entry.controls,
                }
            })
        } else {
            Some(OwnedPacket {
                info,
                payload: component.data,
                controls: controls.clone(),
            })
        };
        let Some(packet) = completed else { continue };
        match decision.1 {
            ReceiveAction::Process => send_upstream(mac, packet),
            ReceiveAction::Ubend => {
                let now = now_us();
                let when = now.saturating_add(decision.2 as i64);
                let queued = enqueue(&mut mac.state.lock().unwrap(), decision.0, packet, when);
                if queued {
                    if when <= now {
                        drive(mac, decision.0, now);
                    } else {
                        schedule_transmit(mac, decision.0, when);
                    }
                }
            }
            ReceiveAction::Unknown => {}
        }
    }
}

extern "C" fn downstream(
    plugin: *mut c_void,
    packet: *const FfiPacket,
    messages: *const FfiControlMessage,
    count: usize,
) {
    let Some(mac) = (unsafe { (plugin as *mut BentpipeMac).as_ref() }) else {
        return;
    };
    if packet.is_null() {
        (mac.framework.send_downstream_control)(
            mac.framework.framework_ctx,
            mac.id,
            messages,
            count,
        );
        return;
    }
    let Some(packet) = own_packet(packet, messages, count) else {
        return;
    };
    mac.counters
        .downstream_rx(mac.framework, packet.info.destination, packet.payload.len());
    let index = {
        let state = mac.state.lock().unwrap();
        state
            .transponders
            .iter()
            .find(|(_, transponder)| {
                transponder.transmit_enable
                    && (transponder.all_tos || transponder.tos.contains(&packet.info.priority))
            })
            .map(|(index, _)| *index)
    };
    let Some(index) = index else { return };
    let now = now_us();
    if enqueue(&mut mac.state.lock().unwrap(), index, packet, now) {
        drive(mac, index, now);
    }
}

extern "C" fn timed(
    plugin: *mut c_void,
    timer_id: u64,
    event_id: u32,
    data: *const u8,
    len: usize,
) {
    let Some(mac) = (unsafe { (plugin as *mut BentpipeMac).as_ref() }) else {
        return;
    };
    mac.state.lock().unwrap().timers.remove(&timer_id);
    if event_id == EVENT_REASSEMBLY_CHECK {
        let mut state = mac.state.lock().unwrap();
        if !state.started {
            return;
        }
        let now = now_us();
        let timeout = state.fragment_timeout_us;
        state
            .reassembly
            .retain(|_, entry| now.saturating_sub(entry.last_update) < timeout);
        drop(state);
        schedule_reassembly_check(mac);
        return;
    }
    if event_id != EVENT_TRANSMIT || len != 10 || data.is_null() {
        return;
    }
    let data = unsafe { std::slice::from_raw_parts(data, len) };
    let index = u16::from_be_bytes(data[..2].try_into().unwrap());
    let expected = i64::from_be_bytes(data[2..].try_into().unwrap());
    {
        let mut state = mac.state.lock().unwrap();
        if state.wakeups.get(&index) != Some(&expected) {
            return;
        }
        state.wakeups.remove(&index);
    }
    drive(mac, index, now_us());
}

extern "C" fn process_event(_: *mut c_void, _: u16, _: *const u8, _: usize) {}

#[no_mangle]
pub extern "C" fn emane_plugin_create() -> *const PluginApi {
    static API: OnceLock<PluginApi> = OnceLock::new();
    API.get_or_init(|| PluginApi {
        abi_version: PLUGIN_ABI_VERSION,
        struct_size: std::mem::size_of::<PluginApi>(),
        name: c"bentpipemaclayer".as_ptr(),
        plugin_type: 1,
        init,
        configure,
        start,
        post_start,
        stop,
        destroy,
        process_upstream: upstream,
        process_downstream: downstream,
        process_timed_event: timed,
        process_event,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Capture {
        packet: Vec<u8>,
        controls: Vec<(u32, Vec<u8>)>,
    }

    static CAPTURE: Mutex<Capture> = Mutex::new(Capture {
        packet: Vec::new(),
        controls: Vec::new(),
    });
    static UPSTREAM_CAPTURE: Mutex<Vec<Vec<u8>>> = Mutex::new(Vec::new());

    extern "C" fn capture_packet(
        _: *mut c_void,
        _: u16,
        packet: *const FfiPacket,
        messages: *const FfiControlMessage,
        count: usize,
    ) {
        let packet = unsafe { &*packet };
        let mut capture = CAPTURE.lock().unwrap();
        capture.packet = if packet.payload.len == 0 {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(packet.payload.data, packet.payload.len) }.to_vec()
        };
        capture.controls = controls(messages, count)
            .unwrap()
            .iter()
            .map(|message| {
                let payload = if message.payload.len == 0 {
                    Vec::new()
                } else {
                    unsafe { std::slice::from_raw_parts(message.payload.data, message.payload.len) }
                        .to_vec()
                };
                (message.msg_type, payload)
            })
            .collect();
    }

    extern "C" fn packet_noop(
        _: *mut c_void,
        _: u16,
        _: *const FfiPacket,
        _: *const FfiControlMessage,
        _: usize,
    ) {
    }
    extern "C" fn capture_upstream_packet(
        _: *mut c_void,
        _: u16,
        packet: *const FfiPacket,
        _: *const FfiControlMessage,
        _: usize,
    ) {
        let packet = unsafe { &*packet };
        let payload = if packet.payload.len == 0 {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(packet.payload.data, packet.payload.len) }.to_vec()
        };
        UPSTREAM_CAPTURE.lock().unwrap().push(payload);
    }
    extern "C" fn control_noop(_: *mut c_void, _: u16, _: *const FfiControlMessage, _: usize) {}
    extern "C" fn schedule_noop(
        _: *mut c_void,
        _: u16,
        _: u64,
        _: u32,
        _: u32,
        _: *const u8,
        _: usize,
    ) -> u64 {
        0
    }
    extern "C" fn cancel_noop(_: *mut c_void, _: u16, _: u64) {}
    extern "C" fn log_noop(_: *mut c_void, _: u32, _: *const std::os::raw::c_char) {}
    extern "C" fn register_noop(
        _: *mut c_void,
        _: *const std::os::raw::c_char,
        _: *const std::os::raw::c_char,
        _: bool,
    ) -> u64 {
        0
    }
    extern "C" fn increment_noop(_: *mut c_void, _: u64, _: u64) -> bool {
        true
    }
    extern "C" fn neighbor_tx_noop(_: *mut c_void, _: u16, _: u64, _: u64) {}
    extern "C" fn neighbor_rx_noop(
        _: *mut c_void,
        _: u16,
        _: u64,
        _: f64,
        _: f64,
        _: u64,
        _: u64,
        _: u64,
    ) {
    }
    extern "C" fn queue_noop(_: *mut c_void, _: u16, _: u32, _: u32, _: u32, _: u64) {}
    extern "C" fn publish_noop(_: *mut c_void, _: u64, _: u64, _: u64, _: u64) {}
    extern "C" fn register_rf_noop(_: *mut c_void, _: u16) -> u64 {
        1
    }
    extern "C" fn configure_rf_noop(_: *mut c_void, _: u64, _: bool, _: bool) -> bool {
        true
    }
    extern "C" fn update_rf_noop(
        _: *mut c_void,
        _: u64,
        _: u16,
        _: u16,
        _: u64,
        _: f64,
        _: f64,
        _: f64,
        _: f64,
    ) -> bool {
        true
    }
    extern "C" fn publish_event_noop(_: *mut c_void, _: u16, _: *const u8, _: usize) -> bool {
        true
    }

    fn framework(capture: bool) -> FfiFrameworkService {
        FfiFrameworkService {
            framework_ctx: std::ptr::null_mut(),
            send_downstream_packet: if capture { capture_packet } else { packet_noop },
            send_upstream_packet: capture_upstream_packet,
            send_downstream_control: control_noop,
            send_upstream_control: control_noop,
            schedule_timed_event: schedule_noop,
            cancel_timed_event: cancel_noop,
            log: log_noop,
            register_counter: register_noop,
            increment_counter: increment_noop,
            update_neighbor_tx: neighbor_tx_noop,
            update_neighbor_rx: neighbor_rx_noop,
            update_queue_metric: queue_noop,
            publish_r2ri: publish_noop,
            register_rf_signal_table: register_rf_noop,
            configure_rf_signal_table: configure_rf_noop,
            update_rf_signal_table: update_rf_noop,
            publish_event: publish_event_noop,
        }
    }

    fn packet(destination: u16, data: &[u8]) -> OwnedPacket {
        OwnedPacket {
            info: FfiPacketInfo {
                source: 1,
                destination,
                priority: 0,
                creation_time_sec: 0,
                creation_time_usec: 0,
            },
            payload: data.to_vec(),
            controls: Vec::new(),
        }
    }

    #[test]
    fn parses_ranges_units_and_slots() {
        assert_eq!(parse_scaled_u64("2.5M"), Some(2_500_000));
        assert_eq!(parse_ranges("1-3;8").unwrap().len(), 4);
        assert_eq!(parse_slots("1-2;4").unwrap(), vec![1, 2, 4]);
        let (index, antenna) = parse_antenna("7:omni;3.5;2").unwrap();
        assert_eq!(index, 7);
        assert_eq!(antenna.pattern, AntennaPattern::IdealOmni { gain_db: 3.5 });
        assert_eq!(antenna.spectral_mask_index, 2);
        assert_eq!(
            parse_antenna("7:42;20;-5;2").unwrap().1.pattern,
            AntennaPattern::Profile(TxAntennaProfile {
                profile_id: 42,
                azimuth_degrees: 20.0,
                elevation_degrees: -5.0,
            })
        );
        assert!(parse_antenna("7:0;0;0;2").is_none());
    }

    #[test]
    fn legacy_aggregation_combines_components_and_respects_disable() {
        let mac = BentpipeMac::new(1, framework(false));
        let mut state = mac.state.into_inner().unwrap();
        state.transponders.insert(
            1,
            Transponder {
                transmit_enable: true,
                transmit_mtu_bytes: 10,
                transmit_data_rate_bps: 1_000_000,
                ..Transponder::default()
            },
        );
        assert!(enqueue(&mut state, 1, packet(2, b"abc"), 10));
        assert!(enqueue(&mut state, 1, packet(3, b"defg"), 10));
        let components = dequeue_components(&mut state, 1, 10, 10);
        assert_eq!(components.len(), 2);
        assert_eq!(components[0].packet.payload, b"abc");
        assert_eq!(components[1].packet.payload, b"defg");

        state.aggregation_enable = false;
        assert!(enqueue(&mut state, 1, packet(2, b"a"), 10));
        assert!(enqueue(&mut state, 1, packet(2, b"b"), 10));
        assert_eq!(dequeue_components(&mut state, 1, 10, 10).len(), 1);
        assert_eq!(state.queues[&1].len(), 1);
    }

    #[test]
    fn aggregation_wire_frame_preserves_profile_and_component_destinations() {
        *CAPTURE.lock().unwrap() = Capture::default();
        let mac = BentpipeMac::new(1, framework(true));
        let transponder = Transponder {
            curve_index: 4,
            transmit_frequency_hz: 2_400_000_000,
            transmit_bandwidth_hz: 1_000_000,
            transmit_data_rate_bps: 1_000_000,
            transmit_antenna_index: 7,
            transmit_power_dbm: 5.0,
            ..Transponder::default()
        };
        let component = |destination, data: &[u8], sequence| TxComponent {
            packet: packet(destination, data),
            sequence,
            offset: 0,
            fragment_index: 0,
            more: false,
            fragmented: false,
        };
        send_downstream(
            &mac,
            vec![component(2, b"one", 10), component(3, b"two", 11)],
            9,
            &transponder,
            AntennaDefinition {
                pattern: AntennaPattern::Profile(TxAntennaProfile {
                    profile_id: 42,
                    azimuth_degrees: 20.0,
                    elevation_degrees: -5.0,
                }),
                spectral_mask_index: 3,
            },
        );
        let capture = CAPTURE.lock().unwrap();
        let wire = BentPipeMessage::decode(capture.packet.as_slice()).unwrap();
        assert_eq!(wire.curve_index, 4);
        assert_eq!(wire.messages.len(), 2);
        assert_eq!(wire.messages[0].destination, 2);
        assert_eq!(wire.messages[1].destination, 3);
        let mimo = capture
            .controls
            .iter()
            .find(|(kind, _)| *kind == CONTROL_MIMO_TX_PROPERTIES)
            .and_then(|(_, payload)| MimoTxProperties::decode(payload))
            .unwrap();
        assert_eq!(
            mimo.transmit_antennas[0].pattern,
            AntennaPattern::Profile(TxAntennaProfile {
                profile_id: 42,
                azimuth_degrees: 20.0,
                elevation_degrees: -5.0,
            })
        );
    }

    #[test]
    fn fragmentation_is_reassembled_by_index_and_offset() {
        let mac = BentpipeMac::new(1, framework(false));
        let mut state = mac.state.into_inner().unwrap();
        state.transponders.insert(
            1,
            Transponder {
                transmit_enable: true,
                transmit_mtu_bytes: 4,
                transmit_data_rate_bps: 1_000_000,
                ..Transponder::default()
            },
        );
        assert!(enqueue(&mut state, 1, packet(2, b"abcdefgh"), 0));
        let first = dequeue_components(&mut state, 1, 4, 0);
        let second = dequeue_components(&mut state, 1, 4, 0);
        assert_eq!(first[0].packet.payload, b"abcd");
        assert!(first[0].more);
        assert_eq!(first[0].fragment_index, 0);
        assert_eq!(second[0].packet.payload, b"efgh");
        assert!(!second[0].more);
        assert_eq!(second[0].offset, 4);
        assert_eq!(second[0].fragment_index, 1);
    }

    #[test]
    fn aggregated_wire_frame_delivers_each_component_through_receive_path() {
        *UPSTREAM_CAPTURE.lock().unwrap() = Vec::new();
        let receiver = BentpipeMac::new(2, framework(false));
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let pcr_path = std::env::temp_dir().join(format!("emane-bentpipe-pcr-{unique}.xml"));
        std::fs::write(
            &pcr_path,
            r#"<bentpipe-model-pcr packetsize="1024"><curve index="0"><entry sinr="-10" por="0"/><entry sinr="10" por="100"/></curve></bentpipe-model-pcr>"#,
        )
        .unwrap();
        {
            let mut state = receiver.state.lock().unwrap();
            state.pcr.load(pcr_path.to_str().unwrap()).unwrap();
            state.transponders.insert(
                1,
                Transponder {
                    receive_frequency_hz: 2_400_000_000,
                    receive_bandwidth_hz: 1_000_000,
                    receive_antenna_index: 7,
                    receive_action: ReceiveAction::Process,
                    receive_enable: true,
                    ..Transponder::default()
                },
            );
        }
        let wire = BentPipeMessage {
            start_of_transmission_microseconds: 0,
            curve_index: 0,
            messages: vec![
                BentPipeComponent {
                    destination: 2,
                    data: b"one".to_vec(),
                    fragment: None,
                },
                BentPipeComponent {
                    destination: 2,
                    data: b"two".to_vec(),
                    fragment: None,
                },
            ],
        }
        .encode_to_vec();
        let header = ModelHeader {
            registration_id: MAC_REGISTRATION_BENTPIPE,
            sequence: 1,
            data_rate_bps: 1_000_000,
            category: 0,
            message_type: BENTPIPE_MESSAGE_TYPE,
            flags: 0,
        }
        .encode();
        let mimo = MimoRxProperties {
            tx_time_microseconds: 0,
            propagation_microseconds: 0,
            antenna_infos: vec![emane_plugin_api::MimoRxAntennaInfo {
                receive_antenna_index: 7,
                transmit_antenna_index: 7,
                span_microseconds: 10,
                receiver_sensitivity_dbm: -100.0,
                noise_floor_dbm: -90.0,
                segments: vec![emane_plugin_api::RxFrequencySegment {
                    frequency_hz: 2_400_000_000,
                    rx_power_dbm: -70.0,
                    duration_microseconds: 10,
                    offset_microseconds: 0,
                }],
            }],
            doppler_shifts_hz: Vec::new(),
        }
        .encode()
        .unwrap();
        let controls = [
            FfiControlMessage {
                msg_type: CONTROL_MODEL_HEADER,
                payload: FfiSlice {
                    data: header.as_ptr(),
                    len: header.len(),
                },
            },
            FfiControlMessage {
                msg_type: CONTROL_MIMO_RX_PROPERTIES,
                payload: FfiSlice {
                    data: mimo.as_ptr(),
                    len: mimo.len(),
                },
            },
        ];
        let packet = FfiPacket {
            info: FfiPacketInfo {
                source: 1,
                destination: 2,
                priority: 0,
                creation_time_sec: 0,
                creation_time_usec: 0,
            },
            payload: FfiSlice {
                data: wire.as_ptr(),
                len: wire.len(),
            },
        };
        upstream(
            &receiver as *const BentpipeMac as *mut c_void,
            &packet,
            controls.as_ptr(),
            controls.len(),
        );
        assert_eq!(
            *UPSTREAM_CAPTURE.lock().unwrap(),
            vec![b"one".to_vec(), b"two".to_vec()]
        );

        *UPSTREAM_CAPTURE.lock().unwrap() = Vec::new();
        for (index, offset, more, data) in [
            (0, 0, true, b"abc".as_slice()),
            (1, 3, false, b"def".as_slice()),
        ] {
            let wire = BentPipeMessage {
                start_of_transmission_microseconds: 0,
                curve_index: 0,
                messages: vec![BentPipeComponent {
                    destination: 2,
                    data: data.to_vec(),
                    fragment: Some(BentPipeFragment {
                        more,
                        index,
                        offset,
                        sequence: 77,
                    }),
                }],
            }
            .encode_to_vec();
            let packet = FfiPacket {
                info: FfiPacketInfo {
                    source: 1,
                    destination: 2,
                    priority: 0,
                    creation_time_sec: 0,
                    creation_time_usec: 0,
                },
                payload: FfiSlice {
                    data: wire.as_ptr(),
                    len: wire.len(),
                },
            };
            upstream(
                &receiver as *const BentpipeMac as *mut c_void,
                &packet,
                controls.as_ptr(),
                controls.len(),
            );
            if more {
                assert!(UPSTREAM_CAPTURE.lock().unwrap().is_empty());
            }
        }
        assert_eq!(*UPSTREAM_CAPTURE.lock().unwrap(), vec![b"abcdef".to_vec()]);
        std::fs::remove_file(pcr_path).unwrap();
    }
}
