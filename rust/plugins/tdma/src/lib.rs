mod pcr;

use emane_plugin_api::{
    FfiConfigRequest, FfiControlMessage, FfiFrameworkService, FfiPacket, FfiPacketInfo, FfiSlice,
    FrequencyOfInterest, ModelHeader, PluginApi, RxProperties, TxProperties,
    CONTROL_FREQUENCY_INTEREST, CONTROL_MODEL_HEADER, CONTROL_RX_PROPERTIES, CONTROL_TX_PROPERTIES,
    MAC_REGISTRATION_TDMA, PLUGIN_ABI_VERSION,
};
use pcr::PcrManager;
use prost::Message;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::ffi::{c_void, CStr};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const EVENT_TDMA_SCHEDULE: u16 = 105;
const TIMER_TX_SLOT: u32 = 1;
const TIMER_RX_COMPLETE: u32 = 2;
const TIMER_REASSEMBLY_CHECK: u32 = 3;
const BROADCAST_NEM: u16 = u16::MAX;
const HEADER_LEN: usize = ModelHeader::ENCODED_LEN + 19;
const FRAME_OVERHEAD_BYTES: usize = 64;

#[derive(Clone, PartialEq, Message)]
struct ScheduleEvent {
    #[prost(message, repeated, tag = "1")]
    frames: Vec<ScheduleFrame>,
    #[prost(message, optional, tag = "2")]
    structure: Option<ScheduleStructure>,
    #[prost(uint64, optional, tag = "3")]
    frequency_hz: Option<u64>,
    #[prost(uint64, optional, tag = "4")]
    data_rate_bps: Option<u64>,
    #[prost(uint32, optional, tag = "5")]
    service_class: Option<u32>,
    #[prost(double, optional, tag = "6")]
    power_dbm: Option<f64>,
}

#[derive(Clone, PartialEq, Message)]
struct ScheduleFrame {
    #[prost(uint32, required, tag = "1")]
    index: u32,
    #[prost(uint64, optional, tag = "2")]
    frequency_hz: Option<u64>,
    #[prost(uint64, optional, tag = "3")]
    data_rate_bps: Option<u64>,
    #[prost(uint32, optional, tag = "4")]
    service_class: Option<u32>,
    #[prost(double, optional, tag = "5")]
    power_dbm: Option<f64>,
    #[prost(message, repeated, tag = "6")]
    slots: Vec<ScheduleSlot>,
}

#[derive(Clone, PartialEq, Message)]
struct ScheduleSlot {
    #[prost(uint32, required, tag = "1")]
    index: u32,
    #[prost(enumeration = "SlotType", required, tag = "2")]
    slot_type: i32,
    #[prost(message, optional, tag = "3")]
    tx: Option<ScheduleTx>,
    #[prost(message, optional, tag = "4")]
    rx: Option<ScheduleRx>,
}

#[derive(Clone, PartialEq, Message)]
struct ScheduleTx {
    #[prost(uint64, optional, tag = "1")]
    frequency_hz: Option<u64>,
    #[prost(uint64, optional, tag = "2")]
    data_rate_bps: Option<u64>,
    #[prost(uint32, optional, tag = "3")]
    service_class: Option<u32>,
    #[prost(double, optional, tag = "4")]
    power_dbm: Option<f64>,
    #[prost(uint32, optional, tag = "5")]
    destination: Option<u32>,
}

#[derive(Clone, PartialEq, Message)]
struct ScheduleRx {
    #[prost(uint64, optional, tag = "1")]
    frequency_hz: Option<u64>,
}

#[derive(Clone, PartialEq, Message)]
struct ScheduleStructure {
    #[prost(uint32, required, tag = "1")]
    slots_per_frame: u32,
    #[prost(uint32, required, tag = "2")]
    frames_per_multiframe: u32,
    #[prost(uint64, required, tag = "3")]
    slot_duration_microseconds: u64,
    #[prost(uint64, required, tag = "4")]
    slot_overhead_microseconds: u64,
    #[prost(uint64, required, tag = "5")]
    bandwidth_hz: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, prost::Enumeration)]
#[repr(i32)]
enum SlotType {
    Tx = 1,
    Rx = 2,
    Idle = 3,
}

#[derive(Clone)]
enum Slot {
    Tx {
        frequency_hz: u64,
        data_rate_bps: u64,
        service_class: u8,
        power_dbm: f64,
        destination: u16,
    },
    Rx {
        frequency_hz: u64,
    },
    Idle,
}

#[derive(Clone)]
struct Schedule {
    slots_per_frame: u32,
    frames_per_multiframe: u32,
    slot_duration_us: u64,
    overhead_us: u64,
    bandwidth_hz: u64,
    slots: Vec<Slot>,
}

