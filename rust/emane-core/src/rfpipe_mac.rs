use std::time::Duration;

use crate::flow_control_manager::FlowControlManager;
use crate::neighbor_metric_manager::NeighborMetricManager;
use crate::pcr_manager::PCRManager;
use crate::queue_metric_manager::QueueMetricManager;
use crate::rf_signal_table::RFSignalTable;

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

#[no_mangle]
pub extern "C" fn emane_rs_rfpipe_mac_new(id: u16) -> *mut RfpipeMac {
    Box::into_raw(Box::new(RfpipeMac::new(id)))
}

use rand::Rng;

#[repr(C)]
pub struct RfpipeUpstreamAction {
    pub action: u8, // 0 = drop, 1 = send_upstream
    pub drop_code: u16,
}

#[repr(C)]
pub struct RfpipeDownstreamAction {
    pub action: u8, // 0 = drop, 1 = enqueue
    pub drop_code: u16,
    pub duration_microseconds: u64,
    pub delay_microseconds: u64,
}

#[no_mangle]
pub extern "C" fn emane_rs_rfpipe_mac_process_upstream(
    ptr: *mut RfpipeMac,
    sinr: f64,
    pkt_length: usize,
    src: u16,
    dst: u16,
    sequence_number: u64,
    noise_floor_db: f64,
    start_of_reception_microseconds: u64,
    duration_microseconds: u64,
    data_rate: u64,
    frequency_hz: u64,
    rx_power_dbm: f64,
    receiver_sensitivity_db: f64,
) -> RfpipeUpstreamAction {
    let state = unsafe { &mut *ptr };

    let pcr = state.pcr_manager.get_pcr(sinr as f32, pkt_length);
    let rnd: f32 = rand::thread_rng().gen();

    if pcr < rnd {
        return RfpipeUpstreamAction { action: 0, drop_code: 1 }; // DROP_CODE_SINR
    }

    state.neighbor_metric_manager.handle_rx_activity(
        src,
        sequence_number,
        &[0; 16], // uuid placeholder
        sinr,
        noise_floor_db,
        Duration::from_micros(start_of_reception_microseconds),
        Duration::from_micros(duration_microseconds),
        data_rate,
    );

    state.rf_signal_table.update(
        src,
        0,
        frequency_hz,
        rx_power_dbm,
        sinr,
        noise_floor_db,
        receiver_sensitivity_db,
    );

    if state.promiscuous_mode || dst == state.id || dst == 0xFFFF {
        RfpipeUpstreamAction { action: 1, drop_code: 0 }
    } else {
        RfpipeUpstreamAction { action: 0, drop_code: 3 } // DROP_CODE_DST_MAC
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_rfpipe_mac_process_downstream(
    ptr: *mut RfpipeMac,
    pkt_length: usize,
) -> RfpipeDownstreamAction {
    let state = unsafe { &mut *ptr };

    if state.flow_control_enable {
        let (_tokens, success) = state.flow_control_manager.remove_token();
        if !success {
            return RfpipeDownstreamAction {
                action: 0,
                drop_code: 7, // DROP_CODE_FLOW_CONTROL_ERROR
                duration_microseconds: 0,
                delay_microseconds: 0,
            };
        }
    }

    let duration_microseconds = if state.data_rate_bps > 0 {
        ((pkt_length as f64 * 8.0) / state.data_rate_bps as f64 * 1_000_000.0) as u64
    } else {
        0
    };

    let delay_microseconds = state.delay.as_micros() as u64;
    // Jitter would be added here if fJitterSeconds_ > 0

    RfpipeDownstreamAction {
        action: 1,
        drop_code: 0,
        duration_microseconds,
        delay_microseconds,
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_rfpipe_mac_downstream_dequeue(
    ptr: *mut RfpipeMac,
    dst: u16,
    delay_microseconds: u64,
    queue_size: u32,
    queue_depth: u32,
    queue_discards: u32,
) {
    let state = unsafe { &mut *ptr };

    if state.flow_control_enable {
        state.flow_control_manager.add_token(1);
    }

    state.queue_metric_manager.update_queue_metric(
        0,
        queue_size,
        queue_depth,
        queue_discards,
        delay_microseconds,
    );

    state.neighbor_metric_manager.handle_tx_activity(
        dst,
        state.data_rate_bps,
        Duration::from_micros(0), // using now internally is complex, maybe just omit?
    );
}

#[no_mangle]
pub extern "C" fn emane_rs_rfpipe_mac_flow_control_add_token(ptr: *mut RfpipeMac) -> bool {
    let state = unsafe { &mut *ptr };
    if state.flow_control_enable {
        let (_, success, _) = state.flow_control_manager.add_token(1);
        success
    } else {
        true
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_rfpipe_mac_free(ptr: *mut RfpipeMac) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr)) }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_rfpipe_mac_configure(
    ptr: *mut RfpipeMac,
    promiscuous_mode: bool,
    data_rate_bps: u64,
    delay_microseconds: u64,
    flow_control_enable: bool,
    radio_metric_enable: bool,
    flow_control_tokens: u16,
    pcr_curve_uri: *const std::os::raw::c_char,
    radio_metric_report_interval_microseconds: u64,
    neighbor_metric_delete_time_microseconds: u64,
) {
    let state = unsafe { &mut *ptr };
    state.promiscuous_mode = promiscuous_mode;
    state.data_rate_bps = data_rate_bps;
    state.delay = Duration::from_micros(delay_microseconds);
    state.flow_control_enable = flow_control_enable;
    state.radio_metric_enable = radio_metric_enable;
    state.flow_control_tokens = flow_control_tokens;
    
    if !pcr_curve_uri.is_null() {
        let c_str = unsafe { std::ffi::CStr::from_ptr(pcr_curve_uri) };
        if let Ok(s) = c_str.to_str() {
            state.pcr_curve_uri = s.to_owned();
        }
    }
    
    state.radio_metric_report_interval = Duration::from_micros(radio_metric_report_interval_microseconds);
    state.neighbor_metric_delete_time = Duration::from_micros(neighbor_metric_delete_time_microseconds);
}

#[no_mangle]
pub extern "C" fn emane_rs_rfpipe_mac_start(ptr: *mut RfpipeMac) {
    let state = unsafe { &mut *ptr };
    if state.flow_control_enable {
        state.flow_control_manager.start(state.flow_control_tokens);
    }
    state.neighbor_metric_manager.set_neighbor_delete_time_microseconds(state.neighbor_metric_delete_time);
    if !state.pcr_curve_uri.is_empty() {
        let _ = state.pcr_manager.load(&state.pcr_curve_uri);
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_rfpipe_mac_process_flow_control_message(
    ptr: *mut RfpipeMac,
    msg_tokens: u16,
) -> bool {
    let state = unsafe { &mut *ptr };
    if state.flow_control_enable {
        state.flow_control_manager.process_flow_control_message(msg_tokens)
    } else {
        false
    }
}

