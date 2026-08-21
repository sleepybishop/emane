use crate::build_id_service::{emane_rs_buildid_register_layer, unregister_native_layer};
use crate::event_service::{
    emane_rs_event_service_register_event, emane_rs_event_service_send_event,
    register_native_user as register_event_user, unregister_user as unregister_event_user,
};
use crate::neighbor_metric_manager::NeighborMetricManager;
use crate::ota_manager::{
    emane_rs_ota_manager_send_ota_packet, register_native_user, unregister_native_user,
};
use crate::plugin_interface::{
    AntennaPattern, CommonLayerCounters, FfiConfigItem, FfiConfigRequest, FfiConfigStringArray,
    FfiControlMessage, FfiFrameworkService, FfiPacket, FfiPacketInfo, FfiSlice, FlowControlToken,
    FrequencyOfInterest, MimoRxAntennaInfo, MimoRxProperties, MimoTxAntenna,
    MimoTxFrequencySegment, MimoTxProperties, PluginApi, PluginEntryFunc, R2riNeighborMetric,
    R2riNeighborMetrics, R2riQueueMetric, R2riQueueMetrics, R2riSelfMetric, RxAntennaAdd,
    RxAntennaRemove, RxFrequencySegment, RxFrequencySegments, RxProperties, TxAntennaProfile,
    TxFrequencySegment, TxFrequencySegments, TxProperties, TxTransmitter, TxTransmitters,
    CONTROL_FLOW_CONTROL_TOKEN, CONTROL_FREQUENCY_INTEREST, CONTROL_MIMO_RX_PROPERTIES,
    CONTROL_MIMO_TX_PROPERTIES, CONTROL_R2RI_NEIGHBOR_METRIC, CONTROL_R2RI_QUEUE_METRIC,
    CONTROL_R2RI_SELF_METRIC, CONTROL_RX_ANTENNA_ADD, CONTROL_RX_ANTENNA_REMOVE,
    CONTROL_RX_ANTENNA_UPDATE, CONTROL_RX_FREQUENCY_SEGMENTS, CONTROL_RX_PROPERTIES,
    CONTROL_TX_ANTENNA_PROFILE, CONTROL_TX_FREQUENCY_SEGMENTS, CONTROL_TX_PROPERTIES,
    CONTROL_TX_TRANSMITTERS, PLUGIN_ABI_VERSION,
};
use crate::queue_metric_manager::QueueMetricManager;
use crate::spectrum_monitor::{FfiFrequencySegment, NoiseMode, SpectrumMonitor};
use crate::statistics::{
    configure_native_rf_signal_table, increment_native_counter, register_native_counter,
    register_native_rf_signal_table, unregister_native_statistics, update_native_rf_signal_table,
};
use crate::timer_service::{emane_rs_timer_cancel, emane_rs_timer_schedule};
use crate::{
    common::ethernet_transport::{
        emane_rs_ethernet_transport_parse_frame, emane_rs_ethernet_transport_update_arp_cache,
        emane_rs_ethernet_transport_verify_frame, EthernetTransportState,
    },
    r#virtual::virtual_transport::{
        emane_rs_virtual_transport_free, emane_rs_virtual_transport_new,
        emane_rs_virtual_transport_process_upstream_packet, emane_rs_virtual_transport_start,
        emane_rs_virtual_transport_stop, VirtualTransport,
    },
    raw_transport::{
        emane_rs_raw_transport_free, emane_rs_raw_transport_new,
        emane_rs_raw_transport_process_upstream_packet, emane_rs_raw_transport_start,
        emane_rs_raw_transport_stop, RawTransport,
    },
};
use crate::{LognormalFadingParameters, LognormalFadingState};
use libloading::{Library, Symbol};
use prost::Message;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::ffi::{c_void, CStr, CString};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock, Weak};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const BROADCAST_NEM: u16 = u16::MAX;

#[derive(Clone, Copy)]
struct Invocation {
    api: usize,
    plugin_ctx: usize,
}

impl Invocation {
    fn api(self) -> &'static PluginApi {
        unsafe { &*(self.api as *const PluginApi) }
    }

    fn context(self) -> *mut c_void {
        self.plugin_ctx as *mut c_void
    }
}

struct FrameworkContext {
    runtime: Weak<Runtime>,
    nem_id: u16,
    layer_index: usize,
    build_id: u16,
    neighbor_metrics: Mutex<NeighborMetricManager>,
    queue_metrics: Mutex<QueueMetricManager>,
}

struct NemLayer {
    // The library must outlive every function pointer and plugin instance.
    _library: Option<Library>,
    invocation: Invocation,
    _framework_context: Box<FrameworkContext>,
    event_build_id: u16,
    started: bool,
    destroyed: bool,
}

fn next_event_build_id() -> u16 {
    static NEXT: AtomicU16 = AtomicU16::new(1);
    NEXT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
        Some(if value == u16::MAX { 1 } else { value + 1 })
    })
    .unwrap_or(1)
}

struct Runtime {
    invocations: RwLock<HashMap<u16, Vec<Invocation>>>,
}

impl Runtime {
    fn invocation(&self, nem_id: u16, index: usize) -> Option<Invocation> {
        self.invocations
            .read()
            .ok()?
            .get(&nem_id)
            .and_then(|layers| layers.get(index))
            .copied()
    }

    fn last_invocation(&self, nem_id: u16) -> Option<Invocation> {
        self.invocations
            .read()
            .ok()?
            .get(&nem_id)
            .and_then(|layers| layers.last())
            .copied()
    }

    fn route_downstream(
        &self,
        nem_id: u16,
        current_index: usize,
        packet: *const FfiPacket,
        messages: *const FfiControlMessage,
        message_count: usize,
    ) {
        if let Some(next) = self.invocation(nem_id, current_index + 1) {
            (next.api().process_downstream)(next.context(), packet, messages, message_count);
            return;
        }

        if packet.is_null() {
            return;
        }

        let packet_ref = unsafe { &*packet };
        if let Some(control_data) = encode_controls(messages, message_count) {
            let _ = emane_rs_ota_manager_send_ota_packet(
                nem_id,
                packet_ref.info.destination,
                packet_ref.payload.data,
                packet_ref.payload.len,
                control_data.as_ptr(),
                control_data.len(),
                std::ptr::null(),
                0,
            );
        }

        // OTA is a shared medium: every other local PHY must observe the
        // transmission. Packet destination filtering happens in the MAC, not
        // here, so that promiscuous reception and interference remain correct.
        let targets: Vec<Invocation> = {
            let Ok(layers) = self.invocations.read() else {
                return;
            };
            layers
                .iter()
                .filter(|(id, _)| **id != nem_id)
                .filter_map(|(_, stack)| stack.last().copied())
                .collect()
        };
        for target in targets {
            (target.api().process_upstream)(target.context(), packet, messages, message_count);
        }
    }

    fn route_upstream(
        &self,
        nem_id: u16,
        current_index: usize,
        packet: *const FfiPacket,
        messages: *const FfiControlMessage,
        message_count: usize,
    ) {
        if current_index > 0 {
            if let Some(previous) = self.invocation(nem_id, current_index - 1) {
                (previous.api().process_upstream)(
                    previous.context(),
                    packet,
                    messages,
                    message_count,
                );
            }
        }
    }

    fn route_control(
        &self,
        nem_id: u16,
        current_index: usize,
        downstream: bool,
        messages: *const FfiControlMessage,
        message_count: usize,
    ) {
        let target = if downstream {
            self.invocation(nem_id, current_index + 1)
        } else if current_index > 0 {
            self.invocation(nem_id, current_index - 1)
        } else {
            None
        };
        if let Some(target) = target {
            let process = if downstream {
                target.api().process_downstream
            } else {
                target.api().process_upstream
            };
            process(target.context(), std::ptr::null(), messages, message_count);
        }
    }
}

extern "C" fn ota_packet(
    context_ptr: *mut c_void,
    source: u16,
    destination: u16,
    priority: u8,
    _uuid: *const u8,
    data: *const u8,
    data_len: usize,
    controls: *const u8,
    controls_len: usize,
) {
    let Some(context) = context(context_ptr) else {
        return;
    };
    if (data_len != 0 && data.is_null()) || (controls_len != 0 && controls.is_null()) {
        return;
    }
    let control_bytes = if controls_len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(controls, controls_len) }
    };
    let Some(decoded) = decode_controls(control_bytes) else {
        return;
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let packet = FfiPacket {
        info: FfiPacketInfo {
            source,
            destination,
            priority,
            creation_time_sec: now.as_secs(),
            creation_time_usec: now.subsec_micros(),
        },
        payload: FfiSlice {
            data,
            len: data_len,
        },
    };
    if let Some(runtime) = context.runtime.upgrade() {
        if let Some(phy) = runtime.last_invocation(context.nem_id) {
            (phy.api().process_upstream)(
                phy.context(),
                &packet,
                decoded.messages.as_ptr(),
                decoded.messages.len(),
            );
        }
    }
}

extern "C" fn framework_event(
    context_ptr: *mut c_void,
    event_id: u16,
    data: *const u8,
    data_len: usize,
) {
    let Some(context) = context(context_ptr) else {
        return;
    };
    if data_len != 0 && data.is_null() {
        return;
    }
    if let Some(runtime) = context.runtime.upgrade() {
        if let Some(layer) = runtime.invocation(context.nem_id, context.layer_index) {
            (layer.api().process_event)(layer.context(), event_id, data, data_len);
        }
    }
}

const MAX_CONTROL_WIRE_SIZE: usize = 16 * 1024 * 1024;

fn encode_controls(messages: *const FfiControlMessage, count: usize) -> Option<Vec<u8>> {
    let messages = ffi_control_messages(messages, count)?;
    let count = u16::try_from(messages.len()).ok()?;
    let mut data = Vec::new();
    data.extend_from_slice(&count.to_be_bytes());
    for message in messages {
        if message.payload.len > u32::MAX as usize
            || (message.payload.len != 0 && message.payload.data.is_null())
        {
            return None;
        }
        let required = 8usize.checked_add(message.payload.len)?;
        if data.len().checked_add(required)? > MAX_CONTROL_WIRE_SIZE {
            return None;
        }
        data.extend_from_slice(&message.msg_type.to_be_bytes());
        data.extend_from_slice(&(message.payload.len as u32).to_be_bytes());
        if message.payload.len != 0 {
            data.extend_from_slice(unsafe {
                std::slice::from_raw_parts(message.payload.data, message.payload.len)
            });
        }
    }
    Some(data)
}

struct DecodedControls {
    _payloads: Vec<Vec<u8>>,
    messages: Vec<FfiControlMessage>,
}

fn decode_controls(data: &[u8]) -> Option<DecodedControls> {
    if data.len() < 2 || data.len() > MAX_CONTROL_WIRE_SIZE {
        return None;
    }
    let count = u16::from_be_bytes(data[..2].try_into().ok()?) as usize;
    let mut offset = 2usize;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let header_end = offset.checked_add(8)?;
        if header_end > data.len() {
            return None;
        }
        let msg_type = u32::from_be_bytes(data[offset..offset + 4].try_into().ok()?);
        let len = u32::from_be_bytes(data[offset + 4..header_end].try_into().ok()?) as usize;
        offset = header_end;
        let end = offset.checked_add(len)?;
        if end > data.len() {
            return None;
        }
        entries.push((msg_type, data[offset..end].to_vec()));
        offset = end;
    }
    if offset != data.len() {
        return None;
    }
    let payloads: Vec<Vec<u8>> = entries.iter().map(|(_, payload)| payload.clone()).collect();
    let messages = entries
        .iter()
        .zip(payloads.iter())
        .map(|((msg_type, _), payload)| FfiControlMessage {
            msg_type: *msg_type,
            payload: FfiSlice {
                data: payload.as_ptr(),
                len: payload.len(),
            },
        })
        .collect();
    Some(DecodedControls {
        _payloads: payloads,
        messages,
    })
}

struct TimerDispatch {
    runtime: Weak<Runtime>,
    nem_id: u16,
    layer_index: usize,
    plugin_event_id: u32,
    data: Vec<u8>,
}

extern "C" fn timer_dispatch(
    timer_id: usize,
    _expire: u64,
    _schedule: u64,
    _fire: u64,
    _arg: *const c_void,
    user: *mut c_void,
) {
    let Some(dispatch) = (unsafe { (user as *const TimerDispatch).as_ref() }) else {
        return;
    };
    if let Some(runtime) = dispatch.runtime.upgrade() {
        if let Some(layer) = runtime.invocation(dispatch.nem_id, dispatch.layer_index) {
            (layer.api().process_timed_event)(
                layer.context(),
                timer_id as u64,
                dispatch.plugin_event_id,
                dispatch.data.as_ptr(),
                dispatch.data.len(),
            );
        }
    }
}

extern "C" fn timer_dispatch_free(user: *mut c_void) {
    if !user.is_null() {
        unsafe { drop(Box::from_raw(user as *mut TimerDispatch)) };
    }
}

fn context(ctx: *mut c_void) -> Option<&'static FrameworkContext> {
    unsafe { (ctx as *const FrameworkContext).as_ref() }
}

extern "C" fn send_downstream_packet(
    ctx: *mut c_void,
    _nem_id: u16,
    packet: *const FfiPacket,
    messages: *const FfiControlMessage,
    count: usize,
) {
    if let Some(ctx) = context(ctx) {
        if let Some(runtime) = ctx.runtime.upgrade() {
            runtime.route_downstream(ctx.nem_id, ctx.layer_index, packet, messages, count);
        }
    }
}

extern "C" fn send_upstream_packet(
    ctx: *mut c_void,
    _nem_id: u16,
    packet: *const FfiPacket,
    messages: *const FfiControlMessage,
    count: usize,
) {
    if let Some(ctx) = context(ctx) {
        if let Some(runtime) = ctx.runtime.upgrade() {
            runtime.route_upstream(ctx.nem_id, ctx.layer_index, packet, messages, count);
        }
    }
}

extern "C" fn send_downstream_control(
    ctx: *mut c_void,
    _nem_id: u16,
    messages: *const FfiControlMessage,
    count: usize,
) {
    if let Some(ctx) = context(ctx) {
        if let Some(runtime) = ctx.runtime.upgrade() {
            runtime.route_control(ctx.nem_id, ctx.layer_index, true, messages, count);
        }
    }
}

extern "C" fn send_upstream_control(
    ctx: *mut c_void,
    _nem_id: u16,
    messages: *const FfiControlMessage,
    count: usize,
) {
    if let Some(ctx) = context(ctx) {
        if let Some(runtime) = ctx.runtime.upgrade() {
            runtime.route_control(ctx.nem_id, ctx.layer_index, false, messages, count);
        }
    }
}

extern "C" fn schedule_timed_event(
    ctx: *mut c_void,
    _nem_id: u16,
    time_sec: u64,
    time_usec: u32,
    event_id: u32,
    data: *const u8,
    data_len: usize,
) -> u64 {
    let Some(ctx) = context(ctx) else { return 0 };
    let Some(runtime) = ctx.runtime.upgrade() else {
        return 0;
    };
    let payload = if data.is_null() || data_len == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(data, data_len) }.to_vec()
    };
    let dispatch = Box::new(TimerDispatch {
        runtime: Arc::downgrade(&runtime),
        nem_id: ctx.nem_id,
        layer_index: ctx.layer_index,
        plugin_event_id: event_id,
        data: payload,
    });
    let requested = time_sec
        .saturating_mul(1_000_000)
        .saturating_add(time_usec as u64);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64;
    let expiration = if requested < now {
        now.saturating_add(requested)
    } else {
        requested
    };
    emane_rs_timer_schedule(
        expiration,
        0,
        std::ptr::null(),
        Box::into_raw(dispatch) as *mut c_void,
        timer_dispatch,
        Some(timer_dispatch_free),
    ) as u64
}

extern "C" fn cancel_timed_event(_ctx: *mut c_void, _nem_id: u16, timer_id: u64) {
    emane_rs_timer_cancel(timer_id as usize);
}

extern "C" fn log(_ctx: *mut c_void, level: u32, message: *const std::os::raw::c_char) {
    if !message.is_null() {
        let message = unsafe { CStr::from_ptr(message) }.to_string_lossy();
        eprintln!("[EMANE:{level}] {message}");
    }
}

extern "C" fn register_counter(
    ctx: *mut c_void,
    name: *const std::os::raw::c_char,
    description: *const std::os::raw::c_char,
    clearable: bool,
) -> u64 {
    let (Some(ctx), Some(name), Some(description)) = (
        context(ctx),
        (!name.is_null()).then(|| unsafe { CStr::from_ptr(name) }),
        (!description.is_null()).then(|| unsafe { CStr::from_ptr(description) }),
    ) else {
        return 0;
    };
    let (Ok(name), Ok(description)) = (name.to_str(), description.to_str()) else {
        return 0;
    };
    register_native_counter(ctx.build_id, name, description, clearable).unwrap_or(0)
}

extern "C" fn increment_counter(ctx: *mut c_void, handle: u64, amount: u64) -> bool {
    context(ctx).is_some() && handle != 0 && increment_native_counter(handle, amount)
}

extern "C" fn register_rf_signal_table(ctx: *mut c_void, nem_id: u16) -> u64 {
    let Some(ctx) = context(ctx) else { return 0 };
    register_native_rf_signal_table(ctx.build_id, nem_id).unwrap_or(0)
}

extern "C" fn configure_rf_signal_table(
    ctx: *mut c_void,
    handle: u64,
    average_all_antennas: bool,
    average_all_frequencies: bool,
) -> bool {
    context(ctx).is_some()
        && handle != 0
        && configure_native_rf_signal_table(handle, average_all_antennas, average_all_frequencies)
}

#[allow(clippy::too_many_arguments)]
extern "C" fn update_rf_signal_table(
    ctx: *mut c_void,
    handle: u64,
    source: u16,
    antenna: u16,
    frequency_hz: u64,
    rx_power_dbm: f64,
    sinr_db: f64,
    noise_floor_dbm: f64,
    receiver_sensitivity_dbm: f64,
) -> bool {
    context(ctx).is_some()
        && handle != 0
        && update_native_rf_signal_table(
            handle,
            source,
            antenna,
            frequency_hz,
            rx_power_dbm,
            sinr_db,
            noise_floor_dbm,
            receiver_sensitivity_dbm,
        )
}

