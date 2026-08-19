use std::collections::VecDeque;
use std::os::raw::c_void;

const MAX_ACCESS_CATEGORIES: usize = 4;
const QUEUE_SIZE_DEFAULT: u8 = 255;
const MAX_PACKET_SIZE: u16 = 0xFFFF;

extern "C" {
    fn ieee80211abg_downstream_queue_stat_add(stat: *mut c_void, val: u32);
    fn ieee80211abg_downstream_queue_stat_set(stat: *mut c_void, val: u32);
    fn ieee80211abg_downstream_queue_stat_get(stat: *mut c_void) -> u32;
    fn ieee80211abg_downstream_queue_free_entry(entry: *mut c_void);
}

pub struct AccessCategory {
    pub max_queue_capacity: u8,
    pub max_packet_size: u16,
    pub num_packet_overflow: usize,
    pub queue: VecDeque<*mut c_void>,
    pub num_unicast_packets_too_large: *mut c_void,
    pub num_unicast_bytes_too_large: *mut c_void,
    pub num_broadcast_packets_too_large: *mut c_void,
    pub num_broadcast_bytes_too_large: *mut c_void,
    pub num_high_water_mark: *mut c_void,
    pub num_high_water_max: *mut c_void,
}

impl Default for AccessCategory {
    fn default() -> Self {
        Self {
            max_queue_capacity: QUEUE_SIZE_DEFAULT,
            max_packet_size: MAX_PACKET_SIZE,
            num_packet_overflow: 0,
            queue: VecDeque::new(),
            num_unicast_packets_too_large: std::ptr::null_mut(),
            num_unicast_bytes_too_large: std::ptr::null_mut(),
            num_broadcast_packets_too_large: std::ptr::null_mut(),
            num_broadcast_bytes_too_large: std::ptr::null_mut(),
            num_high_water_mark: std::ptr::null_mut(),
            num_high_water_max: std::ptr::null_mut(),
        }
    }
}

pub struct FfiDownstreamQueue {
    pub id: u16,
    pub num_active_categories: u8,
    pub categories: [AccessCategory; MAX_ACCESS_CATEGORIES],
    pub num_unicast_packets_unsupported: *mut c_void,
    pub num_unicast_bytes_unsupported: *mut c_void,
    pub num_broadcast_packets_unsupported: *mut c_void,
    pub num_broadcast_bytes_unsupported: *mut c_void,
}

#[no_mangle]
pub unsafe extern "C" fn ieee80211abg_downstream_queue_create(id: u16) -> *mut FfiDownstreamQueue {
    let queue = Box::new(FfiDownstreamQueue {
        id,
        num_active_categories: MAX_ACCESS_CATEGORIES as u8,
        categories: Default::default(),
        num_unicast_packets_unsupported: std::ptr::null_mut(),
        num_unicast_bytes_unsupported: std::ptr::null_mut(),
        num_broadcast_packets_unsupported: std::ptr::null_mut(),
        num_broadcast_bytes_unsupported: std::ptr::null_mut(),
    });
    Box::into_raw(queue)
}

