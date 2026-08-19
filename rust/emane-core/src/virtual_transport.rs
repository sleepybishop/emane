use crate::tuntap::TunTap;
use libc::{c_void, iovec, readv, writev};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;

unsafe impl Send for VirtualTransport {}
unsafe impl Sync for VirtualTransport {}
pub struct VirtualTransport {
    id: u16,
    tun_tap: Option<Arc<TunTap>>,
    thread: Option<thread::JoinHandle<()>>,
    canceled: Arc<AtomicBool>,
    cpp_obj: *mut c_void,
    cb: extern "C" fn(*mut c_void, *const u8, usize),
}

#[no_mangle]
pub extern "C" fn emane_rs_virtual_transport_new(id: u16, cpp_obj: *mut c_void, cb: extern "C" fn(*mut c_void, *const u8, usize)) -> *mut VirtualTransport {
    let vt = Box::new(VirtualTransport {
        id,
        tun_tap: None,
        thread: None,
        canceled: Arc::new(AtomicBool::new(false)),
        cpp_obj,
        cb,
    });
    Box::into_raw(vt)
}

#[no_mangle]
pub extern "C" fn emane_rs_virtual_transport_free(ptr: *mut VirtualTransport) {
    if !ptr.is_null() {
        unsafe {
            let mut vt = Box::from_raw(ptr);
            vt.stop();
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_virtual_transport_start(
    ptr: *mut VirtualTransport,
    device_path: *const c_char,
    device_name: *const c_char,
    arp_mode: bool,
) -> i32 {
    let vt = unsafe { &mut *ptr };
    let path = unsafe { CStr::from_ptr(device_path).to_str().unwrap() };
    let name = unsafe { CStr::from_ptr(device_name).to_str().unwrap() };

    match TunTap::new(path, name) {
        Ok(tun) => {
            if let Err(_) = tun.set_ethaddr(vt.id) {
                return -1;
            }
            if let Err(_) = tun.activate(arp_mode) {
                return -1;
            }
            vt.canceled.store(false, Ordering::SeqCst);
            let tun_arc = Arc::new(tun);
            vt.tun_tap = Some(tun_arc.clone());
            
            let canceled = vt.canceled.clone();
            let cpp_obj_usize = vt.cpp_obj as usize;
            let cb = vt.cb;
            
            vt.thread = Some(thread::spawn(move || {
                let mut buf = [0u8; 65535];
                while !canceled.load(Ordering::SeqCst) {
                    let mut iov = iovec {
                        iov_base: buf.as_mut_ptr() as *mut c_void,
                        iov_len: buf.len(),
                    };
                    let len = unsafe { readv(tun_arc.fd, &mut iov, 1) };
                    if len > 0 {
                        unsafe {
                            let cpp_obj = cpp_obj_usize as *mut c_void;
                            cb(
                                cpp_obj,
                                buf.as_ptr(),
                                len as usize,
                            );
                        }
                    } else if len < 0 {
                        break;
                    }
                }
            }));
            
            0
        }
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_virtual_transport_stop(ptr: *mut VirtualTransport) {
    if ptr.is_null() { return; }
    let vt = unsafe { &mut *ptr };
    vt.stop();
}

impl VirtualTransport {
    fn stop(&mut self) {
        self.canceled.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        if let Some(tun) = self.tun_tap.take() {
            let _ = tun.deactivate();
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_virtual_transport_process_upstream_packet(
    ptr: *mut VirtualTransport,
    buf: *const u8,
    len: usize,
) -> i32 {
    if ptr.is_null() { return -1; }
    let vt = unsafe { &*ptr };
    if let Some(tun) = &vt.tun_tap {
        let mut iov = iovec {
            iov_base: buf as *const _ as *mut c_void,
            iov_len: len,
        };
        let ret = unsafe { writev(tun.fd, &mut iov, 1) };
        if ret < 0 {
            return -1;
        }
        return 0;
    }
    -1
}
