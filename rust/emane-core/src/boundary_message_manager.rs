use libc::iovec;
use std::ffi::CStr;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs, UdpSocket};
use std::os::raw::{c_char, c_void};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

extern "C" {
    fn emane_c_boundary_manager_handle_message(manager_ptr: *mut c_void, buf: *mut u8, len: usize);
}

pub struct BoundaryMessageManagerState {
    id: u16,
    c_manager_ptr: *mut c_void,
    thread_handle: Mutex<Option<thread::JoinHandle<()>>>,
    cancel: Mutex<bool>,
    udp_socket: Mutex<Option<UdpSocket>>,
    tcp_stream: Mutex<Option<TcpStream>>,
    remote_addr: Mutex<Option<String>>,
}

unsafe impl Send for BoundaryMessageManagerState {}
unsafe impl Sync for BoundaryMessageManagerState {}

#[no_mangle]
pub extern "C" fn emane_rs_boundary_manager_new(
    id: u16,
    manager_ptr: *mut c_void,
) -> *mut Arc<BoundaryMessageManagerState> {
    let state = Arc::new(BoundaryMessageManagerState {
        id,
        c_manager_ptr: manager_ptr,
        thread_handle: Mutex::new(None),
        cancel: Mutex::new(false),
        udp_socket: Mutex::new(None),
        tcp_stream: Mutex::new(None),
        remote_addr: Mutex::new(None),
    });
    Box::into_raw(Box::new(state))
}

#[no_mangle]
pub extern "C" fn emane_rs_boundary_manager_free(ptr: *mut Arc<BoundaryMessageManagerState>) {
    if !ptr.is_null() {
        let state_box = unsafe { Box::from_raw(ptr) };
        let state = *state_box;
        stop_state(&state);
    }
}

fn handle_udp(state: Arc<BoundaryMessageManagerState>, local_addr: String, _remote_addr: String) {
    let Ok(socket) = UdpSocket::bind(&local_addr) else {
        return;
    };
    if socket
        .set_read_timeout(Some(Duration::from_millis(100)))
        .is_err()
    {
        return;
    }
    let Ok(clone) = socket.try_clone() else {
        return;
    };
    state.udp_socket.lock().unwrap().replace(clone);

    let mut buf = [0u8; 65536];
    loop {
        if *state.cancel.lock().unwrap() {
            break;
        }
        match socket.recv_from(&mut buf) {
            Ok((len, _)) => unsafe {
                emane_c_boundary_manager_handle_message(state.c_manager_ptr, buf.as_mut_ptr(), len);
            },
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(_) => break,
        }
    }
}

