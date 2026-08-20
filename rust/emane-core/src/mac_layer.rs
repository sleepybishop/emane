use std::os::raw::c_void;

pub struct MacLayer {
    plugin_impl: *mut c_void,
}

unsafe impl Send for MacLayer {}

impl MacLayer {
    pub fn new(_id: u16, plugin_impl: *mut c_void) -> Self {
        Self { plugin_impl }
    }
}

// FFI imports from C++ plugin wrappers
extern "C" {
    fn emane_c_mac_plugin_initialize(plugin: *mut c_void, registrar: *mut c_void);
    fn emane_c_mac_plugin_configure(plugin: *mut c_void, update: *mut c_void);
    fn emane_c_mac_plugin_start(plugin: *mut c_void);
    fn emane_c_mac_plugin_post_start(plugin: *mut c_void);
    fn emane_c_mac_plugin_stop(plugin: *mut c_void);
    fn emane_c_mac_plugin_destroy(plugin: *mut c_void);
    // Wait, the C++ macro in `maclayer_ffi.cc` processUpstreamPacket expects (plugin, hdr, pkt, msgs)!
    // BUT RustNemLayerProxy receives (pkt, msgs)!
    // Wait, MACLayerImplementor expects (CommonMACHeader, pkt, msgs) for processUpstreamPacket!
    // But `doProcessUpstreamPacket` doesn`t have CommonMACHeader!
    // Let`s look at maclayer.cc again: it casts to UpstreamTransport* which has processUpstreamPacket(pkt, msgs).
    // Yes! `MACLayerImplementor` has TWO `processUpstreamPacket` methods!
    // One from `UpstreamTransport` (which takes pkt, msgs) and one virtual which takes (hdr, pkt, msgs)!
    // The `doProcessUpstreamPacket` overrides `UpstreamTransport::processUpstreamPacket`!
    // Which then calls `processUpstreamPacket(pkt, msgs)` private method, which calls the pure virtual `processUpstreamPacket(hdr, pkt, msgs)`!
    // Wait, if it casts to `UpstreamTransport*`, we should cast to `UpstreamTransport*` in `maclayer_ffi.cc`!

    fn emane_c_mac_plugin_process_upstream_packet(
        plugin: *mut c_void,
        pkt: *mut c_void,
        msgs: *const c_void,
    );
    fn emane_c_mac_plugin_process_downstream_packet(
        plugin: *mut c_void,
        pkt: *mut c_void,
        msgs: *const c_void,
    );
    fn emane_c_mac_plugin_process_upstream_control(plugin: *mut c_void, msgs: *const c_void);
    fn emane_c_mac_plugin_process_downstream_control(plugin: *mut c_void, msgs: *const c_void);
    fn emane_c_mac_plugin_set_upstream_transport(plugin: *mut c_void, transport: *mut c_void);
    fn emane_c_mac_plugin_set_downstream_transport(plugin: *mut c_void, transport: *mut c_void);
    fn emane_c_mac_plugin_process_event(
        plugin: *mut c_void,
        event_id: *const c_void,
        serialization: *const c_void,
    );
    fn emane_c_mac_plugin_process_timed_event(
        plugin: *mut c_void,
        event_id: u32,
        expire: *const c_void,
        sched: *const c_void,
        fire: *const c_void,
        arg: *const c_void,
    );
    fn emane_c_mac_plugin_process_configuration(plugin: *mut c_void, update: *const c_void);
}

// C FFI exports for C++ RustNemLayerProxy
#[no_mangle]
pub extern "C" fn emane_rs_mac_layer_new(
    id: u16,
    _platform_service: *mut c_void,
    plugin_impl: *mut c_void,
) -> *mut MacLayer {
    Box::into_raw(Box::new(MacLayer::new(id, plugin_impl)))
}

