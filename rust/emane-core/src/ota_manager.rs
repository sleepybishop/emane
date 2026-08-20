use libc::{c_uint, setsockopt, SOL_SOCKET, SO_BINDTODEVICE};
use prost::Message;
use socket2::{Domain, Protocol, Socket, Type};
use std::collections::{BTreeMap, HashMap};
use std::ffi::CStr;
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::os::fd::AsRawFd;
use std::os::raw::{c_char, c_void};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime};

pub use crate::protobufs::emane_message as ota;

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
struct PartInfo {
    u8_more: u8,
    u32_offset: u32,
    u32_size: u32,
}

#[derive(Hash, Eq, PartialEq, Clone, Copy)]
struct PartKey {
    source: u16,
    sequence: u64,
}

struct PartsData {
    total_size: usize,
    events_size: usize,
    controls_size: usize,
    data_size: usize,
    parts: BTreeMap<usize, Vec<u8>>,
    last_part_time: SystemTime,
    uuid: [u8; 16],
}

pub struct OtaManager {
    socket: Option<UdpSocket>,
    uuid: [u8; 16],
    ota_mtu: usize,
    part_check_threshold: Duration,
    part_timeout_threshold: Duration,
    u64_sequence_number: u64,
    part_store: HashMap<PartKey, PartsData>,
    last_part_check_time: SystemTime,
    pub nem_users: HashMap<u16, usize>,
    group_addr: Option<std::net::SocketAddr>,
}

impl OtaManager {
    pub fn new() -> Self {
        Self {
            socket: None,
            uuid: [0; 16],
            ota_mtu: 0,
            part_check_threshold: Duration::from_secs(2),
            part_timeout_threshold: Duration::from_secs(5),
            u64_sequence_number: 0,
            part_store: HashMap::new(),
            last_part_check_time: SystemTime::now(),
            nem_users: HashMap::new(),
            group_addr: None,
        }
    }
}

static OTA_MANAGER: OnceLock<Mutex<OtaManager>> = OnceLock::new();

pub fn get_ota_manager() -> &'static Mutex<OtaManager> {
    OTA_MANAGER.get_or_init(|| Mutex::new(OtaManager::new()))
}

extern "C" {
    fn emane_c_ota_manager_update_stat(uuid_ptr: *const u8, src_nem: u16, stat_type: u32);

    fn emane_c_ota_manager_init_publishers();

    fn emane_c_ota_manager_deliver_event(
        src_nem: u16,
        event_id: u16,
        data: *const u8,
        data_len: usize,
    );

    fn emane_c_ota_user_process_packet(
        p_user: *mut c_void,
        source: u16,
        destination: u16,
        priority: u8,
        uuid_ptr: *const u8,
        data: *const u8,
        data_len: usize,
        controls: *const u8,
        controls_len: usize,
    );
}

#[no_mangle]
pub extern "C" fn emane_rs_ota_manager_register_user(id: u16, p_user: *mut c_void) {
    let mut manager = get_ota_manager().lock().unwrap();
    manager.nem_users.insert(id, p_user as usize);
}

#[no_mangle]
pub extern "C" fn emane_rs_ota_manager_unregister_user(id: u16) {
    let mut manager = get_ota_manager().lock().unwrap();
    manager.nem_users.remove(&id);
}

