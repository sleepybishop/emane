pub mod collision_table;
pub mod downstream_queue;
pub mod mac_config;
pub mod mac_layer;
pub mod mac_statistics;
pub mod mode_timing_parameters;
pub mod neighbor_manager;
pub mod pcr_manager;
pub mod tx_state_machine;
pub mod wmm_manager;

use std::os::raw::{c_char, c_void};
use std::ffi::CString;
use emane_core::plugin_interface::{PluginApi, FfiPacket, FfiControlMessage};

extern "C" fn init(id: u16) -> *mut c_void {
    println!("ieee80211abg init {}", id);
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
) {
    // We will hook this into the Rust MacLayer loop
}

extern "C" fn process_downstream(
    _plugin_ptr: *mut c_void, 
    _pkt: *const FfiPacket, 
    _msgs: *const FfiControlMessage, 
    _num_msgs: usize
) {
    // We will hook this into the Rust MacLayer loop
}

extern "C" fn process_timed_event(
    _plugin_ptr: *mut c_void,
    _timer_id: u64,
    _event_id: u32,
    _data: *const u8,
    _data_len: usize,
) {
}

static mut API: Option<PluginApi> = None;

#[no_mangle]
pub extern "C" fn emane_plugin_create() -> *const PluginApi {
    unsafe {
        if API.is_none() {
            API = Some(PluginApi {
                name: CString::new("ieee80211abgmaclayer").unwrap().into_raw(),
                plugin_type: 1, // MAC
                init: init2,
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

static mut FW_SERVICE: Option<emane_core::plugin_interface::FfiFrameworkService> = None;

// Update init to match the new signature
extern "C" fn init2(id: u16, fw_service: *const emane_core::plugin_interface::FfiFrameworkService) -> *mut c_void {
    println!("ieee80211abg init {}", id);
    unsafe {
        FW_SERVICE = Some((*fw_service).clone());
    }
    std::ptr::null_mut()
}
