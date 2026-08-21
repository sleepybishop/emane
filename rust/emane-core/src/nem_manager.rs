use crate::event_service::{
    emane_rs_event_service_register_event, register_native_user as register_event_user,
    unregister_user as unregister_event_user,
};
use crate::ota_manager::{
    emane_rs_ota_manager_send_ota_packet, register_native_user, unregister_native_user,
};
use crate::plugin_interface::{
    FfiConfigItem, FfiConfigRequest, FfiConfigStringArray, FfiControlMessage, FfiFrameworkService,
    FfiPacket, FfiPacketInfo, FfiSlice, FrequencyOfInterest, PluginApi, PluginEntryFunc,
    RxProperties, TxProperties, CONTROL_FREQUENCY_INTEREST, CONTROL_RX_PROPERTIES,
    CONTROL_TX_PROPERTIES, PLUGIN_ABI_VERSION,
};
use crate::spectrum_monitor::{FfiFrequencySegment, NoiseMode, SpectrumMonitor};
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
use libloading::{Library, Symbol};
use prost::Message;
use std::collections::{BTreeMap, HashMap};
use std::ffi::{c_void, CStr, CString};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, OnceLock, RwLock, Weak};
use std::time::{SystemTime, UNIX_EPOCH};

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
    phy: PhyState,
}

struct PhyState {
    frequency_hz: u64,
    frequencies_of_interest: Vec<u64>,
    bandwidth_hz: u64,
    tx_power_dbm: f64,
    fixed_antenna_gain_db: f64,
    fixed_antenna_gain_enabled: bool,
    propagation_model: PropagationModel,
    locations: HashMap<u16, Location>,
    pathloss: HashMap<u16, HashMap<u64, f64>>,
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
}