#[no_mangle]
pub extern "C" fn emane_rs_ota_manager_open(
    group_addr_str: *const c_char,
    device_str: *const c_char,
    ttl: u8,
    loopback: bool,
    uuid: *const u8,
    ota_mtu: usize,
    part_check_threshold_secs: u16,
    part_timeout_threshold_secs: u16,
) -> bool {
    let group_addr_c = unsafe { CStr::from_ptr(group_addr_str) }
        .to_string_lossy()
        .into_owned();
    let device_c = unsafe { CStr::from_ptr(device_str) }
        .to_string_lossy()
        .into_owned();

    let mut uuid_arr = [0u8; 16];
    unsafe {
        uuid_arr.copy_from_slice(std::slice::from_raw_parts(uuid, 16));
    }

    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).unwrap();
    socket.set_reuse_address(true).unwrap();

    let (ip, port) = {
        let parts: Vec<&str> = group_addr_c.split(':').collect();
        (
            parts[0].parse::<Ipv4Addr>().unwrap(),
            parts[1].parse::<u16>().unwrap(),
        )
    };

    let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);
    socket.bind(&addr.into()).unwrap();

    if ip.is_multicast() {
        socket
            .join_multicast_v4(&ip, &Ipv4Addr::UNSPECIFIED)
            .unwrap();
        socket.set_multicast_loop_v4(loopback).unwrap();
        socket.set_multicast_ttl_v4(ttl as u32).unwrap();
    }

    if !device_c.is_empty() {
        let mut dev_bytes = device_c.into_bytes();
        dev_bytes.push(0);
        unsafe {
            setsockopt(
                socket.as_raw_fd(),
                SOL_SOCKET,
                SO_BINDTODEVICE,
                dev_bytes.as_ptr() as *const c_void,
                dev_bytes.len() as c_uint,
            );
        }
    }

    let mut manager = get_ota_manager().lock().unwrap();
    manager.socket = Some(socket.into());
    manager.uuid = uuid_arr;
    manager.ota_mtu = ota_mtu;
    manager.part_check_threshold = Duration::from_secs(part_check_threshold_secs as u64);
    manager.part_timeout_threshold = Duration::from_secs(part_timeout_threshold_secs as u64);
    manager.group_addr = Some(std::net::SocketAddr::V4(SocketAddrV4::new(ip, port)));

    unsafe {
        emane_c_ota_manager_init_publishers();
    }

    true
}

