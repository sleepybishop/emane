use pcap::{Active, Capture};
use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;

unsafe impl Send for RawTransport {}
unsafe impl Sync for RawTransport {}
pub struct RawTransport {
    _id: u16,
    thread: Option<thread::JoinHandle<()>>,
    canceled: Arc<AtomicBool>,
    cpp_obj: *mut c_void,
    cb: extern "C" fn(*mut c_void, *const u8, usize),
    // We keep a tx capture handle to send packets.
    tx_cap: Option<Capture<Active>>,
}

#[no_mangle]
pub extern "C" fn emane_rs_raw_transport_new(
    id: u16,
    cpp_obj: *mut c_void,
    cb: extern "C" fn(*mut c_void, *const u8, usize),
) -> *mut RawTransport {
    let rt = Box::new(RawTransport {
        _id: id,
        thread: None,
        canceled: Arc::new(AtomicBool::new(false)),
        cpp_obj,
        tx_cap: None,
        cb,
    });
    Box::into_raw(rt)
}

#[no_mangle]
pub extern "C" fn emane_rs_raw_transport_free(ptr: *mut RawTransport) {
    if !ptr.is_null() {
        unsafe {
            let mut rt = Box::from_raw(ptr);
            rt.stop();
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
    let rt = unsafe { &mut *ptr };
    if rt.thread.is_some() {
        return -1;
    }
    let Ok(name) = (unsafe { CStr::from_ptr(device_name) }).to_str() else {
        return -1;
    };

    // Create the RX capture handle
    let rx_cap = match Capture::from_device(name) {
        Ok(c) => c,
        Err(_) => return -1,
    };

    let mut rx_cap = match rx_cap
        .promisc(true)
        .snaplen(65535)
        .immediate_mode(true)
        .timeout(100) // 100ms timeout so we can periodically check `canceled`
        .open()
    {
        Ok(c) => c,
        Err(_) => return -1,
    };

    if rx_cap.set_datalink(pcap::Linktype(1)).is_err() {
        return -1;
    }
    if rx_cap.direction(pcap::Direction::In).is_err() {
        return -1;
    }

    // Create the TX capture handle
    let tx_cap = match Capture::from_device(name) {
        Ok(c) => c,
        Err(_) => return -1,
    };

    let tx_cap = match tx_cap
        .promisc(true)
        .snaplen(65535)
        .immediate_mode(true)
        .open()
    {
        Ok(c) => c,
        Err(_) => return -1,
    };
    rt.tx_cap = Some(tx_cap);

    rt.canceled.store(false, Ordering::SeqCst);
    let canceled = rt.canceled.clone();
    let cpp_obj_usize = rt.cpp_obj as usize;
    let cb = rt.cb;

    rt.thread = Some(thread::spawn(move || {
        while !canceled.load(Ordering::SeqCst) {
            match rx_cap.next_packet() {
                Ok(packet) => {
                    let cpp_obj = cpp_obj_usize as *mut c_void;
                    cb(cpp_obj, packet.data.as_ptr(), packet.data.len());
                }
                Err(pcap::Error::TimeoutExpired) => {
                    // Just loop again and check `canceled`
                    continue;
                }
                Err(_) => {
                    // Error occurred, stop reading
                    break;
                }
            }
        }
    }));

    0
}

#[no_mangle]
pub extern "C" fn emane_rs_raw_transport_stop(ptr: *mut RawTransport) {
    if ptr.is_null() {
        return;
    }
    let rt = unsafe { &mut *ptr };
    rt.stop();
}

impl RawTransport {
    fn stop(&mut self) {
        self.canceled.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        self.tx_cap = None; // Drop TX capture handle
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
    let rt = unsafe { &mut *ptr };
    if let Some(tx) = &mut rt.tx_cap {
        let slice = unsafe { std::slice::from_raw_parts(buf, len) };
        if tx.sendpacket(slice).is_ok() {
            return 0;
        }
    }
    -1
}
