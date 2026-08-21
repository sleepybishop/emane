mod pcr_manager;

use emane_plugin_api::{
    CommonLayerCounters, FfiConfigRequest, FfiControlMessage, FfiFrameworkService, FfiPacket,
    FfiPacketInfo, FfiSlice, FlowControlToken, ModelHeader, PluginApi, RxProperties, TxProperties,
    CONTROL_FLOW_CONTROL_TOKEN, CONTROL_MODEL_HEADER, CONTROL_RX_PROPERTIES, CONTROL_TX_PROPERTIES,
    MAC_REGISTRATION_IEEE80211ABG, PLUGIN_ABI_VERSION,
};
use pcr_manager::PCRManager;
use std::collections::{HashMap, HashSet, VecDeque};
use std::ffi::{c_void, CStr};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const EVENT_TRANSMIT: u32 = 1;
const EVENT_RECEIVE: u32 = 2;
const EVENT_R2RI_REPORT: u32 = 3;
const EVENT_CHANNEL_ESTIMATE: u32 = 4;
const ONE_HOP_NEIGHBORS_EVENT_ID: u16 = 104;
const BROADCAST_NEM: u16 = u16::MAX;
const MSG_TYPE_BROADCAST_DATA: u8 = 1;
const MSG_TYPE_UNICAST_DATA: u8 = 2;
const MSG_TYPE_UNICAST_RTS_CTS_DATA: u8 = 4;
const MSG_TYPE_UNICAST_CTS_CTRL: u8 = 8;
const DUPLICATE_HISTORY_SIZE: usize = 16;
const DUPLICATE_VALIDITY_US: i64 = 5_000_000;
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
    acquired_at: i64,
    txop_microseconds: u64,
    ready_at: i64,
}

#[derive(Clone, Copy, Default)]
struct NeighborActivity {
    last_activity: i64,
    utilization_microseconds: u64,
    packets: u64,
    rx_power_milliwatts: f64,
}

struct NeighborList {
    last_update: i64,
    neighbors: HashSet<u16>,
}

struct PendingRx {
    packet: OwnedPacket,
    source: u16,
    sequence: u64,
    probability: f32,
    retries: u8,
    message_type: u8,
    deliverable: bool,
    cts_required: bool,
    collided: bool,
    end_of_reception: i64,
}

#[derive(Clone, Copy)]
struct CategoryConfig {
    queue_size: usize,
    max_entry_size: usize,
    cw_min: u16,
    cw_max: u16,
    aifs_us: u64,
    retry_limit: u8,
    txop_microseconds: u64,
}

struct State {
    local_id: u16,
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
    channel_activity_interval_microseconds: u64,
    neighbor_timeout_microseconds: u64,
    channel_activity: HashMap<u16, NeighborActivity>,
    estimated_one_hop_neighbors: f64,
    estimated_two_hop_neighbors: f64,
    neighbor_lists: HashMap<u16, NeighborList>,
    channel_utilization: f64,
    average_message_duration_microseconds: u64,
    next_wakeup: i64,
    pending_rx: HashMap<u64, PendingRx>,
    next_rx_id: u64,
    radiometric_enabled: bool,
    radiometric_report_interval_microseconds: u64,
    neighbor_metric_delete_microseconds: u64,
    queue_discards: [u32; 4],
    duplicate_history: HashMap<u16, VecDeque<(u64, i64)>>,
    timers: HashSet<u64>,
    random_state: u64,
    started: bool,
}

