import sys

with open("rust/emane-core/src/nem_manager.rs", "w") as f:
    f.write("""
use std::os::raw::{c_char, c_void};
use std::ffi::CStr;
use std::collections::HashMap;
use libloading::{Library, Symbol};
use crate::plugin_interface::{FfiFrameworkService, PluginApi, PluginEntryFunc, FfiPacket, FfiControlMessage};

pub struct NemLayer {
    lib: Library,
    api: *const PluginApi,
    plugin_ctx: *mut c_void,
}

pub struct NemManager {
    uuid: [u8; 16],
    layers: HashMap<u16, Vec<NemLayer>>,
}

impl NemManager {
    pub fn new(uuid: [u8; 16]) -> Self {
        Self {
            uuid,
            layers: HashMap::new(),
        }
    }

    pub fn add_layer(&mut self, nem_id: u16, lib_path: &str) {
        unsafe {
            println!("Loading plugin: {}", lib_path);
            let lib = Library::new(lib_path).expect("Failed to load plugin");
            let create_func: Symbol<PluginEntryFunc> = lib.get(b"emane_plugin_create").expect("Missing emane_plugin_create");
            
            let api_ptr = create_func();
            
            let fw_svc = FfiFrameworkService {
                framework_ctx: std::ptr::null_mut(),
                send_downstream_packet,
                send_upstream_packet,
                send_downstream_control,
                send_upstream_control,
                schedule_timed_event,
                cancel_timed_event,
                log,
            };
            
            let plugin_ctx = ((*api_ptr).init)(nem_id, &fw_svc as *const _);
            
            self.layers.entry(nem_id).or_default().push(NemLayer {
                lib,
                api: api_ptr,
                plugin_ctx,
            });
        }
    }
}

extern "C" fn send_downstream_packet(_ctx: *mut c_void, _nem_id: u16, _pkt: *const FfiPacket, _msgs: *const FfiControlMessage, _num_msgs: usize) {}
extern "C" fn send_upstream_packet(_ctx: *mut c_void, _nem_id: u16, _pkt: *const FfiPacket, _msgs: *const FfiControlMessage, _num_msgs: usize) {}
extern "C" fn send_downstream_control(_ctx: *mut c_void, _nem_id: u16, _msgs: *const FfiControlMessage, _num_msgs: usize) {}
extern "C" fn send_upstream_control(_ctx: *mut c_void, _nem_id: u16, _msgs: *const FfiControlMessage, _num_msgs: usize) {}
extern "C" fn schedule_timed_event(_ctx: *mut c_void, _nem_id: u16, _time_sec: u64, _time_usec: u32, _event_id: u32, _data: *const u8, _data_len: usize) -> u64 { 0 }
extern "C" fn cancel_timed_event(_ctx: *mut c_void, _nem_id: u16, _timer_id: u64) {}
extern "C" fn log(_ctx: *mut c_void, _level: u32, msg: *const c_char) {
    let s = unsafe { CStr::from_ptr(msg) };
    println!("[EMANE] {}", s.to_string_lossy());
}
""")
