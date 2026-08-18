use std::os::raw::c_void;

#[derive(Clone)]
pub struct PointingRs {
    pub antenna_profile_id: u16,
    pub azimuth_degrees: f64,
    pub elevation_degrees: f64,
    pub is_valid: bool,
}

#[derive(Clone)]
pub struct AntennaRs {
    pub antenna_index: u16,
    pub fixed_gain_dbi: f64,
    pub pointing: PointingRs,
    pub is_ideal_omni: bool,
    pub is_profile_defined: bool,
    pub frequency_group_index: u16,
    pub bandwidth_hz: u64,
    pub spectral_mask_index: u16,
}

#[derive(Clone)]
pub struct RxAntennaAddControlMessageRs {
    pub antenna: AntennaRs,
    pub frequency_of_interest_set: Vec<u64>,
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_rx_antenna_add_create(
    antenna_index: u16,
    fixed_gain_dbi: f64,
    pointing_profile_id: u16,
    pointing_azimuth: f64,
    pointing_elevation: f64,
    pointing_is_valid: bool,
    is_ideal_omni: bool,
    is_profile_defined: bool,
    frequency_group_index: u16,
    bandwidth_hz: u64,
    spectral_mask_index: u16,
) -> *mut c_void {
    let msg = Box::new(RxAntennaAddControlMessageRs {
        antenna: AntennaRs {
            antenna_index,
            fixed_gain_dbi,
            pointing: PointingRs {
                antenna_profile_id: pointing_profile_id,
                azimuth_degrees: pointing_azimuth,
                elevation_degrees: pointing_elevation,
                is_valid: pointing_is_valid,
            },
            is_ideal_omni,
            is_profile_defined,
            frequency_group_index,
            bandwidth_hz,
            spectral_mask_index,
        },
        frequency_of_interest_set: Vec::new(),
    });
    Box::into_raw(msg) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_rx_antenna_add_add_frequency(
    ptr: *mut c_void,
    frequency_hz: u64,
) {
    let msg = unsafe { &mut *(ptr as *mut RxAntennaAddControlMessageRs) };
    msg.frequency_of_interest_set.push(frequency_hz);
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_rx_antenna_add_clone(ptr: *const c_void) -> *mut c_void {
    let msg = unsafe { &*(ptr as *const RxAntennaAddControlMessageRs) };
    let cloned = Box::new(msg.clone());
    Box::into_raw(cloned) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_rx_antenna_add_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe { let _ = Box::from_raw(ptr as *mut RxAntennaAddControlMessageRs); }
    }
}
