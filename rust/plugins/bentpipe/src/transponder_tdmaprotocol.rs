pub struct TransponderTdmaProtocol {}

#[no_mangle]
pub extern "C" fn rust_bentpipe_ttp_new() -> *mut TransponderTdmaProtocol {
    Box::into_raw(Box::new(TransponderTdmaProtocol {}))
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_ttp_free(ptr: *mut TransponderTdmaProtocol) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}
#[no_mangle]
pub extern "C" fn rust_bentpipe_ttp_start(_ptr: *mut TransponderTdmaProtocol) {}
#[no_mangle]
pub extern "C" fn rust_bentpipe_ttp_stop(_ptr: *mut TransponderTdmaProtocol) {}
