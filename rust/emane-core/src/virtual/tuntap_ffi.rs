use super::tuntap::TunTap;
use libc::{c_char, iovec};
use std::ffi::CStr;

#[no_mangle]
pub extern "C" fn emane_rs_tuntap_new() -> *mut TunTap {
    // We can't really create it without open... wait, original TunTap constructor does nothing much.
    // open() actually does the work. So we return null here, or we return an empty struct?
    // Let's create an empty one and modify it later, or just handle it in open.
    Box::into_raw(Box::new(TunTap { fd: -1, name: String::new(), index: -1 }))
}

#[no_mangle]
pub extern "C" fn emane_rs_tuntap_free(ptr: *mut TunTap) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr)); }
    }
}