extern "C" fn publish_event(
    ctx: *mut c_void,
    event_id: u16,
    data: *const u8,
    data_len: usize,
) -> bool {
    let Some(ctx) = context(ctx) else {
        return false;
    };
    if data_len > 16 * 1024 * 1024 || (data_len != 0 && data.is_null()) {
        return false;
    }
    emane_rs_event_service_send_event(ctx.build_id, ctx.nem_id, event_id, data.cast(), data_len);
    true
}

extern "C" fn update_neighbor_tx(
    ctx: *mut c_void,
    destination: u16,
    data_rate_bps: u64,
    tx_time_microseconds: u64,
) {
    let Some(context) = context(ctx) else {
        return;
    };
    if let Ok(mut metrics) = context.neighbor_metrics.lock() {
        metrics.handle_tx_activity(
            destination,
            data_rate_bps,
            Duration::from_micros(tx_time_microseconds),
        );
    }
}

extern "C" fn update_neighbor_rx(
    ctx: *mut c_void,
    source: u16,
    sequence: u64,
    sinr_db: f64,
    noise_floor_dbm: f64,
    rx_time_microseconds: u64,
    duration_microseconds: u64,
    data_rate_bps: u64,
) {
    let Some(context) = context(ctx) else {
        return;
    };
    if let Ok(mut metrics) = context.neighbor_metrics.lock() {
        metrics.handle_rx_activity(
            source,
            sequence,
            &[0; 16],
            sinr_db,
            noise_floor_dbm,
            Duration::from_micros(rx_time_microseconds),
            Duration::from_micros(duration_microseconds),
            data_rate_bps,
        );
    }
}

extern "C" fn update_queue_metric(
    ctx: *mut c_void,
    queue_id: u16,
    max_size: u32,
    current_depth: u32,
    num_discards: u32,
    delay_microseconds: u64,
) {
    let Some(context) = context(ctx) else {
        return;
    };
    if let Ok(mut metrics) = context.queue_metrics.lock() {
        metrics.update_queue_metric(
            queue_id,
            max_size,
            current_depth,
            num_discards,
            delay_microseconds,
        );
    }
}

extern "C" fn publish_r2ri(
    ctx: *mut c_void,
    broadcast_data_rate_bps: u64,
    max_data_rate_bps: u64,
    report_interval_microseconds: u64,
    neighbor_delete_microseconds: u64,
) {
    let Some(context) = context(ctx) else {
        return;
    };
    let queue_metrics = context
        .queue_metrics
        .lock()
        .map(|mut manager| manager.get_queue_metrics())
        .unwrap_or_default();
    let neighbor_metrics = context
        .neighbor_metrics
        .lock()
        .map(|mut manager| {
            manager.set_neighbor_delete_time_microseconds(Duration::from_micros(
                neighbor_delete_microseconds,
            ));
            manager
                .get_neighbor_metrics(Duration::from_micros(unix_time_microseconds().max(0) as u64))
        })
        .unwrap_or_default();
    let self_bytes = R2riSelfMetric {
        broadcast_data_rate_bps,
        max_data_rate_bps,
        report_interval_microseconds,
    }
    .encode();
    let queue_bytes = R2riQueueMetrics {
        metrics: queue_metrics
            .into_iter()
            .map(|metric| R2riQueueMetric {
                queue_id: metric.queue_id,
                max_size: metric.queue_max_size,
                current_depth_high_water: metric.queue_current_depth_high_water,
                num_discards_high_water: metric.num_discards_high_water,
                average_delay_microseconds: metric.avg_delay_microseconds,
            })
            .collect(),
    }
    .encode()
    .unwrap_or_default();
    let neighbor_bytes = R2riNeighborMetrics {
        metrics: neighbor_metrics
            .into_iter()
            .map(|metric| R2riNeighborMetric {
                nem_id: metric.neighbor_id,
                num_rx_frames: metric.num_rx_frames,
                num_tx_frames: metric.num_tx_frames,
                num_missed_frames: metric.num_rx_missed_frames,
                bandwidth_consumption_microseconds: metric.rx_utilization_microseconds,
                sinr_average_db: metric.sinr_avg,
                sinr_stddev: metric.sinr_std,
                noise_floor_average_dbm: metric.noise_floor_avg,
                noise_floor_stddev: metric.noise_floor_stdv,
                rx_average_data_rate_bps: metric.rx_data_rate_avg,
                tx_average_data_rate_bps: metric.tx_data_rate_avg,
            })
            .collect(),
    }
    .encode()
    .unwrap_or_default();
    let messages = [
        FfiControlMessage {
            msg_type: CONTROL_R2RI_SELF_METRIC,
            payload: FfiSlice {
                data: self_bytes.as_ptr(),
                len: self_bytes.len(),
            },
        },
        FfiControlMessage {
            msg_type: CONTROL_R2RI_QUEUE_METRIC,
            payload: FfiSlice {
                data: queue_bytes.as_ptr(),
                len: queue_bytes.len(),
            },
        },
        FfiControlMessage {
            msg_type: CONTROL_R2RI_NEIGHBOR_METRIC,
            payload: FfiSlice {
                data: neighbor_bytes.as_ptr(),
                len: neighbor_bytes.len(),
            },
        },
    ];
    send_upstream_control(ctx, context.nem_id, messages.as_ptr(), messages.len());
}

fn canonical_plugin_name(name: &str) -> &str {
    match name {
        "ieee80211abgmaclayer" | "emane-model-ieee80211abg" => "ieee80211abg",
        "bentpipemaclayer" | "emane-model-bentpipe" => "bentpipe",
        "rfpipemaclayer" | "emane-model-rfpipe" => "rfpipe",
        "tdmaeventschedulerradiomodel" | "tdmamaclayer" => "tdma",
        "dummy-mac" => "dummy_mac",
        other => other,
    }
}

