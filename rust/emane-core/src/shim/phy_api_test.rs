use std::os::raw::c_void;
use crate::log_service::emane_rs_log;
use std::ffi::CString;

#[no_mangle]
pub extern "C" fn emane_phyapitest_processUpstreamControl(_layer: *mut c_void, _msgs: *const c_void) {
    let msg = CString::new("SHIMI unexpected control message, drop").unwrap();
    crate::log_service::emane_rs_log(4, msg.as_ptr());
}

#[no_mangle]
pub extern "C" fn emane_phyapitest_processDownstreamControl(_layer: *mut c_void, _msgs: *const c_void) {
    let msg = CString::new("SHIMI unexpected control message, drop").unwrap();
    crate::log_service::emane_rs_log(4, msg.as_ptr());
}

#[no_mangle]
pub extern "C" fn emane_phyapitest_processUpstreamPacket(_layer: *mut c_void, _pkt: *mut c_void, _msgs: *const c_void) {
    // Ported business logic for logging
    let msg = CString::new("SHIMI processUpstreamPacket log from rust").unwrap();
    crate::log_service::emane_rs_log(4, msg.as_ptr());
}

#[no_mangle]
pub extern "C" fn emane_phyapitest_processDownstreamPacket(_layer: *mut c_void, _pkt: *mut c_void, _msgs: *const c_void) {
    let msg = CString::new("SHIMI unexpected packet, drop").unwrap();
    crate::log_service::emane_rs_log(4, msg.as_ptr());
}
