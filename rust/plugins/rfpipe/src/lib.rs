mod pcr;

use emane_plugin_api::{
    FfiConfigRequest, FfiControlMessage, FfiFrameworkService, FfiPacket, FfiPacketInfo, FfiSlice,
    ModelHeader, PluginApi, RxProperties, TxProperties, CONTROL_MODEL_HEADER,
    CONTROL_RX_PROPERTIES, CONTROL_TX_PROPERTIES, MAC_REGISTRATION_RFPIPE, PLUGIN_ABI_VERSION,
};
use pcr::PcrCurve;
use std::collections::{HashMap, HashSet, VecDeque};
use std::ffi::{c_void, CStr};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const EVENT_TRANSMIT: u32 = 1;
const EVENT_RECEIVE: u32 = 2;
const QUEUE_CAPACITY: usize = 255;
const BROADCAST_NEM: u16 = u16::MAX;

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
    ready_at: i64,
    duration: u64,
    data_rate: u64,
}

struct State {
    promiscuous_mode: bool,
    data_rate_bps: u64,
    delay_microseconds: i64,
    jitter_microseconds: i64,
    flow_control_enable: bool,
    flow_control_tokens: u16,
    available_tokens: u16,
    pcr_curve_uri: String,
    pcr_curve: Option<PcrCurve>,
    sequence: u64,
    queue: VecDeque<PendingTx>,
    current_end_of_transmission: i64,
    next_transmit_wakeup: i64,
    pending_receptions: HashMap<u64, OwnedPacket>,
    next_reception_id: u64,
    timers: HashSet<u64>,
    random_state: u64,
    started: bool,
}

struct RfpipeMac {
    id: u16,
    framework: FfiFrameworkService,
    state: Mutex<State>,
}

