use crate::plugin_interface::{
    FfiConfigItem, FfiConfigRequest, FfiConfigStringArray, FfiControlMessage, FfiFrameworkService,
    FfiPacket, FfiPacketInfo, FfiSlice, PluginApi, PluginEntryFunc, PLUGIN_ABI_VERSION,
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
use libloading::{Library, Symbol};
use std::collections::{BTreeMap, HashMap};
use std::ffi::{c_void, CStr, CString};
use std::path::{Path, PathBuf};
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
    started: bool,
    destroyed: bool,
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

        // At the PHY boundary, preserve the original NEM addressing semantics.
        // Local NEMs can communicate without a C++ OTA adapter; a remote OTA
        // backend can be installed at this same boundary later.
        let destination = unsafe { (*packet).info.destination };
        let targets: Vec<Invocation> = {
            let Ok(layers) = self.invocations.read() else {
                return;
            };
            if destination == BROADCAST_NEM {
                layers
                    .iter()
                    .filter(|(id, _)| **id != nem_id)
                    .filter_map(|(_, stack)| stack.last().copied())
                    .collect()
            } else {
                layers
                    .get(&destination)
                    .and_then(|stack| stack.last().copied())
                    .into_iter()
                    .collect()
            }
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

fn parse_u16(value: &str) -> Option<u16> {
    value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .map_or_else(
            || value.parse().ok(),
            |value| u16::from_str_radix(value, 16).ok(),
        )
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
            // These settings are consumed by higher-level services in the
            // original transport and do not alter Ethernet frame routing.
            "bitrate" | "flowcontrolenable" | "address" | "mask" => {}
            _ => {}
        }
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
        BuiltinKind::Phy => (state.framework.send_upstream_packet)(
            state.framework.framework_ctx,
            state.id,
            packet,
            messages,
            count,
        ),
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
    let Some(state) = (unsafe { (state as *const BuiltinState).as_ref() }) else {
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

extern "C" fn builtin_timed(_: *mut c_void, _: u64, _: u32, _: *const u8, _: usize) {}

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
        self.layers.entry(nem_id).or_default().push(NemLayer {
            _library: library,
            invocation,
            _framework_context: framework_context,
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