impl Schedule {
    fn cycle_slots(&self) -> u64 {
        u64::from(self.slots_per_frame) * u64::from(self.frames_per_multiframe)
    }

    fn slot(&self, absolute: u64) -> &Slot {
        &self.slots[(absolute % self.cycle_slots()) as usize]
    }

    fn absolute_slot(&self, time_us: i64) -> u64 {
        (time_us.max(0) as u64) / self.slot_duration_us
    }

    fn slot_start(&self, absolute: u64) -> i64 {
        i64::try_from(absolute.saturating_mul(self.slot_duration_us)).unwrap_or(i64::MAX)
    }

    fn next_tx(&self, after_absolute: u64) -> Option<u64> {
        (1..=self.cycle_slots())
            .map(|offset| after_absolute.saturating_add(offset))
            .find(|index| matches!(self.slot(*index), Slot::Tx { .. }))
    }
}

struct OwnedPacket {
    info: FfiPacketInfo,
    payload: Vec<u8>,
}

struct QueuedPacket {
    packet: OwnedPacket,
    sequence: u64,
    offset: usize,
    fragment_index: u16,
}

struct Reassembly {
    info: FfiPacketInfo,
    total_len: usize,
    pieces: BTreeMap<usize, Vec<u8>>,
    last_update_us: i64,
}

struct State {
    promiscuous: bool,
    flow_control_enable: bool,
    flow_control_tokens: u16,
    available_tokens: u16,
    pcr_uri: String,
    pcr: Option<PcrManager>,
    queue_depth: usize,
    aggregation_enable: bool,
    aggregation_threshold: f64,
    fragmentation_enable: bool,
    strict_dequeue: bool,
    fragment_timeout_us: i64,
    fragment_check_us: i64,
    schedule: Option<Schedule>,
    queues: [VecDeque<QueuedPacket>; 4],
    sequence: u64,
    next_tx_slot: Option<u64>,
    reassembly: HashMap<(u16, u64), Reassembly>,
    pending_rx: HashMap<u64, OwnedPacket>,
    next_rx_id: u64,
    timers: HashMap<u64, u32>,
    random_state: u64,
    started: bool,
}

struct TdmaMac {
    id: u16,
    framework: FfiFrameworkService,
    state: Mutex<State>,
}

impl TdmaMac {
    fn new(id: u16, framework: FfiFrameworkService) -> Self {
        Self {
            id,
            framework,
            state: Mutex::new(State {
                promiscuous: false,
                flow_control_enable: false,
                flow_control_tokens: 10,
                available_tokens: 10,
                pcr_uri: String::new(),
                pcr: None,
                queue_depth: 255,
                aggregation_enable: true,
                aggregation_threshold: 90.0,
                fragmentation_enable: true,
                strict_dequeue: false,
                fragment_timeout_us: 5_000_000,
                fragment_check_us: 2_000_000,
                schedule: None,
                queues: std::array::from_fn(|_| VecDeque::new()),
                sequence: 0,
                next_tx_slot: None,
                reassembly: HashMap::new(),
                pending_rx: HashMap::new(),
                next_rx_id: 0,
                timers: HashMap::new(),
                random_state: 0xD1B5_4A32_D192_ED03 ^ u64::from(id),
                started: false,
            }),
        }
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "1" | "on" | "true" | "yes" => Some(true),
        "0" | "off" | "false" | "no" => Some(false),
        _ => None,
    }
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
        let raw = if item.values.len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(item.values.data, item.values.len) }
        };
        let mut values = Vec::with_capacity(raw.len());
        for value in raw {
            if value.is_null() {
                return None;
            }
            values.push(unsafe { CStr::from_ptr(*value) }.to_str().ok()?.to_string());
        }
        result.push((name, values));
    }
    Some(result)
}

fn controls(
    messages: *const FfiControlMessage,
    count: usize,
) -> Option<&'static [FfiControlMessage]> {
    if count > 4096 || (count != 0 && messages.is_null()) {
        None
    } else if count == 0 {
        Some(&[])
    } else {
        Some(unsafe { std::slice::from_raw_parts(messages, count) })
    }
}

fn control_payload(message: &FfiControlMessage) -> Option<&'static [u8]> {
    if message.payload.len != 0 && message.payload.data.is_null() {
        None
    } else if message.payload.len == 0 {
        Some(&[])
    } else {
        Some(unsafe { std::slice::from_raw_parts(message.payload.data, message.payload.len) })
    }
}

fn own_packet(packet: *const FfiPacket) -> Option<OwnedPacket> {
    let packet = unsafe { packet.as_ref() }?;
    if packet.payload.len > 64 << 20 || (packet.payload.len != 0 && packet.payload.data.is_null()) {
        return None;
    }
    let payload = if packet.payload.len == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(packet.payload.data, packet.payload.len) }.to_vec()
    };
    Some(OwnedPacket {
        info: packet.info,
        payload,
    })
}