impl RfpipeMac {
    fn new(id: u16, framework: FfiFrameworkService) -> Self {
        Self {
            id,
            framework,
            state: Mutex::new(State {
                promiscuous_mode: false,
                data_rate_bps: 1_000_000,
                delay_microseconds: 0,
                jitter_microseconds: 0,
                flow_control_enable: false,
                flow_control_tokens: 10,
                available_tokens: 10,
                pcr_curve_uri: String::new(),
                pcr_curve: None,
                sequence: 0,
                queue: VecDeque::new(),
                current_end_of_transmission: 0,
                next_transmit_wakeup: 0,
                pending_receptions: HashMap::new(),
                next_reception_id: 0,
                timers: HashSet::new(),
                random_state: 0x9E37_79B9_7F4A_7C15 ^ u64::from(id),
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

fn parse_scaled_u64(value: &str) -> Option<u64> {
    let (number, multiplier) = match value.as_bytes().last().copied() {
        Some(b'K' | b'k') => (&value[..value.len() - 1], 1_000.0),
        Some(b'M' | b'm') => (&value[..value.len() - 1], 1_000_000.0),
        Some(b'G' | b'g') => (&value[..value.len() - 1], 1_000_000_000.0),
        _ => (value, 1.0),
    };
    let number = number.parse::<f64>().ok()?;
    let scaled = number * multiplier;
    (number >= 0.0 && scaled.is_finite() && scaled <= u64::MAX as f64)
        .then_some(scaled.round() as u64)
}

fn seconds_to_microseconds(value: &str) -> Option<i64> {
    let seconds = value.parse::<f64>().ok()?;
    let microseconds = seconds * 1_000_000.0;
    (seconds >= 0.0 && microseconds.is_finite() && microseconds <= i64::MAX as f64)
        .then_some(microseconds.round() as i64)
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
        let values = if item.values.len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(item.values.data, item.values.len) }
        };
        let mut parsed = Vec::with_capacity(values.len());
        for value in values {
            if value.is_null() {
                return None;
            }
            parsed.push(unsafe { CStr::from_ptr(*value) }.to_str().ok()?.to_string());
        }
        result.push((name, parsed));
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

fn own_packet(
    packet: *const FfiPacket,
    messages: *const FfiControlMessage,
    count: usize,
) -> Option<OwnedPacket> {
    let packet = unsafe { packet.as_ref() }?;
    if (packet.payload.len != 0 && packet.payload.data.is_null()) || packet.payload.len > 64 << 20 {
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
            let payload = if message.payload.len == 0 {
                Vec::new()
            } else {
                unsafe { std::slice::from_raw_parts(message.payload.data, message.payload.len) }
                    .to_vec()
            };
            Some(OwnedControl {
                msg_type: message.msg_type,
                payload,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(OwnedPacket {
        info: packet.info,
        payload,
        controls,
    })
}

fn find_header(messages: &[FfiControlMessage]) -> Option<ModelHeader> {
    messages.iter().find_map(|message| {
        (message.msg_type == CONTROL_MODEL_HEADER
            && message.payload.len == ModelHeader::ENCODED_LEN
            && !message.payload.data.is_null())
        .then(|| unsafe { std::slice::from_raw_parts(message.payload.data, message.payload.len) })
        .and_then(ModelHeader::decode)
    })
}

fn find_rx_properties(messages: &[FfiControlMessage]) -> Option<RxProperties> {
    messages.iter().find_map(|message| {
        (message.msg_type == CONTROL_RX_PROPERTIES
            && message.payload.len == RxProperties::ENCODED_LEN
            && !message.payload.data.is_null())
        .then(|| unsafe { std::slice::from_raw_parts(message.payload.data, message.payload.len) })
        .and_then(RxProperties::decode)
    })
}

fn now_microseconds() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros(),
    )
    .unwrap_or(i64::MAX)
}

fn duration_microseconds(length: usize, data_rate: u64) -> u64 {
    if data_rate == 0 {
        0
    } else {
        (length as u64)
            .saturating_mul(8)
            .saturating_mul(1_000_000)
            .div_ceil(data_rate)
    }
}

fn random_unit(state: &mut State) -> f64 {
    let mut value = state.random_state;
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    state.random_state = value;
    (value >> 11) as f64 / ((1u64 << 53) as f64)
}

fn schedule(mac: &RfpipeMac, when: i64, event_id: u32, data: &[u8]) -> u64 {
    let when = when.max(0) as u64;
    (mac.framework.schedule_timed_event)(
        mac.framework.framework_ctx,
        mac.id,
        when / 1_000_000,
        (when % 1_000_000) as u32,
        event_id,
        data.as_ptr(),
        data.len(),
    )
}

fn schedule_transmit(mac: &RfpipeMac, when: i64) {
    let should_schedule = {
        let mut state = mac.state.lock().unwrap();
        if !state.started || (state.next_transmit_wakeup != 0 && state.next_transmit_wakeup <= when)
        {
            false
        } else {
            state.next_transmit_wakeup = when;
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

fn send_packet(mac: &RfpipeMac, packet: OwnedPacket, header: ModelHeader, duration: u64) {
    let header_bytes = header.encode();
    let tx_bytes = TxProperties {
        frequency_hz: 0,
        bandwidth_hz: 0,
        tx_power_dbm: f64::NAN,
        duration_microseconds: duration.max(1),
        offset_microseconds: 0,
        tx_time_microseconds: now_microseconds(),
        antenna_index: 0,
        spectral_mask_index: 0,
        sub_id: 0,
    }
    .encode();
    let mut message_views: Vec<_> = packet
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
    message_views.push(FfiControlMessage {
        msg_type: CONTROL_MODEL_HEADER,
        payload: FfiSlice {
            data: header_bytes.as_ptr(),
            len: header_bytes.len(),
        },
    });
    message_views.push(FfiControlMessage {
        msg_type: CONTROL_TX_PROPERTIES,
        payload: FfiSlice {
            data: tx_bytes.as_ptr(),
            len: tx_bytes.len(),
        },
    });
    let packet_view = FfiPacket {
        info: packet.info,
        payload: FfiSlice {
            data: packet.payload.as_ptr(),
            len: packet.payload.len(),
        },
    };
    (mac.framework.send_downstream_packet)(
        mac.framework.framework_ctx,
        mac.id,
        &packet_view,
        message_views.as_ptr(),
        message_views.len(),
    );
}

fn drive_transmit(mac: &RfpipeMac, now: i64) {
    let (transmission, next_wakeup) = {
        let mut state = mac.state.lock().unwrap();
        if !state.started {
            return;
        }
        let can_send = state.current_end_of_transmission <= now
            && state
                .queue
                .front()
                .is_some_and(|entry| entry.ready_at <= now);
        let transmission = if can_send {
            let pending = state.queue.pop_front().unwrap();
            if state.flow_control_enable {
                state.available_tokens = state
                    .available_tokens
                    .saturating_add(1)
                    .min(state.flow_control_tokens);
            }
            let header = ModelHeader {
                registration_id: MAC_REGISTRATION_RFPIPE,
                sequence: state.sequence,
                data_rate_bps: pending.data_rate,
                category: 0,
                message_type: 0,
                flags: 0,
            };
            state.sequence = state.sequence.wrapping_add(1);
            state.current_end_of_transmission = now.saturating_add(pending.duration as i64);
            Some((pending.packet, header, pending.duration))
        } else {
            None
        };
        let next_wakeup = state.queue.front().map(|entry| {
            entry
                .ready_at
                .max(state.current_end_of_transmission)
                .max(now)
        });
        (transmission, next_wakeup)
    };
    if let Some((packet, header, duration)) = transmission {
        send_packet(mac, packet, header, duration);
    }
    if let Some(when) = next_wakeup {
        schedule_transmit(mac, when);
    }
}

fn send_upstream(mac: &RfpipeMac, packet: OwnedPacket) {
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
    let packet_view = FfiPacket {
        info: packet.info,
        payload: FfiSlice {
            data: packet.payload.as_ptr(),
            len: packet.payload.len(),
        },
    };
    (mac.framework.send_upstream_packet)(
        mac.framework.framework_ctx,
        mac.id,
        &packet_view,
        messages.as_ptr(),
        messages.len(),
    );
}

extern "C" fn init(id: u16, framework: *const FfiFrameworkService) -> *mut c_void {
    let Some(framework) = (unsafe { framework.as_ref() }) else {
        return std::ptr::null_mut();
    };
    Box::into_raw(Box::new(RfpipeMac::new(id, *framework))).cast()
}

extern "C" fn configure(plugin: *mut c_void, request: *const c_void) -> bool {
    let Some(mac) = (unsafe { (plugin as *mut RfpipeMac).as_ref() }) else {
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
            "enablepromiscuousmode" | "promiscuousmode" => {
                let Some(value) = parse_bool(value) else {
                    return false;
                };
                state.promiscuous_mode = value;
            }
            "datarate" => {
                let Some(value) = parse_scaled_u64(value) else {
                    return false;
                };
                if value == 0 {
                    return false;
                }
                state.data_rate_bps = value;
            }
            "delay" => {
                let Some(value) = seconds_to_microseconds(value) else {
                    return false;
                };
                state.delay_microseconds = value;
            }
            "jitter" => {
                let Some(value) = seconds_to_microseconds(value) else {
                    return false;
                };
                state.jitter_microseconds = value;
            }
            "flowcontrolenable" => {
                if parse_bool(value) != Some(false) {
                    return false;
                }
                state.flow_control_enable = false;
            }
            "flowcontroltokens" => {
                let Ok(value) = value.parse::<u16>() else {
                    return false;
                };
                if value == 0 {
                    return false;
                }
                state.flow_control_tokens = value;
                state.available_tokens = value;
            }
            "pcrcurveuri" => state.pcr_curve_uri.clone_from(value),
            "radiometricenable" => {
                // R2RI publication is not part of this plugin ABI. Do not
                // claim to enable it when no metrics can be emitted.
                if parse_bool(value) != Some(false) {
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
            name if name.starts_with("rfsignaltable.") => return false,
            _ => return false,
        }
    }
    true
}

extern "C" fn start(plugin: *mut c_void) -> bool {
    let Some(mac) = (unsafe { (plugin as *mut RfpipeMac).as_ref() }) else {
        return false;
    };
    let mut state = mac.state.lock().unwrap();
    if state.pcr_curve_uri.is_empty() {
        return false;
    }
    state.pcr_curve = match PcrCurve::load(&state.pcr_curve_uri) {
        Ok(curve) => Some(curve),
        Err(_) => return false,
    };
    state.available_tokens = state.flow_control_tokens;
    state.started = true;
    true
}

extern "C" fn post_start(_: *mut c_void) {}

extern "C" fn stop(plugin: *mut c_void) {
    let Some(mac) = (unsafe { (plugin as *mut RfpipeMac).as_ref() }) else {
        return;
    };
    let timers = {
        let mut state = mac.state.lock().unwrap();
        state.started = false;
        state.queue.clear();
        state.pending_receptions.clear();
        state.next_transmit_wakeup = 0;
        state.timers.drain().collect::<Vec<_>>()
    };
    for timer in timers {
        (mac.framework.cancel_timed_event)(mac.framework.framework_ctx, mac.id, timer);
    }
}

extern "C" fn destroy(plugin: *mut c_void) {
    if !plugin.is_null() {
        unsafe { drop(Box::from_raw(plugin as *mut RfpipeMac)) };
    }
}

extern "C" fn process_upstream(
    plugin: *mut c_void,
    packet: *const FfiPacket,
    messages: *const FfiControlMessage,
    count: usize,
) {
    let Some(mac) = (unsafe { (plugin as *mut RfpipeMac).as_ref() }) else {
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
        find_header(message_views),
        find_rx_properties(message_views),
    ) else {
        return;
    };
    if header.registration_id != MAC_REGISTRATION_RFPIPE {
        return;
    }
    let packet_ref = unsafe { &*packet };
    let accepted = {
        let mut state = mac.state.lock().unwrap();
        let sinr = rx.rx_power_dbm - rx.noise_floor_dbm;
        let probability = state
            .pcr_curve
            .as_ref()
            .map_or(1.0, |curve| curve.probability(sinr, packet_ref.payload.len));
        probability >= random_unit(&mut state)
            && (state.promiscuous_mode
                || packet_ref.info.destination == mac.id
                || packet_ref.info.destination == BROADCAST_NEM)
    };
    if !accepted {
        return;
    }
    let Some(packet) = own_packet(packet, messages, count) else {
        return;
    };
    let (reception_id, when) = {
        let mut state = mac.state.lock().unwrap();
        state.next_reception_id = state.next_reception_id.wrapping_add(1).max(1);
        let reception_id = state.next_reception_id;
        state.pending_receptions.insert(reception_id, packet);
        (
            reception_id,
            now_microseconds().saturating_add(rx.duration_microseconds as i64),
        )
    };
    let timer = schedule(mac, when, EVENT_RECEIVE, &reception_id.to_be_bytes());
    if timer == 0 {
        if let Some(packet) = mac
            .state
            .lock()
            .unwrap()
            .pending_receptions
            .remove(&reception_id)
        {
            send_upstream(mac, packet);
        }
    } else {
        mac.state.lock().unwrap().timers.insert(timer);
    }
}

extern "C" fn process_downstream(
    plugin: *mut c_void,
    packet: *const FfiPacket,
    messages: *const FfiControlMessage,
    count: usize,
) {
    let Some(mac) = (unsafe { (plugin as *mut RfpipeMac).as_ref() }) else {
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
    let now = now_microseconds();
    let ready_at = {
        let mut state = mac.state.lock().unwrap();
        if !state.started || (state.flow_control_enable && state.available_tokens == 0) {
            return;
        }
        if state.flow_control_enable {
            state.available_tokens -= 1;
        }
        let data_rate = state.data_rate_bps;
        let duration = duration_microseconds(packet.payload.len(), data_rate).max(1);
        let jitter = if state.jitter_microseconds == 0 {
            0
        } else {
            ((random_unit(&mut state) - 0.5) * state.jitter_microseconds as f64).round() as i64
        };
        let ready_at = now
            .saturating_add(state.delay_microseconds)
            .saturating_add(jitter)
            .max(now);
        while state.queue.len() >= QUEUE_CAPACITY {
            state.queue.pop_front();
            if state.flow_control_enable {
                state.available_tokens = state
                    .available_tokens
                    .saturating_add(1)
                    .min(state.flow_control_tokens);
            }
        }
        state.queue.push_back(PendingTx {
            packet,
            ready_at,
            duration,
            data_rate,
        });
        ready_at.max(state.current_end_of_transmission).max(now)
    };
    if ready_at <= now {
        drive_transmit(mac, now);
    } else {
        schedule_transmit(mac, ready_at);
    }
}

extern "C" fn process_timed_event(
    plugin: *mut c_void,
    timer_id: u64,
    event_id: u32,
    data: *const u8,
    data_len: usize,
) {
    let Some(mac) = (unsafe { (plugin as *mut RfpipeMac).as_ref() }) else {
        return;
    };
    mac.state.lock().unwrap().timers.remove(&timer_id);
    if data_len != 8 || data.is_null() {
        return;
    }
    let value = u64::from_be_bytes(
        unsafe { std::slice::from_raw_parts(data, 8) }
            .try_into()
            .unwrap(),
    );
    match event_id {
        EVENT_TRANSMIT => {
            let expected = value as i64;
            {
                let mut state = mac.state.lock().unwrap();
                if state.next_transmit_wakeup != expected {
                    return;
                }
                state.next_transmit_wakeup = 0;
            }
            drive_transmit(mac, now_microseconds());
        }
        EVENT_RECEIVE => {
            let packet = mac.state.lock().unwrap().pending_receptions.remove(&value);
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
        name: c"rfpipemaclayer".as_ptr(),
        plugin_type: 1,
        init,
        configure,
        start,
        post_start,
        stop,
        destroy,
        process_upstream,
        process_downstream,
        process_timed_event,
        process_event,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_legacy_units_and_airtime() {
        assert_eq!(parse_scaled_u64("1M"), Some(1_000_000));
        assert_eq!(seconds_to_microseconds("0.25"), Some(250_000));
        assert_eq!(duration_microseconds(125, 1_000_000), 1_000);
    }
}