fn handle_tcp_server(state: Arc<BoundaryMessageManagerState>, local_addr: String) {
    let Ok(listener) = TcpListener::bind(&local_addr) else {
        return;
    };
    if listener.set_nonblocking(true).is_err() {
        return;
    }

    loop {
        if *state.cancel.lock().unwrap() {
            break;
        }
        match listener.accept() {
            Ok((mut s, _)) => {
                let Ok(clone) = s.try_clone() else {
                    continue;
                };
                state.tcp_stream.lock().unwrap().replace(clone);
                let mut header_buf = [0u8; 6]; // length of NetAdapterHeader is 6
                loop {
                    if *state.cancel.lock().unwrap() {
                        break;
                    }
                    if s.read_exact(&mut header_buf).is_ok() {
                        let mut len_bytes = [0u8; 4];
                        len_bytes.copy_from_slice(&header_buf[2..6]);
                        let msg_len = u32::from_be_bytes(len_bytes) as usize;

                        if (6..=64 * 1024 * 1024).contains(&msg_len) {
                            let mut full_buf = vec![0u8; msg_len];
                            full_buf[0..6].copy_from_slice(&header_buf);
                            if s.read_exact(&mut full_buf[6..]).is_ok() {
                                unsafe {
                                    emane_c_boundary_manager_handle_message(
                                        state.c_manager_ptr,
                                        full_buf.as_mut_ptr(),
                                        msg_len,
                                    );
                                }
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(50));
            }
            Err(_) => break,
        }
    }
}

fn handle_tcp_client(state: Arc<BoundaryMessageManagerState>, remote_addr: String) {
    loop {
        if *state.cancel.lock().unwrap() {
            break;
        }
        let stream = remote_addr.to_socket_addrs().ok().and_then(|addresses| {
            addresses
                .filter_map(|address| {
                    TcpStream::connect_timeout(&address, Duration::from_millis(200)).ok()
                })
                .next()
        });
        if let Some(mut s) = stream {
            let Ok(clone) = s.try_clone() else {
                continue;
            };
            state.tcp_stream.lock().unwrap().replace(clone);
            let mut header_buf = [0u8; 6];
            loop {
                if *state.cancel.lock().unwrap() {
                    break;
                }
                if s.read_exact(&mut header_buf).is_ok() {
                    let mut len_bytes = [0u8; 4];
                    len_bytes.copy_from_slice(&header_buf[2..6]);
                    let msg_len = u32::from_be_bytes(len_bytes) as usize;

                    if (6..=64 * 1024 * 1024).contains(&msg_len) {
                        let mut full_buf = vec![0u8; msg_len];
                        full_buf[0..6].copy_from_slice(&header_buf);
                        if s.read_exact(&mut full_buf[6..]).is_ok() {
                            unsafe {
                                emane_c_boundary_manager_handle_message(
                                    state.c_manager_ptr,
                                    full_buf.as_mut_ptr(),
                                    msg_len,
                                );
                            }
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        } else {
            thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_boundary_manager_open(
    ptr: *mut Arc<BoundaryMessageManagerState>,
    local_addr: *const c_char,
    remote_addr: *const c_char,
    protocol: i32,
) {
    if ptr.is_null() || local_addr.is_null() || remote_addr.is_null() || !matches!(protocol, 0..=2)
    {
        return;
    }
    let state = unsafe { &*ptr }.clone();

    if state.thread_handle.lock().unwrap().is_some() {
        return;
    }
    *state.cancel.lock().unwrap() = false;

    let local = unsafe { CStr::from_ptr(local_addr) }
        .to_string_lossy()
        .into_owned();
    let remote = unsafe { CStr::from_ptr(remote_addr) }
        .to_string_lossy()
        .into_owned();

    *state.remote_addr.lock().unwrap() = Some(remote.clone());

    let state_clone = state.clone();
    let handle = thread::spawn(move || {
        if protocol == 0 {
            handle_udp(state_clone, local, remote);
        } else if protocol == 1 {
            handle_tcp_server(state_clone, local);
        } else if protocol == 2 {
            handle_tcp_client(state_clone, remote);
        }
    });

    *state.thread_handle.lock().unwrap() = Some(handle);
}

#[no_mangle]
pub extern "C" fn emane_rs_boundary_manager_close(ptr: *mut Arc<BoundaryMessageManagerState>) {
    if ptr.is_null() {
        return;
    }
    let state = unsafe { &*ptr };
    stop_state(state);
}

fn stop_state(state: &Arc<BoundaryMessageManagerState>) {
    {
        let mut cancel = state.cancel.lock().unwrap();
        *cancel = true;
    }

    if let Some(sock) = state.udp_socket.lock().unwrap().take() {
        drop(sock);
    }
    if let Some(sock) = state.tcp_stream.lock().unwrap().take() {
        let _ = sock.shutdown(std::net::Shutdown::Both);
    }

    let handle = state.thread_handle.lock().unwrap().take();
    if let Some(h) = handle {
        let _ = h.join();
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_boundary_manager_send(
    ptr: *mut Arc<BoundaryMessageManagerState>,
    iov: *const iovec,
    iov_len: usize,
) {
    if ptr.is_null() || (iov_len != 0 && iov.is_null()) {
        return;
    }
    let state = unsafe { &*ptr };

    let iov_slice = if iov_len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(iov, iov_len) }
    };
    let mut buf = Vec::new();

    for item in iov_slice {
        if item.iov_len == 0 {
            continue;
        }
        if item.iov_base.is_null()
            || buf.len().checked_add(item.iov_len).is_none()
            || buf.len() + item.iov_len > 64 * 1024 * 1024
        {
            return;
        }
        let slice = unsafe { std::slice::from_raw_parts(item.iov_base as *const u8, item.iov_len) };
        buf.extend_from_slice(slice);
    }

    let udp = state
        .udp_socket
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|socket| socket.try_clone().ok());
    if let Some(sock) = udp {
        if let Some(remote) = state.remote_addr.lock().unwrap().clone() {
            let _ = sock.send_to(&buf, &remote);
        }
        return;
    }

    let tcp = state
        .tcp_stream
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|socket| socket.try_clone().ok());
    if let Some(mut sock) = tcp {
        let _ = sock.write_all(&buf);
    }
}
