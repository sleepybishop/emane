use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use std::os::raw::c_char;
use crate::config::VoidPtr;

extern "C" {
    fn emane_c_event_service_user_process_event(p_user: *mut std::ffi::c_void, event_id: u16, data: *const c_char, len: usize);
    // Note: LogServiceSingleton::instance() logger might be accessed from C++ if we really need to log
    // but for now we just omit the debug log inside Rust, or use a C++ callback to log.
    // We will omit the DEBUG_LEVEL logs that were in the loops for simplicity.
    fn emane_c_log_debug(msg: *const c_char);
}

pub struct EventServiceUser {
    pub build_id: u16,
    pub nem_id: u16,
    pub p_user: VoidPtr,
}

pub struct EventServiceRegistry {
    pub users: HashMap<u16, EventServiceUser>,
    pub registrations: HashMap<u16, Vec<u16>>, // event_id -> build_ids
}

fn get_event_service_registry() -> &'static Mutex<EventServiceRegistry> {
    static REGISTRY: OnceLock<Mutex<EventServiceRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(EventServiceRegistry {
        users: HashMap::new(),
        registrations: HashMap::new(),
    }))
}

#[no_mangle]
pub extern "C" fn emane_rs_event_service_register_user(build_id: u16, nem_id: u16, p_user: *mut std::ffi::c_void) {
    let mut reg = get_event_service_registry().lock().unwrap();
    reg.users.insert(build_id, EventServiceUser {
        build_id,
        nem_id,
        p_user: VoidPtr(p_user),
    });
}

