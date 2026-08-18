#[derive(Clone)]
pub struct FlowControlControlMessage {
    pub tokens: u16,
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_flow_control_create(tokens: u16) -> *mut core::ffi::c_void {
    let msg = Box::new(FlowControlControlMessage { tokens });
    Box::into_raw(msg) as *mut core::ffi::c_void
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_controls_flow_control_clone(ptr: *const core::ffi::c_void) -> *mut core::ffi::c_void {
    let msg = &*(ptr as *const FlowControlControlMessage);
    let cloned = Box::new(msg.clone());
    Box::into_raw(cloned) as *mut core::ffi::c_void
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_controls_flow_control_destroy(ptr: *mut core::ffi::c_void) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr as *mut FlowControlControlMessage));
    }
}
