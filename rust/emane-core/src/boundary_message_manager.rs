use std::os::raw::{c_char, c_void};
use std::ffi::CStr;
use std::sync::{Arc, Mutex};
use std::thread;
use libc::iovec;
use std::net::{UdpSocket, TcpListener, TcpStream};
use std::io::{Read, Write};

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
pub extern "C" fn emane_rs_boundary_manager_new(id: u16, manager_ptr: *mut c_void) -> *mut Arc<BoundaryMessageManagerState> {
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
        
        {
            let mut cancel = state.cancel.lock().unwrap();
            *cancel = true;
        }
        
        let handle = state.thread_handle.lock().unwrap().take();
        if let Some(h) = handle {
            let _ = h.join();
        }
    }
}

fn handle_udp(state: Arc<BoundaryMessageManagerState>, local_addr: String, _remote_addr: String) {
    let socket = UdpSocket::bind(&local_addr).expect("Unable to bind UDP");
    socket.set_nonblocking(false).unwrap();
    state.udp_socket.lock().unwrap().replace(socket.try_clone().unwrap());
    
    let mut buf = [0u8; 65536];
    loop {
        if *state.cancel.lock().unwrap() {
            break;
        }
        if let Ok((len, _)) = socket.recv_from(&mut buf) {
            unsafe {
                emane_c_boundary_manager_handle_message(state.c_manager_ptr, buf.as_mut_ptr(), len);
            }
        } else {
            break;
        }
    }
}

fn handle_tcp_server(state: Arc<BoundaryMessageManagerState>, local_addr: String) {
    let listener = TcpListener::bind(&local_addr).expect("Unable to bind TCP");
    listener.set_nonblocking(false).unwrap();
    
    for stream in listener.incoming() {
        if *state.cancel.lock().unwrap() {
            break;
        }
        if let Ok(mut s) = stream {
            state.tcp_stream.lock().unwrap().replace(s.try_clone().unwrap());
            let mut header_buf = [0u8; 6]; // length of NetAdapterHeader is 6
            loop {
                if *state.cancel.lock().unwrap() {
                    break;
                }
                if s.read_exact(&mut header_buf).is_ok() {
                    let mut len_bytes = [0u8; 4];
                    len_bytes.copy_from_slice(&header_buf[2..6]);
                    let msg_len = u32::from_be_bytes(len_bytes) as usize;
                    
                    if msg_len >= 6 {
                        let mut full_buf = vec![0u8; msg_len];
                        full_buf[0..6].copy_from_slice(&header_buf);
                        if s.read_exact(&mut full_buf[6..]).is_ok() {
                            unsafe {
                                emane_c_boundary_manager_handle_message(state.c_manager_ptr, full_buf.as_mut_ptr(), msg_len);
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
    }
}

fn handle_tcp_client(state: Arc<BoundaryMessageManagerState>, remote_addr: String) {
    loop {
        if *state.cancel.lock().unwrap() {
            break;
        }
        if let Ok(mut s) = TcpStream::connect(&remote_addr) {
            state.tcp_stream.lock().unwrap().replace(s.try_clone().unwrap());
            let mut header_buf = [0u8; 6];
            loop {
                if *state.cancel.lock().unwrap() {
                    break;
                }
                if s.read_exact(&mut header_buf).is_ok() {
                    let mut len_bytes = [0u8; 4];
                    len_bytes.copy_from_slice(&header_buf[2..6]);
                    let msg_len = u32::from_be_bytes(len_bytes) as usize;
                    
                    if msg_len >= 6 {
                        let mut full_buf = vec![0u8; msg_len];
                        full_buf[0..6].copy_from_slice(&header_buf);
                        if s.read_exact(&mut full_buf[6..]).is_ok() {
                            unsafe {
                                emane_c_boundary_manager_handle_message(state.c_manager_ptr, full_buf.as_mut_ptr(), msg_len);
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
pub extern "C" fn emane_rs_boundary_manager_open(ptr: *mut Arc<BoundaryMessageManagerState>, local_addr: *const c_char, remote_addr: *const c_char, protocol: i32) {
    let state = unsafe { &*ptr }.clone();
    
    let local = unsafe { CStr::from_ptr(local_addr) }.to_string_lossy().into_owned();
    let remote = unsafe { CStr::from_ptr(remote_addr) }.to_string_lossy().into_owned();
    
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
    let state = unsafe { &*ptr };
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
pub extern "C" fn emane_rs_boundary_manager_send(ptr: *mut Arc<BoundaryMessageManagerState>, iov: *const iovec, iov_len: usize) {
    let state = unsafe { &*ptr };
    
    let mut total_len = 0;
    let iov_slice = unsafe { std::slice::from_raw_parts(iov, iov_len) };
    let mut buf = Vec::new();
    
    for i in 0..iov_len {
        let slice = unsafe { std::slice::from_raw_parts(iov_slice[i].iov_base as *const u8, iov_slice[i].iov_len) };
        buf.extend_from_slice(slice);
        total_len += iov_slice[i].iov_len;
    }
    
    let udp = state.udp_socket.lock().unwrap().as_ref().map(|s| s.try_clone().unwrap());
    if let Some(sock) = udp {
        let remote = state.remote_addr.lock().unwrap().clone().unwrap();
        let _ = sock.send_to(&buf, &remote);
        return;
    }
    
    let tcp = state.tcp_stream.lock().unwrap().as_ref().map(|s| s.try_clone().unwrap());
    if let Some(mut sock) = tcp {
        let _ = sock.write_all(&buf);
    }
}
