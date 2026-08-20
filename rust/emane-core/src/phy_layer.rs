use std::os::raw::c_void;

#[repr(C)]
pub struct PHYLayer {
    pub cpp_this: *mut c_void,
    pub implementor: *mut c_void,
}

extern "C" {
    fn emane_c_phy_layer_implementor_initialize(impl_ptr: *mut c_void, registrar: *mut c_void);
    fn emane_c_phy_layer_implementor_configure(impl_ptr: *mut c_void, update: *mut c_void);
    fn emane_c_phy_layer_implementor_start(impl_ptr: *mut c_void);
    fn emane_c_phy_layer_implementor_post_start(impl_ptr: *mut c_void);
    fn emane_c_phy_layer_implementor_stop(impl_ptr: *mut c_void);
    fn emane_c_phy_layer_implementor_destroy(impl_ptr: *mut c_void);
    fn emane_c_phy_layer_implementor_process_configuration(
        impl_ptr: *mut c_void,
        update: *mut c_void,
    );
    fn emane_c_phy_layer_implementor_process_upstream_packet(
        impl_ptr: *mut c_void,
        pkt: *mut c_void,
        msgs: *mut c_void,
    );
    fn emane_c_phy_layer_implementor_process_downstream_packet(
        impl_ptr: *mut c_void,
        pkt: *mut c_void,
        msgs: *mut c_void,
    );
    fn emane_c_phy_layer_implementor_process_upstream_control(
        impl_ptr: *mut c_void,
        msgs: *mut c_void,
    );
    fn emane_c_phy_layer_implementor_process_downstream_control(
        impl_ptr: *mut c_void,
        msgs: *mut c_void,
    );
    fn emane_c_phy_layer_implementor_set_upstream_transport(
        impl_ptr: *mut c_void,
        transport: *mut c_void,
    );
    fn emane_c_phy_layer_implementor_set_downstream_transport(
        impl_ptr: *mut c_void,
        transport: *mut c_void,
    );
    fn emane_c_phy_layer_implementor_process_event(
        impl_ptr: *mut c_void,
        event_id: *mut c_void,
        serialization: *mut c_void,
    );
    fn emane_c_phy_layer_implementor_process_timed_event(
        impl_ptr: *mut c_void,
        event_id: *mut c_void,
        expire: *const c_void,
        schedule: *const c_void,
        fire: *const c_void,
        arg: *const c_void,
    );
}

impl PHYLayer {
    pub fn new(cpp_this: *mut c_void, implementor: *mut c_void) -> Self {
        Self {
            cpp_this,
            implementor,
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_phy_layer_create(
    cpp_this: *mut c_void,
    implementor: *mut c_void,
) -> *mut c_void {
    Box::into_raw(Box::new(PHYLayer::new(cpp_this, implementor))) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_phy_layer_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr as *mut PHYLayer));
    }
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_phy_layer_initialize(ptr: *mut c_void, registrar: *mut c_void) {
    let shim = &mut *(ptr as *mut PHYLayer);
    emane_c_phy_layer_implementor_initialize(shim.implementor, registrar);
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_phy_layer_configure(ptr: *mut c_void, update: *mut c_void) {
    let shim = &mut *(ptr as *mut PHYLayer);
    emane_c_phy_layer_implementor_configure(shim.implementor, update);
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_phy_layer_start(ptr: *mut c_void) {
    let shim = &mut *(ptr as *mut PHYLayer);
    emane_c_phy_layer_implementor_start(shim.implementor);
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_phy_layer_post_start(ptr: *mut c_void) {
    let shim = &mut *(ptr as *mut PHYLayer);
    emane_c_phy_layer_implementor_post_start(shim.implementor);
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_phy_layer_stop(ptr: *mut c_void) {
    let shim = &mut *(ptr as *mut PHYLayer);
    emane_c_phy_layer_implementor_stop(shim.implementor);
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_phy_layer_destroy_impl(ptr: *mut c_void) {
    let shim = &mut *(ptr as *mut PHYLayer);
    emane_c_phy_layer_implementor_destroy(shim.implementor);
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_phy_layer_do_process_configuration(
    ptr: *mut c_void,
    update: *mut c_void,
) {
    let shim = &mut *(ptr as *mut PHYLayer);
    emane_c_phy_layer_implementor_process_configuration(shim.implementor, update);
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_phy_layer_do_process_upstream_packet(
    ptr: *mut c_void,
    pkt: *mut c_void,
    msgs: *mut c_void,
) {
    let shim = &mut *(ptr as *mut PHYLayer);
    emane_c_phy_layer_implementor_process_upstream_packet(shim.implementor, pkt, msgs);
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_phy_layer_do_process_downstream_packet(
    ptr: *mut c_void,
    pkt: *mut c_void,
    msgs: *mut c_void,
) {
    let shim = &mut *(ptr as *mut PHYLayer);
    emane_c_phy_layer_implementor_process_downstream_packet(shim.implementor, pkt, msgs);
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_phy_layer_do_process_upstream_control(
    ptr: *mut c_void,
    msgs: *mut c_void,
) {
    let shim = &mut *(ptr as *mut PHYLayer);
    emane_c_phy_layer_implementor_process_upstream_control(shim.implementor, msgs);
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_phy_layer_do_process_downstream_control(
    ptr: *mut c_void,
    msgs: *mut c_void,
) {
    let shim = &mut *(ptr as *mut PHYLayer);
    emane_c_phy_layer_implementor_process_downstream_control(shim.implementor, msgs);
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_phy_layer_set_upstream_transport(
    ptr: *mut c_void,
    transport: *mut c_void,
) {
    let shim = &mut *(ptr as *mut PHYLayer);
    emane_c_phy_layer_implementor_set_upstream_transport(shim.implementor, transport);
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_phy_layer_set_downstream_transport(
    ptr: *mut c_void,
    transport: *mut c_void,
) {
    let shim = &mut *(ptr as *mut PHYLayer);
    emane_c_phy_layer_implementor_set_downstream_transport(shim.implementor, transport);
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_phy_layer_do_process_event(
    ptr: *mut c_void,
    event_id: *mut c_void,
    serialization: *mut c_void,
) {
    let shim = &mut *(ptr as *mut PHYLayer);
    emane_c_phy_layer_implementor_process_event(shim.implementor, event_id, serialization);
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_phy_layer_do_process_timed_event(
    ptr: *mut c_void,
    event_id: *mut c_void,
    expire: *const c_void,
    schedule: *const c_void,
    fire: *const c_void,
    arg: *const c_void,
) {
    let shim = &mut *(ptr as *mut PHYLayer);
    emane_c_phy_layer_implementor_process_timed_event(
        shim.implementor,
        event_id,
        expire,
        schedule,
        fire,
        arg,
    );
}