pub fn resolve_plugin_path(name: &str) -> Result<PathBuf, String> {
    let supplied = Path::new(name);
    if supplied.exists() {
        return Ok(supplied.to_path_buf());
    }
    if supplied.components().count() > 1 || name.ends_with(".so") {
        return Err(format!(
            "plugin path does not exist: {}",
            supplied.display()
        ));
    }
    let canonical = canonical_plugin_name(name);
    let filename = if canonical.starts_with("lib") && canonical.ends_with(".so") {
        canonical.to_string()
    } else {
        format!("lib{canonical}.so")
    };
    let mut directories = Vec::new();
    if let Some(paths) = std::env::var_os("EMANE_PLUGIN_PATH") {
        directories.extend(std::env::split_paths(&paths));
    }
    if let Ok(executable) = std::env::current_exe() {
        if let Some(parent) = executable.parent() {
            // Unit-test executables live in target/{profile}/deps. Loading a
            // bare cdylib from there can select a stale, pre-ABI-change copy;
            // Cargo's deployable cdylibs are emitted one directory above it.
            if parent.file_name().is_some_and(|name| name == "deps") {
                if let Some(profile) = parent.parent() {
                    directories.push(profile.to_path_buf());
                }
            } else {
                directories.push(parent.to_path_buf());
            }
        }
    }
    directories.extend([
        PathBuf::from("rust/target/debug"),
        PathBuf::from("rust/target/release"),
        PathBuf::from("target/debug"),
        PathBuf::from("target/release"),
    ]);
    for directory in directories {
        let candidate = directory.join(&filename);
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    // A bare filename lets the platform dynamic loader search its configured
    // system paths (LD_LIBRARY_PATH, ld.so cache, and the platform defaults).
    Ok(PathBuf::from(filename))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BuiltinKind {
    Phy,
    VirtualTransport,
    RawTransport,
}

struct BuiltinState {
    id: u16,
    framework: FfiFrameworkService,
    kind: BuiltinKind,
    virtual_transport: *mut VirtualTransport,
    raw_transport: *mut RawTransport,
    ethernet: Box<EthernetTransportState>,
    device_path: String,
    device_name: String,
    arp_mode: bool,
    broadcast_mode: bool,
    arp_cache_mode: bool,
    arp_priority: u8,
    unknown_priorities: HashMap<u16, u8>,
    flow_control_enabled: bool,
    flow_control_tokens: u16,
    pending_transport_frames: VecDeque<PendingTransportFrame>,
    bitrate_bps: u64,
    counters: CommonLayerCounters,
    phy: PhyState,
}

struct PendingTransportFrame {
    payload: Vec<u8>,
    destination: u16,
    priority: u8,
    creation_time_sec: u64,
    creation_time_usec: u32,
}

struct PhyState {
    compatibility_mode: u8,
    frequency_hz: u64,
    frequencies_of_interest: Vec<u64>,
    bandwidth_hz: u64,
    tx_power_dbm: f64,
    fixed_antenna_gain_db: f64,
    fixed_antenna_gain_enabled: bool,
    propagation_model: PropagationModel,
    locations: HashMap<u16, Location>,
    pathloss: HashMap<u16, HashMap<u64, f64>>,
    antenna_profiles: HashMap<u16, AntennaProfileSelection>,
    fading_selections: HashMap<u16, FadingMode>,
    fading_mode: FadingMode,
    nakagami: crate::nakagami_fading_algorithm::NakagamiFadingAlgorithm,
    nakagami_parameters: NakagamiParameters,
    lognormal_states: HashMap<u16, LognormalFadingState>,
    lognormal_parameters: LognormalFadingParameters,
    doppler_shift_enabled: bool,
    spectral_mask_index: u16,
    radio_silence_enabled: bool,
    exclude_same_sub_id_from_filter: bool,
    sub_id: u16,
    noise_mode: NoiseMode,
    noise_bin_size: i64,
    max_segment_offset: i64,
    max_message_propagation: i64,
    max_segment_duration: i64,
    time_sync_threshold: i64,
    noise_max_clamp: bool,
    system_noise_figure_db: f64,
    monitor: SpectrumMonitor,
    receive_antennas: HashMap<u16, ReceiveAntennaState>,
}

struct ReceiveAntennaState {
    antenna: MimoTxAntenna,
    frequencies_hz: Vec<u64>,
    monitor: SpectrumMonitor,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PropagationModel {
    Precomputed,
    FreeSpace,
    TwoRay,
}

#[derive(Clone, Copy)]
struct Location {
    latitude_degrees: f64,
    longitude_degrees: f64,
    altitude_meters: f64,
    velocity: Option<Velocity>,
    orientation: Orientation,
}

#[derive(Clone, Copy)]
struct Velocity {
    azimuth_degrees: f64,
    elevation_degrees: f64,
    magnitude_meters_per_second: f64,
}

#[derive(Clone, Copy, Default)]
struct Orientation {
    roll_degrees: f64,
    pitch_degrees: f64,
    yaw_degrees: f64,
}

#[derive(Clone, Copy)]
struct AntennaProfileSelection {
    profile_id: u16,
    azimuth_degrees: f64,
    elevation_degrees: f64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FadingMode {
    None,
    Event,
    Nakagami,
    Lognormal,
}

struct NakagamiParameters {
    distance0_meters: f64,
    distance1_meters: f64,
    m0: f64,
    m1: f64,
    m2: f64,
}

impl PhyState {
    fn new() -> Self {
        Self {
            compatibility_mode: 1,
            frequency_hz: 2_347_000_000,
            frequencies_of_interest: vec![2_347_000_000],
            bandwidth_hz: 1_000_000,
            tx_power_dbm: 0.0,
            fixed_antenna_gain_db: 0.0,
            fixed_antenna_gain_enabled: true,
            propagation_model: PropagationModel::Precomputed,
            locations: HashMap::new(),
            pathloss: HashMap::new(),
            antenna_profiles: HashMap::new(),
            fading_selections: HashMap::new(),
            fading_mode: FadingMode::None,
            nakagami: crate::nakagami_fading_algorithm::NakagamiFadingAlgorithm::new(),
            nakagami_parameters: NakagamiParameters {
                distance0_meters: 100.0,
                distance1_meters: 250.0,
                m0: 0.75,
                m1: 1.0,
                m2: 200.0,
            },
            lognormal_states: HashMap::new(),
            lognormal_parameters: LognormalFadingParameters {
                dmu: 5.0,
                dsigma: 1.0,
                dlthresh: 0.25,
                maxpathloss: 100.0,
                duthresh: 0.75,
                minpathloss: 0.0,
                lmean: 0.005,
                lstddev: 0.001,
                counter: 1,
            },
            doppler_shift_enabled: true,
            spectral_mask_index: 0,
            radio_silence_enabled: false,
            exclude_same_sub_id_from_filter: false,
            sub_id: 1,
            noise_mode: NoiseMode::All,
            noise_bin_size: 20,
            max_segment_offset: 300_000,
            max_message_propagation: 200_000,
            max_segment_duration: 1_000_000,
            time_sync_threshold: 10_000,
            noise_max_clamp: false,
            system_noise_figure_db: 4.0,
            monitor: SpectrumMonitor::new(),
            receive_antennas: HashMap::new(),
        }
    }

    fn receiver_sensitivity_dbm(&self) -> f64 {
        -174.0 + 10.0 * (self.bandwidth_hz.max(1) as f64).log10() + self.system_noise_figure_db
    }

    fn initialize_monitor(&mut self) {
        let sensitivity_mw = 10.0f64.powf(self.receiver_sensitivity_dbm() / 10.0);
        self.monitor.initialize(
            self.sub_id,
            &self.frequencies_of_interest,
            self.bandwidth_hz,
            sensitivity_mw,
            self.noise_mode,
            self.noise_bin_size,
            self.max_segment_offset,
            self.max_message_propagation,
            self.max_segment_duration,
            self.time_sync_threshold,
            self.noise_max_clamp,
            self.exclude_same_sub_id_from_filter,
        );
    }

    fn receiver_sensitivity_for_bandwidth_dbm(&self, bandwidth_hz: u64) -> f64 {
        -174.0 + 10.0 * (bandwidth_hz.max(1) as f64).log10() + self.system_noise_figure_db
    }

    fn make_monitor(&self, frequencies_hz: &[u64], bandwidth_hz: u64) -> SpectrumMonitor {
        let sensitivity_mw =
            10.0f64.powf(self.receiver_sensitivity_for_bandwidth_dbm(bandwidth_hz) / 10.0);
        let mut monitor = SpectrumMonitor::new();
        monitor.initialize(
            self.sub_id,
            frequencies_hz,
            bandwidth_hz,
            sensitivity_mw,
            self.noise_mode,
            self.noise_bin_size,
            self.max_segment_offset,
            self.max_message_propagation,
            self.max_segment_duration,
            self.time_sync_threshold,
            self.noise_max_clamp,
            self.exclude_same_sub_id_from_filter,
        );
        monitor
    }

    fn default_antenna_pattern(&self, nem_id: u16) -> Option<AntennaPattern> {
        if self.fixed_antenna_gain_enabled {
            Some(AntennaPattern::IdealOmni {
                gain_db: self.fixed_antenna_gain_db,
            })
        } else {
            self.antenna_profiles.get(&nem_id).map(|profile| {
                AntennaPattern::Profile(TxAntennaProfile {
                    profile_id: profile.profile_id,
                    azimuth_degrees: profile.azimuth_degrees,
                    elevation_degrees: profile.elevation_degrees,
                })
            })
        }
    }

    fn antenna_pair_gain(
        &self,
        local_id: u16,
        source: u16,
        local_pattern: AntennaPattern,
        remote_pattern: AntennaPattern,
    ) -> Option<f64> {
        let local_pattern = match local_pattern {
            AntennaPattern::Default => self.default_antenna_pattern(local_id)?,
            pattern => pattern,
        };
        let remote_pattern = match remote_pattern {
            AntennaPattern::Default => self.default_antenna_pattern(source)?,
            pattern => pattern,
        };
        let fixed = |pattern| match pattern {
            AntennaPattern::IdealOmni { gain_db } => Some(gain_db),
            _ => None,
        };
        if let (Some(local_gain), Some(remote_gain)) = (fixed(local_pattern), fixed(remote_pattern))
        {
            return Some(local_gain + remote_gain);
        }
        let local = *self.locations.get(&local_id)?;
        let remote = *self.locations.get(&source)?;
        let side_gain = |pattern: AntennaPattern, from: Location, to: Location| match pattern {
            AntennaPattern::IdealOmni { gain_db } => Some(gain_db),
            AntennaPattern::Profile(profile) => {
                let (reference_azimuth, reference_elevation) = oriented_direction_angles(from, to)?;
                crate::antenna::get_manager().get_profile_gain(
                    profile.profile_id,
                    normalize_azimuth(reference_azimuth - profile.azimuth_degrees),
                    normalize_elevation(reference_elevation - profile.elevation_degrees),
                    reference_azimuth,
                    reference_elevation,
                )
            }
            AntennaPattern::Default => None,
        };
        Some(side_gain(local_pattern, local, remote)? + side_gain(remote_pattern, remote, local)?)
    }

    fn propagation(&self, local_id: u16, source: u16, frequency_hz: u64) -> Option<(f64, i64)> {
        match self.propagation_model {
            PropagationModel::Precomputed => {
                let values = self.pathloss.get(&source)?;
                let pathloss = values
                    .get(&frequency_hz)
                    .or_else(|| values.get(&0))
                    .copied()?;
                Some((pathloss, 0))
            }
            PropagationModel::FreeSpace | PropagationModel::TwoRay => {
                let local = self.locations.get(&local_id)?;
                let remote = self.locations.get(&source)?;
                let distance = distance_meters(*local, *remote);
                let pathloss = match self.propagation_model {
                    PropagationModel::FreeSpace => {
                        crate::emane_rs_freespace_pathloss_single(distance, frequency_hz as f64)
                    }
                    PropagationModel::TwoRay => crate::emane_rs_tworay_pathloss(
                        distance,
                        local.altitude_meters,
                        remote.altitude_meters,
                    ),
                    PropagationModel::Precomputed => unreachable!(),
                };
                let delay = (distance / 299_792_458.0 * 1_000_000.0).round();
                Some((pathloss, delay.clamp(0.0, i64::MAX as f64) as i64))
            }
        }
    }

    fn distance(&self, local_id: u16, source: u16) -> Option<f64> {
        Some(distance_meters(
            *self.locations.get(&local_id)?,
            *self.locations.get(&source)?,
        ))
    }

    fn profile_gain_with_remote(
        &self,
        local_id: u16,
        source: u16,
        remote_override: Option<TxAntennaProfile>,
    ) -> Option<f64> {
        if self.fixed_antenna_gain_enabled {
            return Some(self.fixed_antenna_gain_db);
        }
        let local = *self.locations.get(&local_id)?;
        let remote = *self.locations.get(&source)?;
        let local_profile = *self.antenna_profiles.get(&local_id)?;
        let remote_profile = remote_override
            .map(|profile| AntennaProfileSelection {
                profile_id: profile.profile_id,
                azimuth_degrees: profile.azimuth_degrees,
                elevation_degrees: profile.elevation_degrees,
            })
            .or_else(|| self.antenna_profiles.get(&source).copied())?;

        let (local_reference_azimuth, local_reference_elevation) =
            oriented_direction_angles(local, remote)?;
        let local_gain = crate::antenna::get_manager().get_profile_gain(
            local_profile.profile_id,
            normalize_azimuth(local_reference_azimuth - local_profile.azimuth_degrees),
            normalize_elevation(local_reference_elevation - local_profile.elevation_degrees),
            local_reference_azimuth,
            local_reference_elevation,
        )?;

        let (remote_reference_azimuth, remote_reference_elevation) =
            oriented_direction_angles(remote, local)?;
        let remote_gain = crate::antenna::get_manager().get_profile_gain(
            remote_profile.profile_id,
            normalize_azimuth(remote_reference_azimuth - remote_profile.azimuth_degrees),
            normalize_elevation(remote_reference_elevation - remote_profile.elevation_degrees),
            remote_reference_azimuth,
            remote_reference_elevation,
        )?;
        Some(local_gain + remote_gain)
    }

    fn doppler_fraction(&self, local_id: u16, source: u16) -> f64 {
        if !self.doppler_shift_enabled {
            return 0.0;
        }
        let Some(local) = self.locations.get(&local_id).copied() else {
            return 0.0;
        };
        let Some(remote) = self.locations.get(&source).copied() else {
            return 0.0;
        };
        let (Some(local_velocity), Some(remote_velocity)) = (local.velocity, remote.velocity)
        else {
            return 0.0;
        };
        let Some(line_of_sight) = line_of_sight_neu(local, remote) else {
            return 0.0;
        };
        let local_velocity = velocity_neu(local_velocity);
        let remote_velocity = velocity_neu(remote_velocity);
        let radial_velocity = line_of_sight.0 * (local_velocity.0 - remote_velocity.0)
            + line_of_sight.1 * (local_velocity.1 - remote_velocity.1)
            + line_of_sight.2 * (local_velocity.2 - remote_velocity.2);
        const SPEED_OF_LIGHT: f64 = 299_792_458.0;
        let denominator = SPEED_OF_LIGHT - radial_velocity;
        if denominator <= 0.0 || !denominator.is_finite() {
            0.0
        } else {
            SPEED_OF_LIGHT / denominator - 1.0
        }
    }

    fn apply_fading(
        &mut self,
        local_id: u16,
        source: u16,
        power_dbm: f64,
        now_microseconds: u64,
    ) -> Option<f64> {
        let mode = match self.fading_mode {
            FadingMode::Event => *self.fading_selections.get(&source)?,
            mode => mode,
        };
        let power_mw = match mode {
            FadingMode::None => return Some(power_dbm),
            FadingMode::Event => return None,
            FadingMode::Nakagami => {
                let distance = self.distance(local_id, source)?;
                self.nakagami.compute(
                    power_dbm,
                    distance,
                    self.nakagami_parameters.distance0_meters,
                    self.nakagami_parameters.distance1_meters,
                    self.nakagami_parameters.m0,
                    self.nakagami_parameters.m1,
                    self.nakagami_parameters.m2,
                )
            }
            FadingMode::Lognormal => self.lognormal_states.entry(source).or_default().process(
                power_dbm,
                &self.lognormal_parameters,
                now_microseconds,
            )?,
        };
        (power_mw.is_finite() && power_mw > 0.0).then_some(10.0 * power_mw.log10())
    }
}

fn distance_meters(a: Location, b: Location) -> f64 {
    const EARTH_RADIUS_METERS: f64 = 6_371_000.0;
    let latitude_a = a.latitude_degrees.to_radians();
    let latitude_b = b.latitude_degrees.to_radians();
    let delta_latitude = (b.latitude_degrees - a.latitude_degrees).to_radians();
    let delta_longitude = (b.longitude_degrees - a.longitude_degrees).to_radians();
    let haversine = (delta_latitude / 2.0).sin().powi(2)
        + latitude_a.cos() * latitude_b.cos() * (delta_longitude / 2.0).sin().powi(2);
    let surface = 2.0 * EARTH_RADIUS_METERS * haversine.sqrt().atan2((1.0 - haversine).sqrt());
    surface.hypot(b.altitude_meters - a.altitude_meters)
}

fn oriented_direction_angles(local: Location, remote: Location) -> Option<(f64, f64)> {
    let (north, east, up) = relative_neu(local, remote);
    let magnitude = north.hypot(east).hypot(up);
    if magnitude <= f64::EPSILON || !magnitude.is_finite() {
        return None;
    }

    // Rotate the world-space line of sight into the platform body frame.
    // Yaw is clockwise from north, pitch is nose-up, and roll is about the
    // forward axis. Applying the inverse platform rotations makes antenna
    // and blockage patterns respond to all three orientation components.
    let yaw = local.orientation.yaw_degrees.to_radians();
    let pitch = local.orientation.pitch_degrees.to_radians();
    let roll = local.orientation.roll_degrees.to_radians();
    let forward_yaw = yaw.cos() * north + yaw.sin() * east;
    let right_yaw = -yaw.sin() * north + yaw.cos() * east;
    let forward = pitch.cos() * forward_yaw + pitch.sin() * up;
    let up_pitch = -pitch.sin() * forward_yaw + pitch.cos() * up;
    let right = roll.cos() * right_yaw + roll.sin() * up_pitch;
    let body_up = -roll.sin() * right_yaw + roll.cos() * up_pitch;
    Some((
        normalize_azimuth(right.atan2(forward).to_degrees()),
        normalize_elevation((body_up / magnitude).clamp(-1.0, 1.0).asin().to_degrees()),
    ))
}

fn relative_neu(local: Location, remote: Location) -> (f64, f64, f64) {
    const EARTH_RADIUS_METERS: f64 = 6_371_000.0;
    let mean_latitude = ((local.latitude_degrees + remote.latitude_degrees) * 0.5).to_radians();
    let north =
        (remote.latitude_degrees - local.latitude_degrees).to_radians() * EARTH_RADIUS_METERS;
    let east = (remote.longitude_degrees - local.longitude_degrees).to_radians()
        * EARTH_RADIUS_METERS
        * mean_latitude.cos();
    let up = remote.altitude_meters - local.altitude_meters;
    (north, east, up)
}

fn line_of_sight_neu(local: Location, remote: Location) -> Option<(f64, f64, f64)> {
    let vector = relative_neu(local, remote);
    let magnitude = vector.0.hypot(vector.1).hypot(vector.2);
    (magnitude > f64::EPSILON && magnitude.is_finite()).then_some((
        vector.0 / magnitude,
        vector.1 / magnitude,
        vector.2 / magnitude,
    ))
}

fn velocity_neu(velocity: Velocity) -> (f64, f64, f64) {
    let azimuth = velocity.azimuth_degrees.to_radians();
    let elevation = velocity.elevation_degrees.to_radians();
    let horizontal = velocity.magnitude_meters_per_second * elevation.cos();
    (
        horizontal * azimuth.cos(),
        horizontal * azimuth.sin(),
        velocity.magnitude_meters_per_second * elevation.sin(),
    )
}

fn normalize_azimuth(value: f64) -> f64 {
    value.rem_euclid(360.0)
}

fn normalize_elevation(value: f64) -> f64 {
    let value = value.rem_euclid(360.0);
    match value {
        value if value > 270.0 => value - 360.0,
        value if value > 90.0 => 180.0 - value,
        value => value,
    }
}

fn builtin_init(id: u16, framework: *const FfiFrameworkService, kind: BuiltinKind) -> *mut c_void {
    if framework.is_null() {
        return std::ptr::null_mut();
    }
    let framework = unsafe { *framework };
    let mut state = Box::new(BuiltinState {
        id,
        framework,
        kind,
        virtual_transport: std::ptr::null_mut(),
        raw_transport: std::ptr::null_mut(),
        ethernet: Box::new(EthernetTransportState::new()),
        device_path: "/dev/net/tun".to_string(),
        device_name: "emane0".to_string(),
        arp_mode: true,
        broadcast_mode: false,
        arp_cache_mode: true,
        arp_priority: 0,
        unknown_priorities: HashMap::new(),
        flow_control_enabled: false,
        flow_control_tokens: 0,
        pending_transport_frames: VecDeque::new(),
        bitrate_bps: 0,
        counters: CommonLayerCounters::register(framework),
        phy: PhyState::new(),
    });
    let state_ptr = state.as_mut() as *mut BuiltinState;
    match kind {
        BuiltinKind::VirtualTransport => {
            state.virtual_transport =
                emane_rs_virtual_transport_new(id, state_ptr.cast(), builtin_ethernet_downstream);
            if state.virtual_transport.is_null() {
                return std::ptr::null_mut();
            }
        }
        BuiltinKind::RawTransport => {
            state.raw_transport =
                emane_rs_raw_transport_new(id, state_ptr.cast(), builtin_ethernet_downstream);
            if state.raw_transport.is_null() {
                return std::ptr::null_mut();
            }
        }
        BuiltinKind::Phy => {}
    }
    Box::into_raw(state).cast()
}

extern "C" fn builtin_phy_init(id: u16, framework: *const FfiFrameworkService) -> *mut c_void {
    builtin_init(id, framework, BuiltinKind::Phy)
}

extern "C" fn builtin_virtual_init(id: u16, framework: *const FfiFrameworkService) -> *mut c_void {
    builtin_init(id, framework, BuiltinKind::VirtualTransport)
}

extern "C" fn builtin_raw_init(id: u16, framework: *const FfiFrameworkService) -> *mut c_void {
    builtin_init(id, framework, BuiltinKind::RawTransport)
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn parse_scaled_u64(value: &str) -> Option<u64> {
    let value = value.trim();
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

fn parse_u16(value: &str) -> Option<u16> {
    value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .map_or_else(
            || value.parse().ok(),
            |value| u16::from_str_radix(value, 16).ok(),
        )
}

fn parse_i64_config(value: &str) -> Option<i64> {
    value
        .parse::<u64>()
        .ok()
        .and_then(|value| i64::try_from(value).ok())
}

unsafe fn config_items(request: *const c_void) -> Option<Vec<(String, Vec<String>)>> {
    let request = (request as *const FfiConfigRequest).as_ref()?;
    if request.len > 4096 || (request.len != 0 && request.data.is_null()) {
        return None;
    }
    let mut result = Vec::with_capacity(request.len);
    let items = if request.len == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(request.data, request.len)
    };
    for item in items {
        if item.name.is_null()
            || item.values.len > 4096
            || (item.values.len != 0 && item.values.data.is_null())
        {
            return None;
        }
        let name = CStr::from_ptr(item.name).to_str().ok()?.to_string();
        let mut values = Vec::with_capacity(item.values.len);
        let item_values = if item.values.len == 0 {
            &[]
        } else {
            std::slice::from_raw_parts(item.values.data, item.values.len)
        };
        for value in item_values {
            if value.is_null() {
                return None;
            }
            values.push(CStr::from_ptr(*value).to_str().ok()?.to_string());
        }
        result.push((name, values));
    }
    Some(result)
}

extern "C" fn builtin_configure(state: *mut c_void, request: *const c_void) -> bool {
    let Some(state) = (unsafe { (state as *mut BuiltinState).as_mut() }) else {
        return false;
    };
    let Some(items) = (unsafe { config_items(request) }) else {
        return false;
    };
    for (name, values) in items {
        let Some(value) = values.first() else {
            return false;
        };
        let valid_for_kind = match state.kind {
            BuiltinKind::Phy => matches!(
                name.as_str(),
                "frequency"
                    | "frequencyofinterest"
                    | "bandwidth"
                    | "txpower"
                    | "fixedantennagain"
                    | "fixedantennagainenable"
                    | "subid"
                    | "noisemode"
                    | "noisebinsize"
                    | "noisemaxsegmentoffset"
                    | "noisemaxmessagepropagation"
                    | "noisemaxsegmentduration"
                    | "timesyncthreshold"
                    | "noisemaxclampenable"
                    | "systemnoisefigure"
                    | "propagationmodel"
                    | "excludesamesubidfromfilterenable"
                    | "compatibilitymode"
                    | "dopplershiftenable"
                    | "spectralmaskindex"
                    | "radiosilenceenable"
                    | "fading.model"
                    | "fading.nakagami.m0"
                    | "fading.nakagami.m1"
                    | "fading.nakagami.m2"
                    | "fading.nakagami.distance0"
                    | "fading.nakagami.distance1"
                    | "fading.lognormal.dmu"
                    | "fading.lognormal.dsigma"
                    | "fading.lognormal.dlthresh"
                    | "fading.lognormal.duthresh"
                    | "fading.lognormal.maxpathloss"
                    | "fading.lognormal.minpathloss"
                    | "fading.lognormal.lmean"
                    | "fading.lognormal.lstddev"
            ),
            BuiltinKind::VirtualTransport => matches!(
                name.as_str(),
                "devicepath"
                    | "device"
                    | "arpmodeenable"
                    | "broadcastmodeenable"
                    | "arpcacheenable"
                    | "ethernet.type.arp.priority"
                    | "ethernet.type.unknown.priority"
                    | "bitrate"
                    | "flowcontrolenable"
                    | "address"
                    | "mask"
            ),
            BuiltinKind::RawTransport => matches!(
                name.as_str(),
                "device"
                    | "broadcastmodeenable"
                    | "arpcacheenable"
                    | "ethernet.type.arp.priority"
                    | "ethernet.type.unknown.priority"
                    | "bitrate"
            ),
        };
        if !valid_for_kind {
            return false;
        }
        match name.as_str() {
            "devicepath" => state.device_path.clone_from(value),
            "device" => state.device_name.clone_from(value),
            "arpmodeenable" => {
                let Some(value) = parse_bool(value) else {
                    return false;
                };
                state.arp_mode = value;
            }
            "broadcastmodeenable" => {
                let Some(value) = parse_bool(value) else {
                    return false;
                };
                state.broadcast_mode = value;
            }
            "arpcacheenable" => {
                let Some(value) = parse_bool(value) else {
                    return false;
                };
                state.arp_cache_mode = value;
            }
            "ethernet.type.arp.priority" => {
                let Ok(value) = value.parse() else {
                    return false;
                };
                state.arp_priority = value;
            }
            "ethernet.type.unknown.priority" => {
                for value in values {
                    let Some((ether_type, priority)) = value.split_once(':') else {
                        return false;
                    };
                    let (Some(ether_type), Ok(priority)) =
                        (parse_u16(ether_type), priority.parse::<u8>())
                    else {
                        return false;
                    };
                    state.unknown_priorities.insert(ether_type, priority);
                }
            }
            "frequency" => {
                let Some(value) = parse_scaled_u64(value) else {
                    return false;
                };
                if value == 0 {
                    return false;
                }
                state.phy.frequency_hz = value;
            }
            "frequencyofinterest" => {
                let mut frequencies = Vec::with_capacity(values.len());
                for value in values {
                    let Some(value) = parse_scaled_u64(&value) else {
                        return false;
                    };
                    if value == 0 {
                        return false;
                    }
                    frequencies.push(value);
                }
                if frequencies.is_empty() {
                    return false;
                }
                state.phy.frequencies_of_interest = frequencies;
            }
            "bandwidth" => {
                let Some(value) = parse_scaled_u64(value) else {
                    return false;
                };
                if value == 0 {
                    return false;
                }
                state.phy.bandwidth_hz = value;
            }
            "txpower" => {
                let Ok(value) = value.parse::<f64>() else {
                    return false;
                };
                if !value.is_finite() {
                    return false;
                }
                state.phy.tx_power_dbm = value;
            }
            "fixedantennagain" => {
                let Ok(value) = value.parse::<f64>() else {
                    return false;
                };
                if !value.is_finite() {
                    return false;
                }
                state.phy.fixed_antenna_gain_db = value;
            }
            "subid" => {
                let Ok(value) = value.parse::<u16>() else {
                    return false;
                };
                if value == 0 {
                    return false;
                }
                state.phy.sub_id = value;
            }
            "noisemode" => {
                state.phy.noise_mode = match value.as_str() {
                    "none" => NoiseMode::None,
                    "all" => NoiseMode::All,
                    "outofband" => NoiseMode::OutOfBand,
                    "passthrough" => NoiseMode::PassThrough,
                    _ => return false,
                };
            }
            "noisebinsize" => {
                let Some(value) = parse_i64_config(value) else {
                    return false;
                };
                if value == 0 {
                    return false;
                }
                state.phy.noise_bin_size = value;
            }
            "noisemaxsegmentoffset" => {
                let Some(value) = parse_i64_config(value) else {
                    return false;
                };
                state.phy.max_segment_offset = value;
            }
            "noisemaxmessagepropagation" => {
                let Some(value) = parse_i64_config(value) else {
                    return false;
                };
                state.phy.max_message_propagation = value;
            }
            "noisemaxsegmentduration" => {
                let Some(value) = parse_i64_config(value) else {
                    return false;
                };
                state.phy.max_segment_duration = value;
            }
            "timesyncthreshold" => {
                let Some(value) = parse_i64_config(value) else {
                    return false;
                };
                state.phy.time_sync_threshold = value;
            }
            "noisemaxclampenable" => {
                let Some(value) = parse_bool(value) else {
                    return false;
                };
                state.phy.noise_max_clamp = value;
            }
            "systemnoisefigure" => {
                let Ok(value) = value.parse::<f64>() else {
                    return false;
                };
                if !value.is_finite() {
                    return false;
                }
                state.phy.system_noise_figure_db = value;
            }
            "fixedantennagainenable" => {
                let Some(value) = parse_bool(value) else {
                    return false;
                };
                state.phy.fixed_antenna_gain_enabled = value;
            }
            "propagationmodel" => {
                state.phy.propagation_model = match value.as_str() {
                    "precomputed" => PropagationModel::Precomputed,
                    "freespace" => PropagationModel::FreeSpace,
                    "2ray" => PropagationModel::TwoRay,
                    _ => return false,
                };
            }
            "excludesamesubidfromfilterenable" => {
                let Some(value) = parse_bool(value) else {
                    return false;
                };
                state.phy.exclude_same_sub_id_from_filter = value;
            }
            "compatibilitymode" => {
                state.phy.compatibility_mode = match value.parse::<u8>() {
                    Ok(value @ 1..=2) => value,
                    _ => return false,
                };
            }
            "dopplershiftenable" => {
                let Some(value) = parse_bool(value) else {
                    return false;
                };
                state.phy.doppler_shift_enabled = value;
            }
            "spectralmaskindex" => {
                let Ok(value) = value.parse::<u16>() else {
                    return false;
                };
                state.phy.spectral_mask_index = value;
            }
            "radiosilenceenable" => {
                let Some(value) = parse_bool(value) else {
                    return false;
                };
                state.phy.radio_silence_enabled = value;
            }
            "fading.model" => {
                state.phy.fading_mode = match value.as_str() {
                    "none" => FadingMode::None,
                    "event" => FadingMode::Event,
                    "nakagami" => FadingMode::Nakagami,
                    "lognormal" => FadingMode::Lognormal,
                    _ => return false,
                };
            }
            "fading.nakagami.m0" | "fading.nakagami.m1" | "fading.nakagami.m2" => {
                let Ok(parsed) = value.parse::<f64>() else {
                    return false;
                };
                if !parsed.is_finite() || parsed < 0.5 {
                    return false;
                }
                match name.as_str() {
                    "fading.nakagami.m0" => state.phy.nakagami_parameters.m0 = parsed,
                    "fading.nakagami.m1" => state.phy.nakagami_parameters.m1 = parsed,
                    _ => state.phy.nakagami_parameters.m2 = parsed,
                }
            }
            "fading.nakagami.distance0" | "fading.nakagami.distance1" => {
                let Ok(parsed) = value.parse::<f64>() else {
                    return false;
                };
                if !parsed.is_finite() || parsed < 0.0 {
                    return false;
                }
                if name == "fading.nakagami.distance0" {
                    state.phy.nakagami_parameters.distance0_meters = parsed;
                } else {
                    state.phy.nakagami_parameters.distance1_meters = parsed;
                }
            }
            "fading.lognormal.dmu"
            | "fading.lognormal.dsigma"
            | "fading.lognormal.dlthresh"
            | "fading.lognormal.duthresh"
            | "fading.lognormal.maxpathloss"
            | "fading.lognormal.minpathloss"
            | "fading.lognormal.lmean"
            | "fading.lognormal.lstddev" => {
                let Ok(parsed) = value.parse::<f64>() else {
                    return false;
                };
                if !parsed.is_finite()
                    || (matches!(
                        name.as_str(),
                        "fading.lognormal.dsigma"
                            | "fading.lognormal.lmean"
                            | "fading.lognormal.lstddev"
                    ) && parsed < 0.0)
                {
                    return false;
                }
                match name.as_str() {
                    "fading.lognormal.dmu" => state.phy.lognormal_parameters.dmu = parsed,
                    "fading.lognormal.dsigma" => state.phy.lognormal_parameters.dsigma = parsed,
                    "fading.lognormal.dlthresh" => state.phy.lognormal_parameters.dlthresh = parsed,
                    "fading.lognormal.duthresh" => state.phy.lognormal_parameters.duthresh = parsed,
                    "fading.lognormal.maxpathloss" => {
                        state.phy.lognormal_parameters.maxpathloss = parsed
                    }
                    "fading.lognormal.minpathloss" => {
                        state.phy.lognormal_parameters.minpathloss = parsed
                    }
                    "fading.lognormal.lmean" => state.phy.lognormal_parameters.lmean = parsed,
                    _ => state.phy.lognormal_parameters.lstddev = parsed,
                }
                state.phy.lognormal_parameters.counter =
                    state.phy.lognormal_parameters.counter.wrapping_add(1);
            }
            "bitrate" => {
                state.bitrate_bps = match parse_scaled_u64(value) {
                    Some(value) => value,
                    None => return false,
                };
            }
            "flowcontrolenable" => {
                state.flow_control_enabled = match parse_bool(value) {
                    Some(value) => value,
                    None => return false,
                };
            }
            // These were retained for configuration compatibility by the
            // historical virtual transport but were not applied to the TUN.
            "address" | "mask" => {
                if value.parse::<IpAddr>().is_err() {
                    return false;
                }
            }
            _ => return false,
        }
    }
    if state.kind == BuiltinKind::Phy {
        if state.phy.max_segment_duration < state.phy.noise_bin_size
            || state
                .phy
                .max_segment_offset
                .saturating_add(state.phy.max_message_propagation)
                .saturating_add(state.phy.max_segment_duration.saturating_mul(2))
                % state.phy.noise_bin_size
                != 0
            || state.phy.nakagami_parameters.distance0_meters
                >= state.phy.nakagami_parameters.distance1_meters
            || state.phy.lognormal_parameters.dlthresh > state.phy.lognormal_parameters.duthresh
            || state.phy.lognormal_parameters.minpathloss
                >= state.phy.lognormal_parameters.maxpathloss
        {
            return false;
        }
        state.phy.initialize_monitor();
    }
    true
}

extern "C" fn builtin_start(state: *mut c_void) -> bool {
    let Some(state) = (unsafe { (state as *mut BuiltinState).as_mut() }) else {
        return false;
    };
    let Ok(name) = CString::new(state.device_name.as_str()) else {
        return false;
    };
    match state.kind {
        BuiltinKind::Phy => true,
        BuiltinKind::VirtualTransport => {
            let Ok(path) = CString::new(state.device_path.as_str()) else {
                return false;
            };
            emane_rs_virtual_transport_start(
                state.virtual_transport,
                path.as_ptr(),
                name.as_ptr(),
                state.arp_mode,
            ) == 0
        }
        BuiltinKind::RawTransport => {
            emane_rs_raw_transport_start(state.raw_transport, name.as_ptr()) == 0
        }
    }
}

extern "C" fn builtin_post_start(_: *mut c_void) {}

extern "C" fn builtin_stop(state: *mut c_void) {
    if let Some(state) = unsafe { (state as *mut BuiltinState).as_mut() } {
        state.flow_control_tokens = 0;
        state.pending_transport_frames.clear();
        match state.kind {
            BuiltinKind::VirtualTransport => {
                emane_rs_virtual_transport_stop(state.virtual_transport)
            }
            BuiltinKind::RawTransport => emane_rs_raw_transport_stop(state.raw_transport),
            BuiltinKind::Phy => {}
        }
    }
}

extern "C" fn builtin_destroy(state: *mut c_void) {
    if !state.is_null() {
        let state = unsafe { Box::from_raw(state as *mut BuiltinState) };
        match state.kind {
            BuiltinKind::VirtualTransport => {
                emane_rs_virtual_transport_free(state.virtual_transport)
            }
            BuiltinKind::RawTransport => emane_rs_raw_transport_free(state.raw_transport),
            BuiltinKind::Phy => {}
        }
    }
}

extern "C" fn builtin_query_unknown(
    state: *const c_void,
    ether_type: u16,
    priority: *mut u8,
) -> bool {
    let (Some(state), Some(priority)) =
        (unsafe { (state as *const BuiltinState).as_ref() }, unsafe {
            priority.as_mut()
        })
    else {
        return false;
    };
    if let Some(value) = state.unknown_priorities.get(&ether_type) {
        *priority = *value;
        true
    } else {
        false
    }
}

fn pacing_duration(payload_len: usize, bitrate_bps: u64) -> Duration {
    if bitrate_bps == 0 || payload_len == 0 {
        return Duration::ZERO;
    }
    let nanos = (payload_len as u128)
        .saturating_mul(8_000_000_000)
        .div_ceil(u128::from(bitrate_bps))
        .min(u128::from(u64::MAX)) as u64;
    Duration::from_nanos(nanos)
}

fn pace_transport(payload_len: usize, bitrate_bps: u64) {
    let duration = pacing_duration(payload_len, bitrate_bps);
    if !duration.is_zero() {
        std::thread::sleep(duration);
    }
}

fn send_transport_frame(
    framework: FfiFrameworkService,
    id: u16,
    bitrate_bps: u64,
    counters: CommonLayerCounters,
    frame: &PendingTransportFrame,
) {
    let packet = FfiPacket {
        info: FfiPacketInfo {
            source: id,
            destination: frame.destination,
            priority: frame.priority,
            creation_time_sec: frame.creation_time_sec,
            creation_time_usec: frame.creation_time_usec,
        },
        payload: FfiSlice {
            data: frame.payload.as_ptr(),
            len: frame.payload.len(),
        },
    };
    (framework.send_downstream_packet)(framework.framework_ctx, id, &packet, std::ptr::null(), 0);
    counters.downstream_tx(framework, frame.destination, frame.payload.len());
    pace_transport(frame.payload.len(), bitrate_bps);
}

fn release_transport_frames(
    enabled: bool,
    available_tokens: &mut u16,
    pending: &mut VecDeque<PendingTransportFrame>,
    update: FlowControlToken,
) -> Vec<PendingTransportFrame> {
    if !enabled {
        return Vec::new();
    }
    *available_tokens = update.tokens;
    let mut frames = Vec::new();
    while *available_tokens != 0 {
        let Some(frame) = pending.pop_front() else {
            break;
        };
        *available_tokens -= 1;
        frames.push(frame);
    }
    frames
}

extern "C" fn builtin_ethernet_downstream(context: *mut c_void, data: *const u8, len: usize) {
    let Some(state) = (unsafe { (context as *mut BuiltinState).as_mut() }) else {
        return;
    };
    if (len != 0 && data.is_null())
        || emane_rs_ethernet_transport_verify_frame(data.cast(), len) < 0
    {
        return;
    }
    let mut destination = BROADCAST_NEM;
    let mut priority = 0;
    let status = emane_rs_ethernet_transport_parse_frame(
        state.ethernet.as_ref(),
        data.cast(),
        len,
        state.broadcast_mode,
        state.arp_cache_mode,
        state.arp_priority,
        state as *const BuiltinState as *const c_void,
        builtin_query_unknown,
        &mut destination,
        &mut priority,
    );
    if status < 0 {
        return;
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let frame = PendingTransportFrame {
        payload: if len == 0 {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(data, len) }.to_vec()
        },
        destination,
        priority,
        creation_time_sec: now.as_secs(),
        creation_time_usec: now.subsec_micros(),
    };
    state
        .counters
        .downstream_rx(state.framework, destination, frame.payload.len());
    if state.flow_control_enabled {
        if state.flow_control_tokens == 0 {
            state.pending_transport_frames.push_back(frame);
            return;
        }
        state.flow_control_tokens -= 1;
    }
    let framework = state.framework;
    let id = state.id;
    let bitrate_bps = state.bitrate_bps;
    send_transport_frame(framework, id, bitrate_bps, state.counters, &frame);
}

extern "C" fn builtin_upstream(
    state: *mut c_void,
    packet: *const FfiPacket,
    messages: *const FfiControlMessage,
    count: usize,
) {
    let Some(state) = (unsafe { (state as *mut BuiltinState).as_mut() }) else {
        return;
    };
    if packet.is_null()
        && matches!(
            state.kind,
            BuiltinKind::VirtualTransport | BuiltinKind::RawTransport
        )
    {
        let Some(incoming) = ffi_control_messages(messages, count) else {
            return;
        };
        let Some(token) = control_payload(incoming, CONTROL_FLOW_CONTROL_TOKEN)
            .and_then(FlowControlToken::decode)
        else {
            return;
        };
        let frames = release_transport_frames(
            state.flow_control_enabled,
            &mut state.flow_control_tokens,
            &mut state.pending_transport_frames,
            token,
        );
        let framework = state.framework;
        let id = state.id;
        let bitrate_bps = state.bitrate_bps;
        for frame in &frames {
            send_transport_frame(framework, id, bitrate_bps, state.counters, frame);
        }
        return;
    }
    match state.kind {
        BuiltinKind::Phy => {
            let Some(packet) = (unsafe { packet.as_ref() }) else {
                (state.framework.send_upstream_control)(
                    state.framework.framework_ctx,
                    state.id,
                    messages,
                    count,
                );
                return;
            };
            state.counters.upstream_rx(
                state.framework,
                packet.info.destination,
                packet.payload.len,
            );
            let Some(incoming) = ffi_control_messages(messages, count) else {
                state
                    .counters
                    .upstream_drop(state.framework, packet.info.destination);
                return;
            };
            let Some(tx) = find_tx_properties(incoming) else {
                // Keep bypass and third-party MAC plugins interoperable.
                (state.framework.send_upstream_packet)(
                    state.framework.framework_ctx,
                    state.id,
                    packet,
                    messages,
                    count,
                );
                state.counters.upstream_tx(
                    state.framework,
                    packet.info.destination,
                    packet.payload.len,
                );
                return;
            };

            let now = unix_time_microseconds();
            let mimo_tx = (state.phy.compatibility_mode == 2)
                .then(|| find_mimo_tx_properties(incoming))
                .flatten();
            let (frequency_segments, segment_tx_powers, mimo_ranges) = if let Some(mimo) = &mimo_tx
            {
                let mut segments = Vec::new();
                let mut powers = Vec::new();
                let mut ranges = Vec::new();
                for antenna in &mimo.transmit_antennas {
                    let start = segments.len();
                    let group = &mimo.frequency_groups[usize::from(antenna.frequency_group_index)];
                    for segment in group {
                        segments.push(TxFrequencySegment {
                            frequency_hz: segment.frequency_hz,
                            duration_microseconds: segment.duration_microseconds,
                            offset_microseconds: segment.offset_microseconds,
                        });
                        powers.push(Some(segment.tx_power_dbm));
                    }
                    ranges.push((*antenna, start, segments.len()));
                }
                (TxFrequencySegments { segments }, powers, ranges)
            } else {
                let segments =
                    find_tx_frequency_segments(incoming).unwrap_or(TxFrequencySegments {
                        segments: vec![TxFrequencySegment {
                            frequency_hz: tx.frequency_hz,
                            duration_microseconds: tx.duration_microseconds,
                            offset_microseconds: tx.offset_microseconds,
                        }],
                    });
                let powers = vec![None; segments.segments.len()];
                (segments, powers, Vec::new())
            };
            let transmitters = find_tx_transmitters(incoming).unwrap_or(TxTransmitters {
                transmitters: vec![TxTransmitter {
                    nem_id: packet.info.source,
                    tx_power_dbm: tx.tx_power_dbm,
                }],
            });
            let antenna_profile = find_tx_antenna_profile(incoming);
            let mut propagation_microseconds = i64::MAX;
            let mut rx_powers_mw = vec![0.0; frequency_segments.segments.len()];
            let mut valid_path = false;
            for transmitter in &transmitters.transmitters {
                for (index, frequency_segment) in frequency_segments.segments.iter().enumerate() {
                    let antenna_gain_db = if let Some((transmit_antenna, _, _)) = mimo_ranges
                        .iter()
                        .find(|(_, start, end)| *start <= index && index < *end)
                    {
                        let local_patterns = if state.phy.receive_antennas.is_empty() {
                            vec![state
                                .phy
                                .default_antenna_pattern(state.id)
                                .unwrap_or(AntennaPattern::Default)]
                        } else {
                            state
                                .phy
                                .receive_antennas
                                .values()
                                .map(|entry| entry.antenna.pattern)
                                .collect()
                        };
                        local_patterns
                            .into_iter()
                            .filter_map(|local_pattern| {
                                state.phy.antenna_pair_gain(
                                    state.id,
                                    transmitter.nem_id,
                                    local_pattern,
                                    transmit_antenna.pattern,
                                )
                            })
                            .max_by(f64::total_cmp)
                    } else {
                        state.phy.profile_gain_with_remote(
                            state.id,
                            transmitter.nem_id,
                            antenna_profile,
                        )
                    };
                    let Some(antenna_gain_db) = antenna_gain_db else {
                        continue;
                    };
                    let Some((pathloss_db, propagation)) = state.phy.propagation(
                        state.id,
                        transmitter.nem_id,
                        frequency_segment.frequency_hz,
                    ) else {
                        continue;
                    };
                    let transmit_power_dbm =
                        segment_tx_powers[index].unwrap_or(transmitter.tx_power_dbm);
                    let unfaded_power_dbm = transmit_power_dbm - pathloss_db + antenna_gain_db;
                    let Some(rx_power_dbm) = state.phy.apply_fading(
                        state.id,
                        transmitter.nem_id,
                        unfaded_power_dbm,
                        now.max(0) as u64,
                    ) else {
                        continue;
                    };
                    rx_powers_mw[index] += 10.0f64.powf(rx_power_dbm / 10.0);
                    propagation_microseconds = propagation_microseconds.min(propagation);
                    valid_path = true;
                }
            }
            if !valid_path || rx_powers_mw.iter().any(|power| *power <= 0.0) {
                state
                    .counters
                    .upstream_drop(state.framework, packet.info.destination);
                return;
            }
            let doppler_fraction = state.phy.doppler_fraction(state.id, packet.info.source);
            let segments: Vec<_> = frequency_segments
                .segments
                .iter()
                .zip(&rx_powers_mw)
                .map(|(segment, rx_power_mw)| FfiFrequencySegment {
                    frequency_hz: segment.frequency_hz,
                    rx_power_dbm: 10.0 * rx_power_mw.log10(),
                    duration_microsec: i64::try_from(segment.duration_microseconds)
                        .unwrap_or(i64::MAX),
                    offset_microsec: i64::try_from(segment.offset_microseconds).unwrap_or(i64::MAX),
                })
                .collect();
            let is_in_band = tx.sub_id == state.phy.sub_id
                && segments.iter().any(|segment| {
                    state
                        .phy
                        .frequencies_of_interest
                        .contains(&segment.frequency_hz)
                });
            let transmitter_ids: Vec<_> = transmitters
                .transmitters
                .iter()
                .map(|transmitter| transmitter.nem_id)
                .collect();
            let (tx_time, propagation, duration, report, report_in_band, sensitivity_mw) =
                state.phy.monitor.update(
                    now,
                    tx.tx_time_microseconds,
                    propagation_microseconds,
                    doppler_fraction,
                    &segments,
                    tx.bandwidth_hz,
                    &rx_powers_mw,
                    is_in_band,
                    &transmitter_ids,
                    tx.sub_id,
                    tx.antenna_index,
                    tx.spectral_mask_index,
                    std::ptr::null(),
                    0,
                );
            if report.is_empty() || !report_in_band {
                state
                    .counters
                    .upstream_drop(state.framework, packet.info.destination);
                return;
            }
            let noise_floor_dbm = reception_noise_floor_dbm(
                &state.phy.monitor,
                now,
                tx_time,
                propagation,
                duration,
                &report[0],
            );

            let rx = RxProperties {
                frequency_hz: report[0].frequency_hz,
                bandwidth_hz: tx.bandwidth_hz,
                rx_power_dbm: report[0].rx_power_dbm,
                noise_floor_dbm,
                tx_time_microseconds: tx_time,
                propagation_microseconds: propagation.max(0) as u64,
                duration_microseconds: duration.max(0) as u64,
                antenna_index: tx.antenna_index,
                sub_id: tx.sub_id,
                signal_in_noise: state.phy.noise_mode == NoiseMode::All,
            };
            let rx_bytes = rx.encode();
            let rx_segment_bytes = RxFrequencySegments {
                segments: report
                    .iter()
                    .map(|segment| RxFrequencySegment {
                        frequency_hz: segment.frequency_hz,
                        rx_power_dbm: segment.rx_power_dbm,
                        duration_microseconds: segment.duration_microsec.max(0) as u64,
                        offset_microseconds: segment.offset_microsec.max(0) as u64,
                    })
                    .collect(),
            }
            .encode();
            let mimo_rx_bytes = (!mimo_ranges.is_empty())
                .then(|| {
                    let receive_antennas = if state.phy.receive_antennas.is_empty() {
                        vec![(
                            0,
                            MimoTxAntenna {
                                frequency_group_index: 0,
                                antenna_index: 0,
                                bandwidth_hz: tx.bandwidth_hz,
                                spectral_mask_index: 0,
                                pattern: state
                                    .phy
                                    .default_antenna_pattern(state.id)
                                    .unwrap_or(AntennaPattern::Default),
                            },
                            state.phy.frequencies_of_interest.clone(),
                        )]
                    } else {
                        let mut antennas = state
                            .phy
                            .receive_antennas
                            .iter()
                            .map(|(index, entry)| {
                                (*index, entry.antenna, entry.frequencies_hz.clone())
                            })
                            .collect::<Vec<_>>();
                        antennas.sort_by_key(|entry| entry.0);
                        antennas
                    };
                    let mut antenna_infos = Vec::new();
                    for (receive_index, receive_antenna, receive_frequencies) in receive_antennas {
                        for (transmit_antenna, start, end) in &mimo_ranges {
                            let source_segments = &frequency_segments.segments[*start..*end];
                            let mut powers_mw = vec![0.0; source_segments.len()];
                            let mut combination_propagation = i64::MAX;
                            for transmitter in &transmitters.transmitters {
                                let Some(gain_db) = state.phy.antenna_pair_gain(
                                    state.id,
                                    transmitter.nem_id,
                                    receive_antenna.pattern,
                                    transmit_antenna.pattern,
                                ) else {
                                    continue;
                                };
                                for (segment_index, segment) in source_segments.iter().enumerate() {
                                    let Some((pathloss_db, propagation)) = state.phy.propagation(
                                        state.id,
                                        transmitter.nem_id,
                                        segment.frequency_hz,
                                    ) else {
                                        continue;
                                    };
                                    let tx_power_dbm = mimo_tx.as_ref().unwrap().frequency_groups
                                        [usize::from(transmit_antenna.frequency_group_index)]
                                        [segment_index]
                                        .tx_power_dbm;
                                    let Some(power_dbm) = state.phy.apply_fading(
                                        state.id,
                                        transmitter.nem_id,
                                        tx_power_dbm - pathloss_db + gain_db,
                                        now.max(0) as u64,
                                    ) else {
                                        continue;
                                    };
                                    powers_mw[segment_index] += 10.0f64.powf(power_dbm / 10.0);
                                    combination_propagation =
                                        combination_propagation.min(propagation);
                                }
                            }
                            if powers_mw.iter().any(|power| *power <= 0.0) {
                                continue;
                            }
                            let ffi_segments = source_segments
                                .iter()
                                .zip(&powers_mw)
                                .map(|(segment, power)| FfiFrequencySegment {
                                    frequency_hz: segment.frequency_hz,
                                    rx_power_dbm: 10.0 * power.log10(),
                                    duration_microsec: i64::try_from(segment.duration_microseconds)
                                        .unwrap_or(i64::MAX),
                                    offset_microsec: i64::try_from(segment.offset_microseconds)
                                        .unwrap_or(i64::MAX),
                                })
                                .collect::<Vec<_>>();
                            let in_band = tx.sub_id == state.phy.sub_id
                                && source_segments.iter().any(|segment| {
                                    receive_frequencies.contains(&segment.frequency_hz)
                                });
                            let (info_report, info_sensitivity_mw, info_noise_floor_dbm) =
                                if receive_index == 0
                                    && !state.phy.receive_antennas.contains_key(&receive_index)
                                {
                                    (ffi_segments, sensitivity_mw, noise_floor_dbm)
                                } else {
                                    let entry =
                                        state.phy.receive_antennas.get_mut(&receive_index)?;
                                    let (
                                        info_tx_time,
                                        info_propagation,
                                        info_span,
                                        report,
                                        report_in_band,
                                        sensitivity,
                                    ) = entry.monitor.update(
                                        now,
                                        tx.tx_time_microseconds,
                                        combination_propagation,
                                        doppler_fraction,
                                        &ffi_segments,
                                        transmit_antenna.bandwidth_hz,
                                        &powers_mw,
                                        in_band,
                                        &transmitter_ids,
                                        tx.sub_id,
                                        transmit_antenna.antenna_index,
                                        transmit_antenna.spectral_mask_index,
                                        std::ptr::null(),
                                        0,
                                    );
                                    if !report_in_band || report.is_empty() {
                                        continue;
                                    }
                                    let noise_floor = reception_noise_floor_dbm(
                                        &entry.monitor,
                                        now,
                                        info_tx_time,
                                        info_propagation,
                                        info_span,
                                        &report[0],
                                    );
                                    (report, sensitivity, noise_floor)
                                };
                            let first_offset = source_segments
                                .iter()
                                .map(|segment| segment.offset_microseconds)
                                .min()
                                .unwrap_or(0);
                            let span_microseconds = source_segments
                                .iter()
                                .map(|segment| {
                                    segment
                                        .offset_microseconds
                                        .saturating_add(segment.duration_microseconds)
                                })
                                .max()
                                .unwrap_or(first_offset)
                                .saturating_sub(first_offset);
                            antenna_infos.push(MimoRxAntennaInfo {
                                receive_antenna_index: receive_index,
                                transmit_antenna_index: transmit_antenna.antenna_index,
                                span_microseconds,
                                receiver_sensitivity_dbm: 10.0 * info_sensitivity_mw.log10(),
                                noise_floor_dbm: info_noise_floor_dbm,
                                segments: info_report
                                    .iter()
                                    .map(|segment| RxFrequencySegment {
                                        frequency_hz: segment.frequency_hz,
                                        rx_power_dbm: segment.rx_power_dbm,
                                        duration_microseconds: segment.duration_microsec.max(0)
                                            as u64,
                                        offset_microseconds: segment.offset_microsec.max(0) as u64,
                                    })
                                    .collect(),
                            });
                        }
                    }
                    if antenna_infos.is_empty() {
                        return None;
                    }
                    let mut doppler_shifts_hz = frequency_segments
                        .segments
                        .iter()
                        .map(|segment| {
                            (
                                segment.frequency_hz,
                                (segment.frequency_hz as f64 * doppler_fraction).round() as i64,
                            )
                        })
                        .collect::<Vec<_>>();
                    doppler_shifts_hz.sort_unstable_by_key(|entry| entry.0);
                    doppler_shifts_hz.dedup_by_key(|entry| entry.0);
                    Some(MimoRxProperties {
                        tx_time_microseconds: tx_time,
                        propagation_microseconds: propagation.max(0) as u64,
                        antenna_infos,
                        doppler_shifts_hz,
                    })
                })
                .flatten()
                .and_then(|properties| properties.encode());
            let mut outgoing: Vec<_> = incoming
                .iter()
                .copied()
                .filter(|message| {
                    message.msg_type != CONTROL_RX_PROPERTIES
                        && message.msg_type != CONTROL_RX_FREQUENCY_SEGMENTS
                        && message.msg_type != CONTROL_MIMO_RX_PROPERTIES
                })
                .collect();
            outgoing.push(FfiControlMessage {
                msg_type: CONTROL_RX_PROPERTIES,
                payload: FfiSlice {
                    data: rx_bytes.as_ptr(),
                    len: rx_bytes.len(),
                },
            });
            if let Some(bytes) = &rx_segment_bytes {
                outgoing.push(FfiControlMessage {
                    msg_type: CONTROL_RX_FREQUENCY_SEGMENTS,
                    payload: FfiSlice {
                        data: bytes.as_ptr(),
                        len: bytes.len(),
                    },
                });
            }
            if let Some(bytes) = &mimo_rx_bytes {
                outgoing.push(FfiControlMessage {
                    msg_type: CONTROL_MIMO_RX_PROPERTIES,
                    payload: FfiSlice {
                        data: bytes.as_ptr(),
                        len: bytes.len(),
                    },
                });
            }
            (state.framework.send_upstream_packet)(
                state.framework.framework_ctx,
                state.id,
                packet,
                outgoing.as_ptr(),
                outgoing.len(),
            );
            state.counters.upstream_tx(
                state.framework,
                packet.info.destination,
                packet.payload.len,
            );
        }
        BuiltinKind::VirtualTransport | BuiltinKind::RawTransport if !packet.is_null() => {
            let packet = unsafe { &*packet };
            state.counters.upstream_rx(
                state.framework,
                packet.info.destination,
                packet.payload.len,
            );
            if packet.payload.len != 0 && packet.payload.data.is_null() {
                state
                    .counters
                    .upstream_drop(state.framework, packet.info.destination);
                return;
            }
            emane_rs_ethernet_transport_update_arp_cache(
                state.ethernet.as_ref(),
                packet.payload.data.cast(),
                packet.payload.len,
                packet.info.source,
                state.broadcast_mode,
                state.arp_cache_mode,
            );
            match state.kind {
                BuiltinKind::VirtualTransport => {
                    let _ = emane_rs_virtual_transport_process_upstream_packet(
                        state.virtual_transport,
                        packet.payload.data,
                        packet.payload.len,
                    );
                }
                BuiltinKind::RawTransport => {
                    let _ = emane_rs_raw_transport_process_upstream_packet(
                        state.raw_transport,
                        packet.payload.data,
                        packet.payload.len,
                    );
                }
                BuiltinKind::Phy => {}
            }
            state.counters.upstream_tx(
                state.framework,
                packet.info.destination,
                packet.payload.len,
            );
            pace_transport(packet.payload.len, state.bitrate_bps);
        }
        _ => {}
    }
}

extern "C" fn builtin_downstream(
    state: *mut c_void,
    packet: *const FfiPacket,
    messages: *const FfiControlMessage,
    count: usize,
) {
    let Some(state) = (unsafe { (state as *mut BuiltinState).as_mut() }) else {
        return;
    };
    if state.kind == BuiltinKind::Phy {
        let Some(incoming) = ffi_control_messages(messages, count) else {
            return;
        };
        if let Some(foi) = incoming.iter().find_map(|message| {
            if message.msg_type != CONTROL_FREQUENCY_INTEREST || message.payload.data.is_null() {
                return None;
            }
            FrequencyOfInterest::decode(unsafe {
                std::slice::from_raw_parts(message.payload.data, message.payload.len)
            })
        }) {
            state.phy.bandwidth_hz = foi.bandwidth_hz;
            state.phy.frequencies_of_interest = foi.frequencies_hz;
            state.phy.initialize_monitor();
        }
        for message in incoming {
            let payload = if message.payload.len == 0 || message.payload.data.is_null() {
                &[][..]
            } else {
                unsafe { std::slice::from_raw_parts(message.payload.data, message.payload.len) }
            };
            match message.msg_type {
                CONTROL_RX_ANTENNA_ADD | CONTROL_RX_ANTENNA_UPDATE => {
                    let Some(add) = RxAntennaAdd::decode(payload) else {
                        continue;
                    };
                    let monitor = state
                        .phy
                        .make_monitor(&add.frequencies_hz, add.antenna.bandwidth_hz);
                    state.phy.receive_antennas.insert(
                        add.antenna.antenna_index,
                        ReceiveAntennaState {
                            antenna: add.antenna,
                            frequencies_hz: add.frequencies_hz,
                            monitor,
                        },
                    );
                    state.phy.frequencies_of_interest = state
                        .phy
                        .receive_antennas
                        .values()
                        .flat_map(|entry| entry.frequencies_hz.iter().copied())
                        .collect();
                    state.phy.frequencies_of_interest.sort_unstable();
                    state.phy.frequencies_of_interest.dedup();
                    state.phy.bandwidth_hz = add.antenna.bandwidth_hz;
                    state.phy.initialize_monitor();
                }
                CONTROL_RX_ANTENNA_REMOVE => {
                    if let Some(remove) = RxAntennaRemove::decode(payload) {
                        state.phy.receive_antennas.remove(&remove.antenna_index);
                    }
                }
                _ => {}
            }
        }
        if packet.is_null() {
            return;
        }
        let packet = unsafe { &*packet };
        state
            .counters
            .downstream_rx(state.framework, packet.info.destination, packet.payload.len);
        if state.phy.radio_silence_enabled {
            state
                .counters
                .downstream_drop(state.framework, packet.info.destination);
            return;
        }
        let now = unix_time_microseconds();
        let mut tx = find_tx_properties(incoming).unwrap_or(TxProperties {
            frequency_hz: state.phy.frequency_hz,
            bandwidth_hz: state.phy.bandwidth_hz,
            tx_power_dbm: state.phy.tx_power_dbm,
            duration_microseconds: 1,
            offset_microseconds: 0,
            tx_time_microseconds: now,
            antenna_index: 0,
            spectral_mask_index: 0,
            sub_id: state.phy.sub_id,
        });
        if tx.frequency_hz == 0 {
            tx.frequency_hz = state.phy.frequency_hz;
        }
        if tx.bandwidth_hz == 0 {
            tx.bandwidth_hz = state.phy.bandwidth_hz;
        }
        if !tx.tx_power_dbm.is_finite() {
            tx.tx_power_dbm = state.phy.tx_power_dbm;
        }
        if tx.duration_microseconds == 0 {
            tx.duration_microseconds = 1;
        }
        if tx.tx_time_microseconds == 0 {
            tx.tx_time_microseconds = now;
        }
        if tx.sub_id == 0 {
            tx.sub_id = state.phy.sub_id;
        }
        if state.phy.fixed_antenna_gain_enabled {
            tx.tx_power_dbm += state.phy.fixed_antenna_gain_db;
        }
        tx.spectral_mask_index = state.phy.spectral_mask_index;

        let normalized_segments =
            find_tx_frequency_segments(incoming).map(|segments| TxFrequencySegments {
                segments: segments
                    .segments
                    .into_iter()
                    .map(|segment| TxFrequencySegment {
                        frequency_hz: if segment.frequency_hz == 0 {
                            tx.frequency_hz
                        } else {
                            segment.frequency_hz
                        },
                        duration_microseconds: segment.duration_microseconds.max(1),
                        offset_microseconds: segment.offset_microseconds,
                    })
                    .collect(),
            });
        if let Some(segments) = &normalized_segments {
            let first = segments.segments[0];
            tx.frequency_hz = first.frequency_hz;
            tx.offset_microseconds = segments
                .segments
                .iter()
                .map(|segment| segment.offset_microseconds)
                .min()
                .unwrap_or(0);
            tx.duration_microseconds = segments
                .segments
                .iter()
                .map(|segment| {
                    segment
                        .offset_microseconds
                        .saturating_add(segment.duration_microseconds)
                })
                .max()
                .unwrap_or(1)
                .saturating_sub(tx.offset_microseconds)
                .max(1);
        }

        let normalized_mimo = (state.phy.compatibility_mode == 2).then(|| {
            find_mimo_tx_properties(incoming)
                .map(|mut mimo| {
                    for group in &mut mimo.frequency_groups {
                        for segment in group {
                            if segment.frequency_hz == 0 {
                                segment.frequency_hz = tx.frequency_hz;
                            }
                            segment.duration_microseconds = segment.duration_microseconds.max(1);
                        }
                    }
                    for antenna in &mut mimo.transmit_antennas {
                        if antenna.bandwidth_hz == 0 {
                            antenna.bandwidth_hz = tx.bandwidth_hz;
                        }
                        if antenna.spectral_mask_index == 0 {
                            antenna.spectral_mask_index = state.phy.spectral_mask_index;
                        }
                        if antenna.pattern == AntennaPattern::Default {
                            if let Some(pattern) = state.phy.default_antenna_pattern(state.id) {
                                antenna.pattern = pattern;
                            }
                        }
                    }
                    mimo
                })
                .unwrap_or_else(|| {
                    let group = normalized_segments.as_ref().map_or_else(
                        || {
                            vec![MimoTxFrequencySegment {
                                frequency_hz: tx.frequency_hz,
                                tx_power_dbm: tx.tx_power_dbm,
                                duration_microseconds: tx.duration_microseconds,
                                offset_microseconds: tx.offset_microseconds,
                            }]
                        },
                        |segments| {
                            segments
                                .segments
                                .iter()
                                .map(|segment| MimoTxFrequencySegment {
                                    frequency_hz: segment.frequency_hz,
                                    tx_power_dbm: tx.tx_power_dbm,
                                    duration_microseconds: segment.duration_microseconds,
                                    offset_microseconds: segment.offset_microseconds,
                                })
                                .collect()
                        },
                    );
                    MimoTxProperties {
                        frequency_groups: vec![group],
                        transmit_antennas: vec![MimoTxAntenna {
                            frequency_group_index: 0,
                            antenna_index: tx.antenna_index,
                            bandwidth_hz: tx.bandwidth_hz,
                            spectral_mask_index: tx.spectral_mask_index,
                            pattern: state
                                .phy
                                .default_antenna_pattern(state.id)
                                .unwrap_or(AntennaPattern::Default),
                        }],
                    }
                })
        });
        if let Some(mimo) = &normalized_mimo {
            let antenna = mimo.transmit_antennas[0];
            let group = &mimo.frequency_groups[usize::from(antenna.frequency_group_index)];
            let first = group[0];
            tx.frequency_hz = first.frequency_hz;
            tx.tx_power_dbm = first.tx_power_dbm;
            tx.antenna_index = antenna.antenna_index;
            tx.bandwidth_hz = antenna.bandwidth_hz;
            tx.spectral_mask_index = antenna.spectral_mask_index;
            tx.offset_microseconds = group
                .iter()
                .map(|segment| segment.offset_microseconds)
                .min()
                .unwrap_or(0);
            tx.duration_microseconds = group
                .iter()
                .map(|segment| {
                    segment
                        .offset_microseconds
                        .saturating_add(segment.duration_microseconds)
                })
                .max()
                .unwrap_or(1)
                .saturating_sub(tx.offset_microseconds)
                .max(1);
        }

        let normalized_transmitters = find_tx_transmitters(incoming).map(|transmitters| {
            let mut transmitters = transmitters.transmitters;
            if state.phy.fixed_antenna_gain_enabled {
                for transmitter in &mut transmitters {
                    transmitter.tx_power_dbm += state.phy.fixed_antenna_gain_db;
                }
            }
            if !transmitters
                .iter()
                .any(|transmitter| transmitter.nem_id == state.id)
            {
                transmitters.push(TxTransmitter {
                    nem_id: state.id,
                    tx_power_dbm: tx.tx_power_dbm,
                });
            }
            TxTransmitters { transmitters }
        });

        let tx_bytes = tx.encode();
        let segment_bytes = normalized_segments
            .as_ref()
            .and_then(TxFrequencySegments::encode);
        let transmitter_bytes = normalized_transmitters
            .as_ref()
            .and_then(TxTransmitters::encode);
        let mimo_bytes = normalized_mimo.as_ref().and_then(MimoTxProperties::encode);
        let mut outgoing: Vec<_> = incoming
            .iter()
            .copied()
            .filter(|message| {
                message.msg_type != CONTROL_TX_PROPERTIES
                    && message.msg_type != CONTROL_TX_FREQUENCY_SEGMENTS
                    && message.msg_type != CONTROL_TX_TRANSMITTERS
                    && message.msg_type != CONTROL_MIMO_TX_PROPERTIES
            })
            .collect();
        outgoing.push(FfiControlMessage {
            msg_type: CONTROL_TX_PROPERTIES,
            payload: FfiSlice {
                data: tx_bytes.as_ptr(),
                len: tx_bytes.len(),
            },
        });
        if let Some(bytes) = &segment_bytes {
            outgoing.push(FfiControlMessage {
                msg_type: CONTROL_TX_FREQUENCY_SEGMENTS,
                payload: FfiSlice {
                    data: bytes.as_ptr(),
                    len: bytes.len(),
                },
            });
        }
        if let Some(bytes) = &transmitter_bytes {
            outgoing.push(FfiControlMessage {
                msg_type: CONTROL_TX_TRANSMITTERS,
                payload: FfiSlice {
                    data: bytes.as_ptr(),
                    len: bytes.len(),
                },
            });
        }
        if let Some(bytes) = &mimo_bytes {
            outgoing.push(FfiControlMessage {
                msg_type: CONTROL_MIMO_TX_PROPERTIES,
                payload: FfiSlice {
                    data: bytes.as_ptr(),
                    len: bytes.len(),
                },
            });
        }
        (state.framework.send_downstream_packet)(
            state.framework.framework_ctx,
            state.id,
            packet,
            outgoing.as_ptr(),
            outgoing.len(),
        );
        state
            .counters
            .downstream_tx(state.framework, packet.info.destination, packet.payload.len);
        return;
    }
    if let Some(packet) = unsafe { packet.as_ref() } {
        state
            .counters
            .downstream_rx(state.framework, packet.info.destination, packet.payload.len);
    }
    (state.framework.send_downstream_packet)(
        state.framework.framework_ctx,
        state.id,
        packet,
        messages,
        count,
    );
    if let Some(packet) = unsafe { packet.as_ref() } {
        state
            .counters
            .downstream_tx(state.framework, packet.info.destination, packet.payload.len);
    }
}

fn ffi_control_messages<'a>(
    messages: *const FfiControlMessage,
    count: usize,
) -> Option<&'a [FfiControlMessage]> {
    if count > 4096 || (count != 0 && messages.is_null()) {
        None
    } else if count == 0 {
        Some(&[])
    } else {
        Some(unsafe { std::slice::from_raw_parts(messages, count) })
    }
}

fn find_tx_properties(messages: &[FfiControlMessage]) -> Option<TxProperties> {
    messages.iter().find_map(|message| {
        if message.msg_type != CONTROL_TX_PROPERTIES
            || message.payload.len != TxProperties::ENCODED_LEN
            || message.payload.data.is_null()
        {
            return None;
        }
        TxProperties::decode(unsafe {
            std::slice::from_raw_parts(message.payload.data, message.payload.len)
        })
    })
}

fn control_payload(messages: &[FfiControlMessage], kind: u32) -> Option<&[u8]> {
    messages.iter().find_map(|message| {
        if message.msg_type != kind
            || message.payload.len > MAX_CONTROL_WIRE_SIZE
            || (message.payload.len != 0 && message.payload.data.is_null())
        {
            None
        } else if message.payload.len == 0 {
            Some(&[] as &[u8])
        } else {
            Some(unsafe { std::slice::from_raw_parts(message.payload.data, message.payload.len) })
        }
    })
}

fn find_tx_frequency_segments(messages: &[FfiControlMessage]) -> Option<TxFrequencySegments> {
    control_payload(messages, CONTROL_TX_FREQUENCY_SEGMENTS).and_then(TxFrequencySegments::decode)
}

fn find_tx_transmitters(messages: &[FfiControlMessage]) -> Option<TxTransmitters> {
    control_payload(messages, CONTROL_TX_TRANSMITTERS).and_then(TxTransmitters::decode)
}

fn find_tx_antenna_profile(messages: &[FfiControlMessage]) -> Option<TxAntennaProfile> {
    control_payload(messages, CONTROL_TX_ANTENNA_PROFILE).and_then(TxAntennaProfile::decode)
}

fn find_mimo_tx_properties(messages: &[FfiControlMessage]) -> Option<MimoTxProperties> {
    control_payload(messages, CONTROL_MIMO_TX_PROPERTIES).and_then(MimoTxProperties::decode)
}

fn unix_time_microseconds() -> i64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    i64::try_from(now.as_micros()).unwrap_or(i64::MAX)
}

fn reception_noise_floor_dbm(
    monitor: &SpectrumMonitor,
    now: i64,
    tx_time: i64,
    propagation: i64,
    span: i64,
    segment: &FfiFrequencySegment,
) -> f64 {
    let start = tx_time
        .saturating_add(propagation)
        .saturating_add(segment.offset_microsec);
    let duration = span.max(segment.duration_microsec).max(1);
    let query_time = now.max(start.saturating_add(duration));
    let (bins, _, _, sensitivity_mw, signal_in_noise) =
        monitor.request_i(query_time, segment.frequency_hz, duration, start);
    let max_mw = bins.into_iter().fold(sensitivity_mw, f64::max);
    let signal_mw = 10.0f64.powf(segment.rx_power_dbm / 10.0);
    let noise_mw = if signal_in_noise {
        (max_mw - signal_mw).max(sensitivity_mw)
    } else {
        max_mw.max(sensitivity_mw)
    };
    10.0 * noise_mw.log10()
}

extern "C" fn builtin_timed(_: *mut c_void, _: u64, _: u32, _: *const u8, _: usize) {}
extern "C" fn builtin_event(state: *mut c_void, event_id: u16, data: *const u8, len: usize) {
    let Some(state) = (unsafe { (state as *mut BuiltinState).as_mut() }) else {
        return;
    };
    if state.kind != BuiltinKind::Phy || len > 16 * 1024 * 1024 || (len != 0 && data.is_null()) {
        return;
    }
    let bytes = if len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(data, len) }
    };
    use crate::protobufs::emane_message::{
        fading_selection_event, AntennaProfileEvent, FadingSelectionEvent, LocationEvent,
        PathlossEvent, PathlossExEvent,
    };
    match event_id {
        100 => {
            let Ok(event) = LocationEvent::decode(bytes) else {
                return;
            };
            for location in event.locations {
                let Ok(nem_id) = u16::try_from(location.nem_id) else {
                    continue;
                };
                let position = location.position;
                if position.latitude_degrees.is_finite()
                    && position.longitude_degrees.is_finite()
                    && position.altitude_meters.is_finite()
                    && (-90.0..=90.0).contains(&position.latitude_degrees)
                    && (-180.0..=180.0).contains(&position.longitude_degrees)
                {
                    let velocity = location.velocity.and_then(|velocity| {
                        (velocity.azimuth_degrees.is_finite()
                            && velocity.elevation_degrees.is_finite()
                            && velocity.magnitude_meters_per_second.is_finite()
                            && velocity.magnitude_meters_per_second >= 0.0)
                            .then_some(Velocity {
                                azimuth_degrees: velocity.azimuth_degrees,
                                elevation_degrees: velocity.elevation_degrees,
                                magnitude_meters_per_second: velocity.magnitude_meters_per_second,
                            })
                    });
                    let orientation = location
                        .orientation
                        .and_then(|orientation| {
                            (orientation.roll_degrees.is_finite()
                                && orientation.pitch_degrees.is_finite()
                                && orientation.yaw_degrees.is_finite())
                            .then_some(Orientation {
                                roll_degrees: orientation.roll_degrees,
                                pitch_degrees: orientation.pitch_degrees,
                                yaw_degrees: orientation.yaw_degrees,
                            })
                        })
                        .unwrap_or_default();
                    state.phy.locations.insert(
                        nem_id,
                        Location {
                            latitude_degrees: position.latitude_degrees,
                            longitude_degrees: position.longitude_degrees,
                            altitude_meters: position.altitude_meters,
                            velocity,
                            orientation,
                        },
                    );
                }
            }
        }
        101 => {
            let Ok(event) = PathlossEvent::decode(bytes) else {
                return;
            };
            for pathloss in event.pathlosses {
                let Ok(nem_id) = u16::try_from(pathloss.nem_id) else {
                    continue;
                };
                let value = f64::from(pathloss.forward_pathlossd_b);
                if value.is_finite() {
                    state
                        .phy
                        .pathloss
                        .entry(nem_id)
                        .or_default()
                        .insert(0, value);
                }
            }
        }
        107 => {
            let Ok(event) = PathlossExEvent::decode(bytes) else {
                return;
            };
            for pathloss in event.pathlosses {
                let Ok(nem_id) = u16::try_from(pathloss.nem_id) else {
                    continue;
                };
                let values = state.phy.pathloss.entry(nem_id).or_default();
                for entry in pathloss.entries {
                    let value = f64::from(entry.pathlossd_b);
                    if value.is_finite() {
                        values.insert(entry.frequency_hz, value);
                    }
                }
            }
        }
        102 => {
            let Ok(event) = AntennaProfileEvent::decode(bytes) else {
                return;
            };
            for profile in event.profiles {
                let (Ok(nem_id), Ok(profile_id)) = (
                    u16::try_from(profile.nem_id),
                    u16::try_from(profile.profile_id),
                ) else {
                    continue;
                };
                if profile.antenna_azimuth_degrees.is_finite()
                    && profile.antenna_elevation_degrees.is_finite()
                {
                    state.phy.antenna_profiles.insert(
                        nem_id,
                        AntennaProfileSelection {
                            profile_id,
                            azimuth_degrees: profile.antenna_azimuth_degrees,
                            elevation_degrees: profile.antenna_elevation_degrees,
                        },
                    );
                }
            }
        }
        106 => {
            let Ok(event) = FadingSelectionEvent::decode(bytes) else {
                return;
            };
            for entry in event.entries {
                let Ok(nem_id) = u16::try_from(entry.nem_id) else {
                    continue;
                };
                let mode = match fading_selection_event::Model::try_from(entry.model) {
                    Ok(fading_selection_event::Model::TypeNone) => FadingMode::None,
                    Ok(fading_selection_event::Model::TypeNakagami) => FadingMode::Nakagami,
                    Ok(fading_selection_event::Model::TypeLognormal) => FadingMode::Lognormal,
                    Err(_) => continue,
                };
                state.phy.fading_selections.insert(nem_id, mode);
                state.phy.lognormal_states.remove(&nem_id);
            }
        }
        _ => {}
    }
}

