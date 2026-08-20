use std::os::raw::c_void;

pub struct TxWhileRxInterferenceControlMessageImpl {
    rx_power_dbm: f64,
}

#[no_mangle]
pub extern "C" fn emane_rs_tx_while_rx_interference_control_message_create(
    rx_power_dbm: f64,
) -> *mut c_void {
    let msg = Box::new(TxWhileRxInterferenceControlMessageImpl { rx_power_dbm });
    Box::into_raw(msg) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_tx_while_rx_interference_control_message_clone(
    ptr: *const c_void,
) -> *mut c_void {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let msg = unsafe { &*(ptr as *const TxWhileRxInterferenceControlMessageImpl) };
    let cloned = Box::new(TxWhileRxInterferenceControlMessageImpl {
        rx_power_dbm: msg.rx_power_dbm,
    });
    Box::into_raw(cloned) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_tx_while_rx_interference_control_message_get_rx_power_dbm(
    ptr: *const c_void,
) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    let msg = unsafe { &*(ptr as *const TxWhileRxInterferenceControlMessageImpl) };
    msg.rx_power_dbm
}

#[no_mangle]
pub extern "C" fn emane_rs_tx_while_rx_interference_control_message_free(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(
                ptr as *mut TxWhileRxInterferenceControlMessageImpl,
            ));
        }
    }
}
