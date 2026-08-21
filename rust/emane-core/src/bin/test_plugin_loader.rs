use emane_core::nem_manager::resolve_plugin_path;
use emane_core::plugin_interface::{
    FfiConfigRequest, FfiControlMessage, FfiFrameworkService, FfiPacket, PluginApi,
    PluginEntryFunc, PLUGIN_ABI_VERSION,
};
use libloading::{Library, Symbol};
use std::ffi::{c_void, CStr};

extern "C" fn packet(
    _: *mut c_void,
    _: u16,
    _: *const FfiPacket,
    _: *const FfiControlMessage,
    _: usize,
) {
}
extern "C" fn control(_: *mut c_void, _: u16, _: *const FfiControlMessage, _: usize) {}
extern "C" fn schedule(
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
extern "C" fn cancel(_: *mut c_void, _: u16, _: u64) {}
extern "C" fn log(_: *mut c_void, _: u32, _: *const std::os::raw::c_char) {}

fn main() -> Result<(), String> {
    let plugin = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "dummy-mac".to_string());
    let path = resolve_plugin_path(&plugin)?;
    let library = unsafe { Library::new(&path) }
        .map_err(|error| format!("failed to load {}: {error}", path.display()))?;
    let create: Symbol<PluginEntryFunc> = unsafe { library.get(b"emane_plugin_create") }
        .map_err(|error| format!("missing plugin entry point: {error}"))?;
    let api_ptr = create();
    let Some(api) = (unsafe { api_ptr.as_ref() }) else {
        return Err("plugin returned null API".to_string());
    };
    if api.abi_version != PLUGIN_ABI_VERSION || api.struct_size != std::mem::size_of::<PluginApi>()
    {
        return Err("plugin ABI mismatch".to_string());
    }
    let framework = FfiFrameworkService {
        framework_ctx: std::ptr::null_mut(),
        send_downstream_packet: packet,
        send_upstream_packet: packet,
        send_downstream_control: control,
        send_upstream_control: control,
        schedule_timed_event: schedule,
        cancel_timed_event: cancel,
        log,
    };
    let instance = (api.init)(42, &framework);
    if instance.is_null() {
        return Err("plugin failed to initialize".to_string());
    }
    let request = FfiConfigRequest {
        data: std::ptr::null(),
        len: 0,
    };
    if !(api.configure)(
        instance,
        &request as *const FfiConfigRequest as *const c_void,
    ) {
        (api.destroy)(instance);
        return Err("plugin rejected an empty configuration".to_string());
    }
    if !(api.start)(instance) {
        (api.destroy)(instance);
        return Err("plugin failed to start".to_string());
    }
    (api.post_start)(instance);
    (api.stop)(instance);
    (api.destroy)(instance);
    let name = if api.name.is_null() {
        "<unnamed>".into()
    } else {
        unsafe { CStr::from_ptr(api.name) }.to_string_lossy()
    };
    println!("loaded {name} from {}", path.display());
    Ok(())
}
