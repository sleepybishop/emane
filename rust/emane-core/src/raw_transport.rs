use std::ffi::{CStr, CString};
use std::io;
use std::mem;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::raw::{c_char, c_void};
use std::ptr;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;

const SNAPSHOT_LENGTH: usize = 65_535;
const RECEIVE_TIMEOUT_MILLISECONDS: libc::c_int = 100;

struct PacketSocket {
    descriptor: OwnedFd,
}

impl PacketSocket {
    fn open(name: &str) -> io::Result<Self> {
        let name = CString::new(name)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "interface contains NUL"))?;
        let interface_index = unsafe { libc::if_nametoindex(name.as_ptr()) };
        if interface_index == 0 {
            return Err(io::Error::last_os_error());
        }

        let protocol = (libc::ETH_P_ALL as u16).to_be();
        let descriptor = unsafe {
            libc::socket(
                libc::AF_PACKET,
                libc::SOCK_RAW | libc::SOCK_CLOEXEC,
                i32::from(protocol),
            )
        };
        if descriptor < 0 {
            return Err(io::Error::last_os_error());
        }
        let descriptor = unsafe { OwnedFd::from_raw_fd(descriptor) };
        let address = libc::sockaddr_ll {
            sll_family: libc::AF_PACKET as libc::c_ushort,
            sll_protocol: protocol,
            sll_ifindex: interface_index as libc::c_int,
            sll_hatype: 0,
            sll_pkttype: 0,
            sll_halen: 0,
            sll_addr: [0; 8],
        };
        if unsafe {
            libc::bind(
                descriptor.as_raw_fd(),
                ptr::addr_of!(address).cast(),
                mem::size_of::<libc::sockaddr_ll>() as libc::socklen_t,
            )
        } < 0
        {
            return Err(io::Error::last_os_error());
        }

        let membership = libc::packet_mreq {
            mr_ifindex: interface_index as libc::c_int,
            mr_type: libc::PACKET_MR_PROMISC as libc::c_ushort,
            mr_alen: 0,
            mr_address: [0; 8],
        };
        if unsafe {
            libc::setsockopt(
                descriptor.as_raw_fd(),
                libc::SOL_PACKET,
                libc::PACKET_ADD_MEMBERSHIP,
                ptr::addr_of!(membership).cast(),
                mem::size_of::<libc::packet_mreq>() as libc::socklen_t,
            )
        } < 0
        {
            return Err(io::Error::last_os_error());
        }

        Ok(Self { descriptor })
    }

    fn send(&self, packet: &[u8]) -> io::Result<usize> {
        let sent = unsafe { libc::send(self.as_raw_fd(), packet.as_ptr().cast(), packet.len(), 0) };
        if sent < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(sent as usize)
        }
    }
}

impl AsRawFd for PacketSocket {
    fn as_raw_fd(&self) -> libc::c_int {
        self.descriptor.as_raw_fd()
    }
}

unsafe impl Send for RawTransport {}
unsafe impl Sync for RawTransport {}

pub struct RawTransport {
    _id: u16,
    thread: Option<thread::JoinHandle<()>>,
    canceled: Arc<AtomicBool>,
    context: *mut c_void,
    cb: extern "C" fn(*mut c_void, *const u8, usize),
    tx_socket: Option<Arc<PacketSocket>>,
}

#[no_mangle]
pub extern "C" fn emane_rs_raw_transport_new(
    id: u16,
    context: *mut c_void,
    cb: extern "C" fn(*mut c_void, *const u8, usize),
) -> *mut RawTransport {
    let transport = Box::new(RawTransport {
        _id: id,
        thread: None,
        canceled: Arc::new(AtomicBool::new(false)),
        context,
        tx_socket: None,
        cb,
    });
    Box::into_raw(transport)
}

#[no_mangle]
pub extern "C" fn emane_rs_raw_transport_free(ptr: *mut RawTransport) {
    if !ptr.is_null() {
        unsafe {
            let mut transport = Box::from_raw(ptr);
            transport.stop();
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_raw_transport_start(
    ptr: *mut RawTransport,
    device_name: *const c_char,
) -> i32 {
    if ptr.is_null() || device_name.is_null() {
        return -1;
    }
    let transport = unsafe { &mut *ptr };
    if transport.thread.is_some() {
        return -1;
    }
    let Ok(name) = (unsafe { CStr::from_ptr(device_name) }).to_str() else {
        return -1;
    };
    let Ok(socket) = open_socket(name) else {
        return -1;
    };

    let socket = Arc::new(socket);
    transport.tx_socket = Some(Arc::clone(&socket));
    transport.canceled.store(false, Ordering::SeqCst);

    let canceled = Arc::clone(&transport.canceled);
    let context = transport.context as usize;
    let callback = transport.cb;
    transport.thread = Some(thread::spawn(move || {
        let mut buffer = [0u8; SNAPSHOT_LENGTH];
        while !canceled.load(Ordering::SeqCst) {
            match receive_inbound(&socket, &mut buffer) {
                Ok(Some(length)) => {
                    callback(context as *mut c_void, buffer.as_ptr(), length);
                }
                Ok(None) => {}
                Err(_) => break,
            }
        }
    }));

    0
}

fn open_socket(name: &str) -> io::Result<PacketSocket> {
    PacketSocket::open(name)
}

fn receive_inbound(socket: &PacketSocket, buffer: &mut [u8]) -> io::Result<Option<usize>> {
    let descriptor = socket.as_raw_fd();
    loop {
        let mut poll_descriptor = libc::pollfd {
            fd: descriptor,
            events: libc::POLLIN,
            revents: 0,
        };
        let ready = unsafe {
            libc::poll(
                ptr::addr_of_mut!(poll_descriptor),
                1,
                RECEIVE_TIMEOUT_MILLISECONDS,
            )
        };
        if ready == 0 {
            return Ok(None);
        }
        if ready < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error);
        }
        if poll_descriptor.revents & libc::POLLIN == 0 {
            return Err(io::Error::other(format!(
                "raw transport poll failed with events {:#x}",
                poll_descriptor.revents
            )));
        }

        let mut address: libc::sockaddr_ll = unsafe { mem::zeroed() };
        let mut address_length = mem::size_of::<libc::sockaddr_ll>() as libc::socklen_t;
        let received = unsafe {
            libc::recvfrom(
                descriptor,
                buffer.as_mut_ptr().cast(),
                buffer.len(),
                0,
                ptr::addr_of_mut!(address).cast(),
                ptr::addr_of_mut!(address_length),
            )
        };
        if received < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error);
        }
        let address_header_length =
            mem::size_of::<libc::sockaddr_ll>() - mem::size_of::<[libc::c_uchar; 8]>();
        if address_length < address_header_length as libc::socklen_t
            || address.sll_family != libc::AF_PACKET as libc::c_ushort
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "raw transport received a non-AF_PACKET address",
            ));
        }
        if !is_inbound_packet_type(address.sll_pkttype) {
            return Ok(None);
        }
        return Ok(Some(received as usize));
    }
}

