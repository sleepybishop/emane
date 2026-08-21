pub mod publishers;
// pub mod base_model;

pub mod queue;
pub mod receiver;

use std::os::raw::{c_char, c_void};
use std::ffi::CString;
use emane_core::plugin_interface::{PluginApi, FfiPacket, FfiControlMessage, FfiFrameworkService};

static mut API: Option<PluginApi> = None;
static mut FW_SERVICE: Option<FfiFrameworkService> = None;

extern "C" fn init(id: u16, fw_service: *const FfiFrameworkService) -> *mut c_void {
    unsafe {
        FW_SERVICE = Some((*fw_service).clone());
    }
    std::ptr::null_mut()
}
extern "C" fn configure(_ptr: *mut c_void, _req: *const c_void) {}
extern "C" fn start(_ptr: *mut c_void) {}
extern "C" fn post_start(_ptr: *mut c_void) {}
extern "C" fn stop(_ptr: *mut c_void) {}
extern "C" fn destroy(_ptr: *mut c_void) {}

extern "C" fn process_upstream(
    _plugin_ptr: *mut c_void, 
    _pkt: *const FfiPacket, 
    _msgs: *const FfiControlMessage, 
    _num_msgs: usize
) {}

extern "C" fn process_downstream(
    _plugin_ptr: *mut c_void, 
    _pkt: *const FfiPacket, 
    _msgs: *const FfiControlMessage, 
    _num_msgs: usize
) {}

extern "C" fn process_timed_event(
    _plugin_ptr: *mut c_void,
    _timer_id: u64,
    _event_id: u32,
    _data: *const u8,
    _data_len: usize,
) {}

#[no_mangle]
pub extern "C" fn emane_plugin_create() -> *const PluginApi {
    unsafe {
        if API.is_none() {
            API = Some(PluginApi {
                name: CString::new("tdmamaclayer").unwrap().into_raw(),
                plugin_type: 1, // MAC
                init,
                configure,
                start,
                post_start,
                stop,
                destroy,
                process_upstream,
                process_downstream,
                process_timed_event,
            });
        }
        API.as_ref().unwrap() as *const PluginApi
    }
}