#[no_mangle]
pub extern "C" fn emane_rs_event_service_register_event(build_id: u16, event_id: u16) -> bool {
    let mut reg = get_event_service_registry().lock().unwrap();
    if reg.users.contains_key(&build_id) {
        reg.registrations.entry(event_id).or_insert_with(Vec::new).push(build_id);
        true
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_event_service_route_local_event(
    build_id: u16, nem_id: u16, event_id: u16, data: *const c_char, len: usize
) {
    let reg = get_event_service_registry().lock().unwrap();
    if let Some(build_ids) = reg.registrations.get(&event_id) {
        for &registered_build_id in build_ids {
            if let Some(user) = reg.users.get(&registered_build_id) {
                if build_id == 0 || registered_build_id != build_id {
                    if nem_id == 0 || user.nem_id == nem_id {
                        unsafe {
                            emane_c_event_service_user_process_event(user.p_user.0, event_id, data, len);
                        }
                    }
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_event_service_process_event_message(
    nem_id: u16, event_id: u16, data: *const c_char, len: usize, ignore_nem: u16
) {
    let reg = get_event_service_registry().lock().unwrap();
    if let Some(build_ids) = reg.registrations.get(&event_id) {
        for &registered_build_id in build_ids {
            if let Some(user) = reg.users.get(&registered_build_id) {
                if ignore_nem == 0 || ignore_nem != user.nem_id {
                    if nem_id == 0 || user.nem_id == nem_id {
                        unsafe {
                            emane_c_event_service_user_process_event(user.p_user.0, event_id, data, len);
                        }
                    }
                }
            }
        }
    }
}

use std::net::{UdpSocket, Ipv4Addr, SocketAddr};
use std::os::unix::io::AsRawFd;
use socket2::{Socket, Domain, Type, Protocol};

fn get_event_socket() -> &'static Mutex<Option<UdpSocket>> {
    static EVENT_SOCKET: OnceLock<Mutex<Option<UdpSocket>>> = OnceLock::new();
    EVENT_SOCKET.get_or_init(|| Mutex::new(None))
}

#[no_mangle]
pub extern "C" fn emane_rs_event_service_mcast_open(
    addr: *const c_char,
    device: *const c_char,
    ttl: i32,
    loopback: bool,
) -> bool {
    let addr_str = unsafe { std::ffi::CStr::from_ptr(addr) }.to_string_lossy();
    let sock_addr: SocketAddr = match addr_str.parse() {
        Ok(a) => a,
        Err(_) => return false,
    };
    
    let domain = if sock_addr.is_ipv4() { Domain::IPV4 } else { Domain::IPV6 };
    let socket = match Socket::new(domain, Type::DGRAM, Some(Protocol::UDP)) {
        Ok(s) => s,
        Err(_) => return false,
    };
    
    if let Err(_) = socket.set_reuse_address(true) { return false; }
    
    if sock_addr.is_ipv4() {
        if let Err(_) = socket.set_multicast_ttl_v4(ttl as u32) { return false; }
        if let Err(_) = socket.set_multicast_loop_v4(loopback) { return false; }
        
        let ip = match sock_addr.ip() {
            std::net::IpAddr::V4(ip) => ip,
            _ => return false,
        };
        if let Err(_) = socket.bind(&socket2::SockAddr::from(sock_addr)) { return false; }
        if let Err(_) = socket.join_multicast_v4(&ip, &Ipv4Addr::new(0, 0, 0, 0)) { return false; }
    } else {
        if let Err(_) = socket.set_multicast_loop_v6(loopback) { return false; }
        if let Err(_) = socket.bind(&socket2::SockAddr::from(sock_addr)) { return false; }
        let ip = match sock_addr.ip() {
            std::net::IpAddr::V6(ip) => ip,
            _ => return false,
        };
        if let Err(_) = socket.join_multicast_v6(&ip, 0) { return false; }
    }
    
    if !device.is_null() {
        let dev_str = unsafe { std::ffi::CStr::from_ptr(device) };
        let bytes = dev_str.to_bytes();
        if bytes.len() > 0 {
            let mut dev_name = [0u8; libc::IFNAMSIZ];
            let len = std::cmp::min(bytes.len(), libc::IFNAMSIZ - 1);
            dev_name[..len].copy_from_slice(&bytes[..len]);
            unsafe {
                if libc::setsockopt(
                    socket.as_raw_fd(),
                    libc::SOL_SOCKET,
                    libc::SO_BINDTODEVICE,
                    dev_name.as_ptr() as *const libc::c_void,
                    len as libc::socklen_t,
                ) < 0 {
                    return false;
                }
            }
        }
    }
    
    let udp_socket: UdpSocket = socket.into();
    *get_event_socket().lock().unwrap() = Some(udp_socket);
    
    true
}

use prost::Message;
use crate::protobufs::emane_message::Event;
use crate::protobufs::emane_message::event::{Data, data::Serialization};

extern "C" {
    fn emane_c_event_service_update_stat(type_: i32, uuid: *const u8, event_id: u16);
}

#[no_mangle]
pub extern "C" fn emane_rs_event_service_send_event_multicast(
    uuid: *const u8,
    event_id: u16,
    nem_id: u16,
    data: *const c_char,
    len: usize,
    seq_num: u64,
    addr: *const c_char,
) {
    let mut msg = Event::default();
    
    let mut serialization = Serialization::default();
    serialization.nem_id = nem_id as u32;
    serialization.event_id = event_id as u32;
    serialization.data = unsafe { std::slice::from_raw_parts(data as *const u8, len) }.to_vec();
    
    let mut data_msg = Data::default();
    data_msg.serializations.push(serialization);
    
    msg.data = data_msg;
    msg.uuid = unsafe { std::slice::from_raw_parts(uuid, 16) }.to_vec();
    msg.sequence_number = seq_num;
    
    let mut buf = Vec::new();
    if msg.encode(&mut buf).is_ok() {
        let msg_len = buf.len() as u16;
        let mut final_buf = msg_len.to_be_bytes().to_vec();
        final_buf.extend_from_slice(&buf);
        
        let addr_str = unsafe { std::ffi::CStr::from_ptr(addr) }.to_string_lossy();
        if let Ok(sock_addr) = addr_str.parse::<SocketAddr>() {
            if let Some(sock) = &*get_event_socket().lock().unwrap() {
                if sock.send_to(&final_buf, sock_addr).is_ok() {
                    unsafe {
                        emane_c_event_service_update_stat(0, uuid, event_id); // TYPE_TX
                    }
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_event_service_process_loop(local_uuid: *const u8) {
    let local_uuid_slice = unsafe { std::slice::from_raw_parts(local_uuid, 16) };
    
    let sock = if let Some(s) = &*get_event_socket().lock().unwrap() {
        match s.try_clone() {
            Ok(cloned) => cloned,
            Err(_) => return,
        }
    } else {
        return;
    };
    
    let mut recv_buf = [0u8; 65536];
    loop {
        match sock.recv(&mut recv_buf) {
            Ok(len) if len > 2 => {
                let packet_len = u16::from_be_bytes([recv_buf[0], recv_buf[1]]) as usize;
                if len - 2 == packet_len {
                    if let Ok(msg) = Event::decode(&recv_buf[2..len]) {
                        if msg.uuid != local_uuid_slice {
                            for serialization in msg.data.serializations {
                                let recv_nem_id = serialization.nem_id as u16;
                                let recv_event_id = serialization.event_id as u16;
                                
                                emane_rs_event_service_process_event_message(
                                    recv_nem_id,
                                    recv_event_id,
                                    serialization.data.as_ptr() as *const c_char,
                                    serialization.data.len(),
                                    0
                                );
                                
                                unsafe {
                                    emane_c_event_service_update_stat(1, msg.uuid.as_ptr(), recv_event_id); // TYPE_RX
                                }
                            }
                        }
                    }
                }
            }
            Ok(_) => continue,
            Err(_) => break, // socket closed or error
        }
    }
}

