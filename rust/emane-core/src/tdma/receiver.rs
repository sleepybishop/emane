use std::collections::{BTreeMap, BTreeSet};
use std::os::raw::{c_char, c_void};
use std::ffi::CStr;
use std::time::{SystemTime, Duration};
use crate::tdma_message::{BaseModelMessage, MessageComponent, FfiTdmaMessageType};
use roxmltree::Document;
use std::fs;
use rand::Rng;

#[repr(C)]
pub struct FFIFrequencySegment {
    pub frequency_hz: u64,
    pub rx_power_dbm: f64,
    pub duration_micro: u64,
}

#[repr(C)]
pub struct FFIReceiveManagerCallbacks {
    pub rm_cpp: *mut c_void,
    pub log_error: extern "C" fn(*mut c_void, u16, *const c_char),
    pub log_debug: extern "C" fn(*mut c_void, u16, *const c_char),
    pub spectrum_request_and_noise: extern "C" fn(
        *mut c_void, u64, u64, u64, f64, *mut f64, *mut bool
    ) -> bool, // returns false on SpectrumServiceException
    pub publish_inbound: extern "C" fn(
        *mut c_void, u16, u16, u8, usize, u32
    ),
    pub publish_inbound_component: extern "C" fn(
        *mut c_void, u16, u32,
        u32, u16, u8, *const u8, usize,
        bool, u32, u32, u64, bool
    ),
    pub publish_inbound_message: extern "C" fn(
        *mut c_void, u16, *const c_void, u32 // *const c_void is *const BaseModelMessage
    ),
    pub update_neighbor_rx_metric: extern "C" fn(
        *mut c_void, u16, u64, *const u8, f64, f64, u64, u64, u64
    ),
    pub send_upstream_packet: extern "C" fn(
        *mut c_void, u16, u16, u8, u64, *const u8, *const u8, usize
    ),
    pub process_packet_meta_info: extern "C" fn(
        *mut c_void, u16, u64, f64, f64, u64
    ),
    pub process_scheduler_packet: extern "C" fn(
        *mut c_void, u16, u16, u8, u64, *const u8, *const u8, usize, u64, f64, f64, u64
    ),
}

pub struct PorManager {
    data_rate_table: BTreeMap<u64, (i32, i32, BTreeMap<i32, f32>)>,
    default_curve_data_rate: u64,
    modifier_length_bytes: usize,
}

impl PorManager {
    pub fn new() -> Self {
        Self {
            data_rate_table: BTreeMap::new(),
            default_curve_data_rate: 0,
            modifier_length_bytes: 0,
        }
    }

    pub fn load(&mut self, s_pcr_file_name: &str) -> Result<(), String> {
        let content = fs::read_to_string(s_pcr_file_name).map_err(|e| e.to_string())?;
        let doc = Document::parse(&content).map_err(|e| e.to_string())?;
        
        let root = doc.root_element();
        if root.tag_name().name() != "tdmabasemodel-pcr" {
            return Err("Invalid root element".into());
        }

        if let Some(ps) = root.attribute("packetsize") {
            self.modifier_length_bytes = ps.parse().unwrap_or(0);
        }

        for data_rate_node in root.children().filter(|n| n.has_tag_name("datarate")) {
            let bps_str = data_rate_node.attribute("bps").ok_or("Missing bps attribute")?;
            let u64_data_rate_bps = Self::parse_si_prefix(bps_str).ok_or("Invalid bps")?;
            
            if self.data_rate_table.is_empty() {
                self.default_curve_data_rate = u64_data_rate_bps;
            }

            let mut curve = BTreeMap::new();
            let mut min_sinr = i32::MAX;
            let mut max_sinr = i32::MIN;

            for entry_node in data_rate_node.children().filter(|n| n.has_tag_name("entry")) {
                let sinr_str = entry_node.attribute("sinr").ok_or("Missing sinr")?;
                let por_str = entry_node.attribute("por").ok_or("Missing por")?;

                let scaled_sinr = Self::scale_float_to_integer(sinr_str);
                let por: f32 = por_str.parse().map_err(|_| "Invalid por")?;
                let por = por / 100.0;

                if curve.insert(scaled_sinr, por).is_some() {
                    return Err(format!("duplicate PCR SINR value for datarate: {}", u64_data_rate_bps));
                }

                min_sinr = min_sinr.min(scaled_sinr);
                max_sinr = max_sinr.max(scaled_sinr);
            }

            // Interpolation
            let mut interpolation = BTreeMap::new();
            let entries: Vec<_> = curve.iter().map(|(&x, &y)| (x, y)).collect();
            for w in entries.windows(2) {
                let (x0, y0) = w[0];
                let (x1, y1) = w[1];
                let slope = (y1 - y0) / (x1 - x0) as f32;
                for i in (x0 + 1)..x1 {
                    interpolation.insert(i, y1 - (x1 - i) as f32 * slope);
                }
            }
            curve.extend(interpolation);

            if self.data_rate_table.insert(u64_data_rate_bps, (min_sinr, max_sinr, curve)).is_some() {
                return Err(format!("duplicate PCR datarate: {}", u64_data_rate_bps));
            }
        }
        Ok(())
    }