#[no_mangle]
pub extern "C" fn emane_rs_ota_manager_process_loop() {
    let socket = {
        let manager = get_ota_manager().lock().unwrap();
        manager.socket.as_ref().unwrap().try_clone().unwrap()
    };

    let mut buf = vec![0u8; 65536];

    loop {
        match socket.recv_from(&mut buf) {
            Ok((len, _)) => {
                let now = SystemTime::now();
                if len >= 2 {
                    let header_len = u16::from_be_bytes([buf[0], buf[1]]) as usize;

                    if len >= 2 + header_len + std::mem::size_of::<PartInfo>() {
                        if let Ok(ota_header) = ota::OtaHeader::decode(&buf[2..2 + header_len]) {
                            let part_info_offset = 2 + header_len;
                            let part_info = unsafe {
                                std::ptr::read_unaligned(
                                    buf.as_ptr().add(part_info_offset) as *const PartInfo
                                )
                            };

                            let u32_offset = u32::from_be(part_info.u32_offset) as usize;
                            let u32_size = u32::from_be(part_info.u32_size) as usize;
                            let u8_more = part_info.u8_more;

                            let mut remote_uuid = [0u8; 16];
                            let uuid_data = &ota_header.uuid;
                            if uuid_data.len() == 16 {
                                remote_uuid.copy_from_slice(uuid_data);
                            }

                            let local_uuid = get_ota_manager().lock().unwrap().uuid;
                            if remote_uuid != local_uuid {
                                let payload_index =
                                    part_info_offset + std::mem::size_of::<PartInfo>();

                                if len == payload_index + u32_size {
                                    if u8_more == 0 && u32_offset == 0 {
                                        // Single part message
                                        let payload_info =
                                            ota_header.payload_info.unwrap_or_default();
                                        handle_ota_message(
                                            ota_header.source as u16,
                                            ota_header.destination as u16,
                                            &remote_uuid,
                                            payload_info.event_length as usize,
                                            payload_info.control_length as usize,
                                            payload_info.data_length as usize,
                                            &buf[payload_index..payload_index + u32_size],
                                        );
                                    } else {
                                        // Fragmented message
                                        let mut manager = get_ota_manager().lock().unwrap();
                                        let key = PartKey {
                                            source: ota_header.source as u16,
                                            sequence: ota_header.sequence,
                                        };

                                        let payload_info =
                                            ota_header.payload_info.unwrap_or_default();
                                        let event_len = payload_info.event_length as usize;
                                        let control_len = payload_info.control_length as usize;
                                        let data_len = payload_info.data_length as usize;

                                        let entry =
                                            manager.part_store.entry(key).or_insert_with(|| {
                                                PartsData {
                                                    total_size: 0,
                                                    events_size: event_len,
                                                    controls_size: control_len,
                                                    data_size: data_len,
                                                    parts: BTreeMap::new(),
                                                    last_part_time: now,
                                                    uuid: remote_uuid,
                                                }
                                            });

                                        // Update info if it was 0 (from subsequent parts that lack PayloadInfo)
                                        if entry.events_size == 0
                                            && entry.controls_size == 0
                                            && entry.data_size == 0
                                        {
                                            entry.events_size = event_len;
                                            entry.controls_size = control_len;
                                            entry.data_size = data_len;
                                        }

                                        entry.total_size += u32_size;
                                        entry.parts.insert(
                                            u32_offset,
                                            buf[payload_index..payload_index + u32_size].to_vec(),
                                        );
                                        entry.last_part_time = now;

                                        let expected_total = entry.events_size
                                            + entry.controls_size
                                            + entry.data_size;
                                        if entry.total_size == expected_total {
                                            // All parts received!
                                            let mut full_payload =
                                                Vec::with_capacity(expected_total);
                                            for (_, part) in &entry.parts {
                                                full_payload.extend_from_slice(part);
                                            }

                                            let e_size = entry.events_size;
                                            let c_size = entry.controls_size;
                                            let d_size = entry.data_size;
                                            let remote_uuid = entry.uuid;

                                            manager.part_store.remove(&key);
                                            drop(manager);

                                            handle_ota_message(
                                                ota_header.source as u16,
                                                ota_header.destination as u16,
                                                &remote_uuid,
                                                e_size,
                                                c_size,
                                                d_size,
                                                &full_payload,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Cleanup old parts
                let mut manager = get_ota_manager().lock().unwrap();
                if now
                    .duration_since(manager.last_part_check_time)
                    .unwrap_or(Duration::ZERO)
                    >= manager.part_check_threshold
                {
                    let threshold = manager.part_timeout_threshold;
                    manager.part_store.retain(|key, value| {
                        if now
                            .duration_since(value.last_part_time)
                            .unwrap_or(Duration::ZERO)
                            >= threshold
                        {
                            // Update statistic upstream drop missing parts (Type = 3)
                            unsafe {
                                emane_c_ota_manager_update_stat(value.uuid.as_ptr(), key.source, 3);
                            }
                            false
                        } else {
                            true
                        }
                    });
                    manager.last_part_check_time = now;
                }
            }
            Err(_) => {
                break;
            }
        }
    }
}

fn handle_ota_message(
    source: u16,
    destination: u16,
    remote_uuid: &[u8; 16],
    events_size: usize,
    controls_size: usize,
    data_size: usize,
    payload: &[u8],
) {
    let mut offset = 0;

    if events_size > 0 {
        if let Ok(data) = ota::event::Data::decode(&payload[offset..offset + events_size]) {
            for serialization in data.serializations {
                unsafe {
                    emane_c_ota_manager_deliver_event(
                        serialization.nem_id as u16,
                        serialization.event_id as u16,
                        serialization.data.as_ptr(),
                        serialization.data.len(),
                    );
                }
            }
        }
        offset += events_size;
    }

    let controls_ptr = if controls_size > 0 {
        payload[offset..offset + controls_size].as_ptr()
    } else {
        std::ptr::null()
    };

    offset += controls_size;

    let data_ptr = if data_size > 0 {
        payload[offset..offset + data_size].as_ptr()
    } else {
        std::ptr::null()
    };

    if data_size > 0 {
        let users: Vec<usize> = {
            let manager = get_ota_manager().lock().unwrap();
            manager.nem_users.values().copied().collect()
        };

        unsafe {
            // Update TYPE_UPSTREAM_PACKET_SUCCESS = 2
            emane_c_ota_manager_update_stat(remote_uuid.as_ptr(), source, 2);
        }

        for p_user in users {
            unsafe {
                emane_c_ota_user_process_packet(
                    p_user as *mut c_void,
                    source,
                    destination,
                    0, // Priority is set to 0 as in original
                    remote_uuid.as_ptr(),
                    data_ptr,
                    data_size,
                    controls_ptr,
                    controls_size,
                );
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_ota_manager_send_ota_packet(
    source: u16,
    destination: u16,
    packet_data: *const u8,
    packet_len: usize,
    controls_data: *const u8,
    controls_len: usize,
    events_data: *const u8,
    events_len: usize,
) -> bool {
    let mut manager = get_ota_manager().lock().unwrap();
    let uuid = manager.uuid;
    let ota_mtu = manager.ota_mtu;

    if manager.socket.is_none() {
        return false;
    }

    manager.u64_sequence_number += 1;
    let seq = manager.u64_sequence_number;

    let socket = manager.socket.as_ref().unwrap().try_clone().unwrap();
    let group_addr = manager.group_addr.unwrap();
    drop(manager);

    let mut ota_header = ota::OtaHeader::default();
    ota_header.source = source as u32;
    ota_header.destination = destination as u32;
    ota_header.sequence = seq;
    ota_header.uuid = uuid.to_vec();

    let mut payload_info = ota::ota_header::PayloadInfo::default();
    payload_info.data_length = packet_len as u32;
    payload_info.control_length = controls_len as u32;
    payload_info.event_length = events_len as u32;
    ota_header.payload_info = Some(payload_info);

    let ota_header_bytes = ota_header.encode_to_vec();
    let header_len = ota_header_bytes.len();
    let u16_header_length = (header_len as u16).to_be_bytes();

    let total_payload_size = events_len + controls_len + packet_len;
    let mut sent_bytes = 0;

    let events_slice = if events_data.is_null() {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(events_data, events_len) }
    };
    let controls_slice = if controls_data.is_null() {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(controls_data, controls_len) }
    };
    let packet_slice = if packet_data.is_null() {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(packet_data, packet_len) }
    };

    while sent_bytes < total_payload_size {
        let total_wire_size =
            total_payload_size - sent_bytes + header_len + 2 + std::mem::size_of::<PartInfo>();

        let mut part_info = PartInfo {
            u8_more: 0,
            u32_offset: (sent_bytes as u32).to_be(),
            u32_size: 0,
        };

        let payload_size = if ota_mtu != 0 && total_wire_size > ota_mtu {
            part_info.u8_more = 1;
            ota_mtu - (header_len + 2 + std::mem::size_of::<PartInfo>())
        } else {
            total_payload_size - sent_bytes
        };

        part_info.u32_size = (payload_size as u32).to_be();

        let mut message =
            Vec::with_capacity(2 + header_len + std::mem::size_of::<PartInfo>() + payload_size);
        message.extend_from_slice(&u16_header_length);
        message.extend_from_slice(&ota_header_bytes);

        let part_info_bytes = unsafe {
            std::slice::from_raw_parts(
                &part_info as *const PartInfo as *const u8,
                std::mem::size_of::<PartInfo>(),
            )
        };
        message.extend_from_slice(part_info_bytes);

        // Append payload parts based on sent_bytes and payload_size
        let mut p_size = payload_size;
        let mut p_offset = sent_bytes;

        if p_offset < events_len && p_size > 0 {
            let take = p_size.min(events_len - p_offset);
            message.extend_from_slice(&events_slice[p_offset..p_offset + take]);
            p_size -= take;
            p_offset += take;
        }

        let c_start = events_len;
        if p_offset >= c_start && p_offset < c_start + controls_len && p_size > 0 {
            let offset_in_controls = p_offset - c_start;
            let take = p_size.min(controls_len - offset_in_controls);
            message
                .extend_from_slice(&controls_slice[offset_in_controls..offset_in_controls + take]);
            p_size -= take;
            p_offset += take;
        }

        let d_start = c_start + controls_len;
        if p_offset >= d_start && p_offset < d_start + packet_len && p_size > 0 {
            let offset_in_packet = p_offset - d_start;
            let take = p_size.min(packet_len - offset_in_packet);
            message.extend_from_slice(&packet_slice[offset_in_packet..offset_in_packet + take]);
            p_size -= take;
            //p_offset += take;
        }

        if socket.send_to(&message, group_addr).is_err() {
            return false;
        }

        sent_bytes += payload_size;

        // After the first fragment, we don't need PayloadInfo
        ota_header.payload_info = None;
    }

    true
}
#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message;
    use std::ffi::CString;
    use std::net::UdpSocket;
    use std::os::raw::c_void;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    static STATS_UPDATES: AtomicUsize = AtomicUsize::new(0);
    static PUBLISHERS_INIT: AtomicUsize = AtomicUsize::new(0);
    static EVENTS_DELIVERED: AtomicUsize = AtomicUsize::new(0);
    static PACKETS_PROCESSED: AtomicUsize = AtomicUsize::new(0);

    #[no_mangle]
    pub extern "C" fn emane_c_ota_manager_update_stat(_uuid: *const u8, _src: u16, _stat: u32) {
        STATS_UPDATES.fetch_add(1, Ordering::SeqCst);
    }
    #[no_mangle]
    pub extern "C" fn emane_c_ota_manager_init_publishers() {
        PUBLISHERS_INIT.fetch_add(1, Ordering::SeqCst);
    }
    #[no_mangle]
    pub extern "C" fn emane_c_ota_manager_deliver_event(
        _src: u16,
        _evt: u16,
        _data: *const u8,
        _len: usize,
    ) {
        EVENTS_DELIVERED.fetch_add(1, Ordering::SeqCst);
    }
    #[no_mangle]
    pub extern "C" fn emane_c_ota_user_process_packet(
        _p: *mut c_void,
        _src: u16,
        _dst: u16,
        _pri: u8,
        _uuid: *const u8,
        _data: *const u8,
        _len: usize,
        _ctrl: *const u8,
        _clen: usize,
    ) {
        PACKETS_PROCESSED.fetch_add(1, Ordering::SeqCst);
    }

    fn reset_mocks() {
        STATS_UPDATES.store(0, Ordering::SeqCst);
        PUBLISHERS_INIT.store(0, Ordering::SeqCst);
        EVENTS_DELIVERED.store(0, Ordering::SeqCst);
        PACKETS_PROCESSED.store(0, Ordering::SeqCst);
    }

    #[test]
    fn test_ota_nem_states() {
        let manager = get_ota_manager();
        let id = 123;
        let p_user = 456 as *mut c_void;
        emane_rs_ota_manager_register_user(id, p_user);
        assert_eq!(
            manager.lock().unwrap().nem_users.get(&id),
            Some(&(p_user as usize))
        );
        emane_rs_ota_manager_unregister_user(id);
        assert!(manager.lock().unwrap().nem_users.get(&id).is_none());
    }

    #[test]
    fn test_handle_ota_message() {
        reset_mocks();
        let uuid = [0u8; 16];
        // event data: mock an event
        let mut evt = ota::event::Data::default();
        let ser = ota::event::data::Serialization {
            nem_id: 1,
            event_id: 2,
            data: vec![1, 2, 3],
        };
        evt.serializations.push(ser);
        let evt_bytes = evt.encode_to_vec();

        let controls = vec![4, 5, 6];
        let data = vec![7, 8, 9];

        let mut payload = Vec::new();
        payload.extend_from_slice(&evt_bytes);
        payload.extend_from_slice(&controls);
        payload.extend_from_slice(&data);

        // register a user so packet gets processed
        emane_rs_ota_manager_register_user(1, 123 as *mut c_void);

        handle_ota_message(
            1,
            2,
            &uuid,
            evt_bytes.len(),
            controls.len(),
            data.len(),
            &payload,
        );

        assert_eq!(EVENTS_DELIVERED.load(Ordering::SeqCst), 1);
        assert_eq!(PACKETS_PROCESSED.load(Ordering::SeqCst), 1);
        assert_eq!(STATS_UPDATES.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_ota_open_and_send() {
        reset_mocks();
        let group_addr = CString::new("127.0.0.1:55555").unwrap();
        let device = CString::new("").unwrap();
        let uuid = [0u8; 16];

        let success = emane_rs_ota_manager_open(
            group_addr.as_ptr(),
            device.as_ptr(),
            1,
            true,
            uuid.as_ptr(),
            0,
            2,
            5,
        );
        assert!(success);
        assert_eq!(PUBLISHERS_INIT.load(Ordering::SeqCst), 1);

        let packet_data = vec![1, 2, 3];
        let empty: Vec<u8> = vec![];
        let sent = emane_rs_ota_manager_send_ota_packet(
            1,
            2,
            packet_data.as_ptr(),
            packet_data.len(),
            empty.as_ptr(),
            0,
            empty.as_ptr(),
            0,
        );
        assert!(sent);
    }
}
