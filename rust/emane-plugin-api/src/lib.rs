use std::os::raw::{c_char, c_void};

// Version 2 makes configuration and start failures explicit in the ABI.
pub const PLUGIN_ABI_VERSION: u32 = 2;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FfiSlice {
    pub data: *const u8,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FfiPacketInfo {
    pub source: u16,
    pub destination: u16,
    pub priority: u8,
    pub creation_time_sec: u64,
    pub creation_time_usec: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FfiControlMessage {
    pub msg_type: u32,
    pub payload: FfiSlice,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FfiPacket {
    pub info: FfiPacketInfo,
    pub payload: FfiSlice,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FfiFrameworkService {
    pub framework_ctx: *mut c_void,
    pub send_downstream_packet: extern "C" fn(
        ctx: *mut c_void,
        nem_id: u16,
        packet: *const FfiPacket,
        messages: *const FfiControlMessage,
        message_count: usize,
    ),
    pub send_upstream_packet: extern "C" fn(
        ctx: *mut c_void,
        nem_id: u16,
        packet: *const FfiPacket,
        messages: *const FfiControlMessage,
        message_count: usize,
    ),
    pub send_downstream_control: extern "C" fn(
        ctx: *mut c_void,
        nem_id: u16,
        messages: *const FfiControlMessage,
        message_count: usize,
    ),
    pub send_upstream_control: extern "C" fn(
        ctx: *mut c_void,
        nem_id: u16,
        messages: *const FfiControlMessage,
        message_count: usize,
    ),
    pub schedule_timed_event: extern "C" fn(
        ctx: *mut c_void,
        nem_id: u16,
        time_sec: u64,
        time_usec: u32,
        event_id: u32,
        data: *const u8,
        data_len: usize,
    ) -> u64,
    pub cancel_timed_event: extern "C" fn(ctx: *mut c_void, nem_id: u16, timer_id: u64),
    pub log: extern "C" fn(ctx: *mut c_void, level: u32, message: *const c_char),
}

unsafe impl Send for FfiFrameworkService {}
unsafe impl Sync for FfiFrameworkService {}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FfiConfigStringArray {
    pub data: *const *const c_char,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FfiConfigItem {
    pub name: *const c_char,
    pub values: FfiConfigStringArray,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FfiConfigRequest {
    pub data: *const FfiConfigItem,
    pub len: usize,
}

#[repr(C)]
pub struct PluginApi {
    pub abi_version: u32,
    pub struct_size: usize,
    pub name: *const c_char,
    pub plugin_type: u32,
    pub init: extern "C" fn(id: u16, framework: *const FfiFrameworkService) -> *mut c_void,
    pub configure: extern "C" fn(plugin: *mut c_void, request: *const c_void) -> bool,
    pub start: extern "C" fn(plugin: *mut c_void) -> bool,
    pub post_start: extern "C" fn(plugin: *mut c_void),
    pub stop: extern "C" fn(plugin: *mut c_void),
    pub destroy: extern "C" fn(plugin: *mut c_void),
    pub process_upstream: extern "C" fn(
        plugin: *mut c_void,
        packet: *const FfiPacket,
        messages: *const FfiControlMessage,
        message_count: usize,
    ),
    pub process_downstream: extern "C" fn(
        plugin: *mut c_void,
        packet: *const FfiPacket,
        messages: *const FfiControlMessage,
        message_count: usize,
    ),
    pub process_timed_event: extern "C" fn(
        plugin: *mut c_void,
        timer_id: u64,
        event_id: u32,
        data: *const u8,
        data_len: usize,
    ),
}

unsafe impl Send for PluginApi {}
unsafe impl Sync for PluginApi {}

pub type PluginEntryFunc = extern "C" fn() -> *const PluginApi;