fn is_inbound_packet_type(packet_type: u8) -> bool {
    packet_type != libc::PACKET_OUTGOING
}

#[no_mangle]
pub extern "C" fn emane_rs_raw_transport_stop(ptr: *mut RawTransport) {
    if ptr.is_null() {
        return;
    }
    unsafe { &mut *ptr }.stop();
}

impl RawTransport {
    fn stop(&mut self) {
        self.canceled.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        self.tx_socket = None;
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_raw_transport_process_upstream_packet(
    ptr: *mut RawTransport,
    buf: *const u8,
    len: usize,
) -> i32 {
    if ptr.is_null() || (len != 0 && buf.is_null()) {
        return -1;
    }
    let transport = unsafe { &mut *ptr };
    let packet = if len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(buf, len) }
    };
    match transport
        .tx_socket
        .as_ref()
        .map(|socket| socket.send(packet))
    {
        Some(Ok(sent)) if sent == len => 0,
        _ => -1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    extern "C" fn capture(context: *mut c_void, data: *const u8, length: usize) {
        let sender = unsafe { &*(context as *const mpsc::Sender<Vec<u8>>) };
        let packet = unsafe { std::slice::from_raw_parts(data, length) }.to_vec();
        sender.send(packet).unwrap();
    }

    #[test]
    fn packet_direction_matches_pcap_inbound_capture() {
        assert!(!is_inbound_packet_type(libc::PACKET_OUTGOING));
        for packet_type in [
            libc::PACKET_HOST,
            libc::PACKET_BROADCAST,
            libc::PACKET_MULTICAST,
            libc::PACKET_OTHERHOST,
        ] {
            assert!(is_inbound_packet_type(packet_type));
        }
    }

    #[test]
    #[ignore = "requires CAP_NET_RAW and EMANE_RAW_TEST_INTERFACE/EMANE_RAW_TEST_PEER veth names"]
    fn captures_and_injects_ethernet_frames_over_veth() {
        let interface = std::env::var("EMANE_RAW_TEST_INTERFACE").unwrap();
        let peer = std::env::var("EMANE_RAW_TEST_PEER").unwrap();
        let interface_name = CString::new(interface).unwrap();
        let (sender, receiver) = mpsc::channel::<Vec<u8>>();
        let context = Box::into_raw(Box::new(sender));
        let transport = emane_rs_raw_transport_new(1, context.cast(), capture);
        assert_eq!(
            emane_rs_raw_transport_start(transport, interface_name.as_ptr()),
            0
        );

        let peer_socket = PacketSocket::open(&peer).unwrap();

        let mut inbound = [0u8; 60];
        inbound[..6].fill(0xff);
        inbound[6..12].copy_from_slice(&[0x02, 0, 0, 0, 0, 2]);
        inbound[12..14].copy_from_slice(&0x88b5u16.to_be_bytes());
        inbound[14..].fill(0x31);
        assert_eq!(peer_socket.send(&inbound).unwrap(), inbound.len());
        assert_eq!(
            receiver.recv_timeout(Duration::from_secs(2)).unwrap(),
            inbound
        );

        let mut outbound = [0u8; 60];
        outbound[..6].fill(0xff);
        outbound[6..12].copy_from_slice(&[0x02, 0, 0, 0, 0, 1]);
        outbound[12..14].copy_from_slice(&0x88b5u16.to_be_bytes());
        outbound[14..].fill(0x32);
        assert_eq!(
            emane_rs_raw_transport_process_upstream_packet(
                transport,
                outbound.as_ptr(),
                outbound.len(),
            ),
            0
        );

        let mut captured = [0u8; SNAPSHOT_LENGTH];
        let received = (0..20)
            .find_map(|_| receive_inbound(&peer_socket, &mut captured).unwrap())
            .expect("peer did not receive injected frame");
        assert_eq!(&captured[..received], &outbound);
        assert!(receiver.recv_timeout(Duration::from_millis(200)).is_err());

        emane_rs_raw_transport_free(transport);
        unsafe { drop(Box::from_raw(context)) };
    }
}