fn now_us() -> i64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    i64::try_from(now.as_micros()).unwrap_or(i64::MAX)
}

fn random_unit(state: &mut u64) -> f64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state as f64) / (u64::MAX as f64)
}

fn parse_schedule(data: &[u8], current: Option<&Schedule>) -> Result<(Schedule, bool), String> {
    let event =
        ScheduleEvent::decode(data).map_err(|error| format!("invalid schedule: {error}"))?;
    let full = event.structure.is_some();
    let mut schedule = if let Some(structure) = event.structure {
        if structure.slots_per_frame == 0
            || structure.frames_per_multiframe == 0
            || structure.slot_duration_microseconds == 0
            || structure.slot_overhead_microseconds >= structure.slot_duration_microseconds
            || structure.bandwidth_hz == 0
        {
            return Err("invalid TDMA slot structure".into());
        }
        let count = u64::from(structure.slots_per_frame)
            .checked_mul(u64::from(structure.frames_per_multiframe))
            .and_then(|value| usize::try_from(value).ok())
            .filter(|value| *value <= 1_000_000)
            .ok_or_else(|| "TDMA schedule is too large".to_string())?;
        Schedule {
            slots_per_frame: structure.slots_per_frame,
            frames_per_multiframe: structure.frames_per_multiframe,
            slot_duration_us: structure.slot_duration_microseconds,
            overhead_us: structure.slot_overhead_microseconds,
            bandwidth_hz: structure.bandwidth_hz,
            slots: vec![Slot::Idle; count],
        }
    } else {
        current
            .cloned()
            .ok_or_else(|| "schedule update received before full schedule".to_string())?
    };
    for frame in event.frames {
        if frame.index >= schedule.frames_per_multiframe {
            return Err("TDMA frame index is out of range".into());
        }
        let frame_frequency = frame.frequency_hz.or(event.frequency_hz);
        let frame_rate = frame.data_rate_bps.or(event.data_rate_bps);
        let frame_class = frame.service_class.or(event.service_class).unwrap_or(0);
        let frame_power = frame.power_dbm.or(event.power_dbm).unwrap_or(0.0);
        for slot in frame.slots {
            if slot.index >= schedule.slots_per_frame {
                return Err("TDMA slot index is out of range".into());
            }
            let index = (frame.index * schedule.slots_per_frame + slot.index) as usize;
            schedule.slots[index] = match SlotType::try_from(slot.slot_type).ok() {
                Some(SlotType::Tx) => {
                    let tx = slot.tx.unwrap_or_default();
                    let frequency_hz = tx.frequency_hz.or(frame_frequency).unwrap_or(0);
                    let data_rate_bps = tx.data_rate_bps.or(frame_rate).unwrap_or(0);
                    let service_class = tx.service_class.unwrap_or(frame_class);
                    let power_dbm = tx.power_dbm.unwrap_or(frame_power);
                    // A zero schedule destination means no destination filter,
                    // matching the legacy queue manager contract.
                    let destination = tx.destination.unwrap_or(0);
                    if frequency_hz == 0
                        || data_rate_bps == 0
                        || service_class > 3
                        || destination > u32::from(u16::MAX)
                        || !power_dbm.is_finite()
                    {
                        return Err("invalid TDMA transmit slot".into());
                    }
                    Slot::Tx {
                        frequency_hz,
                        data_rate_bps,
                        service_class: service_class as u8,
                        power_dbm,
                        destination: destination as u16,
                    }
                }
                Some(SlotType::Rx) => {
                    let frequency_hz = slot
                        .rx
                        .and_then(|rx| rx.frequency_hz)
                        .or(frame_frequency)
                        .unwrap_or(0);
                    if frequency_hz == 0 {
                        return Err("invalid TDMA receive slot".into());
                    }
                    Slot::Rx { frequency_hz }
                }
                Some(SlotType::Idle) => Slot::Idle,
                None => return Err("invalid TDMA slot type".into()),
            };
        }
    }
    Ok((schedule, full))
}

extern "C" fn init(id: u16, framework: *const FfiFrameworkService) -> *mut c_void {
    let Some(framework) = (unsafe { framework.as_ref() }) else {
        return std::ptr::null_mut();
    };
    Box::into_raw(Box::new(TdmaMac::new(id, *framework))).cast()
}

