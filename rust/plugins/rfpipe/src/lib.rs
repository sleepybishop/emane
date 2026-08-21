use std::time::Duration;
use std::os::raw::{c_char, c_void};
use std::ffi::CString;

use emane_core::flow_control_manager::FlowControlManager;
use emane_core::neighbor_metric_manager::NeighborMetricManager;
use emane_core::pcr_manager::PCRManager;
use emane_core::queue_metric_manager::QueueMetricManager;
use emane_core::rf_signal_table::RFSignalTable;
use emane_core::plugin_interface::{PluginApi, FfiPacket, FfiControlMessage};

pub struct RfpipeMac {
    id: u16,
    flow_control_manager: FlowControlManager,
    pcr_manager: PCRManager,
    neighbor_metric_manager: NeighborMetricManager,
    queue_metric_manager: QueueMetricManager,
    rf_signal_table: RFSignalTable,

    promiscuous_mode: bool,
    data_rate_bps: u64,
    delay: Duration,
    flow_control_enable: bool,
    radio_metric_enable: bool,
    flow_control_tokens: u16,
    pcr_curve_uri: String,
    radio_metric_report_interval: Duration,
    neighbor_metric_delete_time: Duration,
}

impl RfpipeMac {
    pub fn new(id: u16) -> Self {
        Self {
            id,
            flow_control_manager: FlowControlManager::new(),
            pcr_manager: PCRManager::new(),
            neighbor_metric_manager: NeighborMetricManager::new(id),
            queue_metric_manager: QueueMetricManager::new(id),
            rf_signal_table: RFSignalTable::new(id),
            promiscuous_mode: false,
            data_rate_bps: 1000000,
            delay: Duration::from_secs(0),
            flow_control_enable: false,
            radio_metric_enable: false,
            flow_control_tokens: 10,
            pcr_curve_uri: String::new(),
            radio_metric_report_interval: Duration::from_secs(1),
            neighbor_metric_delete_time: Duration::from_secs(60),
        }
    }
}

use rand::Rng;

pub struct RfpipeUpstreamAction {
    pub action: u8,
    pub drop_code: u16,
}

pub struct RfpipeDownstreamAction {
    pub action: u8,
    pub drop_code: u16,
    pub duration_microseconds: u64,
    pub delay_microseconds: u64,
}

extern "C" fn init2(id: u16, fw_service: *const emane_core::plugin_interface::FfiFrameworkService) -> *mut c_void {
    unsafe {
        FW_SERVICE = Some((*fw_service).clone());
    }
    Box::into_raw(Box::new(RfpipeMac::new(id))) as *mut c_void
}

extern "C" fn configure(_ptr: *mut c_void, _req: *const c_void) {}
extern "C" fn start(_ptr: *mut c_void) {}
extern "C" fn post_start(_ptr: *mut c_void) {}
extern "C" fn stop(_ptr: *mut c_void) {}
extern "C" fn destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr as *mut RfpipeMac)) }
    }
}

extern "C" fn process_upstream(
    _plugin_ptr: *mut c_void, 
    _pkt: *const FfiPacket, 
    _msgs: *const FfiControlMessage, 
    _num_msgs: usize
) {
}

extern "C" fn process_downstream(
    _plugin_ptr: *mut c_void, 
    _pkt: *const FfiPacket, 
    _msgs: *const FfiControlMessage, 
    _num_msgs: usize
) {
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
static mut FW_SERVICE: Option<emane_core::plugin_interface::FfiFrameworkService> = None;

#[no_mangle]
pub extern "C" fn emane_plugin_create() -> *const PluginApi {
    unsafe {
        if API.is_none() {
            API = Some(PluginApi {
                name: CString::new("rfpipemaclayer").unwrap().into_raw(),
                plugin_type: 1,
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