fn builtin_api(plugin: &str, plugin_type: u32) -> &'static PluginApi {
    static PHY: OnceLock<PluginApi> = OnceLock::new();
    static VIRTUAL: OnceLock<PluginApi> = OnceLock::new();
    static RAW: OnceLock<PluginApi> = OnceLock::new();
    let virtual_transport = matches!(plugin, "virtualtransport" | "transvirtual");
    let (slot, name, init): (
        &OnceLock<PluginApi>,
        &'static [u8],
        extern "C" fn(u16, *const FfiFrameworkService) -> *mut c_void,
    ) = if plugin_type == 2 {
        (&PHY, b"emanephy\0", builtin_phy_init)
    } else if virtual_transport {
        (&VIRTUAL, b"virtualtransport\0", builtin_virtual_init)
    } else {
        (&RAW, b"rawtransport\0", builtin_raw_init)
    };
    slot.get_or_init(|| PluginApi {
        abi_version: PLUGIN_ABI_VERSION,
        struct_size: std::mem::size_of::<PluginApi>(),
        name: name.as_ptr().cast(),
        plugin_type,
        init,
        configure: builtin_configure,
        start: builtin_start,
        post_start: builtin_post_start,
        stop: builtin_stop,
        destroy: builtin_destroy,
        process_upstream: builtin_upstream,
        process_downstream: builtin_downstream,
        process_timed_event: builtin_timed,
        process_event: builtin_event,
    })
}

