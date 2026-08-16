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
            tokens_available: 0,
            total_tokens_available: 0,
            shadow_token_count: 0,
            last_tokens_update: 0,
            ack_pending: true,
        }
    }

    pub fn start(&mut self, total_tokens_available: u16) -> bool {
        self.total_tokens_available = total_tokens_available;
        self.tokens_available = total_tokens_available;
        self.send_flow_control_response_message()
    }

    pub fn stop(&mut self) {
        self.total_tokens_available = 0;
        self.tokens_available = 0;
        self.shadow_token_count = 0;
    }

    pub fn add_token(&mut self, tokens: u16) -> (u16, bool, bool) {
        let status = (self.tokens_available + tokens) <= self.total_tokens_available;
        if status {
            self.tokens_available += tokens;
        }

        let mut send_update = false;
        if self.shadow_token_count == 0 && self.tokens_available > 0 {
            send_update = self.send_flow_control_response_message();
        }

        (self.tokens_available, status, send_update)
    }

    pub fn remove_token(&mut self) -> (u16, bool) {
        let status = if self.ack_pending {
            false
        } else if self.tokens_available == 0 {
            false
        } else {
            self.tokens_available -= 1;
            self.shadow_token_count -= 1;
            true
        };
        (self.tokens_available, status)
    }

    pub fn process_flow_control_message(&mut self, msg_tokens: u16) -> bool {
        if !self.ack_pending {
            self.send_flow_control_response_message()
        } else if msg_tokens == self.last_tokens_update {
            self.ack_pending = false;
            false
        } else {
            self.send_flow_control_response_message()
        }
    }

    pub fn tokens_available(&self) -> u16 {
        self.tokens_available
    }

    fn send_flow_control_response_message(&mut self) -> bool {
        self.shadow_token_count = self.tokens_available;
        self.last_tokens_update = self.tokens_available;
        self.ack_pending = true;
        true
    }
}

