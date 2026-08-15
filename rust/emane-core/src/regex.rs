use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::ptr;
use regex::Regex;

#[no_mangle]
pub extern "C" fn emane_rs_regex_compile(pattern: *const c_char, error_buf: *mut c_char, error_buf_len: usize) -> *mut c_void {
    if pattern.is_null() { return ptr::null_mut(); }
    let pat = unsafe { CStr::from_ptr(pattern).to_string_lossy() };
    match Regex::new(&pat) {
        Ok(r) => Box::into_raw(Box::new(r)) as *mut c_void,
        Err(e) => {
            if !error_buf.is_null() && error_buf_len > 0 {
                let msg = e.to_string();
                let bytes = msg.as_bytes();
                let copy_len = std::cmp::min(bytes.len(), error_buf_len - 1);
                unsafe {
                    std::ptr::copy_nonoverlapping(bytes.as_ptr(), error_buf as *mut u8, copy_len);
                    *error_buf.add(copy_len) = 0;
                }
            }
            ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_regex_free(re: *mut c_void) {
    if !re.is_null() {
        unsafe { drop(Box::from_raw(re as *mut Regex)); }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_regex_match(re: *mut c_void, value: *const c_char) -> bool {
    if re.is_null() || value.is_null() { return false; }
    let regex = unsafe { &*(re as *mut Regex) };
    let val = unsafe { CStr::from_ptr(value).to_string_lossy() };
    regex.is_match(&val)
}
