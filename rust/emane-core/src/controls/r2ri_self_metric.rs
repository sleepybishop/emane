#[derive(Clone)]
pub struct R2RISelfMetricControlMessage {
    pub broadcast_data_rate_bps: u64,
    pub max_data_rate_bps: u64,
    pub report_interval_micros: i64,
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_r2ri_self_metric_create(
    broadcast_data_rate_bps: u64,
    max_data_rate_bps: u64,
    report_interval_micros: i64,
) -> *mut core::ffi::c_void {
    let msg = Box::new(R2RISelfMetricControlMessage {
        broadcast_data_rate_bps,
        max_data_rate_bps,
        report_interval_micros,
    });
    Box::into_raw(msg) as *mut core::ffi::c_void
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_controls_r2ri_self_metric_clone(
    ptr: *const core::ffi::c_void,
) -> *mut core::ffi::c_void {
    let msg = &*(ptr as *const R2RISelfMetricControlMessage);
    let cloned = Box::new(msg.clone());
    Box::into_raw(cloned) as *mut core::ffi::c_void
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_controls_r2ri_self_metric_destroy(ptr: *mut core::ffi::c_void) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr as *mut R2RISelfMetricControlMessage));
    }
}
