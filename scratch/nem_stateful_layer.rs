
use std::os::raw::{c_void, c_char};
use std::ffi::CStr;

extern "C" {
    fn emane_c_nem_layer_initialize(layer: *mut c_void, registrar: *mut c_void);
    fn emane_c_nem_layer_configure(layer: *mut c_void, update: *mut c_void);
    fn emane_c_nem_layer_start(layer: *mut c_void);
    fn emane_c_nem_layer_post_start(layer: *mut c_void);
    fn emane_c_nem_layer_stop(layer: *mut c_void);
    fn emane_c_nem_layer_destroy(layer: *mut c_void);
    fn emane_c_nem_layer_process_configuration(layer: *mut c_void, update: *mut c_void);
    fn emane_c_nem_layer_process_downstream_control(layer: *mut c_void, msgs: *mut c_void);
    fn emane_c_nem_layer_process_downstream_packet(layer: *mut c_void, pkt: *mut c_void, msgs: *mut c_void);
    fn emane_c_nem_layer_process_upstream_packet(layer: *mut c_void, pkt: *mut c_void, msgs: *mut c_void);
    fn emane_c_nem_layer_process_upstream_control(layer: *mut c_void, msgs: *mut c_void);
    fn emane_c_nem_layer_process_event(layer: *mut c_void, id: *mut c_void, serialization: *mut c_void);
    fn emane_c_nem_layer_process_timed_event(layer: *mut c_void, timer_id: u32, expire: *mut c_void, schedule: *mut c_void, fire: *mut c_void, arg: *const c_void);
    fn emane_c_nem_layer_set_upstream_transport(layer: *mut c_void, transport: *mut c_void);
    fn emane_c_nem_layer_set_downstream_transport(layer: *mut c_void, transport: *mut c_void);
    
    fn emane_c_log_error(msg: *const c_char);
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NemLayerState {
    Uninitialized,
    Initialized,
    Configured,
    Running,
    Stopped,
    Destroyed,
}

impl NemLayerState {
    fn name(&self) -> &'static str {
        match self {
            NemLayerState::Uninitialized => "UNINITIALIZED",
            NemLayerState::Initialized => "INITIALIZED",
            NemLayerState::Configured => "CONFIGURED",
            NemLayerState::Running => "RUNNING",
            NemLayerState::Stopped => "STOPPED",
            NemLayerState::Destroyed => "DESTROYED",
        }
    }
}

pub struct NemStatefulLayer {
    pub inner_layer: *mut c_void,
    pub state: NemLayerState,
}

impl NemStatefulLayer {
    fn log_invalid_transition(&self, method: &str) {
        let msg = std::ffi::CString::new(format!("NEMLayer invalid {} transition in {} state", method, self.state.name())).unwrap();
        unsafe { emane_c_log_error(msg.as_ptr()); }
    }
    