struct Ieee80211Mac {
    id: u16,
    framework: FfiFrameworkService,
    counters: CommonLayerCounters,
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
                retry_limit: 2,
                txop_microseconds: 0,
            },
            CategoryConfig {
                queue_size: 255,
                max_entry_size: u16::MAX as usize,
                cw_min: 32,
                cw_max: 1024,
                aifs_us: 2,
                retry_limit: 2,
                txop_microseconds: 0,
            },
            CategoryConfig {
                queue_size: 255,
                max_entry_size: u16::MAX as usize,
                cw_min: 16,
                cw_max: 64,
                aifs_us: 2,
                retry_limit: 2,
                txop_microseconds: 0,
            },
            CategoryConfig {
                queue_size: 255,
                max_entry_size: u16::MAX as usize,
                cw_min: 8,
                cw_max: 16,
                aifs_us: 1,
                retry_limit: 2,
                txop_microseconds: 0,
            },
        ];
        Self {
            id,
            framework,
            counters: CommonLayerCounters::register(framework),
            state: Mutex::new(State {
                local_id: id,
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
                channel_activity_interval_microseconds: 100_000,
                neighbor_timeout_microseconds: 30_000_000,
                channel_activity: HashMap::new(),
                estimated_one_hop_neighbors: 0.0,
                estimated_two_hop_neighbors: 0.0,
                neighbor_lists: HashMap::new(),
                channel_utilization: 0.0,
                average_message_duration_microseconds: 0,
                next_wakeup: 0,
                pending_rx: HashMap::new(),
                next_rx_id: 0,
                radiometric_enabled: false,
                radiometric_report_interval_microseconds: 1_000_000,
                neighbor_metric_delete_microseconds: 60_000_000,
                queue_discards: [0; 4],
                duplicate_history: HashMap::new(),
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

fn encode_flags(rate_index: u8, retries: u8) -> u16 {
    u16::from(rate_index) | (u16::from(retries) << 8)
}

fn decode_flags(flags: u16) -> Option<(u8, u8)> {
    let rate_index = flags as u8;
    (1..=12)
        .contains(&rate_index)
        .then_some((rate_index, (flags >> 8) as u8))
}

fn is_duplicate(state: &mut State, source: u16, sequence: u64, now: i64) -> bool {
    let history = state.duplicate_history.entry(source).or_default();
    history.retain(|(_, received)| received.saturating_add(DUPLICATE_VALIDITY_US) >= now);
    if history.iter().any(|(seen, _)| *seen == sequence) {
        return true;
    }
    if history.len() == DUPLICATE_HISTORY_SIZE {
        history.pop_front();
    }
    history.push_back((sequence, now));
    false
}

fn reception_succeeds(state: &mut State, probability: f32, retries: u8, collided: bool) -> bool {
    (0..=retries).any(|attempt| (!collided || attempt != 0) && probability >= random_unit(state))
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

fn record_channel_activity(
    state: &mut State,
    source: u16,
    now: i64,
    duration_microseconds: u64,
    rx_power_dbm: Option<f64>,
) {
    let activity = state.channel_activity.entry(source).or_default();
    activity.last_activity = now;
    activity.utilization_microseconds = activity
        .utilization_microseconds
        .saturating_add(duration_microseconds);
    activity.packets = activity.packets.saturating_add(1);
    if let Some(power) = rx_power_dbm {
        activity.rx_power_milliwatts += 10.0f64.powf(power / 10.0);
    }
}

fn push_varint(data: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        data.push((value as u8 & 0x7f) | 0x80);
        value >>= 7;
    }
    data.push(value as u8);
}

fn read_varint(data: &[u8], offset: &mut usize) -> Option<u64> {
    let mut value = 0u64;
    for index in 0..10 {
        let byte = *data.get(*offset)?;
        *offset += 1;
        if index == 9 && byte > 1 {
            return None;
        }
        value |= u64::from(byte & 0x7f) << (index * 7);
        if byte & 0x80 == 0 {
            return Some(value);
        }
    }
    None
}

fn encode_one_hop_neighbors(source: u16, neighbors: impl IntoIterator<Item = u16>) -> Vec<u8> {
    let mut data = Vec::new();
    data.push(0x08);
    push_varint(&mut data, u64::from(source));
    for neighbor in neighbors {
        let mut entry = vec![0x08];
        push_varint(&mut entry, u64::from(neighbor));
        data.push(0x12);
        push_varint(&mut data, entry.len() as u64);
        data.extend_from_slice(&entry);
    }
    data
}

fn decode_one_hop_neighbors(data: &[u8]) -> Option<(u16, HashSet<u16>)> {
    let mut offset = 0usize;
    let mut source = None;
    let mut neighbors = HashSet::new();
    while offset < data.len() {
        let tag = read_varint(data, &mut offset)?;
        match tag {
            0x08 => source = Some(u16::try_from(read_varint(data, &mut offset)?).ok()?),
            0x12 => {
                let len = usize::try_from(read_varint(data, &mut offset)?).ok()?;
                let end = offset.checked_add(len)?;
                if end > data.len() {
                    return None;
                }
                let mut entry_offset = offset;
                if read_varint(data, &mut entry_offset)? != 0x08 {
                    return None;
                }
                let neighbor = u16::try_from(read_varint(data, &mut entry_offset)?).ok()?;
                if entry_offset != end {
                    return None;
                }
                neighbors.insert(neighbor);
                offset = end;
            }
            _ => return None,
        }
    }
    Some((source?, neighbors))
}

fn estimate_channel_activity(state: &mut State, now: i64) {
    if state.neighbor_timeout_microseconds != 0 {
        let timeout = i64::try_from(state.neighbor_timeout_microseconds).unwrap_or(i64::MAX);
        state
            .channel_activity
            .retain(|_, activity| now.saturating_sub(activity.last_activity) <= timeout);
        state
            .neighbor_lists
            .retain(|_, list| now.saturating_sub(list.last_update) <= timeout);
    }
    let interval = state.channel_activity_interval_microseconds.max(1) as f64;
    let total_duration = state
        .channel_activity
        .values()
        .map(|activity| activity.utilization_microseconds)
        .sum::<u64>();
    let total_packets = state
        .channel_activity
        .values()
        .map(|activity| activity.packets)
        .sum::<u64>();
    state.estimated_one_hop_neighbors = state
        .channel_activity
        .keys()
        .filter(|source| **source != state.local_id)
        .count() as f64;
    let one_hop = state
        .channel_activity
        .keys()
        .copied()
        .filter(|source| *source != state.local_id)
        .collect::<HashSet<_>>();
    let two_hop = one_hop
        .iter()
        .filter_map(|neighbor| state.neighbor_lists.get(neighbor))
        .flat_map(|list| list.neighbors.iter().copied())
        .filter(|neighbor| *neighbor != state.local_id && !one_hop.contains(neighbor))
        .collect::<HashSet<_>>();
    state.estimated_two_hop_neighbors = two_hop.len() as f64;
    state.channel_utilization = (total_duration as f64 / interval).clamp(0.0, 1.0);
    state.average_message_duration_microseconds =
        total_duration.checked_div(total_packets).unwrap_or(0);
    for activity in state.channel_activity.values_mut() {
        activity.utilization_microseconds = 0;
        activity.packets = 0;
        activity.rx_power_milliwatts = 0.0;
    }
}

fn txop_expired(acquired_at: i64, txop_microseconds: u64, now: i64) -> bool {
    txop_microseconds != 0
        && acquired_at.saturating_add(i64::try_from(txop_microseconds).unwrap_or(i64::MAX)) < now
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
    mac.counters.downstream_tx(
        mac.framework,
        pending.packet.info.destination,
        pending.packet.payload.len(),
    );
    (mac.framework.update_neighbor_tx)(
        mac.framework.framework_ctx,
        pending.packet.info.destination,
        header.data_rate_bps,
        now_us().max(0) as u64,
    );
}

fn send_cts(mac: &Ieee80211Mac, destination: u16, sequence: u64, rate_index: u8) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let pending = PendingTx {
        packet: OwnedPacket {
            info: FfiPacketInfo {
                source: mac.id,
                destination,
                priority: 0,
                creation_time_sec: now.as_secs(),
                creation_time_usec: now.subsec_micros(),
            },
            payload: Vec::new(),
            controls: Vec::new(),
        },
        category: 0,
        acquired_at: now_us(),
        txop_microseconds: 0,
        ready_at: now_us(),
    };
    send_downstream(
        mac,
        pending,
        ModelHeader {
            registration_id: MAC_REGISTRATION_IEEE80211ABG,
            sequence,
            data_rate_bps: u64::from(DATA_RATES_KBPS[rate_index as usize]) * 1_000,
            category: 0,
            message_type: MSG_TYPE_UNICAST_CTS_CTRL,
            flags: encode_flags(rate_index, 0),
        },
        1,
    );
}

fn send_flow_update(mac: &Ieee80211Mac, tokens: u16) {
    let bytes = FlowControlToken { tokens }.encode();
    let control = FfiControlMessage {
        msg_type: CONTROL_FLOW_CONTROL_TOKEN,
        payload: FfiSlice {
            data: bytes.as_ptr(),
            len: bytes.len(),
        },
    };
    (mac.framework.send_upstream_control)(mac.framework.framework_ctx, mac.id, &control, 1);
}

fn drive(mac: &Ieee80211Mac, now: i64) {
    let (transmission, next, flow_update) = {
        let mut state = mac.state.lock().unwrap();
        if !state.started {
            return;
        }
        let mut flow_update = None;
        for category in 0..state.queues.len() {
            loop {
                let expired = state.queues[category].front().is_some_and(|entry| {
                    txop_expired(entry.acquired_at, entry.txop_microseconds, now)
                });
                if !expired {
                    break;
                }
                if let Some(expired) = state.queues[category].pop_front() {
                    mac.counters
                        .downstream_drop(mac.framework, expired.packet.info.destination);
                    state.queue_discards[category] =
                        state.queue_discards[category].saturating_add(1);
                    if state.flow_control {
                        state.available_tokens = state
                            .available_tokens
                            .saturating_add(1)
                            .min(state.flow_tokens);
                        flow_update = Some(state.available_tokens);
                    }
                }
            }
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
                flow_update = Some(state.available_tokens);
            }
            let broadcast = pending.packet.info.destination == BROADCAST_NEM;
            let rate_index = if broadcast {
                state.multicast_rate_index
            } else {
                state.unicast_rate_index
            };
            let retries = if broadcast {
                0
            } else {
                state.categories[pending.category].retry_limit
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
                    MSG_TYPE_BROADCAST_DATA
                } else if rts_cts {
                    MSG_TYPE_UNICAST_RTS_CTS_DATA
                } else {
                    MSG_TYPE_UNICAST_DATA
                },
                flags: encode_flags(rate_index, retries),
            };
            state.sequence = state.sequence.wrapping_add(1);
            state.current_eot = now.saturating_add(duration as i64);
            let local_id = state.local_id;
            record_channel_activity(&mut state, local_id, now, duration, None);
            Some((pending, header, duration))
        });
        let next = state
            .queues
            .iter()
            .filter_map(|queue| queue.front().map(|entry| entry.ready_at))
            .min()
            .map(|ready| ready.max(state.current_eot).max(now));
        (transmission, next, flow_update)
    };
    if let Some((pending, header, duration)) = transmission {
        send_downstream(mac, pending, header, duration);
    }
    if let Some(tokens) = flow_update {
        send_flow_update(mac, tokens);
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
    mac.counters
        .upstream_tx(mac.framework, packet.info.destination, packet.payload.len());
}

