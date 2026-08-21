use std::collections::HashMap;
use std::ffi::c_void;
use std::os::raw::c_char;
use std::sync::{Mutex, OnceLock};

pub type EventCallback =
    extern "C" fn(context: *mut c_void, event_id: u16, data: *const u8, len: usize);

pub struct EventServiceUser {
    pub build_id: u16,
    pub nem_id: u16,
    context: usize,
    callback: Option<EventCallback>,
}

pub struct EventServiceRegistry {
    pub users: HashMap<u16, EventServiceUser>,
    pub registrations: HashMap<u16, Vec<u16>>, // event_id -> build_ids
}

fn get_event_service_registry() -> &'static Mutex<EventServiceRegistry> {
    static REGISTRY: OnceLock<Mutex<EventServiceRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        Mutex::new(EventServiceRegistry {
            users: HashMap::new(),
            registrations: HashMap::new(),
        })
    })
}

#[no_mangle]
pub extern "C" fn emane_rs_event_service_register_user(
    build_id: u16,
    nem_id: u16,
    p_user: *mut std::ffi::c_void,
) {
    let mut reg = get_event_service_registry().lock().unwrap();
    reg.users.insert(
        build_id,
        EventServiceUser {
            build_id,
            nem_id,
            context: p_user as usize,
            callback: None,
        },
    );
}

pub fn register_native_user(
    build_id: u16,
    nem_id: u16,
    context: *mut c_void,
    callback: EventCallback,
) {
    let mut registry = get_event_service_registry().lock().unwrap();
    registry.users.insert(
        build_id,
        EventServiceUser {
            build_id,
            nem_id,
            context: context as usize,
            callback: Some(callback),
        },
    );
}