    fn log_invalid_action(&self, method: &str) {
        let msg = std::ffi::CString::new(format!("NEMLayer {} not valid in {} state", method, self.state.name())).unwrap();
        unsafe { emane_c_log_error(msg.as_ptr()); }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_stateful_layer_new(inner_layer: *mut c_void) -> *mut c_void {
    let sl = Box::new(NemStatefulLayer {
        inner_layer,
        state: NemLayerState::Uninitialized,
    });
    Box::into_raw(sl) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_stateful_layer_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr as *mut NemStatefulLayer)); }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_stateful_layer_initialize(ptr: *mut c_void, registrar: *mut c_void) {
    let sl = unsafe { &mut *(ptr as *mut NemStatefulLayer) };
    if sl.state == NemLayerState::Uninitialized {
        unsafe { emane_c_nem_layer_initialize(sl.inner_layer, registrar); }
        sl.state = NemLayerState::Initialized;
    } else {
        sl.log_invalid_transition("initialize");
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_stateful_layer_configure(ptr: *mut c_void, update: *mut c_void) {
    let sl = unsafe { &mut *(ptr as *mut NemStatefulLayer) };
    if sl.state == NemLayerState::Initialized || sl.state == NemLayerState::Configured {
        unsafe { emane_c_nem_layer_configure(sl.inner_layer, update); }
        sl.state = NemLayerState::Configured;
    } else {
        sl.log_invalid_transition("configure");
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_stateful_layer_start(ptr: *mut c_void) {
    let sl = unsafe { &mut *(ptr as *mut NemStatefulLayer) };
    if sl.state == NemLayerState::Configured {
        unsafe { emane_c_nem_layer_start(sl.inner_layer); }
        sl.state = NemLayerState::Running;
    } else {
        sl.log_invalid_transition("start");
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_stateful_layer_post_start(ptr: *mut c_void) {
    let sl = unsafe { &mut *(ptr as *mut NemStatefulLayer) };
    if sl.state == NemLayerState::Running {
        unsafe { emane_c_nem_layer_post_start(sl.inner_layer); }
    } else {
        sl.log_invalid_transition("postStart");
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_stateful_layer_stop(ptr: *mut c_void) {
    let sl = unsafe { &mut *(ptr as *mut NemStatefulLayer) };
    if sl.state == NemLayerState::Running {
        unsafe { emane_c_nem_layer_stop(sl.inner_layer); }
        sl.state = NemLayerState::Stopped;
    } else {
        sl.log_invalid_transition("stop");
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_stateful_layer_handle_destroy(ptr: *mut c_void) {
    let sl = unsafe { &mut *(ptr as *mut NemStatefulLayer) };
    if sl.state == NemLayerState::Stopped || sl.state == NemLayerState::Configured || sl.state == NemLayerState::Initialized || sl.state == NemLayerState::Uninitialized {
        unsafe { emane_c_nem_layer_destroy(sl.inner_layer); }
        sl.state = NemLayerState::Destroyed;
    } else {
        sl.log_invalid_transition("destroy");
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_stateful_layer_process_configuration(ptr: *mut c_void, update: *mut c_void) {
    let sl = unsafe { &mut *(ptr as *mut NemStatefulLayer) };
    if sl.state == NemLayerState::Running {
        unsafe { emane_c_nem_layer_process_configuration(sl.inner_layer, update); }
    } else {
        sl.log_invalid_action("processConfigurationUpdate");
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_stateful_layer_process_downstream_control(ptr: *mut c_void, msgs: *mut c_void) {
    let sl = unsafe { &mut *(ptr as *mut NemStatefulLayer) };
    if sl.state == NemLayerState::Running {
        unsafe { emane_c_nem_layer_process_downstream_control(sl.inner_layer, msgs); }
    } else {
        sl.log_invalid_action("processDownstreamControl");
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_stateful_layer_process_downstream_packet(ptr: *mut c_void, pkt: *mut c_void, msgs: *mut c_void) {
    let sl = unsafe { &mut *(ptr as *mut NemStatefulLayer) };
    if sl.state == NemLayerState::Running {
        unsafe { emane_c_nem_layer_process_downstream_packet(sl.inner_layer, pkt, msgs); }
    } else {
        sl.log_invalid_action("processDownstreamPacket");
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_stateful_layer_process_upstream_packet(ptr: *mut c_void, pkt: *mut c_void, msgs: *mut c_void) {
    let sl = unsafe { &mut *(ptr as *mut NemStatefulLayer) };
    if sl.state == NemLayerState::Running {
        unsafe { emane_c_nem_layer_process_upstream_packet(sl.inner_layer, pkt, msgs); }
    } else {
        sl.log_invalid_action("processUpstreamPacket");
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_stateful_layer_process_upstream_control(ptr: *mut c_void, msgs: *mut c_void) {
    let sl = unsafe { &mut *(ptr as *mut NemStatefulLayer) };
    if sl.state == NemLayerState::Running {
        unsafe { emane_c_nem_layer_process_upstream_control(sl.inner_layer, msgs); }
    } else {
        sl.log_invalid_action("processUpstreamControl");
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_stateful_layer_process_event(ptr: *mut c_void, id: *mut c_void, serialization: *mut c_void) {
    let sl = unsafe { &mut *(ptr as *mut NemStatefulLayer) };
    if sl.state == NemLayerState::Running {
        unsafe { emane_c_nem_layer_process_event(sl.inner_layer, id, serialization); }
    } else {
        sl.log_invalid_action("processEvent");
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_stateful_layer_process_timed_event(ptr: *mut c_void, timer_id: u32, expire: *mut c_void, schedule: *mut c_void, fire: *mut c_void, arg: *const c_void) {
    let sl = unsafe { &mut *(ptr as *mut NemStatefulLayer) };
    if sl.state == NemLayerState::Running {
        unsafe { emane_c_nem_layer_process_timed_event(sl.inner_layer, timer_id, expire, schedule, fire, arg); }
    } else {
        sl.log_invalid_action("processTimedEvent");
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_stateful_layer_set_upstream_transport(ptr: *mut c_void, t: *mut c_void) {
    let sl = unsafe { &mut *(ptr as *mut NemStatefulLayer) };
    unsafe { emane_c_nem_layer_set_upstream_transport(sl.inner_layer, t); }
}

#[no_mangle]
pub extern "C" fn emane_rs_nem_stateful_layer_set_downstream_transport(ptr: *mut c_void, t: *mut c_void) {
    let sl = unsafe { &mut *(ptr as *mut NemStatefulLayer) };
    unsafe { emane_c_nem_layer_set_downstream_transport(sl.inner_layer, t); }
}
