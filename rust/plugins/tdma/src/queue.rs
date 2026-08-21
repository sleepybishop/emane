use std::collections::{BTreeMap, HashMap};

pub struct MetaInfo {
    pub index: usize,
    pub offset: usize,
}

pub struct PacketEntry {
    pub pkt_ptr: *mut std::ffi::c_void,
    pub dest: u16,
    pub length: usize,
    pub priority: u8,
}

pub struct TdmaQueue {
    queue: BTreeMap<u64, (PacketEntry, MetaInfo)>,
    dest_queue: HashMap<u16, BTreeMap<u64, ()>>,
    pub queue_depth: u16,
    pub fragment: bool,
    counter: u64,
    pub current_bytes: usize,
    pub is_control: bool,
    pub aggregate: bool,
}

impl TdmaQueue {
    pub fn new() -> Self {
        Self {
            queue: BTreeMap::new(),
            dest_queue: HashMap::new(),
            queue_depth: 0,
            fragment: false,
            counter: 0,
            current_bytes: 0,
            is_control: false,
            aggregate: false,
        }
    }

    pub fn enqueue(
        &mut self,
        pkt_ptr: *mut std::ffi::c_void,
        dest: u16,
        length: usize,
        priority: u8,
        dropped_pkt: *mut *mut std::ffi::c_void,
    ) -> bool {
        let mut dropped = false;

        if self.queue.len() == self.queue_depth as usize {
            let mut drop_key = None;
            for (&k, v) in self.queue.iter() {
                if v.1.index == 0 {
                    drop_key = Some(k);
                    break;
                }
            }
            let drop_key = drop_key.unwrap_or_else(|| *self.queue.keys().next().unwrap());

            let (entry, meta) = self.queue.remove(&drop_key).unwrap();
            self.dest_queue
                .get_mut(&entry.dest)
                .unwrap()
                .remove(&drop_key);
            self.current_bytes -= entry.length - meta.offset;
            unsafe { *dropped_pkt = entry.pkt_ptr };
            dropped = true;
        }

        let entry = PacketEntry {
            pkt_ptr,
            dest,
            length,
            priority,
        };
        let meta = MetaInfo {
            index: 0,
            offset: 0,
        };
        self.queue.insert(self.counter, (entry, meta));
        self.dest_queue
            .entry(dest)
            .or_default()
            .insert(self.counter, ());
        self.current_bytes += length;
        self.counter += 1;

        dropped
    }

    pub fn dequeue_impl(
        &mut self,
        requested_bytes: usize,
        destination: u16,
        b_drop: bool,
        queue_index: u8,
    ) -> (Vec<DequeueAction>, usize) {
        let mut total_bytes = 0;
        let mut actions = Vec::new();

        while total_bytes <= requested_bytes {
            let mut selected_key = None;
            if destination != 0 {
                if let Some(dq) = self.dest_queue.get(&destination) {
                    if let Some((&k, _)) = dq.iter().next() {
                        selected_key = Some(k);
                    }
                }
            } else {
                if let Some((&k, _)) = self.queue.iter().next() {
                    selected_key = Some(k);
                }
            }

            let k = match selected_key {
                Some(k) => k,
                None => break,
            };

            let (entry, meta) = self.queue.get_mut(&k).unwrap();
            let pkt_len = entry.length;
            let rem_len = pkt_len - meta.offset;

            if rem_len <= requested_bytes - total_bytes {
                let is_frag = meta.offset > 0;
                let act = DequeueAction {
                    action_type: if is_frag { 2 } else { 1 },
                    queue_index,
                    pkt_ptr: entry.pkt_ptr,
                    dest: entry.dest,
                    priority: entry.priority,
                    seq: k,
                    fragment_index: meta.index,
                    fragment_offset: meta.offset,
                    fragment_size: rem_len,
                    is_control: self.is_control,
                    more_fragments: false,
                };
                total_bytes += rem_len;
                actions.push(act);

                let dest = entry.dest;
                self.queue.remove(&k);
                self.dest_queue.get_mut(&dest).unwrap().remove(&k);

                if !self.aggregate {
                    break;
                }
            } else {
                if self.fragment {
                    let amount = requested_bytes - total_bytes;
                    let act = DequeueAction {
                        action_type: 2,
                        queue_index,
                        pkt_ptr: entry.pkt_ptr,
                        dest: entry.dest,
                        priority: entry.priority,
                        seq: k,
                        fragment_index: meta.index,
                        fragment_offset: meta.offset,
                        fragment_size: amount,
                        is_control: self.is_control,
                        more_fragments: true,
                    };
                    total_bytes += amount;
                    actions.push(act);

                    meta.offset += amount;
                    meta.index += 1;
                    break;
                } else {
                    let components_empty = !actions.iter().any(|a| a.action_type != 0);
                    if b_drop && components_empty {
                        self.current_bytes -= pkt_len;
                        let act = DequeueAction {
                            action_type: 0,
                            queue_index,
                            pkt_ptr: entry.pkt_ptr,
                            dest: entry.dest,
                            priority: entry.priority,
                            seq: 0,
                            fragment_index: 0,
                            fragment_offset: 0,
                            fragment_size: 0,
                            is_control: false,
                            more_fragments: false,
                        };
                        actions.push(act);

                        let dest = entry.dest;
                        self.queue.remove(&k);
                        self.dest_queue.get_mut(&dest).unwrap().remove(&k);
                    } else {
                        break;
                    }
                }
            }
        }

        self.current_bytes -= total_bytes;
        (actions, total_bytes)
    }
}

