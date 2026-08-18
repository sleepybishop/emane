use std::os::raw::c_void;
use std::slice;

#[derive(Clone)]
pub struct SpectrumFilterDataControlMessageRs {
    pub filter_data: Vec<u8>,
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_spectrum_filter_data_create(
    data: *const u8,
    len: usize,
) -> *mut c_void {
    let filter_data = if !data.is_null() && len > 0 {
        unsafe { slice::from_raw_parts(data, len).to_vec() }
    } else {
        Vec::new()
    };
    let msg = Box::new(SpectrumFilterDataControlMessageRs { filter_data });
    Box::into_raw(msg) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_spectrum_filter_data_clone(
    ptr: *const c_void,
) -> *mut c_void {
    let msg = unsafe { &*(ptr as *const SpectrumFilterDataControlMessageRs) };
    let cloned = Box::new(msg.clone());
    Box::into_raw(cloned) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_spectrum_filter_data_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr as *mut SpectrumFilterDataControlMessageRs);
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_spectrum_filter_data_get_filter_data(
    ptr: *const c_void,
    len: *mut usize,
) -> *const u8 {
    let msg = unsafe { &*(ptr as *const SpectrumFilterDataControlMessageRs) };
    if !len.is_null() {
        unsafe {
            *len = msg.filter_data.len();
        }
    }
    msg.filter_data.as_ptr()
}