    pub fn get_por(&self, u64_data_rate_bps: u64, f_sinr: f32, packet_length_bytes: usize) -> f32 {
        let curve_data = self.data_rate_table.get(&u64_data_rate_bps)
            .or_else(|| self.data_rate_table.get(&self.default_curve_data_rate));

        if let Some((min_sinr, max_sinr, curve)) = curve_data {
            let scaled_sinr = (f_sinr * 100.0) as i32;
            if scaled_sinr < *min_sinr {
                return 0.0;
            }
            if scaled_sinr > *max_sinr {
                return 1.0;
            }
            if let Some(&por) = curve.get(&scaled_sinr) {
                if self.modifier_length_bytes > 0 {
                    return por.powf(packet_length_bytes as f32 / self.modifier_length_bytes as f32);
                }
                return por;
            }
        }
        0.0
    }

    fn scale_float_to_integer(val: &str) -> i32 {
        let parsed: f32 = val.parse().unwrap_or(0.0);
        (parsed * 100.0) as i32
    }

    fn parse_si_prefix(val: &str) -> Option<u64> {
        let mut num_str = String::new();
        let mut multiplier = 1u64;
        for c in val.chars() {
            if c.is_digit(10) || c == '.' {
                num_str.push(c);
            } else {
                multiplier = match c {
                    'K' | 'k' => 1_000,
                    'M' | 'm' => 1_000_000,
                    'G' | 'g' => 1_000_000_000,
                    _ => 1,
                };
                break;
            }
        }
        let parsed: f64 = num_str.parse().ok()?;
        Some((parsed * (multiplier as f64)) as u64)
    }
}


pub struct ReceiveManager {
    id: u16,
    callbacks: FFIReceiveManagerCallbacks,
    
    // pendingInfo_ elements
    pending_base_model_message: Option<Box<BaseModelMessage>>,
    pending_pkt_source: u16,
    pending_pkt_destination: u16,
    pending_pkt_creation_time_micro: u64,
    pending_pkt_uuid: [u8; 16],
    pending_length: usize,
    pending_start_of_reception_micro: u64,
    pending_frequency_segments: Vec<FFIFrequencySegment>,
    pending_span_micro: u64,
    pending_begin_time_micro: u64,
    pending_packet_sequence: u64,
    
    pending_absolute_slot_index: u64,
    por_manager: PorManager,
    
    promiscuous_mode: bool,
    fragment_check_threshold_sec: u64,
    fragment_timeout_threshold_sec: u64,
    
    // fragmentStore_
    // key: (source, priority, fragment_sequence)
    // val: (index_set, parts, last_fragment_time_micro, destination, total_num_fragments)
    fragment_store: BTreeMap<(u16, u8, u64), (BTreeSet<u32>, BTreeMap<u32, Vec<u8>>, u64, u16, u32)>,
    last_fragment_check_time_micro: u64,
}

impl ReceiveManager {
    pub fn new(id: u16, callbacks: FFIReceiveManagerCallbacks) -> Self {
        Self {
            id,
            callbacks,
            pending_base_model_message: None,
            pending_pkt_source: 0,
            pending_pkt_destination: 0,
            pending_pkt_creation_time_micro: 0,
            pending_pkt_uuid: [0; 16],
            pending_length: 0,
            pending_start_of_reception_micro: 0,
            pending_frequency_segments: Vec::new(),
            pending_span_micro: 0,
            pending_begin_time_micro: 0,
            pending_packet_sequence: 0,
            pending_absolute_slot_index: 0,
            por_manager: PorManager::new(),
            promiscuous_mode: false,
            fragment_check_threshold_sec: 2,
            fragment_timeout_threshold_sec: 5,
            fragment_store: BTreeMap::new(),
            last_fragment_check_time_micro: 0,
        }
    }

