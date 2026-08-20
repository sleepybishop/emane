use std::collections::HashMap;
use std::ffi::c_void;

extern "C" {
    fn emane_tdma_aggregation_table_add_row(
        p_table: *mut c_void,
        key: u64,
        components: u64,
        count: u64,
    );
    fn emane_tdma_aggregation_table_set_cell(p_table: *mut c_void, key: u64, count: u64);
}

pub struct AggregationStatusPublisher {
    histogram: HashMap<u64, u64>,
}

impl AggregationStatusPublisher {
    pub fn new() -> Self {
        Self {
            histogram: HashMap::new(),
        }
    }

    pub fn update(&mut self, p_table: *mut c_void, components_size: u64) {
        let count = self.histogram.entry(components_size).or_insert(0);
        if *count == 0 {
            *count = 1;
            unsafe {
                emane_tdma_aggregation_table_add_row(p_table, components_size, components_size, 1);
            }
        } else {
            *count += 1;
            unsafe {
                emane_tdma_aggregation_table_set_cell(p_table, components_size, *count);
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_aggregation_publisher_create() -> *mut c_void {
    Box::into_raw(Box::new(AggregationStatusPublisher::new())) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_aggregation_publisher_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr as *mut AggregationStatusPublisher));
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_aggregation_publisher_update(
    ptr: *mut c_void,
    p_table: *mut c_void,
    components_size: u64,
) {
    if p_table.is_null() || ptr.is_null() {
        return;
    }
    let publisher = unsafe { &mut *(ptr as *mut AggregationStatusPublisher) };
    publisher.update(p_table, components_size);
}

extern "C" {
    fn emane_tdma_packet_table_add_row(
        p_table: *mut c_void,
        key: u16,
        num_cols: u64,
        values: *const u64,
    );
    fn emane_tdma_packet_table_set_cell(p_table: *mut c_void, key: u16, column: usize, value: u64);
}

#[derive(Clone, Default)]
struct PacketAcceptInfo {
    tx_bytes: u64,
    rx_bytes: u64,
}

#[derive(Clone, Default)]
struct PacketDropInfo {
    sinr: u64,
    reg_id: u64,
    dst_mac: u64,
    queue_overflow: u64,
    bad_control: u64,
    bad_spectrum_query: u64,
    flow_control: u64,
    too_big: u64,
    too_long: u64,
    freq: u64,
    slot_error: u64,
    miss_fragment: u64,
}

pub struct PacketStatusPublisher {
    broadcast_accept_infos: [HashMap<u16, PacketAcceptInfo>; 5],
    broadcast_drop_infos: [HashMap<u16, PacketDropInfo>; 5],
    unicast_accept_infos: [HashMap<u16, PacketAcceptInfo>; 5],
    unicast_drop_infos: [HashMap<u16, PacketDropInfo>; 5],

    broadcast_accept_tables: [*mut c_void; 5],
    broadcast_drop_tables: [*mut c_void; 5],
    unicast_accept_tables: [*mut c_void; 5],
    unicast_drop_tables: [*mut c_void; 5],
}

impl PacketStatusPublisher {
    pub fn new() -> Self {
        Self {
            broadcast_accept_infos: Default::default(),
            broadcast_drop_infos: Default::default(),
            unicast_accept_infos: Default::default(),
            unicast_drop_infos: Default::default(),
            broadcast_accept_tables: [std::ptr::null_mut(); 5],
            broadcast_drop_tables: [std::ptr::null_mut(); 5],
            unicast_accept_tables: [std::ptr::null_mut(); 5],
            unicast_drop_tables: [std::ptr::null_mut(); 5],
        }
    }

    pub fn clear(&mut self, table_type: i32, queue_index: usize) {
        if queue_index >= 5 {
            return;
        }
        match table_type {
            0 => self.broadcast_accept_infos[queue_index].clear(),
            1 => self.broadcast_drop_infos[queue_index].clear(),
            2 => self.unicast_accept_infos[queue_index].clear(),
            3 => self.unicast_drop_infos[queue_index].clear(),
            _ => {}
        }
    }

    pub fn register(
        &mut self,
        b_acc: *const *mut c_void,
        b_drop: *const *mut c_void,
        u_acc: *const *mut c_void,
        u_drop: *const *mut c_void,
    ) {
        unsafe {
            for i in 0..5 {
                self.broadcast_accept_tables[i] = *b_acc.add(i);
                self.broadcast_drop_tables[i] = *b_drop.add(i);
                self.unicast_accept_tables[i] = *u_acc.add(i);
                self.unicast_drop_tables[i] = *u_drop.add(i);
            }
        }
    }

    fn priority_to_queue(priority: u8) -> usize {
        match priority {
            0 => 0,
            1 | 2 => 1,
            3 | 4 => 2,
            5 => 3,
            6 | 7 => 4,
            _ => 0,
        }
    }

    pub fn inbound(&mut self, src: u16, dst: u16, priority: u8, size: usize, action: i32) {
        let q_idx = Self::priority_to_queue(priority);
        let is_bcast = dst == 0xFFFF; // NEM_BROADCAST_MAC_ADDRESS

        if action == 1 {
            // ACCEPT_GOOD
            let (infos, tables) = if is_bcast {
                (
                    &mut self.broadcast_accept_infos[q_idx],
                    self.broadcast_accept_tables[q_idx],
                )
            } else {
                (
                    &mut self.unicast_accept_infos[q_idx],
                    self.unicast_accept_tables[q_idx],
                )
            };
            let info = infos.entry(src).or_insert_with(|| {
                unsafe {
                    let vals: [u64; 3] = [src as u64, 0, 0];
                    emane_tdma_packet_table_add_row(tables, src, 3, vals.as_ptr());
                }
                PacketAcceptInfo::default()
            });
            info.rx_bytes += size as u64;
            unsafe {
                emane_tdma_packet_table_set_cell(tables, src, 2, info.rx_bytes);
            }
        } else {
            let (infos, tables) = if is_bcast {
                (
                    &mut self.broadcast_drop_infos[q_idx],
                    self.broadcast_drop_tables[q_idx],
                )
            } else {
                (
                    &mut self.unicast_drop_infos[q_idx],
                    self.unicast_drop_tables[q_idx],
                )
            };
            let info = infos.entry(src).or_insert_with(|| {
                unsafe {
                    let vals: [u64; 13] = [src as u64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
                    emane_tdma_packet_table_add_row(tables, src, 13, vals.as_ptr());
                }
                PacketDropInfo::default()
            });

            match action {
                2 => {
                    info.bad_control += size as u64;
                    unsafe {
                        emane_tdma_packet_table_set_cell(tables, src, 5, info.bad_control);
                    }
                } // DROP_BAD_CONTROL
                3 => {
                    info.slot_error += size as u64;
                    unsafe {
                        emane_tdma_packet_table_set_cell(tables, src, 11, info.slot_error);
                    }
                } // DROP_SLOT_ERROR
                4 => {
                    info.miss_fragment += size as u64;
                    unsafe {
                        emane_tdma_packet_table_set_cell(tables, src, 12, info.miss_fragment);
                    }
                } // DROP_MISS_FRAGMENT
                5 => {
                    info.bad_spectrum_query += size as u64;
                    unsafe {
                        emane_tdma_packet_table_set_cell(tables, src, 6, info.bad_spectrum_query);
                    }
                } // DROP_SPECTRUM_SERVICE
                6 => {
                    info.sinr += size as u64;
                    unsafe {
                        emane_tdma_packet_table_set_cell(tables, src, 1, info.sinr);
                    }
                } // DROP_SINR
                7 => {
                    info.reg_id += size as u64;
                    unsafe {
                        emane_tdma_packet_table_set_cell(tables, src, 2, info.reg_id);
                    }
                } // DROP_REGISTRATION_ID
                8 => {
                    info.dst_mac += size as u64;
                    unsafe {
                        emane_tdma_packet_table_set_cell(tables, src, 3, info.dst_mac);
                    }
                } // DROP_DESTINATION_MAC
                9 => {
                    info.too_long += size as u64;
                    unsafe {
                        emane_tdma_packet_table_set_cell(tables, src, 9, info.too_long);
                    }
                } // DROP_TOO_LONG
                10 => {
                    info.freq += size as u64;
                    unsafe {
                        emane_tdma_packet_table_set_cell(tables, src, 10, info.freq);
                    }
                } // DROP_FREQUENCY
                _ => {}
            }
        }
    }

    pub fn outbound(&mut self, src: u16, dst: u16, priority: u8, size: usize, action: i32) {
        let q_idx = Self::priority_to_queue(priority);
        let is_bcast = dst == 0xFFFF; // NEM_BROADCAST_MAC_ADDRESS

        if action == 1 {
            // ACCEPT_GOOD
            let (infos, tables) = if is_bcast {
                (
                    &mut self.broadcast_accept_infos[q_idx],
                    self.broadcast_accept_tables[q_idx],
                )
            } else {
                (
                    &mut self.unicast_accept_infos[q_idx],
                    self.unicast_accept_tables[q_idx],
                )
            };
            let info = infos.entry(src).or_insert_with(|| {
                unsafe {
                    let vals: [u64; 3] = [src as u64, 0, 0];
                    emane_tdma_packet_table_add_row(tables, src, 3, vals.as_ptr());
                }
                PacketAcceptInfo::default()
            });
            info.tx_bytes += size as u64;
            unsafe {
                emane_tdma_packet_table_set_cell(tables, src, 1, info.tx_bytes);
            }
        } else {
            let (infos, tables) = if is_bcast {
                (
                    &mut self.broadcast_drop_infos[q_idx],
                    self.broadcast_drop_tables[q_idx],
                )
            } else {
                (
                    &mut self.unicast_drop_infos[q_idx],
                    self.unicast_drop_tables[q_idx],
                )
            };
            let info = infos.entry(src).or_insert_with(|| {
                unsafe {
                    let vals: [u64; 13] = [src as u64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
                    emane_tdma_packet_table_add_row(tables, src, 13, vals.as_ptr());
                }
                PacketDropInfo::default()
            });

            match action {
                2 => {
                    info.too_big += size as u64;
                    unsafe {
                        emane_tdma_packet_table_set_cell(tables, src, 8, info.too_big);
                    }
                } // DROP_TOO_BIG
                3 => {
                    info.queue_overflow += size as u64;
                    unsafe {
                        emane_tdma_packet_table_set_cell(tables, src, 4, info.queue_overflow);
                    }
                } // DROP_OVERFLOW
                4 => {
                    info.flow_control += size as u64;
                    unsafe {
                        emane_tdma_packet_table_set_cell(tables, src, 7, info.flow_control);
                    }
                } // DROP_FLOW_CONTROL
                _ => {}
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_packet_publisher_create() -> *mut c_void {
    Box::into_raw(Box::new(PacketStatusPublisher::new())) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_packet_publisher_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr as *mut PacketStatusPublisher));
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_packet_publisher_clear(
    ptr: *mut c_void,
    table_type: i32,
    queue_index: i32,
) {
    if ptr.is_null() {
        return;
    }
    let publisher = unsafe { &mut *(ptr as *mut PacketStatusPublisher) };
    publisher.clear(table_type, queue_index as usize);
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_packet_publisher_register(
    ptr: *mut c_void,
    _reg: *mut c_void,
    b_acc: *const *mut c_void,
    b_drop: *const *mut c_void,
    u_acc: *const *mut c_void,
    u_drop: *const *mut c_void,
) {
    if ptr.is_null() {
        return;
    }
    let publisher = unsafe { &mut *(ptr as *mut PacketStatusPublisher) };
    publisher.register(b_acc, b_drop, u_acc, u_drop);
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_packet_publisher_inbound(
    ptr: *mut c_void,
    src: u16,
    dst: u16,
    priority: u8,
    size: usize,
    action: i32,
) {
    if ptr.is_null() {
        return;
    }
    let publisher = unsafe { &mut *(ptr as *mut PacketStatusPublisher) };
    publisher.inbound(src, dst, priority, size, action);
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_packet_publisher_outbound(
    ptr: *mut c_void,
    src: u16,
    dst: u16,
    priority: u8,
    size: usize,
    action: i32,
) {
    if ptr.is_null() {
        return;
    }
    let publisher = unsafe { &mut *(ptr as *mut PacketStatusPublisher) };
    publisher.outbound(src, dst, priority, size, action);
}

extern "C" {
    fn emane_tdma_queue_table_add_row_status(
        p_table: *mut c_void,
        key: u8,
        v1: u64,
        v2: u64,
        v3: u64,
        v4: u64,
        v5: u64,
        v6: u64,
        v7: u64,
        v8: u64,
        v9: u64,
    );
    fn emane_tdma_queue_table_add_row_fragment(
        p_table: *mut c_void,
        key: u8,
        v1: u64,
        v2: u64,
        v3: u64,
        v4: u64,
        v5: u64,
        v6: u64,
        v7: u64,
        v8: u64,
        v9: u64,
        v10: u64,
    );
    fn emane_tdma_queue_table_set_cell(p_table: *mut c_void, key: u8, column: usize, value: u64);
    fn emane_tdma_numeric_u64_get(p_numeric: *mut c_void) -> u64;
    fn emane_tdma_numeric_u64_set(p_numeric: *mut c_void, val: u64);
}

#[derive(Clone, Default)]
struct StatusTableInfo {
    enqueued: u64,
    dequeued: u64,
    overflow: u64,
    too_big: u64,
    q0: u64,
    q1: u64,
    q2: u64,
    q3: u64,
    q4: u64,
}

pub struct QueueStatusPublisher {
    status_info: HashMap<u8, StatusTableInfo>,
    fragment_histogram: HashMap<u8, [u64; 10]>,
    depth_info: [u64; 5],

    status_table: *mut c_void,
    fragment_table: *mut c_void,
    hw_numerics: [*mut c_void; 5],
}

impl QueueStatusPublisher {
    pub fn new() -> Self {
        let mut status_info = HashMap::new();
        let mut fragment_histogram = HashMap::new();
        for i in 0..5 {
            status_info.insert(i, Default::default());
            fragment_histogram.insert(i, [0; 10]);
        }
        Self {
            status_info,
            fragment_histogram,
            depth_info: [0; 5],
            status_table: std::ptr::null_mut(),
            fragment_table: std::ptr::null_mut(),
            hw_numerics: [std::ptr::null_mut(); 5],
        }
    }

    pub fn register(
        &mut self,
        status: *mut c_void,
        fragment: *mut c_void,
        hw0: *mut c_void,
        hw1: *mut c_void,
        hw2: *mut c_void,
        hw3: *mut c_void,
        hw4: *mut c_void,
    ) {
        self.status_table = status;
        self.fragment_table = fragment;
        self.hw_numerics = [hw0, hw1, hw2, hw3, hw4];

        for (k, v) in &self.status_info {
            unsafe {
                emane_tdma_queue_table_add_row_status(
                    status, *k, v.enqueued, v.dequeued, v.overflow, v.too_big, v.q0, v.q1, v.q2,
                    v.q3, v.q4,
                );
            }
        }
        for (k, v) in &self.fragment_histogram {
            unsafe {
                emane_tdma_queue_table_add_row_fragment(
                    fragment, *k, v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], v[8], v[9],
                );
            }
        }
    }

    pub fn drop(&mut self, q: u8, reason: i32, count: usize) {
        if let Some(info) = self.status_info.get_mut(&q) {
            if reason == 0 {
                // DROP_OVERFLOW
                info.overflow += count as u64;
                unsafe {
                    emane_tdma_queue_table_set_cell(self.status_table, q, 3, info.overflow);
                }
            } else {
                // DROP_TOOBIG
                info.too_big += count as u64;
                unsafe {
                    emane_tdma_queue_table_set_cell(self.status_table, q, 4, info.too_big);
                }
            }
        }
    }

    pub fn enqueue(&mut self, q: u8) {
        if let Some(info) = self.status_info.get_mut(&q) {
            info.enqueued += 1;
            unsafe {
                emane_tdma_queue_table_set_cell(self.status_table, q, 1, info.enqueued);
            }
        }
        if q < 5 {
            self.depth_info[q as usize] += 1;
            unsafe {
                let hw = emane_tdma_numeric_u64_get(self.hw_numerics[q as usize]);
                if self.depth_info[q as usize] > hw {
                    emane_tdma_numeric_u64_set(
                        self.hw_numerics[q as usize],
                        self.depth_info[q as usize],
                    );
                }
            }
        }
    }

    pub fn dequeue(&mut self, req_q: u8, act_q: u8, more_frag: &[u8], frag_idx: &[usize]) {
        let mut packets_completed = 0;

        for i in 0..more_frag.len() {
            if more_frag[i] == 0 {
                packets_completed += 1;
                let parts = frag_idx[i] + 1;
                let idx = if parts <= 9 { parts } else { 10 };
                if let Some(hist) = self.fragment_histogram.get_mut(&act_q) {
                    hist[idx - 1] += 1;
                    unsafe {
                        emane_tdma_queue_table_set_cell(
                            self.fragment_table,
                            act_q,
                            idx,
                            hist[idx - 1],
                        );
                    }
                }
            }
        }

        if let Some(info) = self.status_info.get_mut(&act_q) {
            info.dequeued += packets_completed;
            unsafe {
                emane_tdma_queue_table_set_cell(self.status_table, act_q, 2, info.dequeued);
            }

            let comp_len = more_frag.len() as u64;
            match req_q {
                0 => {
                    info.q0 += comp_len;
                    unsafe {
                        emane_tdma_queue_table_set_cell(self.status_table, act_q, 5, info.q0);
                    }
                }
                1 => {
                    info.q1 += comp_len;
                    unsafe {
                        emane_tdma_queue_table_set_cell(self.status_table, act_q, 6, info.q1);
                    }
                }
                2 => {
                    info.q2 += comp_len;
                    unsafe {
                        emane_tdma_queue_table_set_cell(self.status_table, act_q, 7, info.q2);
                    }
                }
                3 => {
                    info.q3 += comp_len;
                    unsafe {
                        emane_tdma_queue_table_set_cell(self.status_table, act_q, 8, info.q3);
                    }
                }
                4 => {
                    info.q4 += comp_len;
                    unsafe {
                        emane_tdma_queue_table_set_cell(self.status_table, act_q, 9, info.q4);
                    }
                }
                _ => {}
            }
        }

        if act_q < 5 {
            if self.depth_info[act_q as usize] >= packets_completed {
                self.depth_info[act_q as usize] -= packets_completed;
            } else {
                self.depth_info[act_q as usize] = 0;
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_queue_publisher_create() -> *mut c_void {
    Box::into_raw(Box::new(QueueStatusPublisher::new())) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_queue_publisher_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr as *mut QueueStatusPublisher));
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_queue_publisher_register(
    ptr: *mut c_void,
    _reg: *mut c_void,
    status: *mut c_void,
    frag: *mut c_void,
    hw0: *mut c_void,
    hw1: *mut c_void,
    hw2: *mut c_void,
    hw3: *mut c_void,
    hw4: *mut c_void,
) {
    if ptr.is_null() {
        return;
    }
    let publ = unsafe { &mut *(ptr as *mut QueueStatusPublisher) };
    publ.register(status, frag, hw0, hw1, hw2, hw3, hw4);
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_queue_publisher_drop(
    ptr: *mut c_void,
    q: u8,
    reason: i32,
    count: usize,
) {
    if ptr.is_null() {
        return;
    }
    let publ = unsafe { &mut *(ptr as *mut QueueStatusPublisher) };
    publ.drop(q, reason, count);
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_queue_publisher_enqueue(ptr: *mut c_void, q: u8) {
    if ptr.is_null() {
        return;
    }
    let publ = unsafe { &mut *(ptr as *mut QueueStatusPublisher) };
    publ.enqueue(q);
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_queue_publisher_dequeue(
    ptr: *mut c_void,
    req: u8,
    act: u8,
    num_comp: usize,
    more_frag: *const u8,
    frag_idx: *const usize,
) {
    if ptr.is_null() {
        return;
    }
    let publ = unsafe { &mut *(ptr as *mut QueueStatusPublisher) };
    let m = unsafe { std::slice::from_raw_parts(more_frag, num_comp) };
    let f = unsafe { std::slice::from_raw_parts(frag_idx, num_comp) };
    publ.dequeue(req, act, m, f);
}

extern "C" {
    fn emane_tdma_slot_table_add_row_tx(
        p_table: *mut c_void,
        key: u32,
        v1: u32,
        v2: u32,
        v3: u64,
        v4: u64,
        v5: u64,
        v6: u64,
        v7: u64,
        v8: u64,
        v9: u64,
        v10: u64,
        v11: u64,
        v12: u64,
        v13: u64,
    );
    fn emane_tdma_slot_table_add_row_rx(
        p_table: *mut c_void,
        key: u32,
        v1: u32,
        v2: u32,
        v3: u64,
        v4: u64,
        v5: u64,
        v6: u64,
        v7: u64,
        v8: u64,
        v9: u64,
        v10: u64,
        v11: u64,
        v12: u64,
        v13: u64,
        v14: u64,
        v15: u64,
        v16: u64,
        v17: u64,
    );
    fn emane_tdma_slot_table_set_cell(p_table: *mut c_void, key: u32, column: usize, value: u64);
    fn emane_tdma_numeric_u64_add(p_numeric: *mut c_void, val: u64);
}

#[derive(Clone, Default)]
struct TxSlotInfo {
    valid: u64,
    missed: u64,
    too_big: u64,
    quantile: [u64; 8],
}

#[derive(Clone, Default)]
struct RxSlotInfo {
    valid: u64,
    missed: u64,
    rx_idle: u64,
    rx_tx: u64,
    rx_too_long: u64,
    rx_wrong_freq: u64,
    rx_lock: u64,
    quantile: [u64; 8],
}

pub struct SlotStatusPublisher {
    tx_infos: HashMap<u32, TxSlotInfo>,
    rx_infos: HashMap<u32, RxSlotInfo>,

    tx_table: *mut c_void,
    rx_table: *mut c_void,

    tx_valid: *mut c_void,
    tx_missed: *mut c_void,
    tx_too_big: *mut c_void,

    rx_valid: *mut c_void,
    rx_missed: *mut c_void,
    rx_idle: *mut c_void,
    rx_tx: *mut c_void,
    rx_too_long: *mut c_void,
    rx_wrong_freq: *mut c_void,
    rx_lock: *mut c_void,
}

impl SlotStatusPublisher {
    pub fn new() -> Self {
        Self {
            tx_infos: HashMap::new(),
            rx_infos: HashMap::new(),
            tx_table: std::ptr::null_mut(),
            rx_table: std::ptr::null_mut(),
            tx_valid: std::ptr::null_mut(),
            tx_missed: std::ptr::null_mut(),
            tx_too_big: std::ptr::null_mut(),
            rx_valid: std::ptr::null_mut(),
            rx_missed: std::ptr::null_mut(),
            rx_idle: std::ptr::null_mut(),
            rx_tx: std::ptr::null_mut(),
            rx_too_long: std::ptr::null_mut(),
            rx_wrong_freq: std::ptr::null_mut(),
            rx_lock: std::ptr::null_mut(),
        }
    }

    pub fn register(
        &mut self,
        tx_table: *mut c_void,
        rx_table: *mut c_void,
        tx_valid: *mut c_void,
        tx_missed: *mut c_void,
        tx_too_big: *mut c_void,
        rx_valid: *mut c_void,
        rx_missed: *mut c_void,
        rx_idle: *mut c_void,
        rx_tx: *mut c_void,
        rx_too_long: *mut c_void,
        rx_wrong_freq: *mut c_void,
        rx_lock: *mut c_void,
    ) {
        self.tx_table = tx_table;
        self.rx_table = rx_table;
        self.tx_valid = tx_valid;
        self.tx_missed = tx_missed;
        self.tx_too_big = tx_too_big;
        self.rx_valid = rx_valid;
        self.rx_missed = rx_missed;
        self.rx_idle = rx_idle;
        self.rx_tx = rx_tx;
        self.rx_too_long = rx_too_long;
        self.rx_wrong_freq = rx_wrong_freq;
        self.rx_lock = rx_lock;
    }

    pub fn clear(&mut self) {
        self.tx_infos.clear();
        self.rx_infos.clear();
    }

    fn get_quantile(ratio: f64) -> usize {
        if ratio <= 0.25 {
            0
        } else if ratio <= 0.50 {
            1
        } else if ratio <= 0.75 {
            2
        } else if ratio <= 1.00 {
            3
        } else if ratio <= 1.25 {
            4
        } else if ratio <= 1.50 {
            5
        } else if ratio <= 1.75 {
            6
        } else {
            7
        }
    }

    pub fn update(&mut self, idx: u32, frame: u32, slot: u32, status: i32, ratio: f64) {
        let q = Self::get_quantile(ratio);

        match status {
            0 | 1 | 2 => {
                // TX_GOOD, TX_MISSED, TX_TOOBIG
                let info = self.tx_infos.entry(idx).or_insert_with(|| {
                    unsafe {
                        emane_tdma_slot_table_add_row_tx(
                            self.tx_table,
                            idx,
                            frame,
                            slot,
                            0,
                            0,
                            0,
                            0,
                            0,
                            0,
                            0,
                            0,
                            0,
                            0,
                            0,
                        );
                    }
                    TxSlotInfo::default()
                });

                match status {
                    0 => {
                        info.valid += 1;
                        unsafe {
                            emane_tdma_numeric_u64_add(self.tx_valid, 1);
                            emane_tdma_slot_table_set_cell(self.tx_table, idx, 3, info.valid);
                        }
                    }
                    1 => {
                        info.missed += 1;
                        unsafe {
                            emane_tdma_numeric_u64_add(self.tx_missed, 1);
                            emane_tdma_slot_table_set_cell(self.tx_table, idx, 4, info.missed);
                        }
                    }
                    2 => {
                        info.too_big += 1;
                        unsafe {
                            emane_tdma_numeric_u64_add(self.tx_too_big, 1);
                            emane_tdma_slot_table_set_cell(self.tx_table, idx, 5, info.too_big);
                        }
                    }
                    _ => {}
                }

                info.quantile[q] += 1;
                unsafe {
                    emane_tdma_slot_table_set_cell(self.tx_table, idx, 6 + q, info.quantile[q]);
                }
            }
            3 | 4 | 5 | 6 | 7 | 8 | 9 => {
                let info = self.rx_infos.entry(idx).or_insert_with(|| {
                    unsafe {
                        emane_tdma_slot_table_add_row_rx(
                            self.rx_table,
                            idx,
                            frame,
                            slot,
                            0,
                            0,
                            0,
                            0,
                            0,
                            0,
                            0,
                            0,
                            0,
                            0,
                            0,
                            0,
                            0,
                            0,
                            0,
                        );
                    }
                    RxSlotInfo::default()
                });

                match status {
                    3 => {
                        info.valid += 1;
                        unsafe {
                            emane_tdma_numeric_u64_add(self.rx_valid, 1);
                            emane_tdma_slot_table_set_cell(self.rx_table, idx, 3, info.valid);
                        }
                    }
                    4 => {
                        info.missed += 1;
                        unsafe {
                            emane_tdma_numeric_u64_add(self.rx_missed, 1);
                            emane_tdma_slot_table_set_cell(self.rx_table, idx, 4, info.missed);
                        }
                    }
                    5 => {
                        info.rx_idle += 1;
                        unsafe {
                            emane_tdma_numeric_u64_add(self.rx_idle, 1);
                            emane_tdma_slot_table_set_cell(self.rx_table, idx, 5, info.rx_idle);
                        }
                    }
                    6 => {
                        info.rx_tx += 1;
                        unsafe {
                            emane_tdma_numeric_u64_add(self.rx_tx, 1);
                            emane_tdma_slot_table_set_cell(self.rx_table, idx, 6, info.rx_tx);
                        }
                    }
                    7 => {
                        info.rx_too_long += 1;
                        unsafe {
                            emane_tdma_numeric_u64_add(self.rx_too_long, 1);
                            emane_tdma_slot_table_set_cell(self.rx_table, idx, 7, info.rx_too_long);
                        }
                    }
                    8 => {
                        info.rx_wrong_freq += 1;
                        unsafe {
                            emane_tdma_numeric_u64_add(self.rx_wrong_freq, 1);
                            emane_tdma_slot_table_set_cell(
                                self.rx_table,
                                idx,
                                8,
                                info.rx_wrong_freq,
                            );
                        }
                    }
                    9 => {
                        info.rx_lock += 1;
                        unsafe {
                            emane_tdma_numeric_u64_add(self.rx_lock, 1);
                            emane_tdma_slot_table_set_cell(self.rx_table, idx, 9, info.rx_lock);
                        }
                    }
                    _ => {}
                }

                info.quantile[q] += 1;
                unsafe {
                    emane_tdma_slot_table_set_cell(self.rx_table, idx, 10 + q, info.quantile[q]);
                }
            }
            _ => {}
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_slot_publisher_create() -> *mut c_void {
    Box::into_raw(Box::new(SlotStatusPublisher::new())) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_slot_publisher_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr as *mut SlotStatusPublisher));
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_slot_publisher_clear(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let publ = unsafe { &mut *(ptr as *mut SlotStatusPublisher) };
    publ.clear();
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_slot_publisher_register(
    ptr: *mut c_void,
    _reg: *mut c_void,
    tx_table: *mut c_void,
    rx_table: *mut c_void,
    tx_valid: *mut c_void,
    tx_missed: *mut c_void,
    tx_too_big: *mut c_void,
    rx_valid: *mut c_void,
    rx_missed: *mut c_void,
    rx_idle: *mut c_void,
    rx_tx: *mut c_void,
    rx_too_long: *mut c_void,
    rx_wrong_freq: *mut c_void,
    rx_lock: *mut c_void,
) {
    if ptr.is_null() {
        return;
    }
    let publ = unsafe { &mut *(ptr as *mut SlotStatusPublisher) };
    publ.register(
        tx_table,
        rx_table,
        tx_valid,
        tx_missed,
        tx_too_big,
        rx_valid,
        rx_missed,
        rx_idle,
        rx_tx,
        rx_too_long,
        rx_wrong_freq,
        rx_lock,
    );
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_slot_publisher_update(
    ptr: *mut c_void,
    idx: u32,
    frame: u32,
    slot: u32,
    status: i32,
    ratio: f64,
) {
    if ptr.is_null() {
        return;
    }
    let publ = unsafe { &mut *(ptr as *mut SlotStatusPublisher) };
    publ.update(idx, frame, slot, status, ratio);
}