struct ConfigGuard {
    request: FfiConfigRequest,
    _names: Vec<CString>,
    _values: Vec<Vec<CString>>,
    _value_pointers: Vec<Vec<*const std::os::raw::c_char>>,
    _items: Vec<FfiConfigItem>,
}

impl ConfigGuard {
    fn new(config: &[(String, Vec<String>)]) -> Self {
        let names: Vec<_> = config
            .iter()
            .map(|(name, _)| CString::new(name.as_str()).unwrap())
            .collect();
        let values: Vec<Vec<_>> = config
            .iter()
            .map(|(_, values)| {
                values
                    .iter()
                    .map(|value| CString::new(value.as_str()).unwrap())
                    .collect()
            })
            .collect();
        let value_pointers: Vec<Vec<_>> = values
            .iter()
            .map(|values| values.iter().map(|value| value.as_ptr()).collect())
            .collect();
        let items: Vec<_> = names
            .iter()
            .zip(value_pointers.iter())
            .map(|(name, values)| FfiConfigItem {
                name: name.as_ptr(),
                values: FfiConfigStringArray {
                    data: values.as_ptr(),
                    len: values.len(),
                },
            })
            .collect();
        let request = FfiConfigRequest {
            data: items.as_ptr(),
            len: items.len(),
        };
        Self {
            request,
            _names: names,
            _values: values,
            _value_pointers: value_pointers,
            _items: items,
        }
    }
}

