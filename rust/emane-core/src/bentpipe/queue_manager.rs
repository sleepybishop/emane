use super::queue::Queue;
use std::collections::BTreeMap;
use std::ffi::c_void;

#[repr(C)]
pub struct DequeueAction {
    pub action_type: u8, // 0 = DROP, 1 = DEQUEUE_FULL, 2 = DEQUEUE_FRAGMENT
    pub transponder_index: u16,
    pub pkt_ptr: *mut c_void,
    pub seq: u64,
    pub fragment_index: usize,
    pub fragment_offset: usize,
    pub fragment_size: usize,
    pub more_fragments: bool,
}

#[repr(C)]
pub struct EnqueueResult {
    pub dropped_pkt: *mut c_void,
    pub dropped: bool,
}

#[repr(C)]
pub struct DequeueResult {
    pub actions: *mut DequeueAction,
    pub num_actions: usize,
    pub total_bytes: usize,
}

pub struct QueueManager {
    queues: BTreeMap<u16, Queue>,
    queue_depth: u16,
    aggregation_enable: bool,
    fragmentation_enable: bool,
}

impl QueueManager {
    pub fn new() -> Self {
        Self {
            queues: BTreeMap::new(),
            queue_depth: 256,
            aggregation_enable: true,
            fragmentation_enable: true,
        }
    }
}

#[no_mangle]
pub extern "C" fn bentpipe_queue_manager_new() -> *mut QueueManager {
    Box::into_raw(Box::new(QueueManager::new()))
}

#[no_mangle]
pub extern "C" fn bentpipe_queue_manager_free(m: *mut QueueManager) {
    if !m.is_null() {
        unsafe { drop(Box::from_raw(m)) };
    }
}

#[no_mangle]
pub extern "C" fn bentpipe_queue_manager_set_config(
    m: *mut QueueManager,
    queue_depth: u16,
    aggregation_enable: bool,
    fragmentation_enable: bool,
) {
    let m = unsafe { &mut *m };
    m.queue_depth = queue_depth;
    m.aggregation_enable = aggregation_enable;
    m.fragmentation_enable = fragmentation_enable;
    for q in m.queues.values_mut() {
        q.queue_depth = queue_depth;
        q.aggregate = aggregation_enable;
        q.fragment = fragmentation_enable;
    }
}

#[no_mangle]
pub extern "C" fn bentpipe_queue_manager_add_queue(m: *mut QueueManager, transponder_index: u16) {
    let m = unsafe { &mut *m };
    let mut q = Queue::new();
    q.queue_depth = m.queue_depth;
    q.aggregate = m.aggregation_enable;
    q.fragment = m.fragmentation_enable;
    m.queues.insert(transponder_index, q);
}

#[no_mangle]
pub extern "C" fn bentpipe_queue_manager_remove_queue(m: *mut QueueManager, transponder_index: u16) {
    let m = unsafe { &mut *m };
    m.queues.remove(&transponder_index);
}

#[no_mangle]
pub extern "C" fn bentpipe_queue_manager_enqueue(
    m: *mut QueueManager,
    transponder_index: u16,
    pkt_ptr: *mut c_void,
    length: usize,
) -> EnqueueResult {
    let m = unsafe { &mut *m };
    if let Some(q) = m.queues.get_mut(&transponder_index) {
        let mut dropped_pkt = std::ptr::null_mut();
        let dropped = q.enqueue(pkt_ptr, length, &mut dropped_pkt);
        EnqueueResult { dropped_pkt, dropped }
    } else {
        EnqueueResult { dropped_pkt: std::ptr::null_mut(), dropped: false }
    }
}

#[no_mangle]
pub extern "C" fn bentpipe_queue_manager_dequeue(
    m: *mut QueueManager,
    transponder_index: u16,
    requested_bytes: usize,
    res: *mut DequeueResult,
) {
    let m = unsafe { &mut *m };
    let mut actions = Vec::new();
    let mut total_length = 0;
    
    if let Some(q) = m.queues.get_mut(&transponder_index) {
        let (mut q_actions, q_len) = q.dequeue_impl(requested_bytes, true, transponder_index);
        total_length += q_len;
        actions.append(&mut q_actions);
    }
    
    actions.shrink_to_fit();
    unsafe {
        (*res).actions = actions.as_mut_ptr();
        (*res).num_actions = actions.len();
        (*res).total_bytes = total_length;
    }
    std::mem::forget(actions);
}

#[no_mangle]
pub extern "C" fn bentpipe_queue_manager_free_dequeue_result(res: *mut DequeueResult) {
    if !res.is_null() {
        unsafe {
            let num = (*res).num_actions;
            let ptr = (*res).actions;
            if !ptr.is_null() && num > 0 {
                let _ = Vec::from_raw_parts(ptr, num, num);
            }
        }
    }
}

#[repr(C)]
pub struct QueueInfo {
    pub transponder_index: u16,
    pub packets: usize,
    pub bytes: usize,
}

#[repr(C)]
pub struct QueueInfosResult {
    pub infos: *mut QueueInfo,
    pub num_infos: usize,
}

#[no_mangle]
pub extern "C" fn bentpipe_queue_manager_get_queue_infos(
    m: *const QueueManager,
    res: *mut QueueInfosResult,
) {
    let m = unsafe { &*m };
    let mut infos = Vec::with_capacity(m.queues.len());
    for (&transponder_index, q) in &m.queues {
        infos.push(QueueInfo {
            transponder_index,
            packets: q.queue.len(),
            bytes: q.current_bytes,
        });
    }
    infos.shrink_to_fit();
    unsafe {
        (*res).infos = infos.as_mut_ptr();
        (*res).num_infos = infos.len();
    }
    std::mem::forget(infos);
}

#[no_mangle]
pub extern "C" fn bentpipe_queue_manager_free_queue_infos_result(res: *mut QueueInfosResult) {
    if !res.is_null() {
        unsafe {
            let num = (*res).num_infos;
            let ptr = (*res).infos;
            if !ptr.is_null() && num > 0 {
                let _ = Vec::from_raw_parts(ptr, num, num);
            }
        }
    }
}
