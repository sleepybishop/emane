use std::os::raw::c_void;

#[derive(Clone)]
pub struct RxAntennaRemoveControlMessageRs {
    pub antenna_index: u16,
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_rx_antenna_remove_create(antenna_index: u16) -> *mut c_void {
    let msg = Box::new(RxAntennaRemoveControlMessageRs { antenna_index });
    Box::into_raw(msg) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_rx_antenna_remove_clone(ptr: *const c_void) -> *mut c_void {
    let msg = unsafe { &*(ptr as *const RxAntennaRemoveControlMessageRs) };
    let cloned = Box::new(msg.clone());
    Box::into_raw(cloned) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_rx_antenna_remove_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr as *mut RxAntennaRemoveControlMessageRs);
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_rx_antenna_remove_get_antenna_index(ptr: *const c_void) -> u16 {
    let msg = unsafe { &*(ptr as *const RxAntennaRemoveControlMessageRs) };
    msg.antenna_index
}
