#[derive(Clone)]
pub struct FrequencySegment {
    pub frequency_hz: u64,
    pub power_dbm: f64,
    pub duration: core::time::Duration,
    pub offset: core::time::Duration,
}

#[derive(Clone)]
pub struct FrequencyControlMessage {
    pub bandwidth_hz: u64,
    pub segments: Vec<FrequencySegment>,
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_freq_create(bandwidth_hz: u64) -> *mut core::ffi::c_void {
    let msg = Box::new(FrequencyControlMessage {
        bandwidth_hz,
        segments: Vec::new(),
    });
    Box::into_raw(msg) as *mut core::ffi::c_void
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_controls_freq_add_segment(
    ptr: *mut core::ffi::c_void,
    freq: u64,
    power: f64,
    duration: u64,
    offset: u64,
) {
    let msg = &mut *(ptr as *mut FrequencyControlMessage);
    msg.segments.push(FrequencySegment {
        frequency_hz: freq,
        power_dbm: power,
        duration: core::time::Duration::from_micros(duration),
        offset: core::time::Duration::from_micros(offset),
    });
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_controls_freq_clone(ptr: *const core::ffi::c_void) -> *mut core::ffi::c_void {
    let msg = &*(ptr as *const FrequencyControlMessage);
    let cloned = Box::new(msg.clone());
    Box::into_raw(cloned) as *mut core::ffi::c_void
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_controls_freq_destroy(ptr: *mut core::ffi::c_void) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr as *mut FrequencyControlMessage));
    }
}
