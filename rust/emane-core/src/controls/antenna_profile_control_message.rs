use std::os::raw::c_void;

#[derive(Clone)]
pub struct AntennaProfileControlMessageRs {
    pub id: u16,
    pub azimuth: f64,
    pub elevation: f64,
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_antenna_profile_create(
    id: u16,
    azimuth: f64,
    elevation: f64,
) -> *mut c_void {
    let msg = Box::new(AntennaProfileControlMessageRs {
        id,
        azimuth,
        elevation,
    });
    Box::into_raw(msg) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_antenna_profile_clone(
    ptr: *const c_void,
) -> *mut c_void {
    let msg = unsafe { &*(ptr as *const AntennaProfileControlMessageRs) };
    let cloned = Box::new(msg.clone());
    Box::into_raw(cloned) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_antenna_profile_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr as *mut AntennaProfileControlMessageRs);
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_antenna_profile_get_id(ptr: *const c_void) -> u16 {
    let msg = unsafe { &*(ptr as *const AntennaProfileControlMessageRs) };
    msg.id
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_antenna_profile_get_azimuth(ptr: *const c_void) -> f64 {
    let msg = unsafe { &*(ptr as *const AntennaProfileControlMessageRs) };
    msg.azimuth
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_antenna_profile_get_elevation(ptr: *const c_void) -> f64 {
    let msg = unsafe { &*(ptr as *const AntennaProfileControlMessageRs) };
    msg.elevation
}
