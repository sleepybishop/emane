mod pcr_manager;

use emane_plugin_api::{
    FfiConfigRequest, FfiControlMessage, FfiFrameworkService, FfiPacket, FfiPacketInfo, FfiSlice,
    ModelHeader, PluginApi, RxProperties, TxProperties, CONTROL_MODEL_HEADER,
    CONTROL_RX_PROPERTIES, CONTROL_TX_PROPERTIES, MAC_REGISTRATION_IEEE80211ABG,
    PLUGIN_ABI_VERSION,
};
use pcr_manager::PCRManager;
use std::collections::{HashMap, HashSet, VecDeque};
use std::ffi::{c_void, CStr};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const EVENT_TRANSMIT: u32 = 1;
const EVENT_RECEIVE: u32 = 2;
const BROADCAST_NEM: u16 = u16::MAX;
const DATA_RATES_KBPS: [u32; 13] = [
    0, 1_000, 2_000, 5_500, 11_000, 6_000, 9_000, 12_000, 18_000, 24_000, 36_000, 48_000, 54_000,
];

#[derive(Clone)]
struct OwnedControl {
    msg_type: u32,
    payload: Vec<u8>,
}

struct OwnedPacket {
    info: FfiPacketInfo,
    payload: Vec<u8>,
    controls: Vec<OwnedControl>,
}

struct PendingTx {
    packet: OwnedPacket,
    category: usize,
    ready_at: i64,
}

#[derive(Clone, Copy)]
struct CategoryConfig {
    queue_size: usize,
    max_entry_size: usize,
    cw_min: u16,
    cw_max: u16,
    aifs_us: u64,
    retry_limit: u8,
}

struct State {
    promiscuous: bool,
    wmm: bool,
    mode: u8,
    unicast_rate_index: u8,
    multicast_rate_index: u8,
    rts_threshold: u16,
    max_distance_meters: u32,
    flow_control: bool,
    flow_tokens: u16,
    available_tokens: u16,
    categories: [CategoryConfig; 4],
    queues: [VecDeque<PendingTx>; 4],
    pcr_uri: String,
    pcr: Option<PCRManager>,
    sequence: u64,
    current_eot: i64,
    next_wakeup: i64,
    pending_rx: HashMap<u64, OwnedPacket>,
    next_rx_id: u64,
    last_sequence: HashMap<u16, u64>,
    timers: HashSet<u64>,
    random_state: u64,
    started: bool,
}

struct Ieee80211Mac {
    id: u16,
    framework: FfiFrameworkService,
    state: Mutex<State>,
}