#[no_mangle]
pub extern "C" fn emane_rs_mac_layer_free(ptr: *mut MacLayer) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_mac_layer_initialize(ptr: *mut MacLayer, registrar: *mut c_void) {
    let layer = unsafe { &mut *ptr };
    unsafe {
        emane_c_mac_plugin_initialize(layer.plugin_impl, registrar);
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_mac_layer_configure(_ptr: *mut MacLayer, _update: *mut c_void) {
    // wait, our configure from proxy takes const
}

#[no_mangle]
pub extern "C" fn emane_rs_mac_layer_start(ptr: *mut MacLayer) {
    let layer = unsafe { &mut *ptr };
    unsafe {
        emane_c_mac_plugin_start(layer.plugin_impl);
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_mac_layer_post_start(ptr: *mut MacLayer) {
    let layer = unsafe { &mut *ptr };
    unsafe {
        emane_c_mac_plugin_post_start(layer.plugin_impl);
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_mac_layer_stop(ptr: *mut MacLayer) {
    let layer = unsafe { &mut *ptr };
    unsafe {
        emane_c_mac_plugin_stop(layer.plugin_impl);
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_mac_layer_destroy(ptr: *mut MacLayer) {
    let layer = unsafe { &mut *ptr };
    unsafe {
        emane_c_mac_plugin_destroy(layer.plugin_impl);
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_mac_layer_do_process_upstream_packet(
    ptr: *mut MacLayer,
    pkt: *mut c_void,
    msgs: *const c_void,
) {
    let layer = unsafe { &mut *ptr };
    unsafe {
        emane_c_mac_plugin_process_upstream_packet(layer.plugin_impl, pkt, msgs);
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_mac_layer_do_process_downstream_packet(
    ptr: *mut MacLayer,
    pkt: *mut c_void,
    msgs: *const c_void,
) {
    let layer = unsafe { &mut *ptr };
    unsafe {
        emane_c_mac_plugin_process_downstream_packet(layer.plugin_impl, pkt, msgs);
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_mac_layer_do_process_upstream_control(
    ptr: *mut MacLayer,
    msgs: *const c_void,
) {
    let layer = unsafe { &mut *ptr };
    unsafe {
        emane_c_mac_plugin_process_upstream_control(layer.plugin_impl, msgs);
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_mac_layer_do_process_downstream_control(
    ptr: *mut MacLayer,
    msgs: *const c_void,
) {
    let layer = unsafe { &mut *ptr };
    unsafe {
        emane_c_mac_plugin_process_downstream_control(layer.plugin_impl, msgs);
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_mac_layer_do_process_event(
    ptr: *mut MacLayer,
    event_id: *const c_void,
    serialization: *const c_void,
) {
    let layer = unsafe { &mut *ptr };
    unsafe {
        emane_c_mac_plugin_process_event(layer.plugin_impl, event_id, serialization);
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_mac_layer_do_process_timed_event(
    ptr: *mut MacLayer,
    event_id: u32,
    expire: *const c_void,
    sched: *const c_void,
    fire: *const c_void,
    arg: *const c_void,
) {
    let layer = unsafe { &mut *ptr };
    unsafe {
        emane_c_mac_plugin_process_timed_event(
            layer.plugin_impl,
            event_id,
            expire,
            sched,
            fire,
            arg,
        );
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_mac_layer_do_process_configuration(
    ptr: *mut MacLayer,
    update: *const c_void,
) {
    let layer = unsafe { &mut *ptr };
    unsafe {
        emane_c_mac_plugin_process_configuration(layer.plugin_impl, update);
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_mac_layer_set_upstream_transport(
    ptr: *mut MacLayer,
    transport: *mut c_void,
) {
    let layer = unsafe { &mut *ptr };
    unsafe {
        emane_c_mac_plugin_set_upstream_transport(layer.plugin_impl, transport);
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_mac_layer_set_downstream_transport(
    ptr: *mut MacLayer,
    transport: *mut c_void,
) {
    let layer = unsafe { &mut *ptr };
    unsafe {
        emane_c_mac_plugin_set_downstream_transport(layer.plugin_impl, transport);
    }
}