#[no_mangle]
pub unsafe extern "C" fn ieee80211abg_downstream_queue_destroy(queue: *mut FfiDownstreamQueue) {
    if queue.is_null() { return; }
    let mut q = Box::from_raw(queue);
    for cat in q.categories.iter_mut() {
        for entry in cat.queue.drain(..) {
            ieee80211abg_downstream_queue_free_entry(entry);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn ieee80211abg_downstream_queue_set_stats(
    queue: *mut FfiDownstreamQueue,
    p_num_unicast_packets_unsupported: *mut c_void,
    p_num_unicast_bytes_unsupported: *mut c_void,
    p_num_broadcast_packets_unsupported: *mut c_void,
    p_num_broadcast_bytes_unsupported: *mut c_void,
) {
    let q = &mut *queue;
    q.num_unicast_packets_unsupported = p_num_unicast_packets_unsupported;
    q.num_unicast_bytes_unsupported = p_num_unicast_bytes_unsupported;
    q.num_broadcast_packets_unsupported = p_num_broadcast_packets_unsupported;
    q.num_broadcast_bytes_unsupported = p_num_broadcast_bytes_unsupported;
}

#[no_mangle]
pub unsafe extern "C" fn ieee80211abg_downstream_queue_set_category_stats(
    queue: *mut FfiDownstreamQueue,
    category: u8,
    p_num_unicast_packets_too_large: *mut c_void,
    p_num_unicast_bytes_too_large: *mut c_void,
    p_num_broadcast_packets_too_large: *mut c_void,
    p_num_broadcast_bytes_too_large: *mut c_void,
    p_num_high_water_mark: *mut c_void,
    p_num_high_water_max: *mut c_void,
) {
    let q = &mut *queue;
    if (category as usize) < MAX_ACCESS_CATEGORIES {
        let cat = &mut q.categories[category as usize];
        cat.num_unicast_packets_too_large = p_num_unicast_packets_too_large;
        cat.num_unicast_bytes_too_large = p_num_unicast_bytes_too_large;
        cat.num_broadcast_packets_too_large = p_num_broadcast_packets_too_large;
        cat.num_broadcast_bytes_too_large = p_num_broadcast_bytes_too_large;
        cat.num_high_water_mark = p_num_high_water_mark;
        cat.num_high_water_max = p_num_high_water_max;
    }
}

#[no_mangle]
pub unsafe extern "C" fn ieee80211abg_downstream_queue_enqueue(
    queue: *mut FfiDownstreamQueue,
    entry_ptr: *mut c_void,
    category: u8,
    length: usize,
    destination: u16,
    dropped_entries: *mut *mut c_void,
    num_dropped: *mut usize,
    max_dropped: usize,
) {
    let q = &mut *queue;
    *num_dropped = 0;
    
    let mut drop = |ptr: *mut c_void| {
        if *num_dropped < max_dropped {
            *dropped_entries.add(*num_dropped) = ptr;
            *num_dropped += 1;
        } else {
            ieee80211abg_downstream_queue_free_entry(ptr);
        }
    };

    let is_broadcast = destination == 0xFFFF;
    
    if category < q.num_active_categories {
        let cat = &mut q.categories[category as usize];
        if cat.max_queue_capacity == 0 {
            drop(entry_ptr);
        } else if cat.max_packet_size != 0 && length > cat.max_packet_size as usize {
            if is_broadcast {
                ieee80211abg_downstream_queue_stat_add(cat.num_broadcast_packets_too_large, 1);
                ieee80211abg_downstream_queue_stat_add(cat.num_broadcast_bytes_too_large, length as u32);
            } else {
                ieee80211abg_downstream_queue_stat_add(cat.num_unicast_packets_too_large, 1);
                ieee80211abg_downstream_queue_stat_add(cat.num_unicast_bytes_too_large, length as u32);
            }
            drop(entry_ptr);
        } else {
            while cat.queue.len() >= cat.max_queue_capacity as usize {
                if let Some(front) = cat.queue.pop_front() {
                    drop(front);
                }
            }
            cat.queue.push_back(entry_ptr);
            
            let current_len = cat.queue.len() as u32;
            if current_len > ieee80211abg_downstream_queue_stat_get(cat.num_high_water_mark) {
                ieee80211abg_downstream_queue_stat_set(cat.num_high_water_mark, current_len);
                ieee80211abg_downstream_queue_stat_set(cat.num_high_water_max, cat.max_queue_capacity as u32);
            }
        }
    } else {
        if is_broadcast {
            ieee80211abg_downstream_queue_stat_add(q.num_broadcast_packets_unsupported, 1);
            ieee80211abg_downstream_queue_stat_add(q.num_broadcast_bytes_unsupported, length as u32);
        } else {
            ieee80211abg_downstream_queue_stat_add(q.num_unicast_packets_unsupported, 1);
            ieee80211abg_downstream_queue_stat_add(q.num_unicast_bytes_unsupported, length as u32);
        }
        drop(entry_ptr);
    }
}

#[no_mangle]
pub unsafe extern "C" fn ieee80211abg_downstream_queue_dequeue(
    queue: *mut FfiDownstreamQueue,
) -> *mut c_void {
    let q = &mut *queue;
    for i in (0..q.num_active_categories as usize).rev() {
        if let Some(front) = q.categories[i].queue.pop_front() {
            return front;
        }
    }
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn ieee80211abg_downstream_queue_set_max_capacity(
    queue: *mut FfiDownstreamQueue,
    max_entries: usize,
) {
    let q = &mut *queue;
    for i in 0..q.num_active_categories as usize {
        let cat = &mut q.categories[i];
        while cat.queue.len() > max_entries {
            if let Some(front) = cat.queue.pop_front() {
                ieee80211abg_downstream_queue_free_entry(front);
            }
        }
        cat.max_queue_capacity = max_entries as u8;
    }
}

#[no_mangle]
pub unsafe extern "C" fn ieee80211abg_downstream_queue_set_max_capacity_for_category(
    queue: *mut FfiDownstreamQueue,
    max_entries: usize,
    category: u8,
) {
    let q = &mut *queue;
    if category < q.num_active_categories {
        let cat = &mut q.categories[category as usize];
        while cat.queue.len() > max_entries {
            if let Some(front) = cat.queue.pop_front() {
                ieee80211abg_downstream_queue_free_entry(front);
            }
        }
        cat.max_queue_capacity = max_entries as u8;
    }
}

#[no_mangle]
pub unsafe extern "C" fn ieee80211abg_downstream_queue_set_max_entry_size(
    queue: *mut FfiDownstreamQueue,
    max_entry_size: usize,
) {
    let q = &mut *queue;
    for i in 0..q.num_active_categories as usize {
        q.categories[i].max_packet_size = max_entry_size as u16;
    }
}

#[no_mangle]
pub unsafe extern "C" fn ieee80211abg_downstream_queue_set_max_entry_size_for_category(
    queue: *mut FfiDownstreamQueue,
    max_entry_size: usize,
    category: u8,
) {
    let q = &mut *queue;
    if category < q.num_active_categories {
        q.categories[category as usize].max_packet_size = max_entry_size as u16;
    }
}

#[no_mangle]
pub unsafe extern "C" fn ieee80211abg_downstream_queue_get_max_capacity(
    queue: *const FfiDownstreamQueue,
) -> usize {
    let q = &*queue;
    let mut total = 0;
    for i in 0..q.num_active_categories as usize {
        total += q.categories[i].max_queue_capacity as usize;
    }
    total
}

#[no_mangle]
pub unsafe extern "C" fn ieee80211abg_downstream_queue_get_max_capacity_for_category(
    queue: *const FfiDownstreamQueue,
    category: u8,
) -> usize {
    let q = &*queue;
    if category < q.num_active_categories {
        q.categories[category as usize].max_queue_capacity as usize
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn ieee80211abg_downstream_queue_get_depth(
    queue: *const FfiDownstreamQueue,
) -> usize {
    let q = &*queue;
    let mut total = 0;
    for i in 0..q.num_active_categories as usize {
        total += q.categories[i].queue.len();
    }
    total
}

#[no_mangle]
pub unsafe extern "C" fn ieee80211abg_downstream_queue_get_depth_for_category(
    queue: *const FfiDownstreamQueue,
    category: u8,
) -> usize {
    let q = &*queue;
    if category < q.num_active_categories {
        q.categories[category as usize].queue.len()
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn ieee80211abg_downstream_queue_get_available_space(
    queue: *const FfiDownstreamQueue,
) -> usize {
    let q = &*queue;
    let mut total = 0;
    for i in 0..q.num_active_categories as usize {
        let cat = &q.categories[i];
        if cat.max_queue_capacity as usize >= cat.queue.len() {
            total += cat.max_queue_capacity as usize - cat.queue.len();
        }
    }
    total
}

#[no_mangle]
pub unsafe extern "C" fn ieee80211abg_downstream_queue_get_available_space_for_category(
    queue: *const FfiDownstreamQueue,
    category: u8,
) -> usize {
    let q = &*queue;
    if category < q.num_active_categories {
        let cat = &q.categories[category as usize];
        if cat.max_queue_capacity as usize >= cat.queue.len() {
            cat.max_queue_capacity as usize - cat.queue.len()
        } else {
            0
        }
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn ieee80211abg_downstream_queue_get_num_overflow(
    queue: *mut FfiDownstreamQueue,
    clear: bool,
) -> usize {
    let q = &mut *queue;
    let mut total = 0;
    for i in 0..q.num_active_categories as usize {
        total += q.categories[i].num_packet_overflow;
        if clear {
            q.categories[i].num_packet_overflow = 0;
        }
    }
    total
}

#[no_mangle]
pub unsafe extern "C" fn ieee80211abg_downstream_queue_get_num_overflow_for_category(
    queue: *mut FfiDownstreamQueue,
    category: u8,
    clear: bool,
) -> usize {
    let q = &mut *queue;
    if category < q.num_active_categories {
        let res = q.categories[category as usize].num_packet_overflow;
        if clear {
            q.categories[category as usize].num_packet_overflow = 0;
        }
        res
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn ieee80211abg_downstream_queue_set_categories(
    queue: *mut FfiDownstreamQueue,
    num_categories: u8,
) {
    let q = &mut *queue;
    if num_categories > 0 && (num_categories as usize) <= MAX_ACCESS_CATEGORIES {
        for i in num_categories..q.num_active_categories {
            let idx = q.num_active_categories - i;
            for entry in q.categories[idx as usize].queue.drain(..) {
                ieee80211abg_downstream_queue_free_entry(entry);
            }
        }
        q.num_active_categories = num_categories;
    }
}