extern "C" fn configure(plugin: *mut c_void, request: *const c_void) -> bool {
    let Some(mac) = (unsafe { (plugin as *mut TdmaMac).as_ref() }) else {
        return false;
    };
    let Some(items) = configuration(request) else {
        return false;
    };
    let mut state = mac.state.lock().unwrap();
    for (name, values) in items {
        let Some(value) = values.first() else {
            return false;
        };
        match name.as_str() {
            "enablepromiscuousmode" => {
                let Some(v) = parse_bool(value) else {
                    return false;
                };
                state.promiscuous = v;
            }
            "flowcontrolenable" => {
                if parse_bool(value) != Some(false) {
                    return false;
                }
                state.flow_control_enable = false;
            }
            "flowcontroltokens" => {
                let Ok(v) = value.parse::<u16>() else {
                    return false;
                };
                if v == 0 {
                    return false;
                }
                state.flow_control_tokens = v;
                state.available_tokens = v;
            }
            "pcrcurveuri" => state.pcr_uri.clone_from(value),
            "fragmenttimeoutthreshold" => {
                let Ok(v) = value.parse::<u64>() else {
                    return false;
                };
                state.fragment_timeout_us =
                    i64::try_from(v.saturating_mul(1_000_000)).unwrap_or(i64::MAX);
            }
            "fragmentcheckthreshold" => {
                let Ok(v) = value.parse::<u64>() else {
                    return false;
                };
                if v == 0 {
                    return false;
                }
                state.fragment_check_us =
                    i64::try_from(v.saturating_mul(1_000_000)).unwrap_or(i64::MAX);
            }
            "neighbormetricdeletetime" | "neighbormetricupdateinterval" => {
                if value
                    .parse::<f64>()
                    .ok()
                    .filter(|v| v.is_finite() && *v >= 0.0)
                    .is_none()
                {
                    return false;
                }
            }
            "queue.depth" => {
                let Ok(v) = value.parse::<usize>() else {
                    return false;
                };
                if v == 0 || v > 65_535 {
                    return false;
                }
                state.queue_depth = v;
            }
            "queue.aggregationenable" => {
                let Some(v) = parse_bool(value) else {
                    return false;
                };
                state.aggregation_enable = v;
            }
            "queue.aggregationslotthreshold" => {
                let Ok(v) = value.parse::<f64>() else {
                    return false;
                };
                if !v.is_finite() || !(0.0..=100.0).contains(&v) {
                    return false;
                }
                state.aggregation_threshold = v;
            }
            "queue.fragmentationenable" => {
                let Some(v) = parse_bool(value) else {
                    return false;
                };
                state.fragmentation_enable = v;
            }
            "queue.strictdequeueenable" => {
                let Some(v) = parse_bool(value) else {
                    return false;
                };
                state.strict_dequeue = v;
            }
            _ => return false,
        }
    }
    true
}

extern "C" fn start(plugin: *mut c_void) -> bool {
    let Some(mac) = (unsafe { (plugin as *mut TdmaMac).as_ref() }) else {
        return false;
    };
    let mut state = mac.state.lock().unwrap();
    if state.pcr_uri.is_empty() {
        return false;
    }
    let Ok(pcr) = PcrManager::load(&state.pcr_uri) else {
        return false;
    };
    state.pcr = Some(pcr);
    state.started = true;
    drop(state);
    schedule_next_tx(mac, true);
    schedule_reassembly_check(mac);
    true
}

extern "C" fn lifecycle(_: *mut c_void) {}

extern "C" fn stop(plugin: *mut c_void) {
    let Some(mac) = (unsafe { (plugin as *mut TdmaMac).as_ref() }) else {
        return;
    };
    let timers: Vec<_> = {
        let mut state = mac.state.lock().unwrap();
        state.started = false;
        state.next_tx_slot = None;
        state.timers.drain().map(|(timer, _)| timer).collect()
    };
    for timer in timers {
        (mac.framework.cancel_timed_event)(mac.framework.framework_ctx, mac.id, timer);
    }
}

extern "C" fn destroy(plugin: *mut c_void) {
    if !plugin.is_null() {
        stop(plugin);
        unsafe { drop(Box::from_raw(plugin as *mut TdmaMac)) };
    }
}

fn schedule_timer(mac: &TdmaMac, when: i64, event: u32, data: &[u8]) -> u64 {
    let when = when.max(0) as u64;
    (mac.framework.schedule_timed_event)(
        mac.framework.framework_ctx,
        mac.id,
        when / 1_000_000,
        (when % 1_000_000) as u32,
        event,
        data.as_ptr(),
        data.len(),
    )
}

fn schedule_reassembly_check(mac: &TdmaMac) {
    let when = {
        let state = mac.state.lock().unwrap();
        if !state.started {
            return;
        }
        now_us().saturating_add(state.fragment_check_us)
    };
    let timer = schedule_timer(mac, when, TIMER_REASSEMBLY_CHECK, &0u64.to_be_bytes());
    if timer != 0 {
        mac.state
            .lock()
            .unwrap()
            .timers
            .insert(timer, TIMER_REASSEMBLY_CHECK);
    }
}

