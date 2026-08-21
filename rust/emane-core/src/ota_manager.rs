use libc::{setsockopt, SOL_SOCKET, SO_BINDTODEVICE};
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
    pub nem_users: HashMap<u16, Vec<OtaUser>>,
    group_addr: Option<std::net::SocketAddr>,
    running: bool,
    upstream_packets: u64,
    reassembly_timeouts: u64,
}

pub type OtaPacketCallback = extern "C" fn(
    context: *mut c_void,
    source: u16,
    destination: u16,
    priority: u8,
    uuid: *const u8,
    data: *const u8,
    data_len: usize,
    controls: *const u8,
    controls_len: usize,
);

#[derive(Clone, Copy)]
pub struct OtaUser {
    context: usize,
    callback: Option<OtaPacketCallback>,
}

impl Default for OtaManager {
    fn default() -> Self {
        Self::new()
    }
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
            running: false,
            upstream_packets: 0,
            reassembly_timeouts: 0,
        }
    }
}

static OTA_MANAGER: OnceLock<Mutex<OtaManager>> = OnceLock::new();

pub fn get_ota_manager() -> &'static Mutex<OtaManager> {
    OTA_MANAGER.get_or_init(|| Mutex::new(OtaManager::new()))
}

#[no_mangle]
pub extern "C" fn emane_rs_ota_manager_register_user(id: u16, p_user: *mut c_void) {
    let mut manager = get_ota_manager().lock().unwrap();
    manager.nem_users.entry(id).or_default().push(OtaUser {
        context: p_user as usize,
        callback: None,
    });
}

pub fn register_native_user(id: u16, context: *mut c_void, callback: OtaPacketCallback) {
    let mut manager = get_ota_manager().lock().unwrap();
    let users = manager.nem_users.entry(id).or_default();
    users.retain(|user| user.context != context as usize);
    users.push(OtaUser {
        context: context as usize,
        callback: Some(callback),
    });
}

pub fn unregister_native_user(id: u16, context: *mut c_void) {
    let mut manager = get_ota_manager().lock().unwrap();
    if let Some(users) = manager.nem_users.get_mut(&id) {
        users.retain(|user| user.context != context as usize);
        if users.is_empty() {
            manager.nem_users.remove(&id);
        }
    }
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
    if group_addr_str.is_null() || uuid.is_null() {
        return false;
    }
    if get_ota_manager().lock().unwrap().running {
        return false;
    }
    let group_addr_c = unsafe { CStr::from_ptr(group_addr_str) }
        .to_string_lossy()
        .into_owned();
    let device_c = if device_str.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(device_str) }
            .to_string_lossy()
            .into_owned()
    };

    let mut uuid_arr = [0u8; 16];
    unsafe {
        uuid_arr.copy_from_slice(std::slice::from_raw_parts(uuid, 16));
    }

    let Ok(group_addr) = group_addr_c.parse::<SocketAddrV4>() else {
        return false;
    };
    let ip = *group_addr.ip();
    let port = group_addr.port();
    let Ok(socket) = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)) else {
        return false;
    };
    if socket.set_reuse_address(true).is_err() {
        return false;
    }

    let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);
    if socket.bind(&addr.into()).is_err() {
        return false;
    }

    if ip.is_multicast()
        && (socket
            .join_multicast_v4(&ip, &Ipv4Addr::UNSPECIFIED)
            .is_err()
            || socket.set_multicast_loop_v4(loopback).is_err()
            || socket.set_multicast_ttl_v4(ttl as u32).is_err())
    {
        return false;
    }

    if !device_c.is_empty() {
        let mut dev_bytes = device_c.into_bytes();
        dev_bytes.push(0);
        unsafe {
            if setsockopt(
                socket.as_raw_fd(),
                SOL_SOCKET,
                SO_BINDTODEVICE,
                dev_bytes.as_ptr() as *const c_void,
                dev_bytes.len() as libc::socklen_t,
            ) != 0
            {
                return false;
            }
        }
    }

    let mut manager = get_ota_manager().lock().unwrap();
    if manager.running {
        return false;
    }
    manager.socket = Some(socket.into());
    manager.uuid = uuid_arr;
    manager.ota_mtu = ota_mtu;
    manager.part_check_threshold = Duration::from_secs(part_check_threshold_secs as u64);
    manager.part_timeout_threshold = Duration::from_secs(part_timeout_threshold_secs as u64);
    manager.group_addr = Some(std::net::SocketAddr::V4(SocketAddrV4::new(ip, port)));
    manager.part_store.clear();
    manager.running = true;
    true
}

