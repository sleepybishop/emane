use std::collections::{HashMap, VecDeque};
use std::ffi::CString;
use std::fs::File;
use std::io::Write;
use std::os::raw::c_void;
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref QUEUES: Mutex<HashMap<u16, VecDeque<QueueEntry>>> = Mutex::new(HashMap::new());
}

struct QueueEntry {
    tx_time: f64,
    rx_time: f64,
    src: u16,
    pkt_id: u16,
}

#[no_mangle]
pub extern "C" fn emane_timinganalysis_processUpstreamControl(
    _layer: *mut c_void,
    _msgs: *const c_void,
) {
    let msg = CString::new("SHIM TimingAnalysis::ShimLayer::processUpstreamControl").unwrap();
    crate::log_service::emane_rs_log(4, msg.as_ptr());
}

#[no_mangle]
pub extern "C" fn emane_timinganalysis_processDownstreamControl(
    _layer: *mut c_void,
    _msgs: *const c_void,
) {
    let msg = CString::new("SHIM TimingAnalysis::ShimLayer::processDownstreamControl").unwrap();
    crate::log_service::emane_rs_log(4, msg.as_ptr());
}

#[no_mangle]
pub extern "C" fn emane_timinganalysis_processUpstreamPacket(
    _layer: *mut c_void,
    id: u16,
    tx_time: f64,
    rx_time: f64,
    src: u16,
    pkt_id: u16,
    max_queue_size: u32,
) {
    let msg_str = format!("SHIM {:03} TimingAnalysis::ShimLayer::processUpstreamPacket Src: {} PktID: {} TxTime: {} RxTime: {}, deltaT {} sec",
                          id, src, pkt_id, tx_time, rx_time, rx_time - tx_time);
    let msg = CString::new(msg_str).unwrap();
    crate::log_service::emane_rs_log(4, msg.as_ptr());

    let mut queues = QUEUES.lock().unwrap();
    let q = queues.entry(id).or_insert_with(VecDeque::new);
    if max_queue_size != 0 && q.len() >= max_queue_size as usize {
        q.pop_front();
    }
    q.push_back(QueueEntry {
        tx_time,
        rx_time,
        src,
        pkt_id,
    });
}

#[no_mangle]
pub extern "C" fn emane_timinganalysis_processDownstreamPacket(_layer: *mut c_void, _id: u16) {
    let msg = CString::new("SHIM TimingAnalysis::ShimLayer::processDownstreamPacket").unwrap();
    crate::log_service::emane_rs_log(4, msg.as_ptr());
}

#[no_mangle]
pub extern "C" fn emane_timinganalysis_stop(id: u16) {
    let mut queues = QUEUES.lock().unwrap();
    if let Some(mut q) = queues.remove(&id) {
        let filename = format!("/tmp/timinganalysis{}.txt", id);
        if let Ok(mut file) = File::create(&filename) {
            while let Some(entry) = q.pop_front() {
                let _ = writeln!(
                    file,
                    "{} {} {} {}",
                    entry.src, entry.pkt_id, entry.tx_time, entry.rx_time
                );
            }
        }
    }
}