fn complete_receive(mac: &Ieee80211Mac, id: u64) {
    let (packet, cts, dropped_destination) = {
        let mut state = mac.state.lock().unwrap();
        let Some(pending) = state.pending_rx.remove(&id) else {
            return;
        };
        if pending.message_type == MSG_TYPE_UNICAST_CTS_CTRL {
            return;
        }

        let cts = pending.cts_required.then(|| {
            let sequence = state.sequence;
            state.sequence = state.sequence.wrapping_add(1);
            state.current_eot = state.current_eot.max(now_us().saturating_add(1));
            (pending.source, sequence, state.unicast_rate_index)
        });

        let duplicate = is_duplicate(
            &mut state,
            pending.source,
            pending.sequence,
            pending.end_of_reception,
        );
        let accepted = !duplicate
            && reception_succeeds(
                &mut state,
                pending.probability,
                pending.retries,
                pending.collided,
            )
            && pending.deliverable;
        let dropped_destination = (!accepted).then_some(pending.packet.info.destination);
        (accepted.then_some(pending.packet), cts, dropped_destination)
    };
    if let Some((destination, sequence, rate_index)) = cts {
        send_cts(mac, destination, sequence, rate_index);
    }
    if let Some(packet) = packet {
        send_upstream(mac, packet);
    }
    if let Some(destination) = dropped_destination {
        mac.counters.upstream_drop(mac.framework, destination);
    }
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
                state.flow_control = match parse_bool(&value) {
                    Some(value) => value,
                    None => return false,
                };
            }
            "flowcontroltokens" => {
                state.flow_tokens = match value.parse::<u16>() {
                    Ok(value) if value != 0 => value,
                    _ => return false,
                };
                state.available_tokens = state.flow_tokens;
            }
            "pcrcurveuri" => state.pcr_uri = value,
            "channelactivityestimationtimer" => {
                state.channel_activity_interval_microseconds = match seconds_to_us(&value) {
                    Some(value @ 1_000..=1_000_000) => value,
                    _ => return false,
                };
            }
            "neighbortimeout" => {
                state.neighbor_timeout_microseconds = match seconds_to_us(&value) {
                    Some(value @ 0..=3_600_000_000) => value,
                    _ => return false,
                };
            }
            "radiometricenable" => {
                let Some(value) = parse_bool(&value) else {
                    return false;
                };
                state.radiometric_enabled = value;
            }
            "radiometricreportinterval" => {
                let Some(value) = seconds_to_us(&value) else {
                    return false;
                };
                state.radiometric_report_interval_microseconds = value.max(1);
            }
            "neighbormetricdeletetime" => {
                let Some(value) = seconds_to_us(&value) else {
                    return false;
                };
                state.neighbor_metric_delete_microseconds = value.max(1);
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
                        state.categories[index].txop_microseconds = match seconds_to_us(&value) {
                            Some(value @ 0..=1_000_000) => value,
                            _ => return false,
                        };
                    }
                    "retrylimit" => {
                        state.categories[index].retry_limit = match value.parse::<u8>() {
                            Ok(value) => value,
                            Err(_) => return false,
                        };
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
    let (flow_update, report, channel_interval) = {
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
        state.channel_activity.clear();
        state.neighbor_lists.clear();
        state.estimated_one_hop_neighbors = 0.0;
        state.estimated_two_hop_neighbors = 0.0;
        state.channel_utilization = 0.0;
        state.started = true;
        (
            state.flow_control.then_some(state.available_tokens),
            state
                .radiometric_enabled
                .then_some(state.radiometric_report_interval_microseconds),
            state.channel_activity_interval_microseconds,
        )
    };
    if let Some(tokens) = flow_update {
        send_flow_update(mac, tokens);
    }
    if let Some(interval) = report {
        let when = now_us().saturating_add(interval as i64);
        let timer = schedule(mac, when, EVENT_R2RI_REPORT, &when.to_be_bytes());
        if timer != 0 {
            mac.state.lock().unwrap().timers.insert(timer);
        }
    }
    let when = now_us().saturating_add(channel_interval as i64);
    let timer = schedule(mac, when, EVENT_CHANNEL_ESTIMATE, &when.to_be_bytes());
    if timer != 0 {
        mac.state.lock().unwrap().timers.insert(timer);
    }
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
    mac.counters
        .downstream_rx(mac.framework, packet.info.destination, packet.payload.len());
    let now = now_us();
    let (ready, category, depth, max_depth, discards) = {
        let mut state = mac.state.lock().unwrap();
        if !state.started || (state.flow_control && state.available_tokens == 0) {
            mac.counters
                .downstream_drop(mac.framework, packet.info.destination);
            return;
        }
        let category = dscp_to_category(packet.info.priority, state.wmm);
        let config = state.categories[category];
        if packet.payload.len() > config.max_entry_size {
            let update = state.flow_control.then_some(state.available_tokens);
            drop(state);
            if let Some(tokens) = update {
                send_flow_update(mac, tokens);
            }
            mac.counters
                .downstream_drop(mac.framework, packet.info.destination);
            return;
        }
        if state.flow_control {
            state.available_tokens -= 1;
        }
        while state.queues[category].len() >= config.queue_size {
            if let Some(dropped) = state.queues[category].pop_front() {
                mac.counters
                    .downstream_drop(mac.framework, dropped.packet.info.destination);
            }
            state.queue_discards[category] = state.queue_discards[category].saturating_add(1);
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
        let activity_slots = (state.channel_utilization
            * (state.estimated_one_hop_neighbors + state.estimated_two_hop_neighbors)
            * state.average_message_duration_microseconds as f64
            / slot.max(1) as f64)
            .round()
            .clamp(0.0, u64::MAX as f64) as u64;
        let ready = now
            .saturating_add(config.aifs_us.saturating_mul(slot).saturating_add(sifs) as i64)
            .saturating_add(
                backoff_slots
                    .saturating_add(activity_slots)
                    .saturating_mul(slot) as i64,
            );
        state.queues[category].push_back(PendingTx {
            packet,
            category,
            acquired_at: now,
            txop_microseconds: config.txop_microseconds,
            ready_at: ready,
        });
        (
            ready.max(state.current_eot),
            category,
            state.queues[category].len(),
            config.queue_size,
            state.queue_discards[category],
        )
    };
    (mac.framework.update_queue_metric)(
        mac.framework.framework_ctx,
        category as u16,
        u32::try_from(max_depth).unwrap_or(u32::MAX),
        u32::try_from(depth).unwrap_or(u32::MAX),
        discards,
        0,
    );
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
    let Some((rate_index, retries)) = decode_flags(header.flags) else {
        return;
    };
    if header.registration_id != MAC_REGISTRATION_IEEE80211ABG
        || header.category > 3
        || !matches!(
            header.message_type,
            MSG_TYPE_BROADCAST_DATA
                | MSG_TYPE_UNICAST_DATA
                | MSG_TYPE_UNICAST_RTS_CTS_DATA
                | MSG_TYPE_UNICAST_CTS_CTRL
        )
    {
        return;
    }
    let Some(packet) = own_packet(packet, messages, count) else {
        return;
    };
    mac.counters
        .upstream_rx(mac.framework, packet.info.destination, packet.payload.len());
    let now = now_us();
    (mac.framework.update_neighbor_rx)(
        mac.framework.framework_ctx,
        packet.info.source,
        header.sequence,
        rx.rx_power_dbm - rx.noise_floor_dbm,
        rx.noise_floor_dbm,
        now.max(0) as u64,
        rx.duration_microseconds,
        header.data_rate_bps,
    );
    let (id, when) = {
        let mut state = mac.state.lock().unwrap();
        record_channel_activity(
            &mut state,
            packet.info.source,
            now,
            rx.duration_microseconds,
            Some(rx.rx_power_dbm),
        );
        if !state.started || state.current_eot > now {
            return;
        }
        let probability = state.pcr.as_ref().map_or(1.0, |pcr| {
            pcr.get_pcr(
                (rx.rx_power_dbm - rx.noise_floor_dbm) as f32,
                packet.payload.len(),
                rate_index.into(),
            )
        });
        let end_of_reception = now.saturating_add(rx.duration_microseconds as i64);
        let mut collided = false;
        for other in state.pending_rx.values_mut() {
            if other.end_of_reception > now {
                other.collided = true;
                collided = true;
            }
        }
        state.next_rx_id = state.next_rx_id.wrapping_add(1).max(1);
        let id = state.next_rx_id;
        let destination = packet.info.destination;
        let deliverable =
            state.promiscuous || destination == mac.id || destination == BROADCAST_NEM;
        state.pending_rx.insert(
            id,
            PendingRx {
                source: packet.info.source,
                packet,
                sequence: header.sequence,
                probability,
                retries,
                message_type: header.message_type,
                deliverable,
                cts_required: header.message_type == MSG_TYPE_UNICAST_RTS_CTS_DATA
                    && destination == mac.id,
                collided,
                end_of_reception,
            },
        );
        (id, end_of_reception)
    };
    let timer = schedule(mac, when, EVENT_RECEIVE, &id.to_be_bytes());
    if timer == 0 {
        complete_receive(mac, id);
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
            complete_receive(mac, value);
        }
        EVENT_R2RI_REPORT => {
            let report = {
                let state = mac.state.lock().unwrap();
                (state.started && state.radiometric_enabled).then_some((
                    u64::from(DATA_RATES_KBPS[state.multicast_rate_index as usize]) * 1_000,
                    u64::from(
                        DATA_RATES_KBPS[state.unicast_rate_index as usize]
                            .max(DATA_RATES_KBPS[state.multicast_rate_index as usize]),
                    ) * 1_000,
                    state.radiometric_report_interval_microseconds,
                    state.neighbor_metric_delete_microseconds,
                ))
            };
            if let Some((broadcast_rate, max_rate, interval, delete_time)) = report {
                (mac.framework.publish_r2ri)(
                    mac.framework.framework_ctx,
                    broadcast_rate,
                    max_rate,
                    interval,
                    delete_time,
                );
                let when = now_us().saturating_add(interval as i64);
                let timer = schedule(mac, when, EVENT_R2RI_REPORT, &when.to_be_bytes());
                if timer != 0 {
                    mac.state.lock().unwrap().timers.insert(timer);
                }
            }
        }
        EVENT_CHANNEL_ESTIMATE => {
            let (interval, event_data) = {
                let mut state = mac.state.lock().unwrap();
                if !state.started {
                    return;
                }
                estimate_channel_activity(&mut state, now_us());
                let mut neighbors = state
                    .channel_activity
                    .keys()
                    .copied()
                    .filter(|source| *source != state.local_id)
                    .collect::<Vec<_>>();
                neighbors.sort_unstable();
                (
                    state.channel_activity_interval_microseconds,
                    encode_one_hop_neighbors(mac.id, neighbors),
                )
            };
            (mac.framework.publish_event)(
                mac.framework.framework_ctx,
                ONE_HOP_NEIGHBORS_EVENT_ID,
                event_data.as_ptr(),
                event_data.len(),
            );
            let when = now_us().saturating_add(interval as i64);
            let timer = schedule(mac, when, EVENT_CHANNEL_ESTIMATE, &when.to_be_bytes());
            if timer != 0 {
                mac.state.lock().unwrap().timers.insert(timer);
            }
        }
        _ => {}
    }
}