fn schedule_next_tx(mac: &TdmaMac, first_after_schedule: bool) {
    let now = now_us();
    let next = {
        let state = mac.state.lock().unwrap();
        let Some(schedule) = state.schedule.as_ref().filter(|_| state.started) else {
            return;
        };
        let current = schedule.absolute_slot(now);
        let after = if first_after_schedule {
            let cycle = schedule.cycle_slots();
            current / cycle * cycle + cycle - 1
        } else {
            current
        };
        schedule
            .next_tx(after)
            .map(|slot| (slot, schedule.slot_start(slot)))
    };
    let Some((slot, when)) = next else { return };
    let timer = schedule_timer(mac, when, TIMER_TX_SLOT, &slot.to_be_bytes());
    if timer != 0 {
        let mut state = mac.state.lock().unwrap();
        state.timers.insert(timer, TIMER_TX_SLOT);
        state.next_tx_slot = Some(slot);
    }
}

fn queue_index(state: &State, requested: u8, destination: u16) -> Option<usize> {
    let eligible = |index: usize| {
        state.queues[index]
            .iter()
            .any(|queued| destination == 0 || queued.packet.info.destination == destination)
    };
    let requested = usize::from(requested.min(3));
    if eligible(requested) {
        Some(requested)
    } else if state.strict_dequeue {
        None
    } else {
        (0..4).rev().find(|index| eligible(*index))
    }
}

#[allow(clippy::too_many_arguments)]
fn encode_header(
    sequence: u64,
    rate: u64,
    category: u8,
    absolute_slot: u64,
    offset: u32,
    total_len: u32,
    fragment_index: u16,
    more: bool,
) -> [u8; HEADER_LEN] {
    let model = ModelHeader {
        registration_id: MAC_REGISTRATION_TDMA,
        sequence,
        data_rate_bps: rate,
        category,
        message_type: 1,
        flags: u16::from(more || offset != 0),
    };
    let mut data = [0u8; HEADER_LEN];
    data[..24].copy_from_slice(&model.encode());
    data[24..32].copy_from_slice(&absolute_slot.to_be_bytes());
    data[32..36].copy_from_slice(&offset.to_be_bytes());
    data[36..40].copy_from_slice(&total_len.to_be_bytes());
    data[40..42].copy_from_slice(&fragment_index.to_be_bytes());
    data[42] = u8::from(more);
    data
}

fn decode_header(data: &[u8]) -> Option<(ModelHeader, u64, usize, usize, u16, bool)> {
    if data.len() < HEADER_LEN || data[42] > 1 {
        return None;
    }
    Some((
        ModelHeader::decode(data)?,
        u64::from_be_bytes(data[24..32].try_into().ok()?),
        u32::from_be_bytes(data[32..36].try_into().ok()?) as usize,
        u32::from_be_bytes(data[36..40].try_into().ok()?) as usize,
        u16::from_be_bytes(data[40..42].try_into().ok()?),
        data[42] != 0,
    ))
}

struct Transmission {
    info: FfiPacketInfo,
    bytes: Vec<u8>,
    sequence: u64,
    offset: usize,
    total_len: usize,
    fragment_index: u16,
    more: bool,
    frequency: u64,
    rate: u64,
    category: u8,
    power: f64,
    bandwidth: u64,
    duration: u64,
    tx_time: i64,
}

fn take_transmission(state: &mut State, absolute_slot: u64, used: usize) -> Option<Transmission> {
    let schedule = state.schedule.clone()?;
    let Slot::Tx {
        frequency_hz,
        data_rate_bps,
        service_class,
        power_dbm,
        destination,
    } = schedule.slot(absolute_slot).clone()
    else {
        return None;
    };
    let usable_us = schedule
        .slot_duration_us
        .saturating_sub(schedule.overhead_us);
    let capacity = (u128::from(data_rate_bps) * u128::from(usable_us) / 8_000_000)
        .min(usize::MAX as u128) as usize;
    let index = queue_index(state, service_class, destination)?;
    let available = capacity.saturating_sub(FRAME_OVERHEAD_BYTES + used);
    if available == 0 {
        return None;
    }
    let position = state.queues[index]
        .iter()
        .position(|queued| destination == 0 || queued.packet.info.destination == destination)?;
    let front = state.queues[index].get_mut(position)?;
    let remaining = front.packet.payload.len().saturating_sub(front.offset);
    if remaining > available && !state.fragmentation_enable {
        state.queues[index].remove(position);
        return take_transmission(state, absolute_slot, used);
    }
    let take = remaining.min(available);
    let offset = front.offset;
    let total_len = front.packet.payload.len();
    let sequence = front.sequence;
    let fragment_index = front.fragment_index;
    let info = front.packet.info;
    let bytes = front.packet.payload[offset..offset + take].to_vec();
    let more = offset + take < total_len;
    if more {
        front.offset += take;
        front.fragment_index = front.fragment_index.saturating_add(1);
    } else {
        state.queues[index].remove(position);
        if state.flow_control_enable {
            state.available_tokens = state
                .available_tokens
                .saturating_add(1)
                .min(state.flow_control_tokens);
        }
    }
    let bits = (take + FRAME_OVERHEAD_BYTES) as u128 * 8_000_000;
    let duration = bits.div_ceil(u128::from(data_rate_bps)) as u64;
    Some(Transmission {
        info,
        bytes,
        sequence,
        offset,
        total_len,
        fragment_index,
        more,
        frequency: frequency_hz,
        rate: data_rate_bps,
        category: service_class,
        power: power_dbm,
        bandwidth: schedule.bandwidth_hz,
        duration: duration.max(1),
        tx_time: schedule.slot_start(absolute_slot),
    })
}