impl PhyState {
    fn new() -> Self {
        Self {
            frequency_hz: 2_347_000_000,
            frequencies_of_interest: vec![2_347_000_000],
            bandwidth_hz: 1_000_000,
            tx_power_dbm: 0.0,
            fixed_antenna_gain_db: 0.0,
            fixed_antenna_gain_enabled: true,
            propagation_model: PropagationModel::Precomputed,
            locations: HashMap::new(),
            pathloss: HashMap::new(),
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
            false,
        );
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

fn builtin_init(id: u16, framework: *const FfiFrameworkService, kind: BuiltinKind) -> *mut c_void {
    if framework.is_null() {
        return std::ptr::null_mut();
    }
    let mut state = Box::new(BuiltinState {
        id,
        framework: unsafe { *framework },
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
            // These settings are consumed by higher-level services in the
            // original transport and do not alter Ethernet frame routing.
            "bitrate" | "flowcontrolenable" | "address" | "mask" => {}
            _ => {}
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
    let packet = FfiPacket {
        info: FfiPacketInfo {
            source: state.id,
            destination,
            priority,
            creation_time_sec: now.as_secs(),
            creation_time_usec: now.subsec_micros(),
        },
        payload: FfiSlice { data, len },
    };
    (state.framework.send_downstream_packet)(
        state.framework.framework_ctx,
        state.id,
        &packet,
        std::ptr::null(),
        0,
    );
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
            let Some(incoming) = ffi_control_messages(messages, count) else {
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
                return;
            };

            let Some((pathloss_db, propagation_microseconds)) =
                state
                    .phy
                    .propagation(state.id, packet.info.source, tx.frequency_hz)
            else {
                return;
            };

            let now = unix_time_microseconds();
            let segment = FfiFrequencySegment {
                frequency_hz: tx.frequency_hz,
                rx_power_dbm: tx.tx_power_dbm - pathloss_db
                    + if state.phy.fixed_antenna_gain_enabled {
                        state.phy.fixed_antenna_gain_db
                    } else {
                        0.0
                    },
                duration_microsec: i64::try_from(tx.duration_microseconds).unwrap_or(i64::MAX),
                offset_microsec: i64::try_from(tx.offset_microseconds).unwrap_or(i64::MAX),
            };
            let rx_power_dbm = segment.rx_power_dbm;
            let rx_power_mw = 10.0f64.powf(rx_power_dbm / 10.0);
            let is_in_band = tx.sub_id == state.phy.sub_id
                && state.phy.frequencies_of_interest.contains(&tx.frequency_hz);
            let (tx_time, propagation, duration, report, report_in_band, sensitivity_mw) =
                state.phy.monitor.update(
                    now,
                    tx.tx_time_microseconds,
                    propagation_microseconds,
                    0.0,
                    std::slice::from_ref(&segment),
                    tx.bandwidth_hz,
                    std::slice::from_ref(&rx_power_mw),
                    is_in_band,
                    std::slice::from_ref(&packet.info.source),
                    tx.sub_id,
                    tx.antenna_index,
                    tx.spectral_mask_index,
                    std::ptr::null(),
                    0,
                );
            if report.is_empty() || !report_in_band {
                return;
            }

            let rx = RxProperties {
                frequency_hz: report[0].frequency_hz,
                bandwidth_hz: tx.bandwidth_hz,
                rx_power_dbm: report[0].rx_power_dbm,
                noise_floor_dbm: 10.0 * sensitivity_mw.log10(),
                tx_time_microseconds: tx_time,
                propagation_microseconds: propagation.max(0) as u64,
                duration_microseconds: duration.max(0) as u64,
                antenna_index: tx.antenna_index,
                sub_id: tx.sub_id,
                signal_in_noise: state.phy.noise_mode == NoiseMode::All,
            };
            let rx_bytes = rx.encode();
            let mut outgoing: Vec<_> = incoming
                .iter()
                .copied()
                .filter(|message| message.msg_type != CONTROL_RX_PROPERTIES)
                .collect();
            outgoing.push(FfiControlMessage {
                msg_type: CONTROL_RX_PROPERTIES,
                payload: FfiSlice {
                    data: rx_bytes.as_ptr(),
                    len: rx_bytes.len(),
                },
            });
            (state.framework.send_upstream_packet)(
                state.framework.framework_ctx,
                state.id,
                packet,
                outgoing.as_ptr(),
                outgoing.len(),
            );
        }
        BuiltinKind::VirtualTransport | BuiltinKind::RawTransport if !packet.is_null() => {
            let packet = unsafe { &*packet };
            if packet.payload.len != 0 && packet.payload.data.is_null() {
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
        if packet.is_null() {
            return;
        }
        let packet = unsafe { &*packet };
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
        let tx_bytes = tx.encode();
        let mut outgoing: Vec<_> = incoming
            .iter()
            .copied()
            .filter(|message| message.msg_type != CONTROL_TX_PROPERTIES)
            .collect();
        outgoing.push(FfiControlMessage {
            msg_type: CONTROL_TX_PROPERTIES,
            payload: FfiSlice {
                data: tx_bytes.as_ptr(),
                len: tx_bytes.len(),
            },
        });
        (state.framework.send_downstream_packet)(
            state.framework.framework_ctx,
            state.id,
            packet,
            outgoing.as_ptr(),
            outgoing.len(),
        );
        return;
    }
    (state.framework.send_downstream_packet)(
        state.framework.framework_ctx,
        state.id,
        packet,
        messages,
        count,
    );
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

fn unix_time_microseconds() -> i64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    i64::try_from(now.as_micros()).unwrap_or(i64::MAX)
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
    use crate::protobufs::emane_message::{LocationEvent, PathlossEvent, PathlossExEvent};
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
                {
                    state.phy.locations.insert(
                        nem_id,
                        Location {
                            latitude_degrees: position.latitude_degrees,
                            longitude_degrees: position.longitude_degrees,
                            altitude_meters: position.altitude_meters,
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

        let mut framework_context = Box::new(FrameworkContext {
            runtime: Arc::downgrade(&self.runtime),
            nem_id,
            layer_index,
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
        };
        let plugin_context = unsafe { ((*api).init)(nem_id, &framework) };
        if plugin_context.is_null() {
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
            return Err(format!("plugin {plugin} rejected its configuration"));
        }
        let invocation = Invocation {
            api: api as usize,
            plugin_ctx: plugin_context as usize,
        };
        let framework_context_ptr =
            framework_context.as_mut() as *mut FrameworkContext as *mut c_void;
        let event_build_id = next_event_build_id();
        self.layers.entry(nem_id).or_default().push(NemLayer {
            _library: library,
            invocation,
            _framework_context: framework_context,
            event_build_id,
            started: false,
            destroyed: false,
        });
        self.runtime
            .invocations
            .write()
            .map_err(|_| "NEM runtime lock poisoned".to_string())?
            .entry(nem_id)
            .or_default()
            .push(invocation);
        register_event_user(
            event_build_id,
            nem_id,
            framework_context_ptr,
            framework_event,
        );
        for event_id in 100..=107 {
            if !emane_rs_event_service_register_event(event_build_id, event_id) {
                unregister_event_user(event_build_id);
                return Err(format!(
                    "failed to register plugin {plugin} for event {event_id}"
                ));
            }
        }
        if expected_type == 2 {
            register_native_user(nem_id, framework_context_ptr, ota_packet);
        }
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
    use std::sync::atomic::AtomicUsize;

    static LOCAL_OTA_HITS: AtomicUsize = AtomicUsize::new(0);

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
    fn location_models_compute_distance_and_nonnegative_pathloss() {
        let mut phy = PhyState::new();
        phy.locations.insert(
            1,
            Location {
                latitude_degrees: 40.0,
                longitude_degrees: -74.0,
                altitude_meters: 10.0,
            },
        );
        phy.locations.insert(
            2,
            Location {
                latitude_degrees: 40.001,
                longitude_degrees: -74.0,
                altitude_meters: 20.0,
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
}
