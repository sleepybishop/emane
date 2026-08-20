use std::os::raw::c_void;

pub struct TimeStampControlMessageImpl {
    time_stamp_microsec: u64,
}

#[no_mangle]
pub extern "C" fn emane_rs_timestamp_control_message_create(
    time_stamp_microsec: u64,
) -> *mut c_void {
    let msg = Box::new(TimeStampControlMessageImpl {
        time_stamp_microsec,
    });
    Box::into_raw(msg) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_timestamp_control_message_clone(ptr: *const c_void) -> *mut c_void {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let msg = unsafe { &*(ptr as *const TimeStampControlMessageImpl) };
    let cloned = Box::new(TimeStampControlMessageImpl {
        time_stamp_microsec: msg.time_stamp_microsec,
    });
    Box::into_raw(cloned) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_timestamp_control_message_get_time_stamp(ptr: *const c_void) -> u64 {
    if ptr.is_null() {
        return 0;
    }
    let msg = unsafe { &*(ptr as *const TimeStampControlMessageImpl) };
    msg.time_stamp_microsec
}

#[no_mangle]
pub extern "C" fn emane_rs_timestamp_control_message_free(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr as *mut TimeStampControlMessageImpl));
        }
    }
}
