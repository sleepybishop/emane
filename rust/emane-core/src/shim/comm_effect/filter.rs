use std::ffi::c_void;
use std::slice;

#[repr(C, packed)]
struct Ipv4Header {
    vhl: u8,
    tos: u8,
    len: u16,
    id: u16,
    frag: u16,
    ttl: u8,
    proto: u8,
    sum: u16,
    src: u32,
    dst: u32,
}

#[repr(C, packed)]
struct UdpHeader {
    src: u16,
    dst: u16,
    len: u16,
    sum: u16,
}

pub trait IpProtocolRule {
    fn match_rule(&self, buf: &[u8], u16_type: u16) -> bool;
}

pub struct IpProtocolSimpleRule {
    u8_type: u8,
}

impl IpProtocolRule for IpProtocolSimpleRule {
    fn match_rule(&self, _buf: &[u8], u16_type: u16) -> bool {
        u16_type == self.u8_type as u16
    }
}

pub struct IpProtocolUdpRule {
    src_port: u16,
    dst_port: u16,
    b_care: bool,
    u8_type: u8,
}

impl IpProtocolUdpRule {
    pub fn new(src_port: u16, dst_port: u16) -> Self {
        let b_care = src_port != 0 || dst_port != 0;
        Self {
            src_port,
            dst_port,
            b_care,
            u8_type: 17, // IP_PROTO_UDP
        }
    }
}

impl IpProtocolRule for IpProtocolUdpRule {
    fn match_rule(&self, buf: &[u8], u16_type: u16) -> bool {
        if u16_type != self.u8_type as u16 {
            return false;
        }
        if !self.b_care {
            return true;
        }
        if buf.len() < std::mem::size_of::<UdpHeader>() {
            return false;
        }

        let hdr = unsafe { &*(buf.as_ptr() as *const UdpHeader) };

        if self.src_port != 0 && u16::from_be(hdr.src) != self.src_port {
            return false;
        }
        if self.dst_port != 0 && u16::from_be(hdr.dst) != self.dst_port {
            return false;
        }

        true
    }
}

pub struct EthernetProtocolIpv4Rule {
    u16_type: u16,
    src: u32,
    dst: u32,
    len: u16,
    tos: u8,
    ttl: u8,
    b_care: bool,
    ip_rules: Vec<Box<dyn IpProtocolRule>>,
}

impl EthernetProtocolIpv4Rule {
    pub fn new(src: u32, dst: u32, len: u16, tos: u8, ttl: u8) -> Self {
        Self {
            u16_type: 0x0800u16.to_be(), // ETH_P_IPV4 is 0x0800 in network byte order
            src,
            dst,
            len,
            tos,
            ttl,
            b_care: false, // Updated when rules are added
            ip_rules: Vec::new(),
        }
    }

    pub fn add_ip_rule(&mut self, rule: Box<dyn IpProtocolRule>) {
        self.ip_rules.push(rule);
        self.b_care = !(self.src == 0 && self.dst == 0 && self.len == 0 && self.tos == 0 && self.ttl == 0 && self.ip_rules.is_empty());
    }

    pub fn match_rule(&self, buf: &[u8], u16_type: u16) -> bool {
        if u16_type != self.u16_type {
            return false;
        }
        if !self.b_care {
            return true;
        }
        if buf.len() < std::mem::size_of::<Ipv4Header>() {
            return false;
        }

        let hdr = unsafe { &*(buf.as_ptr() as *const Ipv4Header) };

        if self.src != 0 && hdr.src != self.src { return false; }
        if self.dst != 0 && hdr.dst != self.dst { return false; }
        if self.len != 0 && hdr.len != self.len { return false; }
        if self.tos != 0 && hdr.tos != self.tos { return false; }
        if self.ttl != 0 && hdr.ttl != self.ttl { return false; }

        if self.ip_rules.is_empty() {
            return true;
        }

        let offset = ((hdr.vhl & 0x0F) << 2) as usize;
        if buf.len() < offset {
            return false;
        }

        let payload = &buf[offset..];

        for rule in &self.ip_rules {
            if rule.match_rule(payload, hdr.proto as u16) {
                return true;
            }
        }
        false
    }
}

// C FFI
#[no_mangle]
pub extern "C" fn ip_protocol_simple_rule_new(u8_type: u8) -> *mut c_void {
    Box::into_raw(Box::new(IpProtocolSimpleRule { u8_type })) as *mut c_void
}

#[no_mangle]
pub extern "C" fn ip_protocol_simple_rule_drop(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr as *mut IpProtocolSimpleRule)); }
    }
}

#[no_mangle]
pub extern "C" fn ip_protocol_udp_rule_new(src_port: u16, dst_port: u16) -> *mut c_void {
    Box::into_raw(Box::new(IpProtocolUdpRule::new(src_port, dst_port))) as *mut c_void
}

#[no_mangle]
pub extern "C" fn ip_protocol_udp_rule_drop(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr as *mut IpProtocolUdpRule)); }
    }
}

#[no_mangle]
pub extern "C" fn ethernet_protocol_ipv4_rule_new(
    src: u32, dst: u32, len: u16, tos: u8, ttl: u8,
) -> *mut c_void {
    let mut rule = Box::new(EthernetProtocolIpv4Rule::new(src, dst, len, tos, ttl));
    // Initializing b_care
    rule.b_care = !(src == 0 && dst == 0 && len == 0 && tos == 0 && ttl == 0);
    Box::into_raw(rule) as *mut c_void
}

#[no_mangle]
pub extern "C" fn ethernet_protocol_ipv4_rule_add_ip_rule(
    eth_ptr: *mut c_void,
    ip_rule_ptr: *mut c_void,
    is_udp: bool,
) {
    if eth_ptr.is_null() || ip_rule_ptr.is_null() { return; }
    let eth_rule = unsafe { &mut *(eth_ptr as *mut EthernetProtocolIpv4Rule) };
    
    let rule: Box<dyn IpProtocolRule> = if is_udp {
        unsafe { Box::from_raw(ip_rule_ptr as *mut IpProtocolUdpRule) }
    } else {
        unsafe { Box::from_raw(ip_rule_ptr as *mut IpProtocolSimpleRule) }
    };
    eth_rule.add_ip_rule(rule);
}

#[no_mangle]
pub extern "C" fn ethernet_protocol_ipv4_rule_drop(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr as *mut EthernetProtocolIpv4Rule)); }
    }
}

#[no_mangle]
pub extern "C" fn ethernet_protocol_ipv4_rule_match(
    ptr: *const c_void,
    buf: *const u8,
    len: usize,
    u16_type: u16,
) -> bool {
    if ptr.is_null() || buf.is_null() { return false; }
    let rule = unsafe { &*(ptr as *const EthernetProtocolIpv4Rule) };
    let slice = unsafe { slice::from_raw_parts(buf, len) };
    rule.match_rule(slice, u16_type)
}
