use std::os::raw::{c_char, c_void};
use std::ffi::CString;
use emane_core::plugin_interface::PluginApi;

extern "C" fn init(id: u16) -> *mut c_void {
    println!("dummy-mac plugin initialized with id {}", id);
    std::ptr::null_mut()
}
extern "C" fn configure(_plugin_ptr: *mut c_void, _config_req: *const c_void) {}
extern "C" fn start(_plugin_ptr: *mut c_void) { println!("dummy-mac started"); }
extern "C" fn post_start(_plugin_ptr: *mut c_void) {}
extern "C" fn stop(_plugin_ptr: *mut c_void) {}
extern "C" fn destroy(_plugin_ptr: *mut c_void) {}
extern "C" fn process_upstream(_plugin_ptr: *mut c_void, _pkt: *mut c_void, _msgs: *mut c_void) {}
extern "C" fn process_downstream(_plugin_ptr: *mut c_void, _pkt: *mut c_void, _msgs: *mut c_void) {}

static mut API: Option<PluginApi> = None;

#[no_mangle]
pub extern "C" fn emane_plugin_create() -> *const PluginApi {
    unsafe {
        if API.is_none() {
            API = Some(PluginApi {
                name: CString::new("dummy-mac").unwrap().into_raw(),
                plugin_type: 1, // MAC
                init,
                configure,
                start,
                post_start,
                stop,
                destroy,
                process_upstream,
                process_downstream,
            });
        }
        API.as_ref().unwrap() as *const PluginApi
    }
}