extern "C" fn process_event(plugin: *mut c_void, event_id: u16, data: *const u8, len: usize) {
    let Some(mac) = (unsafe { (plugin as *mut Ieee80211Mac).as_ref() }) else {
        return;
    };
    if event_id != ONE_HOP_NEIGHBORS_EVENT_ID || len > 1 << 20 || (len != 0 && data.is_null()) {
        return;
    }
    let bytes = if len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(data, len) }
    };
    let Some((source, neighbors)) = decode_one_hop_neighbors(bytes) else {
        return;
    };
    if source == mac.id || source == 0 {
        return;
    }
    mac.state.lock().unwrap().neighbor_lists.insert(
        source,
        NeighborList {
            last_update: now_us(),
            neighbors,
        },
    );
}

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
    use std::sync::Mutex;

    static EVENT_CAPTURE: Mutex<(u16, Vec<u8>)> = Mutex::new((0, Vec::new()));

    extern "C" fn packet_callback(
        _: *mut c_void,
        _: u16,
        _: *const FfiPacket,
        _: *const FfiControlMessage,
        _: usize,
    ) {
    }

    extern "C" fn control_callback(_: *mut c_void, _: u16, _: *const FfiControlMessage, _: usize) {}

    extern "C" fn schedule_callback(
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

    extern "C" fn cancel_callback(_: *mut c_void, _: u16, _: u64) {}

    extern "C" fn log_callback(_: *mut c_void, _: u32, _: *const std::ffi::c_char) {}

    extern "C" fn register_callback(
        _: *mut c_void,
        _: *const std::ffi::c_char,
        _: *const std::ffi::c_char,
        _: bool,
    ) -> u64 {
        0
    }

    extern "C" fn increment_callback(_: *mut c_void, _: u64, _: u64) -> bool {
        false
    }

    extern "C" fn neighbor_tx_callback(_: *mut c_void, _: u16, _: u64, _: u64) {}
    extern "C" fn neighbor_rx_callback(
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
    extern "C" fn queue_callback(_: *mut c_void, _: u16, _: u32, _: u32, _: u32, _: u64) {}
    extern "C" fn publish_callback(_: *mut c_void, _: u64, _: u64, _: u64, _: u64) {}
    extern "C" fn register_rf_callback(_: *mut c_void, _: u16) -> u64 {
        1
    }
    extern "C" fn configure_rf_callback(_: *mut c_void, _: u64, _: bool, _: bool) -> bool {
        true
    }
    extern "C" fn update_rf_callback(
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
    extern "C" fn publish_event_callback(_: *mut c_void, _: u16, _: *const u8, _: usize) -> bool {
        true
    }

    extern "C" fn capture_event_callback(
        _: *mut c_void,
        event_id: u16,
        data: *const u8,
        len: usize,
    ) -> bool {
        let data = if len == 0 {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(data, len) }.to_vec()
        };
        *EVENT_CAPTURE.lock().unwrap() = (event_id, data);
        true
    }

    fn test_mac() -> Ieee80211Mac {
        Ieee80211Mac::new(
            1,
            FfiFrameworkService {
                framework_ctx: std::ptr::null_mut(),
                send_downstream_packet: packet_callback,
                send_upstream_packet: packet_callback,
                send_downstream_control: control_callback,
                send_upstream_control: control_callback,
                schedule_timed_event: schedule_callback,
                cancel_timed_event: cancel_callback,
                log: log_callback,
                register_counter: register_callback,
                increment_counter: increment_callback,
                update_neighbor_tx: neighbor_tx_callback,
                update_neighbor_rx: neighbor_rx_callback,
                update_queue_metric: queue_callback,
                publish_r2ri: publish_callback,
                register_rf_signal_table: register_rf_callback,
                configure_rf_signal_table: configure_rf_callback,
                update_rf_signal_table: update_rf_callback,
                publish_event: publish_event_callback,
            },
        )
    }

    fn test_mac_with_event_capture(id: u16) -> Ieee80211Mac {
        let mut mac = test_mac();
        mac.id = id;
        mac.framework.publish_event = capture_event_callback;
        mac.state.get_mut().unwrap().local_id = id;
        mac
    }

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

    #[test]
    fn native_header_preserves_legacy_rate_and_retry_fields() {
        assert_eq!(decode_flags(encode_flags(4, 2)), Some((4, 2)));
        assert_eq!(decode_flags(encode_flags(12, u8::MAX)), Some((12, u8::MAX)));
        assert_eq!(decode_flags(encode_flags(0, 2)), None);
        assert_eq!(MSG_TYPE_UNICAST_RTS_CTS_DATA, 4);
        assert_eq!(MSG_TYPE_UNICAST_CTS_CTRL, 8);
    }

    #[test]
    fn retry_can_recover_from_an_initial_collision() {
        let mac = test_mac();
        let mut state = mac.state.into_inner().unwrap();
        assert!(!reception_succeeds(&mut state, 1.0, 0, true));
        assert!(reception_succeeds(&mut state, 1.0, 1, true));
    }

    #[test]
    fn txop_matches_legacy_strict_queue_lifetime_boundary() {
        assert!(!txop_expired(1_000, 0, i64::MAX));
        assert!(!txop_expired(1_000, 500, 1_500));
        assert!(txop_expired(1_000, 500, 1_501));
    }

    #[test]
    fn drive_discards_a_packet_after_its_nonzero_txop() {
        let mac = test_mac();
        {
            let mut state = mac.state.lock().unwrap();
            state.started = true;
            state.queues[0].push_back(PendingTx {
                packet: OwnedPacket {
                    info: FfiPacketInfo {
                        source: 1,
                        destination: 2,
                        priority: 0,
                        creation_time_sec: 0,
                        creation_time_usec: 0,
                    },
                    payload: vec![1, 2, 3],
                    controls: Vec::new(),
                },
                category: 0,
                acquired_at: 1_000,
                txop_microseconds: 500,
                ready_at: i64::MAX,
            });
        }
        drive(&mac, 1_501);
        let state = mac.state.lock().unwrap();
        assert!(state.queues[0].is_empty());
        assert_eq!(state.queue_discards[0], 1);
    }

    #[test]
    fn channel_estimation_rolls_activity_and_expires_neighbors() {
        let mac = test_mac();
        let mut state = mac.state.lock().unwrap();
        state.channel_activity_interval_microseconds = 100;
        state.neighbor_timeout_microseconds = 200;
        record_channel_activity(&mut state, 2, 900, 25, Some(-50.0));
        record_channel_activity(&mut state, 3, 950, 50, Some(-60.0));
        state.neighbor_lists.insert(
            2,
            NeighborList {
                last_update: 950,
                neighbors: HashSet::from([1, 3, 4]),
            },
        );
        estimate_channel_activity(&mut state, 1_000);
        assert_eq!(state.estimated_one_hop_neighbors, 2.0);
        assert_eq!(state.estimated_two_hop_neighbors, 1.0);
        assert_eq!(state.channel_utilization, 0.75);
        assert_eq!(state.average_message_duration_microseconds, 37);
        assert!(state
            .channel_activity
            .values()
            .all(|activity| activity.packets == 0));
        estimate_channel_activity(&mut state, 1_201);
        assert!(state.channel_activity.is_empty());
        assert_eq!(state.estimated_one_hop_neighbors, 0.0);
        assert_eq!(state.estimated_two_hop_neighbors, 0.0);
    }

    #[test]
    fn one_hop_neighbor_event_matches_legacy_protobuf_shape() {
        let data = encode_one_hop_neighbors(7, [2, 9]);
        assert_eq!(
            decode_one_hop_neighbors(&data),
            Some((7, HashSet::from([2, 9])))
        );
        assert!(decode_one_hop_neighbors(&[0x12, 0xff]).is_none());
    }

    #[test]
    fn channel_estimation_event_is_published_and_consumed_by_a_peer() {
        *EVENT_CAPTURE.lock().unwrap() = (0, Vec::new());
        let source = test_mac_with_event_capture(7);
        {
            let mut state = source.state.lock().unwrap();
            state.started = true;
            state.channel_activity_interval_microseconds = 100;
            record_channel_activity(&mut state, 9, now_us(), 25, Some(-50.0));
        }
        let timer_data = now_us().to_be_bytes();
        timed(
            &source as *const Ieee80211Mac as *mut c_void,
            0,
            EVENT_CHANNEL_ESTIMATE,
            timer_data.as_ptr(),
            timer_data.len(),
        );
        let (event_id, payload) = EVENT_CAPTURE.lock().unwrap().clone();
        assert_eq!(event_id, ONE_HOP_NEIGHBORS_EVENT_ID);
        assert_eq!(
            decode_one_hop_neighbors(&payload),
            Some((7, HashSet::from([9])))
        );

        let peer = test_mac_with_event_capture(8);
        process_event(
            &peer as *const Ieee80211Mac as *mut c_void,
            event_id,
            payload.as_ptr(),
            payload.len(),
        );
        let peer_state = peer.state.lock().unwrap();
        assert_eq!(peer_state.neighbor_lists[&7].neighbors, HashSet::from([9]));
    }

    #[test]
    fn duplicate_history_matches_legacy_size_and_validity_window() {
        let mac = test_mac();
        let mut state = mac.state.into_inner().unwrap();
        assert!(!is_duplicate(&mut state, 2, 7, 10));
        assert!(is_duplicate(&mut state, 2, 7, 11));
        assert!(!is_duplicate(
            &mut state,
            2,
            7,
            10 + DUPLICATE_VALIDITY_US + 1
        ));
    }
}
