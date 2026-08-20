use std::os::raw::c_void;

pub struct ReceivePropertiesControlMessageImpl {
    sot_microsec: u64,
    propagation_microsec: u64,
    span_microsec: u64,
    receiver_sensitivity_dbm: f64,
}

#[no_mangle]
pub extern "C" fn emane_rs_receive_properties_control_message_create(
    sot_microsec: u64,
    propagation_microsec: u64,
    span_microsec: u64,
    receiver_sensitivity_dbm: f64,
) -> *mut c_void {
    let msg = Box::new(ReceivePropertiesControlMessageImpl {
        sot_microsec,
        propagation_microsec,
        span_microsec,
        receiver_sensitivity_dbm,
    });
    Box::into_raw(msg) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_receive_properties_control_message_clone(
    ptr: *const c_void,
) -> *mut c_void {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let msg = unsafe { &*(ptr as *const ReceivePropertiesControlMessageImpl) };
    let cloned = Box::new(ReceivePropertiesControlMessageImpl {
        sot_microsec: msg.sot_microsec,
        propagation_microsec: msg.propagation_microsec,
        span_microsec: msg.span_microsec,
        receiver_sensitivity_dbm: msg.receiver_sensitivity_dbm,
    });
    Box::into_raw(cloned) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_receive_properties_control_message_get_tx_time(
    ptr: *const c_void,
) -> u64 {
    if ptr.is_null() {
        return 0;
    }
    let msg = unsafe { &*(ptr as *const ReceivePropertiesControlMessageImpl) };
    msg.sot_microsec
}

#[no_mangle]
pub extern "C" fn emane_rs_receive_properties_control_message_get_propagation_delay(
    ptr: *const c_void,
) -> u64 {
    if ptr.is_null() {
        return 0;
    }
    let msg = unsafe { &*(ptr as *const ReceivePropertiesControlMessageImpl) };
    msg.propagation_microsec
}

#[no_mangle]
pub extern "C" fn emane_rs_receive_properties_control_message_get_span(ptr: *const c_void) -> u64 {
    if ptr.is_null() {
        return 0;
    }
    let msg = unsafe { &*(ptr as *const ReceivePropertiesControlMessageImpl) };
    msg.span_microsec
}

#[no_mangle]
pub extern "C" fn emane_rs_receive_properties_control_message_get_receiver_sensitivity_dbm(
    ptr: *const c_void,
) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    let msg = unsafe { &*(ptr as *const ReceivePropertiesControlMessageImpl) };
    msg.receiver_sensitivity_dbm
}

#[no_mangle]
pub extern "C" fn emane_rs_receive_properties_control_message_free(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(
                ptr as *mut ReceivePropertiesControlMessageImpl,
            ));
        }
    }
}
