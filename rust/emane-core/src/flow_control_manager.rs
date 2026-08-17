
#[derive(Default)]
pub struct FlowControlManager {
    tokens_available: u16,
    total_tokens_available: u16,
    shadow_token_count: u16,
    last_tokens_update: u16,
    ack_pending: bool,
}

impl FlowControlManager {
    pub fn new() -> Self {
        Self {
            ack_pending: true,
            ..Default::default()
        }
    }
    
    pub fn start(&mut self, total_tokens: u16) -> u16 {
        self.total_tokens_available = total_tokens;
        self.tokens_available = total_tokens;
        self.update_shadow_and_ack()
    }
    
    pub fn stop(&mut self) {
        self.total_tokens_available = 0;
        self.tokens_available = 0;
        self.shadow_token_count = 0;
    }
    
    pub fn add_token(&mut self, tokens: u16) -> (u16, bool, Option<u16>) {
        let status = (self.tokens_available as u32 + tokens as u32) <= self.total_tokens_available as u32;
        if status {
            self.tokens_available += tokens;
        }
        
        let mut send_update = None;
        if self.shadow_token_count == 0 && self.tokens_available > 0 {
            send_update = Some(self.update_shadow_and_ack());
        }
        
        (self.tokens_available, status, send_update)
    }
    
    pub fn remove_token(&mut self) -> (u16, bool) {
        if self.ack_pending {
            return (self.tokens_available, false);
        }
        
        if self.tokens_available == 0 {
            return (self.tokens_available, false);
        }
        
        self.tokens_available -= 1;
        self.shadow_token_count -= 1;
        (self.tokens_available, true)
    }
    
    pub fn process_flow_control_message(&mut self, tokens: u16) -> Option<u16> {
        if !self.ack_pending {
            Some(self.update_shadow_and_ack())
        } else {
            if tokens == self.last_tokens_update {
                self.ack_pending = false;
                None
            } else {
                Some(self.update_shadow_and_ack())
            }
        }
    }
    
    fn update_shadow_and_ack(&mut self) -> u16 {
        let tokens = self.tokens_available;
        self.shadow_token_count = tokens;
        self.last_tokens_update = tokens;
        self.ack_pending = true;
        tokens
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_flow_control_manager_create() -> *mut FlowControlManager {
    Box::into_raw(Box::new(FlowControlManager::new()))
}

#[no_mangle]
pub extern "C" fn emane_rs_flow_control_manager_destroy(ptr: *mut FlowControlManager) {
    if !ptr.is_null() {
        unsafe { let _ = Box::from_raw(ptr); }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_flow_control_manager_start(ptr: *mut FlowControlManager, total_tokens: u16) -> u16 {
    if let Some(m) = unsafe { ptr.as_mut() } {
        m.start(total_tokens)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_flow_control_manager_stop(ptr: *mut FlowControlManager) {
    if let Some(m) = unsafe { ptr.as_mut() } {
        m.stop();
    }
}

#[repr(C)]
pub struct FfiAddTokenResult {
    pub tokens_available: u16,
    pub status: bool,
    pub has_send_update: bool,
    pub send_update: u16,
}

#[no_mangle]
pub extern "C" fn emane_rs_flow_control_manager_add_token(ptr: *mut FlowControlManager, tokens: u16) -> FfiAddTokenResult {
    if let Some(m) = unsafe { ptr.as_mut() } {
        let (tokens_available, status, send_update) = m.add_token(tokens);
        FfiAddTokenResult {
            tokens_available,
            status,
            has_send_update: send_update.is_some(),
            send_update: send_update.unwrap_or(0),
        }
    } else {
        FfiAddTokenResult { tokens_available: 0, status: false, has_send_update: false, send_update: 0 }
    }
}

#[repr(C)]
pub struct FfiRemoveTokenResult {
    pub tokens_available: u16,
    pub status: bool,
}

#[no_mangle]
pub extern "C" fn emane_rs_flow_control_manager_remove_token(ptr: *mut FlowControlManager) -> FfiRemoveTokenResult {
    if let Some(m) = unsafe { ptr.as_mut() } {
        let (tokens_available, status) = m.remove_token();
        FfiRemoveTokenResult { tokens_available, status }
    } else {
        FfiRemoveTokenResult { tokens_available: 0, status: false }
    }
}

#[repr(C)]
pub struct FfiProcessMessageResult {
    pub has_send_update: bool,
    pub send_update: u16,
}

#[no_mangle]
pub extern "C" fn emane_rs_flow_control_manager_process_message(ptr: *mut FlowControlManager, tokens: u16) -> FfiProcessMessageResult {
    if let Some(m) = unsafe { ptr.as_mut() } {
        let res = m.process_flow_control_message(tokens);
        FfiProcessMessageResult {
            has_send_update: res.is_some(),
            send_update: res.unwrap_or(0),
        }
    } else {
        FfiProcessMessageResult { has_send_update: false, send_update: 0 }
    }
}
