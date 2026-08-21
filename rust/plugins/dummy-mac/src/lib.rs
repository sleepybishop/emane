use emane_plugin_api::{
    FfiConfigRequest, FfiControlMessage, FfiFrameworkService, FfiPacket, PluginApi,
    PLUGIN_ABI_VERSION,
};
use std::ffi::c_void;
use std::sync::OnceLock;

struct DummyMac {
    id: u16,
    framework: FfiFrameworkService,
}

extern "C" fn init(id: u16, framework: *const FfiFrameworkService) -> *mut c_void {
    if framework.is_null() {
        return std::ptr::null_mut();
    }
    Box::into_raw(Box::new(DummyMac {
        id,
        framework: unsafe { *framework },
    })) as *mut c_void
}

extern "C" fn configure(plugin: *mut c_void, request: *const c_void) -> bool {
    !plugin.is_null()
        && unsafe { (request as *const FfiConfigRequest).as_ref() }
            .is_some_and(|request| request.len == 0)
}
extern "C" fn start(_: *mut c_void) -> bool {
    true
}
extern "C" fn lifecycle(_: *mut c_void) {}

extern "C" fn destroy(plugin: *mut c_void) {
    if !plugin.is_null() {
        unsafe { drop(Box::from_raw(plugin as *mut DummyMac)) };
    }
}

extern "C" fn process_upstream(
    plugin: *mut c_void,
    packet: *const FfiPacket,
    messages: *const FfiControlMessage,
    count: usize,
) {
    let Some(plugin) = (unsafe { (plugin as *const DummyMac).as_ref() }) else {
        return;
    };
    (plugin.framework.send_upstream_packet)(
        plugin.framework.framework_ctx,
        plugin.id,
        packet,
        messages,
        count,
    );
}

extern "C" fn process_downstream(
    plugin: *mut c_void,
    packet: *const FfiPacket,
    messages: *const FfiControlMessage,
    count: usize,
) {
    let Some(plugin) = (unsafe { (plugin as *const DummyMac).as_ref() }) else {
        return;
    };
    (plugin.framework.send_downstream_packet)(
        plugin.framework.framework_ctx,
        plugin.id,
        packet,
        messages,
        count,
    );
}

extern "C" fn process_timed_event(_: *mut c_void, _: u64, _: u32, _: *const u8, _: usize) {}
extern "C" fn process_event(_: *mut c_void, _: u16, _: *const u8, _: usize) {}

#[no_mangle]
pub extern "C" fn emane_plugin_create() -> *const PluginApi {
    static API: OnceLock<PluginApi> = OnceLock::new();
    API.get_or_init(|| PluginApi {
        abi_version: PLUGIN_ABI_VERSION,
        struct_size: std::mem::size_of::<PluginApi>(),
        name: c"dummy-mac".as_ptr(),
        plugin_type: 1,
        init,
        configure,
        start,
        post_start: lifecycle,
        stop: lifecycle,
        destroy,
        process_upstream,
        process_downstream,
        process_timed_event,
        process_event,
    })
}
