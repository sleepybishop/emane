use std::os::raw::c_void;

extern "C" {
    fn emane_bypass_phy_proxy_process_downstream(impl_ptr: *mut c_void, pkt: *mut c_void);
    fn emane_bypass_phy_proxy_process_upstream(impl_ptr: *mut c_void, pkt: *mut c_void);
}

pub struct BypassPhy {
    id: u16,
    p_platform_service: *mut c_void,
    cpp_this: *mut c_void,
}

#[no_mangle]
pub extern "C" fn emane_rs_bypass_phy_create(
    id: u16,
    p_platform_service: *mut c_void,
    cpp_this: *mut c_void,
) -> *mut c_void {
    Box::into_raw(Box::new(BypassPhy {
        id,
        p_platform_service,
        cpp_this,
    })) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_bypass_phy_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr as *mut BypassPhy));
    }
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_bypass_phy_process_downstream(ptr: *mut c_void, pkt: *mut c_void) {
    let state = &mut *(ptr as *mut BypassPhy);
    emane_bypass_phy_proxy_process_downstream(state.cpp_this, pkt);
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_bypass_phy_process_upstream(ptr: *mut c_void, pkt: *mut c_void) {
    let state = &mut *(ptr as *mut BypassPhy);
    emane_bypass_phy_proxy_process_upstream(state.cpp_this, pkt);
}