#[no_mangle]
pub extern "C" fn tdma_queue_new() -> *mut TdmaQueue {
    Box::into_raw(Box::new(TdmaQueue::new()))
}

#[no_mangle]
pub extern "C" fn tdma_queue_free(q: *mut TdmaQueue) {
    if !q.is_null() {
        unsafe { drop(Box::from_raw(q)) };
    }
}

#[no_mangle]
pub extern "C" fn tdma_queue_initialize(
    q: *mut TdmaQueue,
    queue_depth: u16,
    fragment: bool,
    aggregate: bool,
    is_control: bool,
) {
    let q = unsafe { &mut *q };
    q.queue_depth = queue_depth;
    q.fragment = fragment;
    q.aggregate = aggregate;
    q.is_control = is_control;
}

#[repr(C)]
pub struct EnqueueResult {
    pub dropped_pkt: *mut std::ffi::c_void,
    pub dropped: bool,
}

#[no_mangle]
pub extern "C" fn tdma_queue_enqueue(
    q: *mut TdmaQueue,
    pkt_ptr: *mut std::ffi::c_void,
    dest: u16,
    length: usize,
    priority: u8,
) -> EnqueueResult {
    let q = unsafe { &mut *q };
    let mut dropped_pkt = std::ptr::null_mut();
    let dropped = q.enqueue(pkt_ptr, dest, length, priority, &mut dropped_pkt);
    EnqueueResult {
        dropped_pkt,
        dropped,
    }
}

#[repr(C)]
pub struct DequeueAction {
    pub action_type: u8, // 0 = DROP, 1 = DEQUEUE_FULL, 2 = DEQUEUE_FRAGMENT
    pub queue_index: u8,
    pub pkt_ptr: *mut std::ffi::c_void,
    pub dest: u16,
    pub priority: u8,
    pub seq: u64,
    pub fragment_index: usize,
    pub fragment_offset: usize,
    pub fragment_size: usize,
    pub is_control: bool,
    pub more_fragments: bool,
}

#[repr(C)]
pub struct DequeueResult {
    pub actions: *mut DequeueAction,
    pub num_actions: usize,
    pub total_bytes: usize,
}

#[no_mangle]
pub extern "C" fn tdma_queue_dequeue(
    q: *mut TdmaQueue,
    requested_bytes: usize,
    destination: u16,
    b_drop: bool,
    res: *mut DequeueResult,
) {
    let q = unsafe { &mut *q };
    let (mut actions, total_bytes) = q.dequeue_impl(requested_bytes, destination, b_drop, 0);
    actions.shrink_to_fit();
    unsafe {
        (*res).actions = actions.as_mut_ptr();
        (*res).num_actions = actions.len();
        (*res).total_bytes = total_bytes;
    }
    std::mem::forget(actions);
}