fn transmit_slot(mac: &TdmaMac, absolute_slot: u64) {
    let mut used = 0usize;
    loop {
        let (transmission, aggregate, threshold, capacity) = {
            let mut state = mac.state.lock().unwrap();
            let Some(schedule) = state.schedule.as_ref() else {
                return;
            };
            let Slot::Tx { data_rate_bps, .. } = schedule.slot(absolute_slot) else {
                return;
            };
            let capacity = (u128::from(*data_rate_bps)
                * u128::from(
                    schedule
                        .slot_duration_us
                        .saturating_sub(schedule.overhead_us),
                )
                / 8_000_000)
                .min(usize::MAX as u128) as usize;
            let aggregate = state.aggregation_enable;
            let threshold = state.aggregation_threshold;
            let transmission = take_transmission(&mut state, absolute_slot, used);
            (transmission, aggregate, threshold, capacity)
        };
        let Some(tx_item) = transmission else {
            return;
        };
        let header = encode_header(
            tx_item.sequence,
            tx_item.rate,
            tx_item.category,
            absolute_slot,
            u32::try_from(tx_item.offset).unwrap_or(u32::MAX),
            u32::try_from(tx_item.total_len).unwrap_or(u32::MAX),
            tx_item.fragment_index,
            tx_item.more,
        );
        let phy_offset = ((used as u128 * 8_000_000) / u128::from(tx_item.rate))
            .min(u128::from(u64::MAX)) as u64;
        let tx = TxProperties {
            frequency_hz: tx_item.frequency,
            bandwidth_hz: tx_item.bandwidth,
            tx_power_dbm: tx_item.power,
            duration_microseconds: tx_item.duration,
            offset_microseconds: phy_offset,
            tx_time_microseconds: tx_item.tx_time,
            antenna_index: 0,
            spectral_mask_index: 0,
            sub_id: 1,
        };
        let tx_data = tx.encode();
        let messages = [
            FfiControlMessage {
                msg_type: CONTROL_MODEL_HEADER,
                payload: FfiSlice {
                    data: header.as_ptr(),
                    len: header.len(),
                },
            },
            FfiControlMessage {
                msg_type: CONTROL_TX_PROPERTIES,
                payload: FfiSlice {
                    data: tx_data.as_ptr(),
                    len: tx_data.len(),
                },
            },
        ];
        let packet = FfiPacket {
            info: tx_item.info,
            payload: FfiSlice {
                data: tx_item.bytes.as_ptr(),
                len: tx_item.bytes.len(),
            },
        };
        (mac.framework.send_downstream_packet)(
            mac.framework.framework_ctx,
            mac.id,
            &packet,
            messages.as_ptr(),
            messages.len(),
        );
        used = used.saturating_add(tx_item.bytes.len() + FRAME_OVERHEAD_BYTES);
        if !aggregate
            || tx_item.more
            || used >= capacity
            || used as f64 / capacity.max(1) as f64 * 100.0 >= threshold
        {
            return;
        }
    }
}

extern "C" fn downstream(
    plugin: *mut c_void,
    packet: *const FfiPacket,
    _: *const FfiControlMessage,
    _: usize,
) {
    let Some(mac) = (unsafe { (plugin as *mut TdmaMac).as_ref() }) else {
        return;
    };
    let Some(packet) = own_packet(packet) else {
        return;
    };
    let mut state = mac.state.lock().unwrap();
    if state.flow_control_enable && state.available_tokens == 0 {
        return;
    }
    let index = usize::from(packet.info.priority.min(3));
    if state.queues[index].len() >= state.queue_depth {
        return;
    }
    if state.flow_control_enable {
        state.available_tokens -= 1;
    }
    state.sequence = state.sequence.wrapping_add(1);
    let sequence = state.sequence;
    state.queues[index].push_back(QueuedPacket {
        packet,
        sequence,
        offset: 0,
        fragment_index: 0,
    });
}

