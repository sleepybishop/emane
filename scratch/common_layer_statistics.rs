
// rust/emane-core/src/common_layer_statistics.rs
use std::os::raw::{c_void, c_char};
use std::collections::HashMap;

#[repr(C)]
pub struct CommonLayerStatisticsState {
    pub p_statistic_unicast_drop_table: *mut c_void,
    pub p_statistic_broadcast_drop_table: *mut c_void,
    pub p_statistic_unicast_accept_table: *mut c_void,
    pub p_statistic_broadcast_accept_table: *mut c_void,
    // Add counters and drop map logic here
}

#[no_mangle]
pub extern "C" fn emane_rs_common_layer_statistics_new() -> *mut c_void {
    let state = Box::new(CommonLayerStatisticsState {
        p_statistic_unicast_drop_table: std::ptr::null_mut(),
        p_statistic_broadcast_drop_table: std::ptr::null_mut(),
        p_statistic_unicast_accept_table: std::ptr::null_mut(),
        p_statistic_broadcast_accept_table: std::ptr::null_mut(),
    });
    Box::into_raw(state) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_common_layer_statistics_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr as *mut CommonLayerStatisticsState)); }
    }
}
