use std::os::raw::c_void;
use crate::nem_layer_stack::NemLayerStack;

extern "C" {
    fn emane_c_nem_adapter_open_ota(adapter: *mut c_void);
    fn emane_c_nem_adapter_close_ota(adapter: *mut c_void);
    fn emane_c_nem_adapter_open_net(adapter: *mut c_void);
    fn emane_c_nem_adapter_close_net(adapter: *mut c_void);
}

pub struct NemImpl {
    id: u16,
    stack: *mut c_void, // Pointer to NemLayerStack
    b_external_transport: bool,
    ota_adapter: *mut c_void,
    net_adapter: *mut c_void,
}

impl NemImpl {
    pub fn new(id: u16, stack: *mut c_void, b_ext: bool, ota: *mut c_void, net: *mut c_void) -> Self {
        Self {
            id,
            stack,
            b_external_transport: b_ext,
            ota_adapter: ota,
            net_adapter: net,
        }
    }

    pub fn start(&self) {
        unsafe {
            if !self.ota_adapter.is_null() {
                emane_c_nem_adapter_open_ota(self.ota_adapter);
            }
            if self.b_external_transport && !self.net_adapter.is_null() {
                emane_c_nem_adapter_open_net(self.net_adapter);
            }
            if !self.stack.is_null() {
                crate::nem_layer_stack::emane_rs_nem_layer_stack_start(self.stack);
            }
        }
    }

    pub fn post_start(&self) {
        unsafe {
            if !self.stack.is_null() {
                crate::nem_layer_stack::emane_rs_nem_layer_stack_post_start(self.stack);
            }
        }
    }

    pub fn stop(&self) {
        unsafe {
            if self.b_external_transport && !self.net_adapter.is_null() {
                emane_c_nem_adapter_close_net(self.net_adapter);
            }
            if !self.ota_adapter.is_null() {
                emane_c_nem_adapter_close_ota(self.ota_adapter);
            }
            if !self.stack.is_null() {
                crate::nem_layer_stack::emane_rs_nem_layer_stack_stop(self.stack);
            }
        }
    }

    pub fn destroy(&self) {
        unsafe {
            if !self.stack.is_null() {
                crate::nem_layer_stack::emane_rs_nem_layer_stack_destroy_layers(self.stack);
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_impl_create(
    id: u16,
    stack: *mut c_void,
    b_ext: bool,
    ota: *mut c_void,
    net: *mut c_void
) -> *mut c_void {
    let nem = Box::new(NemImpl::new(id, stack, b_ext, ota, net));
    Box::into_raw(nem) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_impl_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe { let _ = Box::from_raw(ptr as *mut NemImpl); }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_impl_start(ptr: *mut c_void) {
    if ptr.is_null() { return; }
    let nem = unsafe { &*(ptr as *mut NemImpl) };
    nem.start();
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_impl_post_start(ptr: *mut c_void) {
    if ptr.is_null() { return; }
    let nem = unsafe { &*(ptr as *mut NemImpl) };
    nem.post_start();
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_impl_stop(ptr: *mut c_void) {
    if ptr.is_null() { return; }
    let nem = unsafe { &*(ptr as *mut NemImpl) };
    nem.stop();
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_impl_destroy_layers(ptr: *mut c_void) {
    if ptr.is_null() { return; }
    let nem = unsafe { &*(ptr as *mut NemImpl) };
    nem.destroy();
}