fn send_upstream(mac: &TdmaMac, packet: OwnedPacket) {
    let ffi = FfiPacket {
        info: packet.info,
        payload: FfiSlice {
            data: packet.payload.as_ptr(),
            len: packet.payload.len(),
        },
    };
    (mac.framework.send_upstream_packet)(
        mac.framework.framework_ctx,
        mac.id,
        &ffi,
        std::ptr::null(),
        0,
    );
}

extern "C" fn upstream(
    plugin: *mut c_void,
    packet: *const FfiPacket,
    messages: *const FfiControlMessage,
    count: usize,
) {
    let Some(mac) = (unsafe { (plugin as *mut TdmaMac).as_ref() }) else {
        return;
    };
    let Some(packet) = own_packet(packet) else {
        return;
    };
    let Some(messages) = controls(messages, count) else {
        return;
    };
    let mut header = None;
    let mut rx = None;
    for message in messages {
        let Some(data) = control_payload(message) else {
            return;
        };
        match message.msg_type {
            CONTROL_MODEL_HEADER => header = decode_header(data),
            CONTROL_RX_PROPERTIES => rx = RxProperties::decode(data),
            _ => {}
        }
    }
    let Some((model, absolute_slot, offset, total_len, _, more)) = header else {
        return;
    };
    let Some(rx) = rx else { return };
    let mut state = mac.state.lock().unwrap();
    if model.registration_id != MAC_REGISTRATION_TDMA
        || (!state.promiscuous
            && packet.info.destination != mac.id
            && packet.info.destination != BROADCAST_NEM)
    {
        return;
    }
    let Some(schedule) = state.schedule.as_ref() else {
        return;
    };
    let Slot::Rx { frequency_hz } = schedule.slot(absolute_slot) else {
        return;
    };
    if *frequency_hz != rx.frequency_hz
        || total_len > 64 << 20
        || offset > total_len
        || offset.saturating_add(packet.payload.len()) > total_len
    {
        return;
    }
    let sinr = rx.rx_power_dbm - rx.noise_floor_dbm;
    if let Some(pcr) = &state.pcr {
        let probability = pcr.probability(model.data_rate_bps, sinr, packet.payload.len());
        if random_unit(&mut state.random_state) > probability {
            return;
        }
    }
    let now = now_us();
    let timeout = state.fragment_timeout_us;
    state
        .reassembly
        .retain(|_, value| now.saturating_sub(value.last_update_us) <= timeout);
    let complete = if offset == 0 && !more && packet.payload.len() == total_len {
        Some(packet)
    } else {
        let key = (packet.info.source, model.sequence);
        let entry = state.reassembly.entry(key).or_insert_with(|| Reassembly {
            info: packet.info,
            total_len,
            pieces: BTreeMap::new(),
            last_update_us: now,
        });
        if entry.total_len != total_len {
            state.reassembly.remove(&key);
            return;
        }
        entry.last_update_us = now;
        entry.pieces.entry(offset).or_insert(packet.payload);
        let mut expected = 0usize;
        let mut assembled = Vec::with_capacity(total_len);
        for (piece_offset, piece) in &entry.pieces {
            if *piece_offset != expected {
                break;
            }
            assembled.extend_from_slice(piece);
            expected += piece.len();
        }
        if expected == total_len {
            let info = entry.info;
            state.reassembly.remove(&key);
            Some(OwnedPacket {
                info,
                payload: assembled,
            })
        } else {
            None
        }
    };
    let Some(packet) = complete else { return };
    state.next_rx_id = state.next_rx_id.wrapping_add(1);
    let id = state.next_rx_id;
    let delivery = rx
        .tx_time_microseconds
        .saturating_add(i64::try_from(rx.propagation_microseconds).unwrap_or(i64::MAX))
        .saturating_add(i64::try_from(rx.duration_microseconds).unwrap_or(i64::MAX));
    state.pending_rx.insert(id, packet);
    drop(state);
    let timer = schedule_timer(mac, delivery.max(now), TIMER_RX_COMPLETE, &id.to_be_bytes());
    if timer == 0 {
        if let Some(packet) = mac.state.lock().unwrap().pending_rx.remove(&id) {
            send_upstream(mac, packet);
        }
    } else {
        mac.state
            .lock()
            .unwrap()
            .timers
            .insert(timer, TIMER_RX_COMPLETE);
    }
}

