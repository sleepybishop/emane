use std::os::raw::c_void;

#[derive(Clone)]
pub struct SpectrumFilterRemoveControlMessageRs {
    pub filter_index: u16,
    pub antenna_index: u16,
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_spectrum_filter_remove_create(
    filter_index: u16,
    antenna_index: u16,
) -> *mut c_void {
    let msg = Box::new(SpectrumFilterRemoveControlMessageRs {
        filter_index,
        antenna_index,
    });
    Box::into_raw(msg) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_spectrum_filter_remove_clone(
    ptr: *const c_void,
) -> *mut c_void {
    let msg = unsafe { &*(ptr as *const SpectrumFilterRemoveControlMessageRs) };
    let cloned = Box::new(msg.clone());
    Box::into_raw(cloned) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_spectrum_filter_remove_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr as *mut SpectrumFilterRemoveControlMessageRs);
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_spectrum_filter_remove_get_filter_index(ptr: *const c_void) -> u16 {
    let msg = unsafe { &*(ptr as *const SpectrumFilterRemoveControlMessageRs) };
    msg.filter_index
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_spectrum_filter_remove_get_antenna_index(ptr: *const c_void) -> u16 {
    let msg = unsafe { &*(ptr as *const SpectrumFilterRemoveControlMessageRs) };
    msg.antenna_index
}