pub fn unregister_user(build_id: u16) {
    let mut registry = get_event_service_registry().lock().unwrap();
    registry.users.remove(&build_id);
    for registrations in registry.registrations.values_mut() {
        registrations.retain(|registered| *registered != build_id);
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_event_service_register_event(build_id: u16, event_id: u16) -> bool {
    let mut reg = get_event_service_registry().lock().unwrap();
    if reg.users.contains_key(&build_id) {
        let build_ids = reg.registrations.entry(event_id).or_default();
        if !build_ids.contains(&build_id) {
            build_ids.push(build_id);
        }
        true
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_event_service_route_local_event(
    build_id: u16,
    nem_id: u16,
    event_id: u16,
    data: *const c_char,
    len: usize,
) {
    if len != 0 && data.is_null() {
        return;
    }
    let callbacks: Vec<(usize, EventCallback)> = {
        let reg = get_event_service_registry().lock().unwrap();
        reg.registrations
            .get(&event_id)
            .into_iter()
            .flatten()
            .filter_map(|registered_build_id| {
                reg.users.get(registered_build_id).and_then(|user| {
                    ((build_id == 0 || *registered_build_id != build_id)
                        && (nem_id == 0 || user.nem_id == nem_id))
                        .then(|| user.callback.map(|callback| (user.context, callback)))
                        .flatten()
                })
            })
            .collect()
    };
    for (context, callback) in callbacks {
        callback(context as *mut c_void, event_id, data.cast(), len);
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_event_service_process_event_message(
    nem_id: u16,
    event_id: u16,
    data: *const c_char,
    len: usize,
    ignore_nem: u16,
) {
    if len != 0 && data.is_null() {
        return;
    }
    let callbacks: Vec<(usize, EventCallback)> = {
        let reg = get_event_service_registry().lock().unwrap();
        reg.registrations
            .get(&event_id)
            .into_iter()
            .flatten()
            .filter_map(|registered_build_id| {
                reg.users.get(registered_build_id).and_then(|user| {
                    ((ignore_nem == 0 || ignore_nem != user.nem_id)
                        && (nem_id == 0 || user.nem_id == nem_id))
                        .then(|| user.callback.map(|callback| (user.context, callback)))
                        .flatten()
                })
            })
            .collect()
    };
    for (context, callback) in callbacks {
        callback(context as *mut c_void, event_id, data.cast(), len);
    }
}

pub fn route_serialized_event(nem_id: u16, event_id: u16, data: &[u8], ignore_nem: u16) {
    emane_rs_event_service_process_event_message(
        nem_id,
        event_id,
        data.as_ptr().cast(),
        data.len(),
        ignore_nem,
    );
}

use socket2::{Domain, Protocol, Socket, Type};
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::os::unix::io::AsRawFd;

fn get_event_socket() -> &'static Mutex<Option<UdpSocket>> {
    static EVENT_SOCKET: OnceLock<Mutex<Option<UdpSocket>>> = OnceLock::new();
    EVENT_SOCKET.get_or_init(|| Mutex::new(None))
}

pub struct EventServiceState {
    pub uuid: [u8; 16],
    pub seq_num: u64,
    pub mcast_addr: Option<String>,
    pub running: bool,
    pub transmitted: u64,
    pub received: u64,
}

fn get_event_service_state() -> &'static Mutex<EventServiceState> {
    static STATE: OnceLock<Mutex<EventServiceState>> = OnceLock::new();
    STATE.get_or_init(|| {
        Mutex::new(EventServiceState {
            uuid: [0; 16],
            seq_num: 0,
            mcast_addr: None,
            running: false,
            transmitted: 0,
            received: 0,
        })
    })
}

#[no_mangle]
pub extern "C" fn emane_rs_event_service_mcast_open(
    addr: *const c_char,
    device: *const c_char,
    ttl: i32,
    loopback: bool,
    uuid: *const u8,
) -> bool {
    if addr.is_null() || uuid.is_null() || !(0..=255).contains(&ttl) {
        return false;
    }
    if get_event_service_state().lock().unwrap().running {
        return false;
    }
    let addr_str = unsafe { std::ffi::CStr::from_ptr(addr) }.to_string_lossy();

    let sock_addr: SocketAddr = match addr_str.parse() {
        Ok(a) => a,
        Err(_) => return false,
    };

    let domain = if sock_addr.is_ipv4() {
        Domain::IPV4
    } else {
        Domain::IPV6
    };
    let socket = match Socket::new(domain, Type::DGRAM, Some(Protocol::UDP)) {
        Ok(s) => s,
        Err(_) => return false,
    };

    if socket.set_reuse_address(true).is_err() {
        return false;
    }

    if sock_addr.is_ipv4() {
        if socket.set_multicast_ttl_v4(ttl as u32).is_err() {
            return false;
        }
        if socket.set_multicast_loop_v4(loopback).is_err() {
            return false;
        }

        let ip = match sock_addr.ip() {
            std::net::IpAddr::V4(ip) => ip,
            _ => return false,
        };
        let bind_addr = SocketAddr::new(
            std::net::IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            sock_addr.port(),
        );
        if socket.bind(&socket2::SockAddr::from(bind_addr)).is_err() {
            return false;
        }
        if socket
            .join_multicast_v4(&ip, &Ipv4Addr::new(0, 0, 0, 0))
            .is_err()
        {
            return false;
        }
    } else {
        if socket.set_multicast_loop_v6(loopback).is_err() {
            return false;
        }
        let bind_addr = SocketAddr::new(
            std::net::IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED),
            sock_addr.port(),
        );
        if socket.bind(&socket2::SockAddr::from(bind_addr)).is_err() {
            return false;
        }
        let ip = match sock_addr.ip() {
            std::net::IpAddr::V6(ip) => ip,
            _ => return false,
        };
        if socket.join_multicast_v6(&ip, 0).is_err() {
            return false;
        }
    }

    if !device.is_null() {
        let dev_str = unsafe { std::ffi::CStr::from_ptr(device) };
        let bytes = dev_str.to_bytes();
        if !bytes.is_empty() {
            let mut dev_name = [0u8; libc::IFNAMSIZ];
            let len = std::cmp::min(bytes.len(), libc::IFNAMSIZ - 1);
            dev_name[..len].copy_from_slice(&bytes[..len]);
            unsafe {
                if libc::setsockopt(
                    socket.as_raw_fd(),
                    libc::SOL_SOCKET,
                    libc::SO_BINDTODEVICE,
                    dev_name.as_ptr() as *const libc::c_void,
                    (len + 1) as libc::socklen_t,
                ) < 0
                {
                    return false;
                }
            }
        }
    }

    let udp_socket: UdpSocket = socket.into();
    *get_event_socket().lock().unwrap() = Some(udp_socket);
    let mut state = get_event_service_state().lock().unwrap();
    state.mcast_addr = Some(addr_str.into_owned());
    unsafe {
        state
            .uuid
            .copy_from_slice(std::slice::from_raw_parts(uuid, 16));
    }
    state.running = true;

    true
}

#[no_mangle]
pub extern "C" fn emane_rs_event_service_mcast_close() {
    get_event_service_state().lock().unwrap().running = false;
    get_event_socket().lock().unwrap().take();
}

use crate::protobufs::emane_message::event::{data::Serialization, Data};
use crate::protobufs::emane_message::Event;
use prost::Message;

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
    if uuid.is_null() || addr.is_null() || (len != 0 && data.is_null()) || len > u32::MAX as usize {
        return;
    }
    let mut msg = Event::default();

    let serialization = Serialization {
        nem_id: nem_id as u32,
        event_id: event_id as u32,
        data: if len == 0 {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(data as *const u8, len) }.to_vec()
        },
    };

    let mut data_msg = Data::default();
    data_msg.serializations.push(serialization);

    msg.data = data_msg;
    msg.uuid = unsafe { std::slice::from_raw_parts(uuid, 16) }.to_vec();
    msg.sequence_number = seq_num;

    let mut buf = Vec::new();
    if msg.encode(&mut buf).is_ok() && buf.len() <= u16::MAX as usize {
        let msg_len = buf.len() as u16;
        let mut final_buf = msg_len.to_be_bytes().to_vec();
        final_buf.extend_from_slice(&buf);

        let addr_str = unsafe { std::ffi::CStr::from_ptr(addr) }.to_string_lossy();
        if let Ok(sock_addr) = addr_str.parse::<SocketAddr>() {
            let socket = get_event_socket()
                .lock()
                .unwrap()
                .as_ref()
                .and_then(|socket| socket.try_clone().ok());
            if let Some(sock) = socket {
                if sock.send_to(&final_buf, sock_addr).is_ok() {
                    let mut state = get_event_service_state().lock().unwrap();
                    state.transmitted = state.transmitted.saturating_add(1);
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_event_service_process_loop(local_uuid: *const u8) {
    if local_uuid.is_null() {
        return;
    }
    let local_uuid_slice = unsafe { std::slice::from_raw_parts(local_uuid, 16) };

    let sock = if let Some(s) = &*get_event_socket().lock().unwrap() {
        match s.try_clone() {
            Ok(cloned) => cloned,
            Err(_) => return,
        }
    } else {
        return;
    };
    if sock
        .set_read_timeout(Some(std::time::Duration::from_millis(200)))
        .is_err()
    {
        return;
    }

    let mut recv_buf = [0u8; 65536];
    loop {
        match sock.recv(&mut recv_buf) {
            Ok(len) if len > 2 => {
                let packet_len = u16::from_be_bytes([recv_buf[0], recv_buf[1]]) as usize;
                if len - 2 == packet_len {
                    if let Ok(msg) = Event::decode(&recv_buf[2..len]) {
                        if msg.uuid.len() == 16 && msg.uuid != local_uuid_slice {
                            for serialization in msg.data.serializations {
                                let Ok(recv_nem_id) = u16::try_from(serialization.nem_id) else {
                                    continue;
                                };
                                let Ok(recv_event_id) = u16::try_from(serialization.event_id)
                                else {
                                    continue;
                                };

                                emane_rs_event_service_process_event_message(
                                    recv_nem_id,
                                    recv_event_id,
                                    serialization.data.as_ptr() as *const c_char,
                                    serialization.data.len(),
                                    0,
                                );

                                let mut state = get_event_service_state().lock().unwrap();
                                state.received = state.received.saturating_add(1);
                            }
                        }
                    }
                }
            }
            Ok(_) => continue,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                if !get_event_service_state().lock().unwrap().running {
                    break;
                }
            }
            Err(_) => break,
        }
        if !get_event_service_state().lock().unwrap().running {
            break;
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_event_service_send_event(
    build_id: u16,
    nem_id: u16,
    event_id: u16,
    data: *const std::os::raw::c_char,
    len: usize,
) {
    if len != 0 && data.is_null() {
        return;
    }
    emane_rs_event_service_route_local_event(build_id, nem_id, event_id, data, len);

    let mut state = get_event_service_state().lock().unwrap();
    if let Some(addr) = state.mcast_addr.clone() {
        state.seq_num = state.seq_num.wrapping_add(1);
        let seq_num = state.seq_num;
        let c_addr = std::ffi::CString::new(addr.clone()).unwrap();
        let uuid = state.uuid; // copy
        drop(state); // drop lock before calling multicast function

        emane_rs_event_service_send_event_multicast(
            uuid.as_ptr(),
            event_id,
            nem_id,
            data,
            len,
            seq_num,
            c_addr.as_ptr(),
        );
    }
}
