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
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::ffi::{c_void, CStr};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const EVENT_TRANSMIT: u32 = 1;
const EVENT_REASSEMBLY_CHECK: u32 = 2;
const DEFAULT_QUEUE_DEPTH: usize = 256;

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
    total_length: usize,
    offset: usize,
    more: bool,
    ready_at: i64,
}

struct Reassembly {
    info: FfiPacketInfo,
    controls: Vec<OwnedControl>,
    total_length: usize,
    parts: BTreeMap<usize, Vec<u8>>,
    saw_last: bool,
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
    fragmentation_enable: bool,
    antennas: HashMap<u16, AntennaDefinition>,
    fragment_check_us: i64,
    fragment_timeout_us: i64,
    pcr_uri: String,
    pcr: PCRManager,
    sequence: u64,
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
                fragmentation_enable: true,
                antennas: HashMap::new(),
                fragment_check_us: 2_000_000,
                fragment_timeout_us: 5_000_000,
                pcr_uri: String::new(),
                pcr: PCRManager::new(),
                sequence: 0,
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
    let sequence = state.sequence;
    state.sequence = state.sequence.wrapping_add(1);
    let total = packet.payload.len();
    let queue = state.queues.entry(index).or_default();
    let mut offset = 0usize;
    loop {
        let end = total.min(offset.saturating_add(mtu.max(1)));
        let mut fragment = packet.clone();
        fragment.payload = packet.payload[offset..end].to_vec();
        while queue.len() >= state.queue_depth {
            queue.pop_front();
        }
        queue.push_back(PendingTx {
            packet: fragment,
            sequence,
            total_length: total,
            offset,
            more: end < total,
            ready_at,
        });
        if end == total {
            break;
        }
        offset = end;
    }
    true
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
    pending: PendingTx,
    transponder: &Transponder,
    antenna: AntennaDefinition,
) {
    let header = ModelHeader {
        registration_id: MAC_REGISTRATION_BENTPIPE,
        sequence: pending.sequence,
        data_rate_bps: transponder.transmit_data_rate_bps,
        category: 0,
        message_type: 0,
        flags: transponder.curve_index,
    };
    let mut header_bytes = header.encode().to_vec();
    header_bytes.extend_from_slice(&(pending.total_length as u32).to_be_bytes());
    header_bytes.extend_from_slice(&(pending.offset as u32).to_be_bytes());
    header_bytes.push(u8::from(pending.more));
    let duration = airtime(
        pending.packet.payload.len(),
        transponder.transmit_data_rate_bps,
    )
    .max(1);
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
        tx_time_microseconds: now_us(),
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
    let mut messages: Vec<_> = pending
        .packet
        .controls
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
        info: pending.packet.info,
        payload: FfiSlice {
            data: pending.packet.payload.as_ptr(),
            len: pending.packet.payload.len(),
        },
    };
    (mac.framework.send_downstream_packet)(
        mac.framework.framework_ctx,
        mac.id,
        &packet,
        messages.as_ptr(),
        messages.len(),
    );
    mac.counters.downstream_tx(
        mac.framework,
        pending.packet.info.destination,
        pending.packet.payload.len(),
    );
}

fn drive(mac: &BentpipeMac, index: u16, now: i64) {
    let (pending, transponder, antenna, next) = {
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
        let pending = if can_send {
            let pending = state.queues.get_mut(&index).unwrap().pop_front();
            if let Some(ref pending) = pending {
                state.current_eot.insert(
                    index,
                    now.saturating_add(airtime(
                        pending.packet.payload.len(),
                        transponder.transmit_data_rate_bps,
                    ) as i64),
                );
            }
            pending
        } else {
            None
        };
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
        (pending, transponder, antenna, next)
    };
    if let Some(pending) = pending {
        send_downstream(mac, pending, &transponder, antenna);
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
            "queue.aggregationenable" => return false,
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
        || header_data.len() < ModelHeader::ENCODED_LEN + 9
    {
        return;
    }
    let total = u32::from_be_bytes(header_data[24..28].try_into().unwrap()) as usize;
    let offset = u32::from_be_bytes(header_data[28..32].try_into().unwrap()) as usize;
    let more = header_data[32] != 0;
    let packet_ref = unsafe { &*packet };
    mac.counters.upstream_rx(
        mac.framework,
        packet_ref.info.destination,
        packet_ref.payload.len,
    );
    if offset
        .checked_add(packet_ref.payload.len)
        .is_none_or(|end| end > total)
        || (more && offset + packet_ref.payload.len == total)
    {
        return;
    }
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
        let probability = state
            .pcr
            .get_por(
                header.flags,
                (rx_power_dbm - noise_floor_dbm) as f32,
                packet_ref.payload.len,
            )
            .unwrap_or(1.0);
        if probability < random_unit(&mut state) {
            return;
        }
        (index, receive_action, transmit_delay_us)
    };
    let Some(fragment) = own_packet(packet, messages, count) else {
        return;
    };
    let completed = {
        let mut state = mac.state.lock().unwrap();
        let now = now_us();
        let timeout = state.fragment_timeout_us;
        state
            .reassembly
            .retain(|_, entry| now.saturating_sub(entry.last_update) < timeout);
        let key = (fragment.info.source, header.sequence);
        let entry = state.reassembly.entry(key).or_insert_with(|| Reassembly {
            info: fragment.info,
            controls: fragment.controls.clone(),
            total_length: total,
            parts: BTreeMap::new(),
            saw_last: false,
            last_update: now,
        });
        if entry.total_length != total {
            state.reassembly.remove(&key);
            return;
        }
        entry.parts.insert(offset, fragment.payload);
        entry.saw_last |= !more;
        entry.last_update = now;
        let mut next = 0usize;
        let contiguous = entry.parts.iter().all(|(offset, data)| {
            if *offset != next {
                false
            } else {
                next += data.len();
                true
            }
        });
        if entry.saw_last && contiguous && next == total {
            let entry = state.reassembly.remove(&key).unwrap();
            let mut payload = Vec::with_capacity(total);
            for part in entry.parts.values() {
                payload.extend_from_slice(part);
            }
            Some(OwnedPacket {
                info: entry.info,
                payload,
                controls: entry.controls,
            })
        } else {
            None
        }
    };
    if let Some(packet) = completed {
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
}