    pub fn set_promiscuous_mode(&mut self, enable: bool) {
        self.promiscuous_mode = enable;
    }

    pub fn load_curves(&mut self, s_pcr_file_name: &str) {
        if let Err(e) = self.por_manager.load(s_pcr_file_name) {
            let msg = format!("Failed to load curves: {}", e);
            let c_msg = std::ffi::CString::new(msg).unwrap();
            (self.callbacks.log_error)(self.callbacks.rm_cpp, self.id, c_msg.as_ptr());
        }
    }

    pub fn set_fragment_check_threshold(&mut self, seconds: u64) {
        self.fragment_check_threshold_sec = seconds;
    }

    pub fn set_fragment_timeout_threshold(&mut self, seconds: u64) {
        self.fragment_timeout_threshold_sec = seconds;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn enqueue(
        &mut self,
        bmm: Box<BaseModelMessage>,
        src: u16,
        dst: u16,
        ctime: u64,
        uuid: &[u8; 16],
        length: usize,
        sor: u64,
        segments: Vec<FFIFrequencySegment>,
        span: u64,
        begin_time: u64,
        seq: u64,
    ) -> bool {
        let mut b_return = false;
        let u64_absolute_slot_index = bmm.abs_slot_index;

        if self.pending_absolute_slot_index == 0 {
            self.pending_absolute_slot_index = u64_absolute_slot_index;
            self.update_pending(bmm, src, dst, ctime, uuid, length, sor, segments, span, begin_time, seq);
            b_return = true;
        } else if self.pending_absolute_slot_index < u64_absolute_slot_index {
            self.process(u64_absolute_slot_index);
            self.pending_absolute_slot_index = u64_absolute_slot_index;
            self.update_pending(bmm, src, dst, ctime, uuid, length, sor, segments, span, begin_time, seq);
            b_return = true;
        } else if self.pending_absolute_slot_index > u64_absolute_slot_index {
            let msg = format!("MACI {:03} TDMA::ReceiveManager enqueue: pending slot: {} greater than enqueue: {}", 
                              self.id, self.pending_absolute_slot_index, u64_absolute_slot_index);
            let c_msg = std::ffi::CString::new(msg).unwrap();
            (self.callbacks.log_error)(self.callbacks.rm_cpp, self.id, c_msg.as_ptr());
            
            self.pending_absolute_slot_index = u64_absolute_slot_index;
            self.update_pending(bmm, src, dst, ctime, uuid, length, sor, segments, span, begin_time, seq);
            b_return = true;
        } else {
            if sor < self.pending_start_of_reception_micro {
                self.update_pending(bmm, src, dst, ctime, uuid, length, sor, segments, span, begin_time, seq);
            }
        }
        b_return
    }

    #[allow(clippy::too_many_arguments)]
    fn update_pending(
        &mut self,
        bmm: Box<BaseModelMessage>,
        src: u16,
        dst: u16,
        ctime: u64,
        uuid: &[u8; 16],
        length: usize,
        sor: u64,
        segments: Vec<FFIFrequencySegment>,
        span: u64,
        begin_time: u64,
        seq: u64,
    ) {
        self.pending_base_model_message = Some(bmm);
        self.pending_pkt_source = src;
        self.pending_pkt_destination = dst;
        self.pending_pkt_creation_time_micro = ctime;
        self.pending_pkt_uuid = *uuid;
        self.pending_length = length;
        self.pending_start_of_reception_micro = sor;
        self.pending_frequency_segments = segments;
        self.pending_span_micro = span;
        self.pending_begin_time_micro = begin_time;
        self.pending_packet_sequence = seq;
    }

    pub fn process(&mut self, u64_absolute_slot_index: u64) {
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_micros() as u64;

        if self.pending_absolute_slot_index + 1 == u64_absolute_slot_index {
            self.pending_absolute_slot_index = 0;

            let mut d_sinr = 0.0;
            let mut d_noise_floor_db = 0.0;
            let mut b_signal_in_noise = false;

            if let Some(bmm) = self.pending_base_model_message.take() {
                if let Some(freq_seg) = self.pending_frequency_segments.first() {
                    let success = (self.callbacks.spectrum_request_and_noise)(
                        self.callbacks.rm_cpp,
                        freq_seg.frequency_hz,
                        self.pending_span_micro,
                        self.pending_start_of_reception_micro,
                        freq_seg.rx_power_dbm,
                        &mut d_noise_floor_db,
                        &mut b_signal_in_noise
                    );

                    if !success {
                        (self.callbacks.publish_inbound_message)(
                            self.callbacks.rm_cpp,
                            self.pending_pkt_source,
                            &*bmm as *const _ as *const c_void,
                            5 // DROP_SPECTRUM_SERVICE
                        );
                        return;
                    }

                    d_sinr = freq_seg.rx_power_dbm - d_noise_floor_db;
                    
                    let por = self.por_manager.get_por(bmm.data_rate_bps, d_sinr as f32, self.pending_length);
                    let mut rng = rand::thread_rng();
                    let random: f32 = rng.gen_range(0.0..1.0);

                    if por < random {
                        (self.callbacks.publish_inbound_message)(
                            self.callbacks.rm_cpp,
                            self.pending_pkt_source,
                            &*bmm as *const _ as *const c_void,
                            6 // DROP_SINR
                        );
                        return;
                    }

                    (self.callbacks.update_neighbor_rx_metric)(
                        self.callbacks.rm_cpp,
                        self.pending_pkt_source,
                        self.pending_packet_sequence,
                        self.pending_pkt_uuid.as_ptr(),
                        d_sinr,
                        d_noise_floor_db,
                        self.pending_start_of_reception_micro,
                        freq_seg.duration_micro,
                        bmm.data_rate_bps
                    );

                    let broadcast_mac: u16 = 0xFFFF;

                    for message in &bmm.messages {
                        let dst = message.destination;
                        let priority = message.priority;

                        if self.promiscuous_mode || dst == self.id || dst == broadcast_mac {
                            if message.is_fragment {
                                let key = (self.pending_pkt_source, priority, message.fragment_sequence);
                                
                                let entry = self.fragment_store.entry(key).or_insert_with(|| {
                                    (BTreeSet::new(), BTreeMap::new(), now, dst, if message.more_fragments { 0 } else { message.fragment_index + 1 })
                                });
                                
                                if entry.0.insert(message.fragment_index) {
                                    entry.1.insert(message.fragment_offset, message.data.clone());
                                    entry.2 = now;
                                    if !message.more_fragments {
                                        entry.4 = message.fragment_index + 1;
                                    }

                                    if entry.4 > 0 && entry.0.len() == entry.4 as usize {
                                        let mut combined_data = Vec::new();
                                        for part in entry.1.values() {
                                            combined_data.extend_from_slice(part);
                                        }

                                        (self.callbacks.publish_inbound)(
                                            self.callbacks.rm_cpp,
                                            self.pending_pkt_source,
                                            dst,
                                            priority,
                                            combined_data.len(),
                                            0 // ACCEPT_GOOD
                                        );

                                        if message.msg_type as i32 == FfiTdmaMessageType::Data as i32 {
                                            (self.callbacks.send_upstream_packet)(
                                                self.callbacks.rm_cpp,
                                                self.pending_pkt_source,
                                                dst,
                                                priority,
                                                self.pending_pkt_creation_time_micro,
                                                self.pending_pkt_uuid.as_ptr(),
                                                combined_data.as_ptr(),
                                                combined_data.len()
                                            );
                                            (self.callbacks.process_packet_meta_info)(
                                                self.callbacks.rm_cpp,
                                                self.pending_pkt_source,
                                                u64_absolute_slot_index - 1,
                                                freq_seg.rx_power_dbm,
                                                d_sinr,
                                                bmm.data_rate_bps
                                            );
                                        } else {
                                            (self.callbacks.process_scheduler_packet)(
                                                self.callbacks.rm_cpp,
                                                self.pending_pkt_source,
                                                dst,
                                                priority,
                                                self.pending_pkt_creation_time_micro,
                                                self.pending_pkt_uuid.as_ptr(),
                                                combined_data.as_ptr(),
                                                combined_data.len(),
                                                u64_absolute_slot_index - 1,
                                                freq_seg.rx_power_dbm,
                                                d_sinr,
                                                bmm.data_rate_bps
                                            );
                                        }
                                        self.fragment_store.remove(&key);
                                    }
                                }
                            } else {
                                (self.callbacks.publish_inbound_component)(
                                    self.callbacks.rm_cpp,
                                    self.pending_pkt_source,
                                    0, // ACCEPT_GOOD
                                    message.msg_type as u32,
                                    message.destination,
                                    message.priority,
                                    message.data.as_ptr(),
                                    message.data.len(),
                                    message.is_fragment,
                                    message.fragment_index,
                                    message.fragment_offset,
                                    message.fragment_sequence,
                                    message.more_fragments
                                );

                                if message.msg_type as i32 == FfiTdmaMessageType::Data as i32 {
                                    (self.callbacks.send_upstream_packet)(
                                        self.callbacks.rm_cpp,
                                        self.pending_pkt_source,
                                        dst,
                                        priority,
                                        self.pending_pkt_creation_time_micro,
                                        self.pending_pkt_uuid.as_ptr(),
                                        message.data.as_ptr(),
                                        message.data.len()
                                    );
                                    (self.callbacks.process_packet_meta_info)(
                                        self.callbacks.rm_cpp,
                                        self.pending_pkt_source,
                                        u64_absolute_slot_index - 1,
                                        freq_seg.rx_power_dbm,
                                        d_sinr,
                                        bmm.data_rate_bps
                                    );
                                } else {
                                    (self.callbacks.process_scheduler_packet)(
                                        self.callbacks.rm_cpp,
                                        self.pending_pkt_source,
                                        dst,
                                        priority,
                                        self.pending_pkt_creation_time_micro,
                                        self.pending_pkt_uuid.as_ptr(),
                                        message.data.as_ptr(),
                                        message.data.len(),
                                        u64_absolute_slot_index - 1,
                                        freq_seg.rx_power_dbm,
                                        d_sinr,
                                        bmm.data_rate_bps
                                    );
                                }
                            }
                        } else {
                            (self.callbacks.publish_inbound_component)(
                                self.callbacks.rm_cpp,
                                self.pending_pkt_source,
                                8, // DROP_DESTINATION_MAC
                                message.msg_type as u32,
                                message.destination,
                                message.priority,
                                message.data.as_ptr(),
                                message.data.len(),
                                message.is_fragment,
                                message.fragment_index,
                                message.fragment_offset,
                                message.fragment_sequence,
                                message.more_fragments
                            );
                        }
                    }
                }
            }
        }

        if self.last_fragment_check_time_micro + self.fragment_check_threshold_sec * 1_000_000 <= now {
            let mut keys_to_remove = Vec::new();
            for (key, val) in &self.fragment_store {
                let last_fragment_time = val.2;
                if last_fragment_time + self.fragment_timeout_threshold_sec * 1_000_000 <= now {
                    let total_bytes: usize = val.1.values().map(|v| v.len()).sum();
                    (self.callbacks.publish_inbound)(
                        self.callbacks.rm_cpp,
                        key.0, // src
                        val.3, // dst
                        key.1, // priority
                        total_bytes,
                        4 // DROP_MISS_FRAGMENT
                    );
                    keys_to_remove.push(key.clone());
                }
            }
            for k in keys_to_remove {
                self.fragment_store.remove(&k);
            }
            self.last_fragment_check_time_micro = now;
        }
    }
}


#[no_mangle]
pub extern "C" fn tdma_receivemanager_new(
    id: u16,
    callbacks: *const FFIReceiveManagerCallbacks
) -> *mut ReceiveManager {
    if callbacks.is_null() {
        return std::ptr::null_mut();
    }
    let cb = unsafe { &*callbacks };
    let cb_clone = FFIReceiveManagerCallbacks {
        rm_cpp: cb.rm_cpp,
        log_error: cb.log_error,
        log_debug: cb.log_debug,
        spectrum_request_and_noise: cb.spectrum_request_and_noise,
        publish_inbound: cb.publish_inbound,
        publish_inbound_component: cb.publish_inbound_component,
        publish_inbound_message: cb.publish_inbound_message,
        update_neighbor_rx_metric: cb.update_neighbor_rx_metric,
        send_upstream_packet: cb.send_upstream_packet,
        process_packet_meta_info: cb.process_packet_meta_info,
        process_scheduler_packet: cb.process_scheduler_packet,
    };
    Box::into_raw(Box::new(ReceiveManager::new(id, cb_clone)))
}

#[no_mangle]
pub extern "C" fn tdma_receivemanager_free(rm: *mut ReceiveManager) {
    if !rm.is_null() {
        unsafe { drop(Box::from_raw(rm)) }
    }
}

#[no_mangle]
pub extern "C" fn tdma_receivemanager_set_promiscuous_mode(rm: *mut ReceiveManager, enable: bool) {
    let rm = unsafe { &mut *rm };
    rm.set_promiscuous_mode(enable);
}

#[no_mangle]
pub extern "C" fn tdma_receivemanager_load_curves(rm: *mut ReceiveManager, file: *const c_char) {
    let rm = unsafe { &mut *rm };
    let c_str = unsafe { CStr::from_ptr(file) };
    if let Ok(s) = c_str.to_str() {
        rm.load_curves(s);
    }
}

#[no_mangle]
pub extern "C" fn tdma_receivemanager_set_fragment_check_threshold(rm: *mut ReceiveManager, seconds: u64) {
    let rm = unsafe { &mut *rm };
    rm.set_fragment_check_threshold(seconds);
}

#[no_mangle]
pub extern "C" fn tdma_receivemanager_set_fragment_timeout_threshold(rm: *mut ReceiveManager, seconds: u64) {
    let rm = unsafe { &mut *rm };
    rm.set_fragment_timeout_threshold(seconds);
}

#[no_mangle]
pub extern "C" fn tdma_receivemanager_enqueue(
    rm: *mut ReceiveManager,
    bmm_bytes: *const u8,
    bmm_len: usize,
    src: u16,
    dst: u16,
    ctime: u64,
    uuid: *const u8,
    length: usize,
    sor: u64,
    segments: *const FFIFrequencySegment,
    segments_count: usize,
    span: u64,
    begin_time: u64,
    seq: u64
) -> bool {
    let rm = unsafe { &mut *rm };
    
    // Deserialize BaseModelMessage
    let bmm_ptr = crate::tdma_message::emane_rs_tdma_message_deserialize(bmm_bytes, bmm_len);
    if bmm_ptr.is_null() {
        return false;
    }
    let bmm = unsafe { Box::from_raw(bmm_ptr) };

    let mut uuid_arr = [0u8; 16];
    if !uuid.is_null() {
        unsafe {
            std::ptr::copy_nonoverlapping(uuid, uuid_arr.as_mut_ptr(), 16);
        }
    }

    let mut segs = Vec::with_capacity(segments_count);
    if !segments.is_null() {
        unsafe {
            for i in 0..segments_count {
                let s = &*segments.add(i);
                segs.push(FFIFrequencySegment {
                    frequency_hz: s.frequency_hz,
                    rx_power_dbm: s.rx_power_dbm,
                    duration_micro: s.duration_micro,
                });
            }
        }
    }

    rm.enqueue(bmm, src, dst, ctime, &uuid_arr, length, sor, segs, span, begin_time, seq)
}

#[no_mangle]
pub extern "C" fn tdma_receivemanager_process(rm: *mut ReceiveManager, u64_absolute_slot_index: u64) {
    let rm = unsafe { &mut *rm };
    rm.process(u64_absolute_slot_index);
}


#[no_mangle]
pub extern "C" fn tdma_pormanager_new() -> *mut PorManager {
    Box::into_raw(Box::new(PorManager::new()))
}

#[no_mangle]
pub extern "C" fn tdma_pormanager_free(pm: *mut PorManager) {
    if !pm.is_null() {
        unsafe { drop(Box::from_raw(pm)) }
    }
}

#[no_mangle]
pub extern "C" fn tdma_pormanager_load(pm: *mut PorManager, s_pcr_file_name: *const c_char) {
    let pm = unsafe { &mut *pm };
    let c_str = unsafe { CStr::from_ptr(s_pcr_file_name) };
    if let Ok(s) = c_str.to_str() {
        let _ = pm.load(s); // Ignoring error for FFI boundary simplicity like in original
    }
}

#[no_mangle]
pub extern "C" fn tdma_pormanager_get_por(
    pm: *mut PorManager,
    u64_data_rate_bps: u64,
    f_sinr: f32,
    packet_length_bytes: usize
) -> f32 {
    let pm = unsafe { &*pm };
    pm.get_por(u64_data_rate_bps, f_sinr, packet_length_bytes)
}