#[no_mangle]
pub extern "C" fn tdma_queue_free_dequeue_result(res: *mut DequeueResult) {
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

#[no_mangle]
pub extern "C" fn tdma_queue_get_status(
    q: *const TdmaQueue,
    packets: *mut usize,
    bytes: *mut usize,
) {
    let q = unsafe { &*q };
    unsafe {
        *packets = q.queue.len();
        *bytes = q.current_bytes;
    }
}

// BasicQueueManager

pub struct BasicQueueManager {
    queues: [TdmaQueue; 5],
    aggregation_enable: bool,
    fragmentation_enable: bool,
    strict_dequeue_enable: bool,
    aggregation_slot_threshold: f64,
}

#[no_mangle]
pub extern "C" fn basic_queue_manager_new() -> *mut BasicQueueManager {
    Box::into_raw(Box::new(BasicQueueManager {
        queues: [
            TdmaQueue::new(),
            TdmaQueue::new(),
            TdmaQueue::new(),
            TdmaQueue::new(),
            TdmaQueue::new(),
        ],
        aggregation_enable: true,
        fragmentation_enable: true,
        strict_dequeue_enable: false,
        aggregation_slot_threshold: 90.0,
    }))
}

#[no_mangle]
pub extern "C" fn basic_queue_manager_free(m: *mut BasicQueueManager) {
    if !m.is_null() {
        unsafe { drop(Box::from_raw(m)) };
    }
}

#[no_mangle]
pub extern "C" fn basic_queue_manager_set_config(
    m: *mut BasicQueueManager,
    queue_depth: u16,
    aggregation_enable: bool,
    fragmentation_enable: bool,
    strict_dequeue_enable: bool,
    aggregation_slot_threshold: f64,
) {
    let m = unsafe { &mut *m };
    m.aggregation_enable = aggregation_enable;
    m.fragmentation_enable = fragmentation_enable;
    m.strict_dequeue_enable = strict_dequeue_enable;
    m.aggregation_slot_threshold = aggregation_slot_threshold;

    for i in 0..4 {
        m.queues[i].queue_depth = queue_depth;
        m.queues[i].fragment = fragmentation_enable;
        m.queues[i].aggregate = aggregation_enable;
        m.queues[i].is_control = false;
    }
    m.queues[4].queue_depth = queue_depth;
    m.queues[4].fragment = fragmentation_enable;
    m.queues[4].aggregate = aggregation_enable;
    m.queues[4].is_control = true;
}

#[no_mangle]
pub extern "C" fn basic_queue_manager_enqueue(
    m: *mut BasicQueueManager,
    u8_queue_index: u8,
    pkt_ptr: *mut std::ffi::c_void,
    dest: u16,
    length: usize,
    priority: u8,
) -> EnqueueResult {
    let m = unsafe { &mut *m };
    if (u8_queue_index as usize) < 5 {
        let q = &mut m.queues[u8_queue_index as usize];
        let mut dropped_pkt = std::ptr::null_mut();
        let dropped = q.enqueue(pkt_ptr, dest, length, priority, &mut dropped_pkt);
        EnqueueResult {
            dropped_pkt,
            dropped,
        }
    } else {
        EnqueueResult {
            dropped_pkt: std::ptr::null_mut(),
            dropped: false,
        }
    }
}

#[no_mangle]
pub extern "C" fn basic_queue_manager_dequeue(
    m: *mut BasicQueueManager,
    u8_queue_index: u8,
    requested_bytes: usize,
    destination: u16,
    res: *mut DequeueResult,
) {
    let m = unsafe { &mut *m };
    let mut actions = Vec::new();
    let mut total_length = 0;

    if (u8_queue_index as usize) < 5 {
        let (mut q_actions, q_len) = m.queues[u8_queue_index as usize].dequeue_impl(
            requested_bytes,
            destination,
            true,
            u8_queue_index,
        );
        total_length += q_len;
        actions.append(&mut q_actions);

        let threshold = (requested_bytes as f64 * m.aggregation_slot_threshold / 100.0) as usize;

        if !m.strict_dequeue_enable {
            let mut i = 5;
            while (total_length == 0 || (total_length > 0 && m.aggregation_enable))
                && total_length <= threshold
                && i > 0
            {
                if (i - 1) != u8_queue_index {
                    let (mut o_actions, o_len) = m.queues[(i - 1) as usize].dequeue_impl(
                        requested_bytes - total_length,
                        destination,
                        false,
                        i - 1,
                    );
                    if o_len > 0 {
                        total_length += o_len;
                        actions.append(&mut o_actions);
                    }
                }
                i -= 1;
            }
        }
    }

    actions.shrink_to_fit();
    unsafe {
        (*res).actions = actions.as_mut_ptr();
        (*res).num_actions = actions.len();
        (*res).total_bytes = total_length;
    }
    std::mem::forget(actions);
}

#[repr(C)]
pub struct QueueStatus {
    pub packets: usize,
    pub bytes: usize,
}

#[no_mangle]
pub extern "C" fn basic_queue_manager_get_status(
    m: *const BasicQueueManager,
    statuses: *mut QueueStatus,
) {
    let m = unsafe { &*m };
    for i in 0..5 {
        let q = &m.queues[i];
        unsafe {
            (*statuses.add(i)).packets = q.queue.len();
            (*statuses.add(i)).bytes = q.current_bytes;
        }
    }
}