extern "C" fn timed(plugin: *mut c_void, timer_id: u64, event: u32, data: *const u8, len: usize) {
    let Some(mac) = (unsafe { (plugin as *mut TdmaMac).as_ref() }) else {
        return;
    };
    mac.state.lock().unwrap().timers.remove(&timer_id);
    if len != 8 || data.is_null() {
        return;
    }
    let value = u64::from_be_bytes(
        unsafe { std::slice::from_raw_parts(data, 8) }
            .try_into()
            .unwrap(),
    );
    match event {
        TIMER_TX_SLOT => {
            {
                let mut state = mac.state.lock().unwrap();
                if state.next_tx_slot != Some(value) {
                    return;
                }
                state.next_tx_slot = None;
            }
            transmit_slot(mac, value);
            schedule_next_tx(mac, false);
        }
        TIMER_RX_COMPLETE => {
            if let Some(packet) = mac.state.lock().unwrap().pending_rx.remove(&value) {
                send_upstream(mac, packet);
            }
        }
        TIMER_REASSEMBLY_CHECK => {
            let mut state = mac.state.lock().unwrap();
            if !state.started {
                return;
            }
            let now = now_us();
            let timeout = state.fragment_timeout_us;
            state
                .reassembly
                .retain(|_, entry| now.saturating_sub(entry.last_update_us) < timeout);
            drop(state);
            schedule_reassembly_check(mac);
        }
        _ => {}
    }
}

extern "C" fn process_event(plugin: *mut c_void, event: u16, data: *const u8, len: usize) {
    let Some(mac) = (unsafe { (plugin as *mut TdmaMac).as_ref() }) else {
        return;
    };
    if event != EVENT_TDMA_SCHEDULE || (len != 0 && data.is_null()) {
        return;
    }
    let bytes = if len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(data, len) }
    };
    let parsed = {
        let state = mac.state.lock().unwrap();
        parse_schedule(bytes, state.schedule.as_ref())
    };
    let Ok((schedule, full)) = parsed else { return };
    let frequencies = schedule
        .slots
        .iter()
        .filter_map(|slot| match slot {
            Slot::Tx { frequency_hz, .. } | Slot::Rx { frequency_hz } => Some(*frequency_hz),
            Slot::Idle => None,
        })
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let foi_data = FrequencyOfInterest {
        bandwidth_hz: schedule.bandwidth_hz,
        frequencies_hz: frequencies,
    }
    .encode();
    let timers: Vec<_> = {
        let mut state = mac.state.lock().unwrap();
        state.schedule = Some(schedule);
        state.next_tx_slot = None;
        if full {
            state.reassembly.clear();
        }
        let timers: Vec<_> = state
            .timers
            .iter()
            .filter_map(|(timer, kind)| (*kind == TIMER_TX_SLOT).then_some(*timer))
            .collect();
        for timer in &timers {
            state.timers.remove(timer);
        }
        timers
    };
    for timer in timers {
        (mac.framework.cancel_timed_event)(mac.framework.framework_ctx, mac.id, timer);
    }
    if let Some(foi_data) = foi_data {
        let message = FfiControlMessage {
            msg_type: CONTROL_FREQUENCY_INTEREST,
            payload: FfiSlice {
                data: foi_data.as_ptr(),
                len: foi_data.len(),
            },
        };
        (mac.framework.send_downstream_control)(mac.framework.framework_ctx, mac.id, &message, 1);
    }
    schedule_next_tx(mac, full);
}

#[no_mangle]
pub extern "C" fn emane_plugin_create() -> *const PluginApi {
    static API: OnceLock<PluginApi> = OnceLock::new();
    API.get_or_init(|| PluginApi {
        abi_version: PLUGIN_ABI_VERSION,
        struct_size: std::mem::size_of::<PluginApi>(),
        name: c"tdmaeventschedulerradiomodel".as_ptr(),
        plugin_type: 1,
        init,
        configure,
        start,
        post_start: lifecycle,
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
    fn full_schedule_and_update_are_validated() {
        let event = ScheduleEvent {
            frames: vec![ScheduleFrame {
                index: 0,
                slots: vec![ScheduleSlot {
                    index: 0,
                    slot_type: SlotType::Tx as i32,
                    tx: Some(ScheduleTx {
                        frequency_hz: Some(1_000_000),
                        data_rate_bps: Some(1_000_000),
                        ..Default::default()
                    }),
                    rx: None,
                }],
                ..Default::default()
            }],
            structure: Some(ScheduleStructure {
                slots_per_frame: 2,
                frames_per_multiframe: 1,
                slot_duration_microseconds: 1_000,
                slot_overhead_microseconds: 100,
                bandwidth_hz: 1_000_000,
            }),
            ..Default::default()
        };
        let (schedule, full) = parse_schedule(&event.encode_to_vec(), None).unwrap();
        assert!(full);
        assert!(matches!(schedule.slots[0], Slot::Tx { .. }));
        assert!(matches!(schedule.slots[1], Slot::Idle));
        assert_eq!(schedule.next_tx(0), Some(2));
    }

    #[test]
    fn model_header_extension_round_trips() {
        let bytes = encode_header(7, 2_000_000, 3, 99, 12, 40, 2, true);
        let (model, slot, offset, total, index, more) = decode_header(&bytes).unwrap();
        assert_eq!(
            (model.sequence, slot, offset, total, index, more),
            (7, 99, 12, 40, 2, true)
        );
    }
}
