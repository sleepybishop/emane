use std::os::raw::c_void;

extern "C" {
    fn emane_c_transport_set_downstream(transport: *mut c_void, downstream: *mut c_void);
    fn emane_c_transport_set_upstream(transport: *mut c_void, upstream: *mut c_void);
    fn emane_c_component_start(comp: *mut c_void);
    fn emane_c_component_post_start(comp: *mut c_void);
    fn emane_c_component_stop(comp: *mut c_void);
    fn emane_c_component_destroy(comp: *mut c_void);
}

pub struct NemLayerStack {
    layers: Vec<*mut c_void>,
}

impl NemLayerStack {
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    pub fn add_layer(&mut self, layer: *mut c_void) {
        self.layers.push(layer);
    }

    pub fn connect_layers(&self, up: *mut c_void, down: *mut c_void) {
        let mut curr_up = up;
        for layer in &self.layers {
            unsafe {
                emane_c_transport_set_downstream(curr_up, *layer);
                emane_c_transport_set_upstream(*layer, curr_up);
            }
            curr_up = *layer;
        }
        unsafe {
            emane_c_transport_set_downstream(curr_up, down);
            emane_c_transport_set_upstream(down, curr_up);
        }
    }

    pub fn start(&self) {
        for layer in &self.layers {
            unsafe {
                emane_c_component_start(*layer);
            }
        }
    }

    pub fn post_start(&self) {
        for layer in &self.layers {
            unsafe {
                emane_c_component_post_start(*layer);
            }
        }
    }

    pub fn stop(&self) {
        for layer in &self.layers {
            unsafe {
                emane_c_component_stop(*layer);
            }
        }
    }

    pub fn destroy(&self) {
        for layer in &self.layers {
            unsafe {
                emane_c_component_destroy(*layer);
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_layer_stack_create() -> *mut c_void {
    let stack = Box::new(NemLayerStack::new());
    Box::into_raw(stack) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_layer_stack_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr as *mut NemLayerStack);
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_layer_stack_add(ptr: *mut c_void, layer: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let stack = unsafe { &mut *(ptr as *mut NemLayerStack) };
    stack.add_layer(layer);
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_layer_stack_connect(
    ptr: *mut c_void,
    up: *mut c_void,
    down: *mut c_void,
) {
    if ptr.is_null() {
        return;
    }
    let stack = unsafe { &mut *(ptr as *mut NemLayerStack) };
    stack.connect_layers(up, down);
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_layer_stack_start(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let stack = unsafe { &mut *(ptr as *mut NemLayerStack) };
    stack.start();
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_layer_stack_post_start(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let stack = unsafe { &mut *(ptr as *mut NemLayerStack) };
    stack.post_start();
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_layer_stack_stop(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let stack = unsafe { &mut *(ptr as *mut NemLayerStack) };
    stack.stop();
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_layer_stack_destroy_layers(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let stack = unsafe { &mut *(ptr as *mut NemLayerStack) };
    stack.destroy();
}