impl Ieee80211Mac {
    fn new(id: u16, framework: FfiFrameworkService) -> Self {
        let categories = [
            CategoryConfig {
                queue_size: 255,
                max_entry_size: u16::MAX as usize,
                cw_min: 32,
                cw_max: 1024,
                aifs_us: 2,
                retry_limit: 0,
            },
            CategoryConfig {
                queue_size: 255,
                max_entry_size: u16::MAX as usize,
                cw_min: 32,
                cw_max: 1024,
                aifs_us: 2,
                retry_limit: 0,
            },
            CategoryConfig {
                queue_size: 255,
                max_entry_size: u16::MAX as usize,
                cw_min: 16,
                cw_max: 64,
                aifs_us: 2,
                retry_limit: 0,
            },
            CategoryConfig {
                queue_size: 255,
                max_entry_size: u16::MAX as usize,
                cw_min: 8,
                cw_max: 16,
                aifs_us: 1,
                retry_limit: 0,
            },
        ];
        Self {
            id,
            framework,
            state: Mutex::new(State {
                promiscuous: false,
                wmm: false,
                mode: 0,
                unicast_rate_index: 4,
                multicast_rate_index: 1,
                rts_threshold: 255,
                max_distance_meters: 1_000,
                flow_control: false,
                flow_tokens: 10,
                available_tokens: 10,
                categories,
                queues: std::array::from_fn(|_| VecDeque::new()),
                pcr_uri: String::new(),
                pcr: None,
                sequence: 0,
                current_eot: 0,
                next_wakeup: 0,
                pending_rx: HashMap::new(),
                next_rx_id: 0,
                last_sequence: HashMap::new(),
                timers: HashSet::new(),
                random_state: 0xA076_1D64_78BD_642F ^ u64::from(id),
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

fn seconds_to_us(value: &str) -> Option<u64> {
    let value = value.parse::<f64>().ok()? * 1_000_000.0;
    (value.is_finite() && value >= 0.0 && value <= u64::MAX as f64).then_some(value.round() as u64)
}

fn config_items(request: *const c_void) -> Option<Vec<(String, String)>> {
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
        if item.name.is_null() || item.values.len != 1 || item.values.data.is_null() {
            return None;
        }
        let value = unsafe { *item.values.data };
        if value.is_null() {
            return None;
        }
        result.push((
            unsafe { CStr::from_ptr(item.name) }
                .to_str()
                .ok()?
                .to_string(),
            unsafe { CStr::from_ptr(value) }.to_str().ok()?.to_string(),
        ));
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

fn mode_parameters(mode: u8) -> (u64, u64, u64) {
    match mode {
        1 => (9, 16, 20),
        3 => (20, 16, 192),
        _ => (20, 10, 192),
    }
}

fn slot_size_microseconds(mode: u8, distance_meters: u32) -> u64 {
    let (base, _, _) = mode_parameters(mode);
    let propagation = (u64::from(distance_meters) * 1_000_000) / 299_792_458;
    base.saturating_add(propagation)
}

fn dscp_to_category(dscp: u8, wmm: bool) -> usize {
    if !wmm {
        0
    } else {
        match dscp {
            8..=23 => 1,
            32..=47 => 2,
            48..=63 => 3,
            _ => 0,
        }
    }
}

fn packet_duration(mode: u8, length: usize, rate_index: u8, rts_cts: bool) -> u64 {
    let (_, sifs, preamble) = mode_parameters(mode);
    let rate = DATA_RATES_KBPS[rate_index as usize] as u64;
    let data = ((length as u64).saturating_mul(8).saturating_add(272))
        .saturating_mul(1_000)
        .saturating_add(rate - 1)
        / rate;
    let mut duration = preamble + data;
    if rts_cts {
        let control_bits = 160u64 + 112 + 112;
        duration = duration
            .saturating_add(3 * preamble)
            .saturating_add(3 * sifs)
            .saturating_add(control_bits.saturating_mul(1_000).saturating_add(rate - 1) / rate);
    }
    duration.max(1)
}

fn random_u64(state: &mut State) -> u64 {
    let mut value = state.random_state;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    state.random_state = value;
    value
}

fn random_unit(state: &mut State) -> f32 {
    (random_u64(state) >> 40) as f32 / ((1u32 << 24) as f32)
}

fn schedule(mac: &Ieee80211Mac, when: i64, event: u32, data: &[u8]) -> u64 {
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

fn schedule_transmit(mac: &Ieee80211Mac, when: i64) {
    let should_schedule = {
        let mut state = mac.state.lock().unwrap();
        if !state.started || (state.next_wakeup != 0 && state.next_wakeup <= when) {
            false
        } else {
            state.next_wakeup = when;
            true
        }
    };
    if should_schedule {
        let timer = schedule(mac, when, EVENT_TRANSMIT, &when.to_be_bytes());
        if timer != 0 {
            mac.state.lock().unwrap().timers.insert(timer);
        }
    }
}

fn send_downstream(mac: &Ieee80211Mac, pending: PendingTx, header: ModelHeader, duration: u64) {
    let header_bytes = header.encode();
    let tx_bytes = TxProperties {
        frequency_hz: 0,
        bandwidth_hz: 0,
        tx_power_dbm: f64::NAN,
        duration_microseconds: duration,
        offset_microseconds: 0,
        tx_time_microseconds: now_us(),
        antenna_index: 0,
        spectral_mask_index: 0,
        sub_id: 0,
    }
    .encode();
    let mut messages: Vec<_> = pending
        .packet
        .controls
        .iter()
        .filter(|control| {
            control.msg_type != CONTROL_MODEL_HEADER && control.msg_type != CONTROL_TX_PROPERTIES
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
}

fn drive(mac: &Ieee80211Mac, now: i64) {
    let (transmission, next) = {
        let mut state = mac.state.lock().unwrap();
        if !state.started {
            return;
        }
        let selected = state
            .queues
            .iter()
            .enumerate()
            .filter_map(|(category, queue)| {
                queue
                    .front()
                    .map(|entry| (entry.ready_at, std::cmp::Reverse(category)))
            })
            .min()
            .map(|(_, category)| category.0);
        let transmission = selected.and_then(|category| {
            if state.current_eot > now || state.queues[category].front()?.ready_at > now {
                return None;
            }
            let pending = state.queues[category].pop_front()?;
            if state.flow_control {
                state.available_tokens = state
                    .available_tokens
                    .saturating_add(1)
                    .min(state.flow_tokens);
            }
            let broadcast = pending.packet.info.destination == BROADCAST_NEM;
            let rate_index = if broadcast {
                state.multicast_rate_index
            } else {
                state.unicast_rate_index
            };
            let rts_cts = !broadcast
                && state.rts_threshold != 0
                && pending.packet.payload.len() >= state.rts_threshold as usize;
            let duration = packet_duration(
                state.mode,
                pending.packet.payload.len(),
                rate_index,
                rts_cts,
            );
            let header = ModelHeader {
                registration_id: MAC_REGISTRATION_IEEE80211ABG,
                sequence: state.sequence,
                data_rate_bps: u64::from(DATA_RATES_KBPS[rate_index as usize]) * 1_000,
                category: pending.category as u8,
                message_type: if broadcast {
                    1
                } else if rts_cts {
                    3
                } else {
                    2
                },
                flags: u16::from(rate_index),
            };
            state.sequence = state.sequence.wrapping_add(1);
            state.current_eot = now.saturating_add(duration as i64);
            Some((pending, header, duration))
        });
        let next = state
            .queues
            .iter()
            .filter_map(|queue| queue.front().map(|entry| entry.ready_at))
            .min()
            .map(|ready| ready.max(state.current_eot).max(now));
        (transmission, next)
    };
    if let Some((pending, header, duration)) = transmission {
        send_downstream(mac, pending, header, duration);
    }
    if let Some(next) = next {
        schedule_transmit(mac, next);
    }
}

fn send_upstream(mac: &Ieee80211Mac, packet: OwnedPacket) {
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
}

extern "C" fn init(id: u16, framework: *const FfiFrameworkService) -> *mut c_void {
    let Some(framework) = (unsafe { framework.as_ref() }) else {
        return std::ptr::null_mut();
    };
    Box::into_raw(Box::new(Ieee80211Mac::new(id, *framework))).cast()
}

extern "C" fn configure(plugin: *mut c_void, request: *const c_void) -> bool {
    let Some(mac) = (unsafe { (plugin as *mut Ieee80211Mac).as_ref() }) else {
        return false;
    };
    let Some(items) = config_items(request) else {
        return false;
    };
    let mut state = mac.state.lock().unwrap();
    for (name, value) in items {
        match name.as_str() {
            "enablepromiscuousmode" | "promiscuousmode" => {
                state.promiscuous = match parse_bool(&value) {
                    Some(value) => value,
                    None => return false,
                }
            }
            "wmmenable" => {
                state.wmm = match parse_bool(&value) {
                    Some(value) => value,
                    None => return false,
                }
            }
            "mode" => {
                state.mode = match value.parse::<u8>() {
                    Ok(value) if value <= 3 => value,
                    _ => return false,
                }
            }
            "unicastrate" => {
                state.unicast_rate_index = match value.parse::<u8>() {
                    Ok(value @ 1..=12) => value,
                    _ => return false,
                }
            }
            "multicastrate" => {
                state.multicast_rate_index = match value.parse::<u8>() {
                    Ok(value @ 1..=12) => value,
                    _ => return false,
                }
            }
            "rtsthreshold" => {
                state.rts_threshold = match value.parse() {
                    Ok(value) => value,
                    Err(_) => return false,
                }
            }
            "distance" => {
                state.max_distance_meters = match value.parse::<u32>() {
                    Ok(value) => value,
                    Err(_) => return false,
                }
            }
            "flowcontrolenable" => {
                if parse_bool(&value) != Some(false) {
                    return false;
                }
                state.flow_control = false;
            }
            "flowcontroltokens" => {
                state.flow_tokens = match value.parse::<u16>() {
                    Ok(value) if value != 0 => value,
                    _ => return false,
                };
                state.available_tokens = state.flow_tokens;
            }
            "pcrcurveuri" => state.pcr_uri = value,
            "channelactivityestimationtimer" | "neighbortimeout" => return false,
            "radiometricenable" => {
                if parse_bool(&value) != Some(false) {
                    return false;
                }
            }
            "radiometricreportinterval" | "neighbormetricdeletetime" => {
                if value
                    .parse::<f64>()
                    .ok()
                    .filter(|value| value.is_finite() && *value >= 0.0)
                    .is_none()
                {
                    return false;
                }
            }
            _ => {
                let Some(index) = name
                    .chars()
                    .last()
                    .and_then(|value| value.to_digit(10))
                    .map(|value| value as usize)
                else {
                    return false;
                };
                if index > 3 {
                    return false;
                }
                let prefix = &name[..name.len() - 1];
                match prefix {
                    "queuesize" => {
                        state.categories[index].queue_size = match value.parse::<usize>() {
                            Ok(value @ 1..=255) => value,
                            _ => return false,
                        }
                    }
                    "msdu" => {
                        state.categories[index].max_entry_size = match value.parse::<usize>() {
                            Ok(value @ 1..=65_535) => value,
                            _ => return false,
                        }
                    }
                    "cwmin" => {
                        state.categories[index].cw_min = match value.parse::<u16>() {
                            Ok(value) if value != 0 => value,
                            _ => return false,
                        }
                    }
                    "cwmax" => {
                        state.categories[index].cw_max = match value.parse::<u16>() {
                            Ok(value) if value != 0 => value,
                            _ => return false,
                        }
                    }
                    "aifs" => {
                        state.categories[index].aifs_us = match seconds_to_us(&value) {
                            Some(value @ 0..=255) => value,
                            _ => return false,
                        }
                    }
                    "txop" => {
                        if seconds_to_us(&value) != Some(0) {
                            return false;
                        }
                    }
                    "retrylimit" => {
                        // Retry/ACK exchange state is not represented by ABI v3.
                        // Zero explicitly disables retry and is the only truthful
                        // value this implementation can accept.
                        if value.parse::<u8>().ok() != Some(0) {
                            return false;
                        }
                        state.categories[index].retry_limit = 0;
                    }
                    _ => return false,
                }
            }
        }
    }
    if state
        .categories
        .iter()
        .any(|category| category.cw_min > category.cw_max)
    {
        return false;
    }
    let valid_rate = |rate: u8| match state.mode {
        0 | 2 => (1..=4).contains(&rate),
        1 => (5..=12).contains(&rate),
        3 => (1..=12).contains(&rate),
        _ => false,
    };
    if !valid_rate(state.unicast_rate_index) || !valid_rate(state.multicast_rate_index) {
        return false;
    }
    true
}

extern "C" fn start(plugin: *mut c_void) -> bool {
    let Some(mac) = (unsafe { (plugin as *mut Ieee80211Mac).as_ref() }) else {
        return false;
    };
    let mut state = mac.state.lock().unwrap();
    if state.pcr_uri.is_empty() {
        return false;
    }
    let mut pcr = PCRManager::new();
    if pcr.load(&state.pcr_uri).is_err()
        || !pcr.contains_rate(state.unicast_rate_index.into())
        || !pcr.contains_rate(state.multicast_rate_index.into())
    {
        return false;
    }
    state.pcr = Some(pcr);
    state.available_tokens = state.flow_tokens;
    state.started = true;
    true
}

extern "C" fn lifecycle(_: *mut c_void) {}

extern "C" fn stop(plugin: *mut c_void) {
    let Some(mac) = (unsafe { (plugin as *mut Ieee80211Mac).as_ref() }) else {
        return;
    };
    let timers = {
        let mut state = mac.state.lock().unwrap();
        state.started = false;
        for queue in &mut state.queues {
            queue.clear();
        }
        state.pending_rx.clear();
        state.next_wakeup = 0;
        state.timers.drain().collect::<Vec<_>>()
    };
    for timer in timers {
        (mac.framework.cancel_timed_event)(mac.framework.framework_ctx, mac.id, timer);
    }
}

extern "C" fn destroy(plugin: *mut c_void) {
    if !plugin.is_null() {
        unsafe { drop(Box::from_raw(plugin as *mut Ieee80211Mac)) };
    }
}

extern "C" fn process_downstream(
    plugin: *mut c_void,
    packet: *const FfiPacket,
    messages: *const FfiControlMessage,
    count: usize,
) {
    let Some(mac) = (unsafe { (plugin as *mut Ieee80211Mac).as_ref() }) else {
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
    let now = now_us();
    let ready = {
        let mut state = mac.state.lock().unwrap();
        if !state.started || (state.flow_control && state.available_tokens == 0) {
            return;
        }
        let category = dscp_to_category(packet.info.priority, state.wmm);
        let config = state.categories[category];
        if packet.payload.len() > config.max_entry_size {
            return;
        }
        if state.flow_control {
            state.available_tokens -= 1;
        }
        while state.queues[category].len() >= config.queue_size {
            state.queues[category].pop_front();
            if state.flow_control {
                state.available_tokens = state
                    .available_tokens
                    .saturating_add(1)
                    .min(state.flow_tokens);
            }
        }
        let (_, sifs, _) = mode_parameters(state.mode);
        let slot = slot_size_microseconds(state.mode, state.max_distance_meters);
        let backoff_slots = random_u64(&mut state) % u64::from(config.cw_min.max(1));
        let ready = now
            .saturating_add(config.aifs_us.saturating_mul(slot).saturating_add(sifs) as i64)
            .saturating_add(backoff_slots.saturating_mul(slot) as i64);
        state.queues[category].push_back(PendingTx {
            packet,
            category,
            ready_at: ready,
        });
        ready.max(state.current_eot)
    };
    if ready <= now {
        drive(mac, now);
    } else {
        schedule_transmit(mac, ready);
    }
}

extern "C" fn process_upstream(
    plugin: *mut c_void,
    packet: *const FfiPacket,
    messages: *const FfiControlMessage,
    count: usize,
) {
    let Some(mac) = (unsafe { (plugin as *mut Ieee80211Mac).as_ref() }) else {
        return;
    };
    if packet.is_null() {
        (mac.framework.send_upstream_control)(mac.framework.framework_ctx, mac.id, messages, count);
        return;
    }
    let Some(message_views) = controls(messages, count) else {
        return;
    };
    let (Some(header), Some(rx)) = (
        find_control(message_views, CONTROL_MODEL_HEADER).and_then(ModelHeader::decode),
        find_control(message_views, CONTROL_RX_PROPERTIES).and_then(RxProperties::decode),
    ) else {
        return;
    };
    if header.registration_id != MAC_REGISTRATION_IEEE80211ABG
        || header.flags == 0
        || header.flags > 12
    {
        return;
    }
    let packet_ref = unsafe { &*packet };
    let accepted = {
        let mut state = mac.state.lock().unwrap();
        if state.current_eot > now_us()
            || state.last_sequence.get(&packet_ref.info.source) == Some(&header.sequence)
        {
            return;
        }
        let probability = state.pcr.as_ref().map_or(1.0, |pcr| {
            pcr.get_pcr(
                (rx.rx_power_dbm - rx.noise_floor_dbm) as f32,
                packet_ref.payload.len,
                header.flags,
            )
        });
        let accepted = probability >= random_unit(&mut state)
            && (state.promiscuous
                || packet_ref.info.destination == mac.id
                || packet_ref.info.destination == BROADCAST_NEM);
        if accepted {
            state
                .last_sequence
                .insert(packet_ref.info.source, header.sequence);
        }
        accepted
    };
    if !accepted {
        return;
    }
    let Some(packet) = own_packet(packet, messages, count) else {
        return;
    };
    let (id, when) = {
        let mut state = mac.state.lock().unwrap();
        state.next_rx_id = state.next_rx_id.wrapping_add(1).max(1);
        let id = state.next_rx_id;
        state.pending_rx.insert(id, packet);
        (id, now_us().saturating_add(rx.duration_microseconds as i64))
    };
    let timer = schedule(mac, when, EVENT_RECEIVE, &id.to_be_bytes());
    if timer == 0 {
        if let Some(packet) = mac.state.lock().unwrap().pending_rx.remove(&id) {
            send_upstream(mac, packet);
        }
    } else {
        mac.state.lock().unwrap().timers.insert(timer);
    }
}

extern "C" fn timed(plugin: *mut c_void, timer_id: u64, event: u32, data: *const u8, len: usize) {
    let Some(mac) = (unsafe { (plugin as *mut Ieee80211Mac).as_ref() }) else {
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
        EVENT_TRANSMIT => {
            let expected = value as i64;
            {
                let mut state = mac.state.lock().unwrap();
                if state.next_wakeup != expected {
                    return;
                }
                state.next_wakeup = 0;
            }
            drive(mac, now_us());
        }
        EVENT_RECEIVE => {
            let packet = mac.state.lock().unwrap().pending_rx.remove(&value);
            if let Some(packet) = packet {
                send_upstream(mac, packet);
            }
        }
        _ => {}
    }
}

extern "C" fn process_event(_: *mut c_void, _: u16, _: *const u8, _: usize) {}

#[no_mangle]
pub extern "C" fn emane_plugin_create() -> *const PluginApi {
    static API: OnceLock<PluginApi> = OnceLock::new();
    API.get_or_init(|| PluginApi {
        abi_version: PLUGIN_ABI_VERSION,
        struct_size: std::mem::size_of::<PluginApi>(),
        name: c"ieee80211abgmaclayer".as_ptr(),
        plugin_type: 1,
        init,
        configure,
        start,
        post_start: lifecycle,
        stop,
        destroy,
        process_upstream,
        process_downstream,
        process_timed_event: timed,
        process_event,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timing_matches_legacy_rate_table() {
        assert_eq!(DATA_RATES_KBPS[4], 11_000);
        assert!(packet_duration(2, 1_500, 4, true) > packet_duration(2, 1_500, 4, false));
        assert_eq!(seconds_to_us("0.000002"), Some(2));
    }

    #[test]
    fn wmm_uses_the_legacy_dscp_category_ranges() {
        assert_eq!(dscp_to_category(0, true), 0);
        assert_eq!(dscp_to_category(3, true), 0);
        assert_eq!(dscp_to_category(8, true), 1);
        assert_eq!(dscp_to_category(23, true), 1);
        assert_eq!(dscp_to_category(32, true), 2);
        assert_eq!(dscp_to_category(47, true), 2);
        assert_eq!(dscp_to_category(48, true), 3);
        assert_eq!(dscp_to_category(63, true), 3);
        assert_eq!(dscp_to_category(63, false), 0);
    }
}
