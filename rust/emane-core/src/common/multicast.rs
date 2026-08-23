use socket2::Socket;
use std::ffi::CString;
use std::io;
use std::net::{IpAddr, Ipv4Addr};
use std::os::fd::AsRawFd;

pub(crate) fn configure(
    socket: &Socket,
    group: IpAddr,
    device: &str,
    ttl: u32,
    loopback: bool,
) -> io::Result<()> {
    match group {
        IpAddr::V4(group) => {
            socket.set_multicast_ttl_v4(ttl)?;
            socket.set_multicast_loop_v4(loopback)?;

            let interface = if device.is_empty() {
                Ipv4Addr::UNSPECIFIED
            } else {
                let address = ipv4_interface_address(socket, device)?;
                socket.set_multicast_if_v4(&address)?;
                address
            };

            if group.is_multicast() {
                socket.join_multicast_v4(&group, &interface)?;
            }
        }
        IpAddr::V6(group) => {
            socket.set_multicast_hops_v6(ttl)?;
            socket.set_multicast_loop_v6(loopback)?;

            let interface = if device.is_empty() {
                0
            } else {
                let device = CString::new(device).map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidInput, "interface name contains NUL")
                })?;
                let index = unsafe { libc::if_nametoindex(device.as_ptr()) };
                if index == 0 {
                    return Err(io::Error::last_os_error());
                }
                socket.set_multicast_if_v6(index)?;
                index
            };

            if group.is_multicast() {
                socket.join_multicast_v6(&group, interface)?;
            }
        }
    }

    Ok(())
}

fn ipv4_interface_address(socket: &Socket, device: &str) -> io::Result<Ipv4Addr> {
    let bytes = device.as_bytes();
    if bytes.is_empty() || bytes.len() >= libc::IFNAMSIZ {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid interface name",
        ));
    }

    let mut request: libc::ifreq = unsafe { std::mem::zeroed() };
    for (target, source) in request.ifr_name.iter_mut().zip(bytes.iter().copied()) {
        *target = source as libc::c_char;
    }

    if unsafe { libc::ioctl(socket.as_raw_fd(), libc::SIOCGIFADDR, &mut request) } < 0 {
        return Err(io::Error::last_os_error());
    }

    let address = unsafe { request.ifr_ifru.ifru_addr };
    if address.sa_family != libc::AF_INET as libc::sa_family_t {
        return Err(io::Error::new(
            io::ErrorKind::AddrNotAvailable,
            "interface has no IPv4 address",
        ));
    }
    let address = unsafe { *(&address as *const libc::sockaddr).cast::<libc::sockaddr_in>() };
    Ok(Ipv4Addr::from(address.sin_addr.s_addr.to_ne_bytes()))
}
