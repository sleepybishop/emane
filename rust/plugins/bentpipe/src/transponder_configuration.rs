#[derive(Clone, Default)]
pub struct TransponderConfiguration {
    pub transponder_index: u16,
    pub receive_frequency_hz: u64,
    pub receive_bandwidth_hz: u64,
    pub receive_antenna_index: u16,
    pub curve_index: u16,
    pub receive_action: u16,
    pub transmit_frequency_hz: u64,
    pub transmit_bandwidth_hz: u64,
    pub transmit_data_rate_bps: u64,
    pub transmit_mtu_bytes: u64,
    pub transmit_antenna_index: u16,
    pub transmit_power_dbm: f64,
    pub transmit_ubend_delay_us: u64,
    pub transmit_slot_size_us: u64,
    pub transmit_slots_per_frame: u16,
    pub receive_enable: bool,
    pub transmit_enable: bool,
}

#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_new(index: u16) -> *mut TransponderConfiguration {
    let mut tc = Box::new(TransponderConfiguration::default());
    tc.transponder_index = index;
    Box::into_raw(tc)
}

#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_clone(
    ptr: *const TransponderConfiguration,
) -> *mut TransponderConfiguration {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let orig = unsafe { &*ptr };
    Box::into_raw(Box::new(orig.clone()))
}

#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_free(ptr: *mut TransponderConfiguration) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

// Getters
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_get_index(ptr: *const TransponderConfiguration) -> u16 {
    unsafe { (*ptr).transponder_index }
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_get_rx_freq(ptr: *const TransponderConfiguration) -> u64 {
    unsafe { (*ptr).receive_frequency_hz }
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_get_rx_bw(ptr: *const TransponderConfiguration) -> u64 {
    unsafe { (*ptr).receive_bandwidth_hz }
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_get_rx_ant(ptr: *const TransponderConfiguration) -> u16 {
    unsafe { (*ptr).receive_antenna_index }
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_get_curve(ptr: *const TransponderConfiguration) -> u16 {
    unsafe { (*ptr).curve_index }
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_get_rx_action(ptr: *const TransponderConfiguration) -> u16 {
    unsafe { (*ptr).receive_action }
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_get_rx_en(ptr: *const TransponderConfiguration) -> bool {
    unsafe { (*ptr).receive_enable }
}

#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_get_tx_freq(ptr: *const TransponderConfiguration) -> u64 {
    unsafe { (*ptr).transmit_frequency_hz }
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_get_tx_bw(ptr: *const TransponderConfiguration) -> u64 {
    unsafe { (*ptr).transmit_bandwidth_hz }
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_get_tx_rate(ptr: *const TransponderConfiguration) -> u64 {
    unsafe { (*ptr).transmit_data_rate_bps }
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_get_tx_mtu(ptr: *const TransponderConfiguration) -> u64 {
    unsafe { (*ptr).transmit_mtu_bytes }
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_get_tx_ant(ptr: *const TransponderConfiguration) -> u16 {
    unsafe { (*ptr).transmit_antenna_index }
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_get_tx_pwr(ptr: *const TransponderConfiguration) -> f64 {
    unsafe { (*ptr).transmit_power_dbm }
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_get_tx_slots_per_frame(
    ptr: *const TransponderConfiguration,
) -> u16 {
    unsafe { (*ptr).transmit_slots_per_frame }
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_get_tx_en(ptr: *const TransponderConfiguration) -> bool {
    unsafe { (*ptr).transmit_enable }
}

// Setters
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_set_rx_freq(ptr: *mut TransponderConfiguration, v: u64) {
    unsafe {
        (*ptr).receive_frequency_hz = v;
    }
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_set_rx_bw(ptr: *mut TransponderConfiguration, v: u64) {
    unsafe {
        (*ptr).receive_bandwidth_hz = v;
    }
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_set_rx_ant(ptr: *mut TransponderConfiguration, v: u16) {
    unsafe {
        (*ptr).receive_antenna_index = v;
    }
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_set_curve(ptr: *mut TransponderConfiguration, v: u16) {
    unsafe {
        (*ptr).curve_index = v;
    }
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_set_rx_action(ptr: *mut TransponderConfiguration, v: u16) {
    unsafe {
        (*ptr).receive_action = v;
    }
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_set_rx_en(ptr: *mut TransponderConfiguration, v: bool) {
    unsafe {
        (*ptr).receive_enable = v;
    }
}

#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_set_tx_freq(ptr: *mut TransponderConfiguration, v: u64) {
    unsafe {
        (*ptr).transmit_frequency_hz = v;
    }
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_set_tx_bw(ptr: *mut TransponderConfiguration, v: u64) {
    unsafe {
        (*ptr).transmit_bandwidth_hz = v;
    }
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_set_tx_rate(ptr: *mut TransponderConfiguration, v: u64) {
    unsafe {
        (*ptr).transmit_data_rate_bps = v;
    }
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_set_tx_mtu(ptr: *mut TransponderConfiguration, v: u64) {
    unsafe {
        (*ptr).transmit_mtu_bytes = v;
    }
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_set_tx_ant(ptr: *mut TransponderConfiguration, v: u16) {
    unsafe {
        (*ptr).transmit_antenna_index = v;
    }
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_set_tx_pwr(ptr: *mut TransponderConfiguration, v: f64) {
    unsafe {
        (*ptr).transmit_power_dbm = v;
    }
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_set_tx_slots_per_frame(
    ptr: *mut TransponderConfiguration,
    v: u16,
) {
    unsafe {
        (*ptr).transmit_slots_per_frame = v;
    }
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_set_tx_en(ptr: *mut TransponderConfiguration, v: bool) {
    unsafe {
        (*ptr).transmit_enable = v;
    }
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_get_tx_delay(ptr: *const TransponderConfiguration) -> u64 {
    unsafe { (*ptr).transmit_ubend_delay_us }
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_get_tx_slot_size(ptr: *const TransponderConfiguration) -> u64 {
    unsafe { (*ptr).transmit_slot_size_us }
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_set_tx_delay(ptr: *mut TransponderConfiguration, v: u64) {
    unsafe {
        (*ptr).transmit_ubend_delay_us = v;
    }
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_tc_set_tx_slot_size(ptr: *mut TransponderConfiguration, v: u64) {
    unsafe {
        (*ptr).transmit_slot_size_us = v;
    }
}
