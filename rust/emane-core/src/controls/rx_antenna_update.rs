use std::os::raw::c_void;

#[derive(Clone, Default)]
pub struct RxAntennaUpdateControlMessageRs {
    pub antenna_idx: u16,
    pub is_ideal_omni: bool,
    pub is_profile_defined: bool,
    pub fixed_gain_dbi: f64,
    pub profile_id: u16,
    pub az_degrees: f64,
    pub el_degrees: f64,
    pub bandwidth_hz: u64,
    pub spectral_mask_idx: u16,
    pub frequency_group_idx: usize,
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_controls_rx_antenna_update_create() -> *mut c_void {
    let msg = Box::new(RxAntennaUpdateControlMessageRs::default());
    Box::into_raw(msg) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_controls_rx_antenna_update_set_antenna(
    ptr: *mut c_void,
    idx: u16,
    is_ideal: bool,
    is_profile: bool,
    fixed_gain: f64,
    profile_id: u16,
    az: f64,
    el: f64,
    bw: u64,
    sm_idx: u16,
    fg_idx: usize,
) {
    if ptr.is_null() {
        return;
    }
    let msg = &mut *(ptr as *mut RxAntennaUpdateControlMessageRs);
    msg.antenna_idx = idx;
    msg.is_ideal_omni = is_ideal;
    msg.is_profile_defined = is_profile;
    msg.fixed_gain_dbi = fixed_gain;
    msg.profile_id = profile_id;
    msg.az_degrees = az;
    msg.el_degrees = el;
    msg.bandwidth_hz = bw;
    msg.spectral_mask_idx = sm_idx;
    msg.frequency_group_idx = fg_idx;
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_controls_rx_antenna_update_clone(
    ptr: *const c_void,
) -> *mut c_void {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let msg = &*(ptr as *const RxAntennaUpdateControlMessageRs);
    let cloned = Box::new(msg.clone());
    Box::into_raw(cloned) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_controls_rx_antenna_update_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr as *mut RxAntennaUpdateControlMessageRs);
    }
}