pub struct NemManager {
    uuid: [u8; 16],
    runtime: Arc<Runtime>,
    layers: BTreeMap<u16, Vec<NemLayer>>,
    stopped: bool,
}

impl NemManager {
    pub fn new(uuid: [u8; 16]) -> Self {
        Self {
            uuid,
            runtime: Arc::new(Runtime {
                invocations: RwLock::new(HashMap::new()),
            }),
            layers: BTreeMap::new(),
            stopped: true,
        }
    }

    pub fn uuid(&self) -> [u8; 16] {
        self.uuid
    }

    pub fn add_layer(&mut self, nem_id: u16, plugin: &str) -> Result<(), String> {
        self.add_layer_configured(nem_id, plugin, 1, &[])
    }

    pub fn add_layer_configured(
        &mut self,
        nem_id: u16,
        plugin: &str,
        expected_type: u32,
        config: &[(String, Vec<String>)],
    ) -> Result<(), String> {
        let layer_index = self.layers.get(&nem_id).map_or(0, Vec::len);
        let (library, api): (Option<Library>, *const PluginApi) = if plugin.is_empty()
            || plugin == "emanephy"
            || plugin == "virtualtransport"
            || plugin == "rawtransport"
            || plugin == "transvirtual"
            || plugin == "transraw"
        {
            (None, builtin_api(plugin, expected_type) as *const PluginApi)
        } else {
            let path = resolve_plugin_path(plugin)?;
            let library = unsafe { Library::new(&path) }
                .map_err(|error| format!("failed to load {}: {error}", path.display()))?;
            let entry: Symbol<PluginEntryFunc> = unsafe { library.get(b"emane_plugin_create") }
                .map_err(|error| {
                    format!("{} has no emane_plugin_create: {error}", path.display())
                })?;
            let api = entry();
            if api.is_null() {
                return Err(format!("{} returned a null plugin API", path.display()));
            }
            (Some(library), api)
        };

        let abi_version = unsafe { (*api).abi_version };
        let struct_size = unsafe { (*api).struct_size };
        if abi_version != PLUGIN_ABI_VERSION || struct_size != std::mem::size_of::<PluginApi>() {
            return Err(format!(
                "plugin ABI mismatch for {plugin}: version {abi_version}, size {struct_size}"
            ));
        }
        let actual_type = unsafe { (*api).plugin_type };
        if actual_type != expected_type {
            return Err(format!(
                "plugin type mismatch for {plugin}: expected {expected_type}, got {actual_type}"
            ));
        }

        let event_build_id = next_event_build_id();
        let mut framework_context = Box::new(FrameworkContext {
            runtime: Arc::downgrade(&self.runtime),
            nem_id,
            layer_index,
            build_id: event_build_id,
            neighbor_metrics: Mutex::new(NeighborMetricManager::new(nem_id)),
            queue_metrics: Mutex::new(QueueMetricManager::new(nem_id)),
        });
        let framework = FfiFrameworkService {
            framework_ctx: framework_context.as_mut() as *mut FrameworkContext as *mut c_void,
            send_downstream_packet,
            send_upstream_packet,
            send_downstream_control,
            send_upstream_control,
            schedule_timed_event,
            cancel_timed_event,
            log,
            register_counter,
            increment_counter,
            update_neighbor_tx,
            update_neighbor_rx,
            update_queue_metric,
            publish_r2ri,
            register_rf_signal_table,
            configure_rf_signal_table,
            update_rf_signal_table,
            publish_event,
        };
        let plugin_context = unsafe { ((*api).init)(nem_id, &framework) };
        if plugin_context.is_null() {
            unregister_native_statistics(event_build_id);
            return Err(format!("plugin {plugin} failed to initialize"));
        }
        let config_guard = ConfigGuard::new(config);
        let configured = unsafe {
            ((*api).configure)(
                plugin_context,
                &config_guard.request as *const FfiConfigRequest as *const c_void,
            )
        };
        if !configured {
            unsafe { ((*api).destroy)(plugin_context) };
            unregister_native_statistics(event_build_id);
            return Err(format!("plugin {plugin} rejected its configuration"));
        }
        let invocation = Invocation {
            api: api as usize,
            plugin_ctx: plugin_context as usize,
        };
        let framework_context_ptr =
            framework_context.as_mut() as *mut FrameworkContext as *mut c_void;
        register_event_user(
            event_build_id,
            nem_id,
            framework_context_ptr,
            framework_event,
        );
        for event_id in 100..=107 {
            if !emane_rs_event_service_register_event(event_build_id, event_id) {
                unregister_event_user(event_build_id);
                unregister_native_statistics(event_build_id);
                unsafe { ((*api).destroy)(plugin_context) };
                return Err(format!(
                    "failed to register plugin {plugin} for event {event_id}"
                ));
            }
        }
        if expected_type == 2 {
            register_native_user(nem_id, framework_context_ptr, ota_packet);
        }
        let runtime_result = self.runtime.invocations.write();
        let Ok(mut runtime_layers) = runtime_result else {
            if expected_type == 2 {
                unregister_native_user(nem_id, framework_context_ptr);
            }
            unregister_event_user(event_build_id);
            unregister_native_statistics(event_build_id);
            unsafe { ((*api).destroy)(plugin_context) };
            return Err("NEM runtime lock poisoned".to_string());
        };
        runtime_layers.entry(nem_id).or_default().push(invocation);
        drop(runtime_layers);
        self.layers.entry(nem_id).or_default().push(NemLayer {
            _library: library,
            invocation,
            _framework_context: framework_context,
            event_build_id,
            started: false,
            destroyed: false,
        });
        emane_rs_buildid_register_layer(
            nem_id,
            event_build_id,
            i32::try_from(actual_type).unwrap_or(i32::MAX),
            unsafe { (*api).name },
        );
        Ok(())
    }

