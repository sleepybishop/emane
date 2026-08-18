use std::os::raw::c_void;

pub struct ReceiveProcessorImpl {
    id: u16,
    sub_id: u16,
}

#[no_mangle]
pub extern "C" fn emane_rs_receive_processor_create(
    id: u16,
    sub_id: u16,
    _rx_antenna_index: u16,
    _antenna_manager: *mut c_void,
    _spectrum_monitor: *mut c_void,
    _propagation_model: *mut c_void,
    _fading_algorithm_store: *mut c_void,
    _populate_receive_power_map: bool,
    _populate_observed_power_map: bool,
    _doppler_shift: bool
) -> *mut c_void {
    let rp = Box::new(ReceiveProcessorImpl { id, sub_id });
    Box::into_raw(rp) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_receive_processor_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr as *mut ReceiveProcessorImpl)); }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_receive_processor_process(
    _rs_ptr: *mut c_void,
    _now_usec: i64,
    _common_phy_header: *const c_void,
    _location_infos: *const c_void,
    _fading_infos: *const c_void,
    _in_band: bool,
    _result_ptr: *mut c_void
) {
    // Rust implementation will go here
}

#[no_mangle]
pub extern "C" fn emane_rs_receive_processor_process_self_interference(
    _rs_ptr: *mut c_void,
    _now_usec: i64,
    _tx_time_usec: i64,
    _frequency_groups: *const c_void,
    _segment_bandwidth_hz: u64,
    _antenna_interferences: *const c_void,
    _optional_filter_data: *const c_void,
    _result_ptr: *mut c_void
) {
    // Rust implementation will go here
}
