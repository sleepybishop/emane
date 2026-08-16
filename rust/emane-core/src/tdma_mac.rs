use std::time::Duration;

use crate::flow_control_manager::FlowControlManager;

pub struct TdmaMac {
    id: u16,
    flow_control_manager: FlowControlManager,
    flow_control_enable: bool,
    flow_control_tokens: u16,
}

impl TdmaMac {
    pub fn new(id: u16) -> Self {
        Self {
            id,
            flow_control_manager: FlowControlManager::new(),
            flow_control_enable: false,
            flow_control_tokens: 10,
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_mac_new(id: u16) -> *mut TdmaMac {
    Box::into_raw(Box::new(TdmaMac::new(id)))
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_mac_free(ptr: *mut TdmaMac) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr)) }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_mac_set_flow_control(
    ptr: *mut TdmaMac,
    enable: bool,
    tokens: u16,
) {
    let state = unsafe { &mut *ptr };
    state.flow_control_enable = enable;
    state.flow_control_tokens = tokens;
    if enable {
        state.flow_control_manager.start(tokens);
    } else {
        state.flow_control_manager.stop();
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_mac_remove_token(ptr: *mut TdmaMac) -> bool {
    let state = unsafe { &mut *ptr };
    if state.flow_control_enable {
        let (_, success) = state.flow_control_manager.remove_token();
        success
    } else {
        true
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_mac_add_token(ptr: *mut TdmaMac) -> bool {
    let state = unsafe { &mut *ptr };
    if state.flow_control_enable {
        let (_, success, _) = state.flow_control_manager.add_token(1);
        success
    } else {
        true
    }
}