    pub fn start(&mut self) -> Result<(), String> {
        if !self.stopped {
            return Err("NEM manager is already started".to_string());
        }
        self.stopped = false;
        let mut failure = None;
        for layers in self.layers.values_mut() {
            for layer in layers {
                if !(layer.invocation.api().start)(layer.invocation.context()) {
                    failure = Some(format!(
                        "plugin type {} failed to start",
                        layer.invocation.api().plugin_type
                    ));
                    break;
                }
                layer.started = true;
            }
            if failure.is_some() {
                break;
            }
        }
        if let Some(error) = failure {
            self.stop();
            return Err(error);
        }
        Ok(())
    }

    pub fn post_start(&self) {
        for layers in self.layers.values() {
            for layer in layers {
                (layer.invocation.api().post_start)(layer.invocation.context());
            }
        }
    }

    pub fn stop(&mut self) {
        if self.stopped {
            return;
        }
        for layers in self.layers.values_mut() {
            for layer in layers {
                if layer.started {
                    (layer.invocation.api().stop)(layer.invocation.context());
                    layer.started = false;
                }
            }
        }
        self.stopped = true;
    }

    pub fn process_downstream(
        &self,
        nem_id: u16,
        packet: &FfiPacket,
        messages: &[FfiControlMessage],
    ) -> Result<(), String> {
        let first = self
            .runtime
            .invocation(nem_id, 0)
            .ok_or_else(|| format!("unknown or empty NEM {nem_id}"))?;
        (first.api().process_downstream)(
            first.context(),
            packet,
            messages.as_ptr(),
            messages.len(),
        );
        Ok(())
    }

    pub fn process_upstream(
        &self,
        nem_id: u16,
        packet: &FfiPacket,
        messages: &[FfiControlMessage],
    ) -> Result<(), String> {
        let last = self
            .runtime
            .last_invocation(nem_id)
            .ok_or_else(|| format!("unknown or empty NEM {nem_id}"))?;
        (last.api().process_upstream)(last.context(), packet, messages.as_ptr(), messages.len());
        Ok(())
    }
}