#[no_mangle]
pub extern "C" fn emane_rs_ota_manager_close() {
    let mut manager = get_ota_manager().lock().unwrap();
    manager.running = false;
    manager.socket.take();
    manager.part_store.clear();
}

#[no_mangle]
pub extern "C" fn emane_rs_ota_manager_process_loop() {
    let socket = {
        let manager = get_ota_manager().lock().unwrap();
        let Some(socket) = manager.socket.as_ref() else {
            return;
        };
        let Ok(socket) = socket.try_clone() else {
            return;
        };
        socket
    };
    if socket
        .set_read_timeout(Some(Duration::from_millis(200)))
        .is_err()
    {
        return;
    }

    let mut buf = vec![0u8; 65536];

    loop {
        match socket.recv_from(&mut buf) {
            Ok((len, _)) => {
                let now = SystemTime::now();
                if len >= 2 {
                    let header_len = u16::from_be_bytes([buf[0], buf[1]]) as usize;

                    if len >= 2 + header_len + std::mem::size_of::<PartInfo>() {
                        if let Ok(ota_header) = ota::OtaHeader::decode(&buf[2..2 + header_len]) {
                            if ota_header.source > u16::MAX as u32
                                || ota_header.destination > u16::MAX as u32
                            {
                                continue;
                            }
                            let part_info_offset = 2 + header_len;
                            let part_info = unsafe {
                                std::ptr::read_unaligned(
                                    buf.as_ptr().add(part_info_offset) as *const PartInfo
                                )
                            };

                            let u32_offset = u32::from_be(part_info.u32_offset) as usize;
                            let u32_size = u32::from_be(part_info.u32_size) as usize;
                            let u8_more = part_info.u8_more;
                            if u8_more > 1 {
                                continue;
                            }

                            let Some(remote_uuid) = parse_uuid(&ota_header.uuid) else {
                                continue;
                            };

                            let local_uuid = get_ota_manager().lock().unwrap().uuid;
                            if remote_uuid != local_uuid {
                                let payload_index =
                                    part_info_offset + std::mem::size_of::<PartInfo>();

                                if payload_index.checked_add(u32_size) == Some(len) {
                                    if u8_more == 0 && u32_offset == 0 {
                                        // Single part message
                                        let payload_info =
                                            ota_header.payload_info.unwrap_or_default();
                                        let _ = handle_ota_message(
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
                                        let Some(expected_total) = event_len
                                            .checked_add(control_len)
                                            .and_then(|size| size.checked_add(data_len))
                                        else {
                                            continue;
                                        };
                                        if expected_total > MAX_OTA_PAYLOAD
                                            || u32_offset.checked_add(u32_size).is_none()
                                            || u32_offset + u32_size > expected_total
                                            || (u8_more == 0
                                                && u32_offset + u32_size != expected_total)
                                        {
                                            continue;
                                        }

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

                                        if entry.events_size != event_len
                                            || entry.controls_size != control_len
                                            || entry.data_size != data_len
                                            || entry.uuid != remote_uuid
                                        {
                                            manager.part_store.remove(&key);
                                            continue;
                                        }
                                        if let Some(previous) = entry.parts.insert(
                                            u32_offset,
                                            buf[payload_index..payload_index + u32_size].to_vec(),
                                        ) {
                                            entry.total_size =
                                                entry.total_size.saturating_sub(previous.len());
                                        }
                                        entry.total_size =
                                            entry.total_size.saturating_add(u32_size);
                                        entry.last_part_time = now;

                                        let mut next_offset = 0usize;
                                        let contiguous =
                                            entry.parts.iter().all(|(offset, part)| {
                                                if *offset != next_offset {
                                                    return false;
                                                }
                                                next_offset =
                                                    next_offset.saturating_add(part.len());
                                                next_offset <= expected_total
                                            });
                                        if entry.total_size == expected_total
                                            && contiguous
                                            && next_offset == expected_total
                                        {
                                            // All parts received!
                                            let mut full_payload =
                                                Vec::with_capacity(expected_total);
                                            for part in entry.parts.values() {
                                                full_payload.extend_from_slice(part);
                                            }

                                            let e_size = entry.events_size;
                                            let c_size = entry.controls_size;
                                            let d_size = entry.data_size;
                                            let remote_uuid = entry.uuid;

                                            manager.part_store.remove(&key);
                                            drop(manager);

                                            let _ = handle_ota_message(
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
                let expired = {
                    let mut manager = get_ota_manager().lock().unwrap();
                    let mut expired = Vec::new();
                    if now
                        .duration_since(manager.last_part_check_time)
                        .unwrap_or(Duration::ZERO)
                        >= manager.part_check_threshold
                    {
                        let threshold = manager.part_timeout_threshold;
                        manager.part_store.retain(|key, value| {
                            let remove = now
                                .duration_since(value.last_part_time)
                                .unwrap_or(Duration::ZERO)
                                >= threshold;
                            if remove {
                                expired.push((value.uuid, key.source));
                            }
                            !remove
                        });
                        manager.last_part_check_time = now;
                    }
                    expired
                };
                if !expired.is_empty() {
                    let mut manager = get_ota_manager().lock().unwrap();
                    manager.reassembly_timeouts = manager
                        .reassembly_timeouts
                        .saturating_add(expired.len() as u64);
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(_) => break,
        }
        if !get_ota_manager().lock().unwrap().running {
            break;
        }
    }
}

const MAX_OTA_PAYLOAD: usize = 64 * 1024 * 1024;

fn parse_uuid(bytes: &[u8]) -> Option<[u8; 16]> {
    bytes.try_into().ok()
}

fn handle_ota_message(
    source: u16,
    destination: u16,
    remote_uuid: &[u8; 16],
    events_size: usize,
    controls_size: usize,
    data_size: usize,
    payload: &[u8],
) -> bool {
    let Some(expected_size) = events_size
        .checked_add(controls_size)
        .and_then(|size| size.checked_add(data_size))
    else {
        return false;
    };
    if expected_size != payload.len() || expected_size > MAX_OTA_PAYLOAD {
        return false;
    }
    let mut offset = 0;

    if events_size > 0 {
        if let Ok(data) = ota::event::Data::decode(&payload[offset..offset + events_size]) {
            for serialization in data.serializations {
                let (Ok(nem_id), Ok(event_id)) = (
                    u16::try_from(serialization.nem_id),
                    u16::try_from(serialization.event_id),
                ) else {
                    continue;
                };
                crate::event_service::route_serialized_event(
                    nem_id,
                    event_id,
                    &serialization.data,
                    0,
                );
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
        let users: Vec<OtaUser> = {
            let mut manager = get_ota_manager().lock().unwrap();
            manager.upstream_packets = manager.upstream_packets.saturating_add(1);
            manager.nem_users.values().flatten().copied().collect()
        };

        for user in users {
            if let Some(callback) = user.callback {
                callback(
                    user.context as *mut c_void,
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
    true
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
    if (packet_len != 0 && packet_data.is_null())
        || (controls_len != 0 && controls_data.is_null())
        || (events_len != 0 && events_data.is_null())
        || packet_len > u32::MAX as usize
        || controls_len > u32::MAX as usize
        || events_len > u32::MAX as usize
    {
        return false;
    }
    let Some(total_payload_size) = events_len
        .checked_add(controls_len)
        .and_then(|size| size.checked_add(packet_len))
    else {
        return false;
    };
    if total_payload_size > MAX_OTA_PAYLOAD {
        return false;
    }
    let mut manager = get_ota_manager().lock().unwrap();
    let uuid = manager.uuid;
    let ota_mtu = manager.ota_mtu;

    if manager.socket.is_none() {
        return false;
    }

    manager.u64_sequence_number = manager.u64_sequence_number.wrapping_add(1);
    let seq = manager.u64_sequence_number;

    let Ok(socket) = manager.socket.as_ref().unwrap().try_clone() else {
        return false;
    };
    let Some(group_addr) = manager.group_addr else {
        return false;
    };
    drop(manager);

    let ota_header = ota::OtaHeader {
        source: source as u32,
        destination: destination as u32,
        sequence: seq,
        uuid: uuid.to_vec(),
        payload_info: Some(ota::ota_header::PayloadInfo {
            data_length: packet_len as u32,
            control_length: controls_len as u32,
            event_length: events_len as u32,
        }),
    };

    let ota_header_bytes = ota_header.encode_to_vec();
    let header_len = ota_header_bytes.len();
    if header_len > u16::MAX as usize {
        return false;
    }
    let u16_header_length = (header_len as u16).to_be_bytes();

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

    let overhead = 2 + header_len + std::mem::size_of::<PartInfo>();
    if ota_mtu != 0 && ota_mtu <= overhead && total_payload_size != 0 {
        return false;
    }

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
            ota_mtu - overhead
        } else {
            total_payload_size - sent_bytes
        };
        if payload_size == 0 || payload_size > u32::MAX as usize {
            return false;
        }

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
        }

        if message.len() != overhead + payload_size {
            return false;
        }

        if socket.send_to(&message, group_addr).is_err() {
            return false;
        }

        sent_bytes += payload_size;
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

    static EVENTS_DELIVERED: AtomicUsize = AtomicUsize::new(0);
    static PACKETS_PROCESSED: AtomicUsize = AtomicUsize::new(0);
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn test_guard() -> std::sync::MutexGuard<'static, ()> {
        TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner())
    }

    extern "C" fn event_callback(
        _context: *mut c_void,
        _event_id: u16,
        _data: *const u8,
        _len: usize,
    ) {
        EVENTS_DELIVERED.fetch_add(1, Ordering::SeqCst);
    }

    extern "C" fn packet_callback(
        _context: *mut c_void,
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
        EVENTS_DELIVERED.store(0, Ordering::SeqCst);
        PACKETS_PROCESSED.store(0, Ordering::SeqCst);
    }

    #[test]
    fn test_ota_nem_states() {
        let _guard = test_guard();
        let manager = get_ota_manager();
        let id = 123;
        let p_user = 456 as *mut c_void;
        emane_rs_ota_manager_register_user(id, p_user);
        assert_eq!(
            manager.lock().unwrap().nem_users.get(&id).unwrap()[0].context,
            p_user as usize
        );
        emane_rs_ota_manager_unregister_user(id);
        assert!(!manager.lock().unwrap().nem_users.contains_key(&id));
    }

    #[test]
    fn test_handle_ota_message() {
        let _guard = test_guard();
        reset_mocks();
        let uuid = [0u8; 16];
        // event data: mock an event
        let mut evt = ota::event::Data::default();
        let test_nem = 60_001;
        let ser = ota::event::data::Serialization {
            nem_id: u32::from(test_nem),
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
        register_native_user(test_nem, std::ptr::null_mut(), packet_callback);
        crate::event_service::register_native_user(
            99,
            test_nem,
            std::ptr::null_mut(),
            event_callback,
        );
        assert!(crate::event_service::emane_rs_event_service_register_event(
            99, 2
        ));

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
        emane_rs_ota_manager_unregister_user(test_nem);
        crate::event_service::unregister_user(99);
    }

    #[test]
    fn test_ota_open_and_send() {
        let _guard = test_guard();
        reset_mocks();
        let Ok(reservation) = UdpSocket::bind("127.0.0.1:0") else {
            // Some hermetic test runners prohibit even loopback sockets.
            return;
        };
        let port = reservation.local_addr().unwrap().port();
        drop(reservation);
        let group_addr = CString::new(format!("127.0.0.1:{port}")).unwrap();
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
        let packet_data = [1, 2, 3];
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
        emane_rs_ota_manager_close();
    }

    #[test]
    fn rejects_malformed_payload_lengths_and_null_inputs() {
        let _guard = test_guard();
        reset_mocks();
        let uuid = [1u8; 16];
        assert!(!handle_ota_message(1, 2, &uuid, 4, 0, 0, &[1, 2, 3]));
        assert_eq!(EVENTS_DELIVERED.load(Ordering::SeqCst), 0);
        assert_eq!(PACKETS_PROCESSED.load(Ordering::SeqCst), 0);

        assert!(!emane_rs_ota_manager_send_ota_packet(
            1,
            2,
            std::ptr::null(),
            1,
            std::ptr::null(),
            0,
            std::ptr::null(),
            0,
        ));
    }

    #[test]
    fn rejects_datagram_with_non_uuid_identifier() {
        let _guard = test_guard();
        let header = ota::OtaHeader {
            source: 1,
            destination: 2,
            uuid: vec![1, 2, 3],
            payload_info: Some(ota::ota_header::PayloadInfo {
                data_length: 1,
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(parse_uuid(&header.uuid).is_none());
    }
}
