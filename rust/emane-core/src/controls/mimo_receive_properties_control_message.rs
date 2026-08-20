use std::os::raw::c_void;

#[derive(Clone)]
pub struct FrequencySegmentRs {
    pub frequency_hz: u64,
    pub rx_power_dbm: f64,
    pub duration_microsec: u64,
    pub offset_microsec: u64,
}

#[derive(Clone)]
pub struct AntennaReceiveInfoRs {
    pub rx_antenna_index: u16,
    pub tx_antenna_index: u16,
    pub span_microsec: u64,
    pub receiver_sensitivity_dbm: f64,
    pub segments: Vec<FrequencySegmentRs>,
}

#[derive(Clone)]
pub struct MimoReceivePropertiesControlMessageRs {
    pub sot_microsec: u64,
    pub propagation_microsec: u64,
    pub infos: Vec<AntennaReceiveInfoRs>,
    pub doppler_shifts: Vec<(u64, i64)>, // (frequency_hz, shift)
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_rx_props_create(
    sot_microsec: u64,
    propagation_microsec: u64,
) -> *mut c_void {
    let msg = Box::new(MimoReceivePropertiesControlMessageRs {
        sot_microsec,
        propagation_microsec,
        infos: Vec::new(),
        doppler_shifts: Vec::new(),
    });
    Box::into_raw(msg) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_rx_props_add_antenna_info(
    ptr: *mut c_void,
    rx_antenna_index: u16,
    tx_antenna_index: u16,
    span_microsec: u64,
    receiver_sensitivity_dbm: f64,
) -> usize {
    let msg = unsafe { &mut *(ptr as *mut MimoReceivePropertiesControlMessageRs) };
    msg.infos.push(AntennaReceiveInfoRs {
        rx_antenna_index,
        tx_antenna_index,
        span_microsec,
        receiver_sensitivity_dbm,
        segments: Vec::new(),
    });
    msg.infos.len() - 1
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_rx_props_add_frequency_segment(
    ptr: *mut c_void,
    info_idx: usize,
    frequency_hz: u64,
    rx_power_dbm: f64,
    duration_microsec: u64,
    offset_microsec: u64,
) {
    let msg = unsafe { &mut *(ptr as *mut MimoReceivePropertiesControlMessageRs) };
    if let Some(info) = msg.infos.get_mut(info_idx) {
        info.segments.push(FrequencySegmentRs {
            frequency_hz,
            rx_power_dbm,
            duration_microsec,
            offset_microsec,
        });
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_rx_props_add_doppler_shift(
    ptr: *mut c_void,
    frequency_hz: u64,
    shift: i64,
) {
    let msg = unsafe { &mut *(ptr as *mut MimoReceivePropertiesControlMessageRs) };
    msg.doppler_shifts.push((frequency_hz, shift));
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_rx_props_clone(ptr: *const c_void) -> *mut c_void {
    let msg = unsafe { &*(ptr as *const MimoReceivePropertiesControlMessageRs) };
    let cloned = Box::new(msg.clone());
    Box::into_raw(cloned) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_rx_props_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr as *mut MimoReceivePropertiesControlMessageRs);
        }
    }
}

// Getters are handled by caching in C++ for now to satisfy the const std::vector& signature.