impl Drop for NemManager {
    fn drop(&mut self) {
        self.stop();
        for (nem_id, layers) in &self.layers {
            for layer in layers
                .iter()
                .filter(|layer| layer.invocation.api().plugin_type == 2)
            {
                unregister_native_user(
                    *nem_id,
                    layer._framework_context.as_ref() as *const FrameworkContext as *mut c_void,
                );
            }
        }
        for layer in self.layers.values().flatten() {
            unregister_event_user(layer.event_build_id);
            unregister_native_statistics(layer.event_build_id);
        }
        for (nem_id, layer) in self
            .layers
            .iter()
            .flat_map(|(nem_id, layers)| layers.iter().map(move |layer| (nem_id, layer)))
        {
            unregister_native_layer(*nem_id, layer.event_build_id);
        }
        if let Ok(mut runtime_layers) = self.runtime.invocations.write() {
            runtime_layers.clear();
        }
        for layers in self.layers.values_mut() {
            for layer in layers {
                if !layer.destroyed {
                    (layer.invocation.api().destroy)(layer.invocation.context());
                    layer.destroyed = true;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin_interface::{FfiPacketInfo, FfiSlice};
    use std::sync::atomic::{AtomicU64, AtomicUsize};

    static LOCAL_OTA_HITS: AtomicUsize = AtomicUsize::new(0);
    static BYPASS_STACK_HITS: AtomicUsize = AtomicUsize::new(0);
    static BYPASS_STACK_BYTES: AtomicUsize = AtomicUsize::new(0);
    static PHY_CAPTURE_SEGMENTS: AtomicUsize = AtomicUsize::new(0);
    static PHY_CAPTURE_POWER_BITS: AtomicU64 = AtomicU64::new(0);
    static PHY_CAPTURE_MIMO_INFOS: AtomicUsize = AtomicUsize::new(0);
    static PHY_CAPTURE_RX_ANTENNA: AtomicUsize = AtomicUsize::new(0);
    static R2RI_CAPTURE_COUNT: AtomicUsize = AtomicUsize::new(0);

    struct CaptureTransport {
        id: u16,
        framework: FfiFrameworkService,
    }

    extern "C" fn test_init(_: u16, _: *const FfiFrameworkService) -> *mut c_void {
        std::ptr::dangling_mut::<u8>().cast()
    }
    extern "C" fn test_configure(_: *mut c_void, _: *const c_void) -> bool {
        true
    }
    extern "C" fn test_start(_: *mut c_void) -> bool {
        true
    }
    extern "C" fn test_lifecycle(_: *mut c_void) {}
    extern "C" fn test_packet(
        _: *mut c_void,
        _: *const FfiPacket,
        _: *const FfiControlMessage,
        _: usize,
    ) {
    }
    extern "C" fn test_upstream(
        _: *mut c_void,
        _: *const FfiPacket,
        _: *const FfiControlMessage,
        _: usize,
    ) {
        LOCAL_OTA_HITS.fetch_add(1, Ordering::Relaxed);
    }
    extern "C" fn test_timed(_: *mut c_void, _: u64, _: u32, _: *const u8, _: usize) {}
    extern "C" fn test_event(_: *mut c_void, _: u16, _: *const u8, _: usize) {}

    extern "C" fn phy_capture_packet(
        _: *mut c_void,
        _: u16,
        _: *const FfiPacket,
        messages: *const FfiControlMessage,
        count: usize,
    ) {
        let Some(messages) = ffi_control_messages(messages, count) else {
            return;
        };
        if let Some(properties) =
            control_payload(messages, CONTROL_MIMO_RX_PROPERTIES).and_then(MimoRxProperties::decode)
        {
            PHY_CAPTURE_MIMO_INFOS.store(properties.antenna_infos.len(), Ordering::Relaxed);
            if let Some(info) = properties.antenna_infos.first() {
                PHY_CAPTURE_RX_ANTENNA
                    .store(usize::from(info.receive_antenna_index), Ordering::Relaxed);
            }
        }
        let Some(segments) = control_payload(messages, CONTROL_RX_FREQUENCY_SEGMENTS)
            .and_then(RxFrequencySegments::decode)
        else {
            return;
        };
        PHY_CAPTURE_SEGMENTS.store(segments.segments.len(), Ordering::Relaxed);
        if let Some(segment) = segments.segments.first() {
            PHY_CAPTURE_POWER_BITS.store(segment.rx_power_dbm.to_bits(), Ordering::Relaxed);
        }
    }

    extern "C" fn phy_capture_control(
        _: *mut c_void,
        _: u16,
        _: *const FfiControlMessage,
        _: usize,
    ) {
    }

    extern "C" fn phy_capture_schedule(
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

    extern "C" fn phy_capture_cancel(_: *mut c_void, _: u16, _: u64) {}

    extern "C" fn phy_capture_log(_: *mut c_void, _: u32, _: *const std::os::raw::c_char) {}

    extern "C" fn phy_capture_register(
        _: *mut c_void,
        _: *const std::os::raw::c_char,
        _: *const std::os::raw::c_char,
        _: bool,
    ) -> u64 {
        0
    }

    extern "C" fn phy_capture_increment(_: *mut c_void, _: u64, _: u64) -> bool {
        false
    }

    extern "C" fn capture_init(id: u16, framework: *const FfiFrameworkService) -> *mut c_void {
        let Some(framework) = (unsafe { framework.as_ref() }) else {
            return std::ptr::null_mut();
        };
        Box::into_raw(Box::new(CaptureTransport {
            id,
            framework: *framework,
        }))
        .cast()
    }

    extern "C" fn capture_destroy(plugin: *mut c_void) {
        if !plugin.is_null() {
            unsafe { drop(Box::from_raw(plugin as *mut CaptureTransport)) };
        }
    }

    extern "C" fn capture_upstream(
        plugin: *mut c_void,
        packet: *const FfiPacket,
        messages: *const FfiControlMessage,
        count: usize,
    ) {
        if plugin.is_null() {
            return;
        }
        if packet.is_null() {
            let Some(messages) = ffi_control_messages(messages, count) else {
                return;
            };
            let valid = messages
                .iter()
                .filter(|message| {
                    let Some(payload) = control_payload(messages, message.msg_type) else {
                        return false;
                    };
                    match message.msg_type {
                        CONTROL_R2RI_SELF_METRIC => R2riSelfMetric::decode(payload).is_some(),
                        CONTROL_R2RI_QUEUE_METRIC => R2riQueueMetrics::decode(payload).is_some(),
                        CONTROL_R2RI_NEIGHBOR_METRIC => {
                            R2riNeighborMetrics::decode(payload).is_some()
                        }
                        _ => false,
                    }
                })
                .count();
            R2RI_CAPTURE_COUNT.store(valid, Ordering::Relaxed);
            return;
        }
        let packet = unsafe { &*packet };
        if packet.payload.len != 0 && !packet.payload.data.is_null() {
            let payload =
                unsafe { std::slice::from_raw_parts(packet.payload.data, packet.payload.len) };
            BYPASS_STACK_BYTES.store(
                payload.iter().map(|value| *value as usize).sum(),
                Ordering::Relaxed,
            );
            BYPASS_STACK_HITS.fetch_add(1, Ordering::Relaxed);
        }
    }

    extern "C" fn capture_downstream(
        plugin: *mut c_void,
        packet: *const FfiPacket,
        messages: *const FfiControlMessage,
        count: usize,
    ) {
        let Some(state) = (unsafe { (plugin as *const CaptureTransport).as_ref() }) else {
            return;
        };
        (state.framework.send_downstream_packet)(
            state.framework.framework_ctx,
            state.id,
            packet,
            messages,
            count,
        );
    }

    fn capture_api() -> &'static PluginApi {
        static API: OnceLock<PluginApi> = OnceLock::new();
        API.get_or_init(|| PluginApi {
            abi_version: PLUGIN_ABI_VERSION,
            struct_size: std::mem::size_of::<PluginApi>(),
            name: c"capture-transport".as_ptr(),
            plugin_type: 4,
            init: capture_init,
            configure: test_configure,
            start: test_start,
            post_start: test_lifecycle,
            stop: test_lifecycle,
            destroy: capture_destroy,
            process_upstream: capture_upstream,
            process_downstream: capture_downstream,
            process_timed_event: test_timed,
            process_event: test_event,
        })
    }

    fn add_capture_transport(manager: &mut NemManager, nem_id: u16) {
        let mut framework_context = Box::new(FrameworkContext {
            runtime: Arc::downgrade(&manager.runtime),
            nem_id,
            layer_index: 0,
            build_id: next_event_build_id(),
            neighbor_metrics: Mutex::new(NeighborMetricManager::new(nem_id)),
            queue_metrics: Mutex::new(QueueMetricManager::new(nem_id)),
        });
        let framework = FfiFrameworkService {
            framework_ctx: framework_context.as_mut() as *mut FrameworkContext as *mut c_void,
            send_downstream_packet,
            send_upstream_packet,
            send_downstream_control,
            send_upstream_control,
            schedule_timed_event,
            cancel_timed_event,
            log,
            register_counter,
            increment_counter,
            update_neighbor_tx,
            update_neighbor_rx,
            update_queue_metric,
            publish_r2ri,
            register_rf_signal_table,
            configure_rf_signal_table,
            update_rf_signal_table,
            publish_event,
        };
        let context = (capture_api().init)(nem_id, &framework);
        let invocation = Invocation {
            api: capture_api() as *const PluginApi as usize,
            plugin_ctx: context as usize,
        };
        manager.layers.entry(nem_id).or_default().push(NemLayer {
            _library: None,
            invocation,
            _framework_context: framework_context,
            event_build_id: next_event_build_id(),
            started: false,
            destroyed: false,
        });
        manager
            .runtime
            .invocations
            .write()
            .unwrap()
            .entry(nem_id)
            .or_default()
            .push(invocation);
    }

    fn test_api() -> &'static PluginApi {
        static API: OnceLock<PluginApi> = OnceLock::new();
        API.get_or_init(|| PluginApi {
            abi_version: PLUGIN_ABI_VERSION,
            struct_size: std::mem::size_of::<PluginApi>(),
            name: c"test".as_ptr(),
            plugin_type: 2,
            init: test_init,
            configure: test_configure,
            start: test_start,
            post_start: test_lifecycle,
            stop: test_lifecycle,
            destroy: test_lifecycle,
            process_upstream: test_upstream,
            process_downstream: test_packet,
            process_timed_event: test_timed,
            process_event: test_event,
        })
    }

    #[test]
    fn scaled_frequency_values_match_emane_configuration_syntax() {
        assert_eq!(parse_scaled_u64("1M"), Some(1_000_000));
        assert_eq!(parse_scaled_u64("2.347G"), Some(2_347_000_000));
        assert_eq!(parse_scaled_u64("500k"), Some(500_000));
        assert_eq!(parse_scaled_u64("-1M"), None);
        assert_eq!(parse_scaled_u64("garbage"), None);
        assert_eq!(pacing_duration(100, 800), Duration::from_secs(1));
        assert_eq!(pacing_duration(100, 0), Duration::ZERO);
    }

    #[test]
    fn transport_flow_control_releases_only_available_frames() {
        let frame = |value| PendingTransportFrame {
            payload: vec![value],
            destination: 2,
            priority: 0,
            creation_time_sec: 0,
            creation_time_usec: 0,
        };
        let mut pending = VecDeque::from([frame(1), frame(2), frame(3)]);
        let mut available = 0;
        let released = release_transport_frames(
            true,
            &mut available,
            &mut pending,
            FlowControlToken { tokens: 2 },
        );
        assert_eq!(released.len(), 2);
        assert_eq!(pending.len(), 1);
        assert_eq!(available, 0);

        assert!(release_transport_frames(
            false,
            &mut available,
            &mut pending,
            FlowControlToken { tokens: 5 },
        )
        .is_empty());
        assert_eq!(pending.len(), 1);
    }

    #[test]
    fn builtin_configuration_rejects_cross_layer_values_and_accepts_transport_pacing() {
        let mut manager = NemManager::new([11; 16]);
        assert!(manager
            .add_layer_configured(
                1,
                "emanephy",
                2,
                &[("frequncy".to_string(), vec!["2.4G".to_string()])],
            )
            .is_err());
        assert!(manager
            .add_layer_configured(
                2,
                "virtualtransport",
                4,
                &[("frequency".to_string(), vec!["2.4G".to_string()])],
            )
            .is_err());
        assert!(manager
            .add_layer_configured(
                3,
                "virtualtransport",
                4,
                &[
                    ("bitrate".to_string(), vec!["1M".to_string()]),
                    ("flowcontrolenable".to_string(), vec!["true".to_string()]),
                ],
            )
            .is_ok());
        manager
            .add_layer_configured(
                4,
                "virtualtransport",
                4,
                &[
                    ("bitrate".to_string(), vec!["0.0".to_string()]),
                    ("address".to_string(), vec!["172.30.1.1".to_string()]),
                    ("mask".to_string(), vec!["255.255.0.0".to_string()]),
                ],
            )
            .unwrap();
    }

    #[test]
    fn precomputed_propagation_prefers_frequency_specific_pathloss() {
        let mut phy = PhyState::new();
        phy.pathloss
            .insert(2, HashMap::from([(0, 50.0), (2_347_000_000, 72.5)]));
        assert_eq!(phy.propagation(1, 2, 2_347_000_000), Some((72.5, 0)));
        assert_eq!(phy.propagation(1, 2, 915_000_000), Some((50.0, 0)));
        assert_eq!(phy.propagation(1, 3, 915_000_000), None);
    }

    #[test]
    fn explicit_mimo_omni_antennas_combine_both_endpoint_gains() {
        let phy = PhyState::new();
        assert_eq!(
            phy.antenna_pair_gain(
                1,
                2,
                AntennaPattern::IdealOmni { gain_db: 3.0 },
                AntennaPattern::IdealOmni { gain_db: -1.5 },
            ),
            Some(1.5)
        );
    }

    #[test]
    fn active_phy_reports_multiple_segments_and_combines_collaborative_transmitters() {
        PHY_CAPTURE_SEGMENTS.store(0, Ordering::Relaxed);
        PHY_CAPTURE_POWER_BITS.store(0, Ordering::Relaxed);
        PHY_CAPTURE_MIMO_INFOS.store(0, Ordering::Relaxed);
        PHY_CAPTURE_RX_ANTENNA.store(0, Ordering::Relaxed);
        let framework = FfiFrameworkService {
            framework_ctx: std::ptr::null_mut(),
            send_downstream_packet: phy_capture_packet,
            send_upstream_packet: phy_capture_packet,
            send_downstream_control: phy_capture_control,
            send_upstream_control: phy_capture_control,
            schedule_timed_event: phy_capture_schedule,
            cancel_timed_event: phy_capture_cancel,
            log: phy_capture_log,
            register_counter: phy_capture_register,
            increment_counter: phy_capture_increment,
            update_neighbor_tx,
            update_neighbor_rx,
            update_queue_metric,
            publish_r2ri,
            register_rf_signal_table,
            configure_rf_signal_table,
            update_rf_signal_table,
            publish_event,
        };
        let context = builtin_init(1, &framework, BuiltinKind::Phy);
        assert!(!context.is_null());
        let state = unsafe { &mut *(context as *mut BuiltinState) };
        state.phy.compatibility_mode = 2;
        state.phy.frequencies_of_interest = vec![2_400_000_000, 2_410_000_000];
        state.phy.pathloss.insert(2, HashMap::from([(0, 50.0)]));
        state.phy.pathloss.insert(3, HashMap::from([(0, 50.0)]));
        state.phy.initialize_monitor();
        assert!(builtin_start(context));

        let receive_antenna = RxAntennaAdd {
            antenna: MimoTxAntenna {
                frequency_group_index: 0,
                antenna_index: 7,
                bandwidth_hz: 1_000_000,
                spectral_mask_index: 0,
                pattern: AntennaPattern::IdealOmni { gain_db: 3.0 },
            },
            frequencies_hz: vec![2_400_000_000, 2_410_000_000],
        }
        .encode()
        .unwrap();
        let receive_control = FfiControlMessage {
            msg_type: CONTROL_RX_ANTENNA_ADD,
            payload: FfiSlice {
                data: receive_antenna.as_ptr(),
                len: receive_antenna.len(),
            },
        };
        builtin_downstream(context, std::ptr::null(), &receive_control, 1);

        let now = unix_time_microseconds();
        let tx = TxProperties {
            frequency_hz: 2_400_000_000,
            bandwidth_hz: 1_000_000,
            tx_power_dbm: 0.0,
            duration_microseconds: 100,
            offset_microseconds: 0,
            tx_time_microseconds: now,
            antenna_index: 0,
            spectral_mask_index: 0,
            sub_id: 1,
        }
        .encode();
        let mimo = MimoTxProperties {
            frequency_groups: vec![
                vec![MimoTxFrequencySegment {
                    frequency_hz: 2_400_000_000,
                    tx_power_dbm: 0.0,
                    duration_microseconds: 100,
                    offset_microseconds: 0,
                }],
                vec![MimoTxFrequencySegment {
                    frequency_hz: 2_410_000_000,
                    tx_power_dbm: 0.0,
                    duration_microseconds: 50,
                    offset_microseconds: 100,
                }],
            ],
            transmit_antennas: vec![
                MimoTxAntenna {
                    frequency_group_index: 0,
                    antenna_index: 1,
                    bandwidth_hz: 1_000_000,
                    spectral_mask_index: 0,
                    pattern: AntennaPattern::Default,
                },
                MimoTxAntenna {
                    frequency_group_index: 1,
                    antenna_index: 2,
                    bandwidth_hz: 1_000_000,
                    spectral_mask_index: 0,
                    pattern: AntennaPattern::Default,
                },
            ],
        }
        .encode()
        .unwrap();
        let transmitters = TxTransmitters {
            transmitters: vec![
                TxTransmitter {
                    nem_id: 2,
                    tx_power_dbm: 0.0,
                },
                TxTransmitter {
                    nem_id: 3,
                    tx_power_dbm: 0.0,
                },
            ],
        }
        .encode()
        .unwrap();
        let messages = [
            FfiControlMessage {
                msg_type: CONTROL_TX_PROPERTIES,
                payload: FfiSlice {
                    data: tx.as_ptr(),
                    len: tx.len(),
                },
            },
            FfiControlMessage {
                msg_type: CONTROL_MIMO_TX_PROPERTIES,
                payload: FfiSlice {
                    data: mimo.as_ptr(),
                    len: mimo.len(),
                },
            },
            FfiControlMessage {
                msg_type: CONTROL_TX_TRANSMITTERS,
                payload: FfiSlice {
                    data: transmitters.as_ptr(),
                    len: transmitters.len(),
                },
            },
        ];
        let payload = [1u8, 2, 3];
        let packet = FfiPacket {
            info: FfiPacketInfo {
                source: 2,
                destination: 1,
                priority: 0,
                creation_time_sec: 0,
                creation_time_usec: 0,
            },
            payload: FfiSlice {
                data: payload.as_ptr(),
                len: payload.len(),
            },
        };
        builtin_upstream(context, &packet, messages.as_ptr(), messages.len());
        assert_eq!(PHY_CAPTURE_SEGMENTS.load(Ordering::Relaxed), 2);
        assert_eq!(PHY_CAPTURE_MIMO_INFOS.load(Ordering::Relaxed), 2);
        assert_eq!(PHY_CAPTURE_RX_ANTENNA.load(Ordering::Relaxed), 7);
        let combined_power = f64::from_bits(PHY_CAPTURE_POWER_BITS.load(Ordering::Relaxed));
        assert!((combined_power - (-43.989_700_043)).abs() < 0.001);
        builtin_destroy(context);
    }

    #[test]
    fn location_models_compute_distance_and_nonnegative_pathloss() {
        let mut phy = PhyState::new();
        phy.locations.insert(
            1,
            Location {
                latitude_degrees: 40.0,
                longitude_degrees: -74.0,
                altitude_meters: 10.0,
                velocity: None,
                orientation: Orientation::default(),
            },
        );
        phy.locations.insert(
            2,
            Location {
                latitude_degrees: 40.001,
                longitude_degrees: -74.0,
                altitude_meters: 20.0,
                velocity: None,
                orientation: Orientation::default(),
            },
        );
        phy.propagation_model = PropagationModel::FreeSpace;
        let (pathloss, delay) = phy.propagation(1, 2, 2_347_000_000).unwrap();
        assert!(pathloss > 0.0);
        assert!(delay >= 0);
        phy.propagation_model = PropagationModel::TwoRay;
        assert!(phy.propagation(1, 2, 2_347_000_000).unwrap().0 >= 0.0);
    }

    #[test]
    fn doppler_uses_relative_velocity_and_can_be_disabled() {
        let mut phy = PhyState::new();
        phy.locations.insert(
            1,
            Location {
                latitude_degrees: 40.0,
                longitude_degrees: -74.0,
                altitude_meters: 10.0,
                velocity: Some(Velocity {
                    azimuth_degrees: 0.0,
                    elevation_degrees: 0.0,
                    magnitude_meters_per_second: 30.0,
                }),
                orientation: Orientation::default(),
            },
        );
        phy.locations.insert(
            2,
            Location {
                latitude_degrees: 40.01,
                longitude_degrees: -74.0,
                altitude_meters: 10.0,
                velocity: Some(Velocity {
                    azimuth_degrees: 0.0,
                    elevation_degrees: 0.0,
                    magnitude_meters_per_second: 0.0,
                }),
                orientation: Orientation::default(),
            },
        );
        assert!(phy.doppler_fraction(1, 2) > 0.0);
        phy.doppler_shift_enabled = false;
        assert_eq!(phy.doppler_fraction(1, 2), 0.0);
    }

    #[test]
    fn antenna_body_frame_applies_roll_pitch_and_yaw() {
        let local = Location {
            latitude_degrees: 0.0,
            longitude_degrees: 0.0,
            altitude_meters: 0.0,
            velocity: None,
            orientation: Orientation::default(),
        };
        let east = Location {
            longitude_degrees: 0.001,
            ..local
        };
        let (azimuth, elevation) = oriented_direction_angles(local, east).unwrap();
        assert!((azimuth - 90.0).abs() < 1.0e-9);
        assert!(elevation.abs() < 1.0e-9);

        let rolled = Location {
            orientation: Orientation {
                roll_degrees: 90.0,
                ..Orientation::default()
            },
            ..local
        };
        let (_, elevation) = oriented_direction_angles(rolled, east).unwrap();
        assert!((elevation + 90.0).abs() < 1.0e-9);
    }

    #[test]
    fn event_fading_requires_a_source_selection() {
        let mut phy = PhyState::new();
        phy.fading_mode = FadingMode::Event;
        assert_eq!(phy.apply_fading(1, 2, -50.0, 1), None);
        phy.fading_selections.insert(2, FadingMode::None);
        assert_eq!(phy.apply_fading(1, 2, -50.0, 1), Some(-50.0));
    }

    #[test]
    fn phy_configuration_accepts_ported_features_and_mimo_mode() {
        let mut manager = NemManager::new([12; 16]);
        manager
            .add_layer_configured(
                1,
                "emanephy",
                2,
                &[
                    ("compatibilitymode".to_string(), vec!["1".to_string()]),
                    ("dopplershiftenable".to_string(), vec!["true".to_string()]),
                    ("fading.model".to_string(), vec!["nakagami".to_string()]),
                    (
                        "fading.nakagami.distance0".to_string(),
                        vec!["100".to_string()],
                    ),
                    (
                        "fading.nakagami.distance1".to_string(),
                        vec!["250".to_string()],
                    ),
                ],
            )
            .unwrap();
        manager
            .add_layer_configured(
                2,
                "emanephy",
                2,
                &[("compatibilitymode".to_string(), vec!["2".to_string()])],
            )
            .unwrap();
        assert!(manager
            .add_layer_configured(
                3,
                "emanephy",
                2,
                &[("compatibilitymode".to_string(), vec!["3".to_string()])],
            )
            .is_err());
    }

    #[test]
    fn native_r2ri_service_publishes_self_queue_and_neighbor_controls() {
        R2RI_CAPTURE_COUNT.store(0, Ordering::Relaxed);
        let mut manager = NemManager::new([13; 16]);
        add_capture_transport(&mut manager, 20);
        manager
            .add_layer_configured(20, "emanephy", 2, &[])
            .unwrap();
        let context = manager.layers[&20][1]._framework_context.as_ref() as *const FrameworkContext
            as *mut c_void;
        let now = unix_time_microseconds().max(0) as u64;
        update_queue_metric(context, 1, 255, 4, 2, 30);
        update_neighbor_tx(context, 21, 1_000_000, now);
        update_neighbor_rx(context, 21, 7, 15.0, -100.0, now, 200, 2_000_000);
        publish_r2ri(context, 1_000_000, 2_000_000, 500_000, 60_000_000);
        assert_eq!(R2RI_CAPTURE_COUNT.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn native_layer_statistics_are_manifested_and_track_packets() {
        use crate::statistics::{
            emane_rs_statistic_free_manifest, emane_rs_statistic_free_query_result,
            emane_rs_statistic_get_manifest, emane_rs_statistic_query, FfiStringArray,
        };

        let mut manager = NemManager::new([14; 16]);
        manager
            .add_layer_configured(30, "emanephy", 2, &[])
            .unwrap();
        let build_id = manager.layers[&30][0].event_build_id;
        let manifest = emane_rs_statistic_get_manifest(build_id);
        assert_eq!(manifest.len, 20);
        emane_rs_statistic_free_manifest(manifest);

        let payload = [1u8, 2, 3, 4];
        let packet = FfiPacket {
            info: FfiPacketInfo {
                source: 30,
                destination: 31,
                priority: 0,
                creation_time_sec: 0,
                creation_time_usec: 0,
            },
            payload: FfiSlice {
                data: payload.as_ptr(),
                len: payload.len(),
            },
        };
        manager.process_downstream(30, &packet, &[]).unwrap();

        let name = CString::new("numDownstreamPacketsUnicastRx").unwrap();
        let names = [name.as_ptr()];
        let mut error = [0i8; 64];
        let query = emane_rs_statistic_query(
            build_id,
            FfiStringArray {
                data: names.as_ptr(),
                len: names.len(),
            },
            error.as_mut_ptr(),
            error.len(),
        );
        assert_eq!(query.len, 1);
        assert_eq!(unsafe { &*query.data }.value.u64_value, 1);
        emane_rs_statistic_free_query_result(query);
    }

    #[test]
    fn local_ota_is_observed_by_every_other_nem() {
        LOCAL_OTA_HITS.store(0, Ordering::Relaxed);
        let invocation = Invocation {
            api: test_api() as *const PluginApi as usize,
            plugin_ctx: std::ptr::dangling_mut::<u8>() as usize,
        };
        let runtime = Runtime {
            invocations: RwLock::new(HashMap::from([
                (1, vec![invocation]),
                (2, vec![invocation]),
                (3, vec![invocation]),
            ])),
        };
        let payload = [1u8];
        let packet = FfiPacket {
            info: FfiPacketInfo {
                source: 1,
                destination: 2,
                priority: 0,
                creation_time_sec: 0,
                creation_time_usec: 0,
            },
            payload: FfiSlice {
                data: payload.as_ptr(),
                len: payload.len(),
            },
        };
        runtime.route_downstream(1, 0, &packet, std::ptr::null(), 0);
        assert_eq!(LOCAL_OTA_HITS.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn builtin_stack_lifecycle_and_routing() {
        let mut manager = NemManager::new([7; 16]);
        manager.add_layer_configured(1, "emanephy", 2, &[]).unwrap();
        manager.add_layer_configured(2, "emanephy", 2, &[]).unwrap();
        manager.start().unwrap();
        manager.post_start();
        let bytes = [1, 2, 3];
        let packet = FfiPacket {
            info: FfiPacketInfo {
                source: 1,
                destination: 2,
                priority: 0,
                creation_time_sec: 0,
                creation_time_usec: 0,
            },
            payload: FfiSlice {
                data: bytes.as_ptr(),
                len: bytes.len(),
            },
        };
        manager.process_downstream(1, &packet, &[]).unwrap();
        manager.stop();
    }

    #[test]
    fn dynamic_plugin_can_be_loaded_and_run_in_stack() {
        let Ok(plugin) = resolve_plugin_path("dummy-mac") else {
            // Package-only test invocations do not necessarily build sibling
            // cdylibs; the workspace smoke test covers that configuration.
            return;
        };
        if !plugin.exists() {
            return;
        }
        let mut manager = NemManager::new([9; 16]);
        if let Err(error) = manager.add_layer_configured(1, plugin.to_str().unwrap(), 1, &[]) {
            // `cargo test` does not rebuild sibling cdylib artifacts. A stale
            // library is correctly rejected; the workspace build/smoke test
            // exercises the current artifact.
            if error.contains("plugin ABI mismatch") {
                return;
            }
            panic!("{error}");
        }
        manager.add_layer_configured(1, "emanephy", 2, &[]).unwrap();
        manager.start().unwrap();
        manager.post_start();
        manager.stop();
    }

    #[test]
    fn dynamic_bypass_stack_delivers_between_two_nems() {
        let (Ok(mac), Ok(phy)) = (
            resolve_plugin_path("bypassmaclayer"),
            resolve_plugin_path("bypassphylayer"),
        ) else {
            return;
        };
        if !mac.exists() || !phy.exists() {
            return;
        }
        BYPASS_STACK_HITS.store(0, Ordering::Relaxed);
        BYPASS_STACK_BYTES.store(0, Ordering::Relaxed);
        let mut manager = NemManager::new([13; 16]);
        for nem_id in [1, 2] {
            add_capture_transport(&mut manager, nem_id);
            if let Err(error) = manager.add_layer_configured(nem_id, mac.to_str().unwrap(), 1, &[])
            {
                if error.contains("plugin ABI mismatch") {
                    return;
                }
                panic!("{error}");
            }
            manager
                .add_layer_configured(nem_id, phy.to_str().unwrap(), 2, &[])
                .unwrap();
        }
        manager.start().unwrap();
        manager.post_start();
        let payload = [4u8, 5, 6];
        let packet = FfiPacket {
            info: FfiPacketInfo {
                source: 1,
                destination: 2,
                priority: 0,
                creation_time_sec: 0,
                creation_time_usec: 0,
            },
            payload: FfiSlice {
                data: payload.as_ptr(),
                len: payload.len(),
            },
        };
        manager.process_downstream(1, &packet, &[]).unwrap();
        assert_eq!(BYPASS_STACK_HITS.load(Ordering::Relaxed), 1);
        assert_eq!(BYPASS_STACK_BYTES.load(Ordering::Relaxed), 15);
        manager.stop();
    }
}
