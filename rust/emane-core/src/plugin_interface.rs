use std::os::raw::{c_char, c_void};

#[repr(C)]
pub struct FfiSlice {
    pub data: *const u8,
    pub len: usize,
}

#[repr(C)]
pub struct FfiPacketInfo {
    pub source: u16,
    pub destination: u16,
    pub priority: u8,
    pub creation_time_sec: u64,
    pub creation_time_usec: u32,
}

#[repr(C)]
pub struct FfiControlMessage {
    pub msg_type: u32,
    pub payload: FfiSlice,
}

#[repr(C)]
pub struct FfiPacket {
    pub info: FfiPacketInfo,
    pub payload: FfiSlice,
}

#[repr(C)]
#[derive(Clone)]
pub struct FfiFrameworkService {
    // Framework context pointer
    pub framework_ctx: *mut c_void,
    
    // Packet sending
    pub send_downstream_packet: extern "C" fn(
        ctx: *mut c_void,
        nem_id: u16,
        pkt: *const FfiPacket,
        msgs: *const FfiControlMessage,
        num_msgs: usize,
    ),
    
    pub send_upstream_packet: extern "C" fn(
        ctx: *mut c_void,
        nem_id: u16,
        pkt: *const FfiPacket,
        msgs: *const FfiControlMessage,
        num_msgs: usize,
    ),
    
    // Control messaging
    pub send_downstream_control: extern "C" fn(
        ctx: *mut c_void,
        nem_id: u16,
        msgs: *const FfiControlMessage,
        num_msgs: usize,
    ),
    
    pub send_upstream_control: extern "C" fn(
        ctx: *mut c_void,
        nem_id: u16,
        msgs: *const FfiControlMessage,
        num_msgs: usize,
    ),
    
    // Timer Service
    pub schedule_timed_event: extern "C" fn(
        ctx: *mut c_void,
        nem_id: u16,
        time_sec: u64,
        time_usec: u32,
        event_id: u32,
        data: *const u8,
        data_len: usize,
    ) -> u64, // returns timer id
    
    pub cancel_timed_event: extern "C" fn(
        ctx: *mut c_void,
        nem_id: u16,
        timer_id: u64,
    ),
    
    // Log Service
    pub log: extern "C" fn(
        ctx: *mut c_void,
        level: u32,
        msg: *const c_char,
    ),
}

#[repr(C)]
pub struct PluginApi {
    pub name: *const c_char,
    pub plugin_type: u32, // 1=MAC, 2=PHY, 3=SHIM, 4=TRANSPORT
    
    // Lifecycle functions
    pub init: extern "C" fn(id: u16, fw_service: *const FfiFrameworkService) -> *mut c_void,
    pub configure: extern "C" fn(plugin_ptr: *mut c_void, config_req: *const c_void),
    pub start: extern "C" fn(plugin_ptr: *mut c_void),
    pub post_start: extern "C" fn(plugin_ptr: *mut c_void),
    pub stop: extern "C" fn(plugin_ptr: *mut c_void),
    pub destroy: extern "C" fn(plugin_ptr: *mut c_void),
    
    // Processing
    pub process_upstream: extern "C" fn(
        plugin_ptr: *mut c_void, 
        pkt: *const FfiPacket, 
        msgs: *const FfiControlMessage, 
        num_msgs: usize
    ),
    
    pub process_downstream: extern "C" fn(
        plugin_ptr: *mut c_void, 
        pkt: *const FfiPacket, 
        msgs: *const FfiControlMessage, 
        num_msgs: usize
    ),
    
    pub process_timed_event: extern "C" fn(
        plugin_ptr: *mut c_void,
        timer_id: u64,
        event_id: u32,
        data: *const u8,
        data_len: usize,
    ),
}

pub type PluginEntryFunc = extern "C" fn() -> *const PluginApi;
