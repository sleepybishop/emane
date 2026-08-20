use std::collections::{BTreeMap, HashMap, HashSet};
use std::os::raw::{c_int, c_void};

pub struct FrameInfo {
    pub msg_ptr: *mut c_void,
    pub pkt_info_ptr: *mut c_void,
    pub length: usize,
    pub sor: u64,
    pub freq_segments_ptr: *mut c_void,
    pub span: u64,
    pub begin_time: u64,
    pub seq: u64,
}

pub struct FragmentInfo {
    pub indices: HashSet<usize>,
    pub parts: BTreeMap<usize, Vec<u8>>,
    pub last_fragment_time: u64,
    pub dst: u16,
    pub total_num_fragments: usize,
}

pub struct ReceiveManager {
    cpp_this: *mut c_void,
    id: u16,
    transponder_index: u16,
    b_process: bool,
    rx_antenna_index: u16,
    fragment_check_threshold: u64,
    fragment_timeout_threshold: u64,

    pending_queue: BTreeMap<u64, FrameInfo>,
    last_end_of_reception: u64,
    fragment_store: HashMap<(u16, u64), FragmentInfo>,
    last_fragment_check_time: u64,
    next_eor_check_time: u64,
}

extern "C" {
    fn emane_bentpipe_receivemanager_cxx_now(cpp_this: *mut c_void) -> u64;
    fn emane_bentpipe_receivemanager_cxx_drop_lock(
        cpp_this: *mut c_void,
        pkt_info_ptr: *mut c_void,
        msg_ptr: *mut c_void,
    );
    fn emane_bentpipe_receivemanager_cxx_drop_spectrum_service(
        cpp_this: *mut c_void,
        pkt_info_ptr: *mut c_void,
        msg_ptr: *mut c_void,
    );
    fn emane_bentpipe_receivemanager_cxx_drop_sinr(
        cpp_this: *mut c_void,
        pkt_info_ptr: *mut c_void,
        msg_ptr: *mut c_void,
    );
    fn emane_bentpipe_receivemanager_cxx_drop_bad_curve(
        cpp_this: *mut c_void,
        pkt_info_ptr: *mut c_void,
        msg_ptr: *mut c_void,
    );
    fn emane_bentpipe_receivemanager_cxx_publish_drop_destination_mac(
        cpp_this: *mut c_void,
        pkt_info_ptr: *mut c_void,
        msg_ptr: *mut c_void,
        msg_index: usize,
    );
    fn emane_bentpipe_receivemanager_cxx_publish_accept_good(
        cpp_this: *mut c_void,
        pkt_info_ptr: *mut c_void,
        msg_ptr: *mut c_void,
        msg_index: usize,
    );
    fn emane_bentpipe_receivemanager_cxx_publish_accept_good_len(
        cpp_this: *mut c_void,
        pkt_info_ptr: *mut c_void,
        dst: u16,
        length: usize,
    );
    fn emane_bentpipe_receivemanager_cxx_check_spectrum(
        cpp_this: *mut c_void,
        rx_antenna_index: u16,
        freq_hz: u64,
        span: u64,
        sor: u64,
        rx_power_dbm: f64,
        out_noise_floor: *mut f64,
        out_signal_in_noise: *mut bool,
    ) -> c_int;
    fn emane_bentpipe_receivemanager_cxx_get_por(
        cpp_this: *mut c_void,
        pcr_curve_index: u16,
        sinr: f64,
        length: usize,
        out_por: *mut f32,
    ) -> c_int;
    fn emane_bentpipe_receivemanager_cxx_get_random(cpp_this: *mut c_void) -> f32;
    fn emane_bentpipe_receivemanager_cxx_update_neighbor_metrics(
        cpp_this: *mut c_void,
        pkt_info_ptr: *mut c_void,
        transponder_index: u16,
        sinr: f64,
        noise_floor: f64,
        sor: u64,
    );
    fn emane_bentpipe_receivemanager_cxx_get_messages_count(msg_ptr: *mut c_void) -> usize;
    fn emane_bentpipe_receivemanager_cxx_msg_get_dst(msg_ptr: *mut c_void, index: usize) -> u16;
    fn emane_bentpipe_receivemanager_cxx_msg_is_fragment(
        msg_ptr: *mut c_void,
        index: usize,
    ) -> bool;
    fn emane_bentpipe_receivemanager_cxx_msg_get_fragment_index(
        msg_ptr: *mut c_void,
        index: usize,
    ) -> usize;
    fn emane_bentpipe_receivemanager_cxx_msg_get_fragment_offset(
        msg_ptr: *mut c_void,
        index: usize,
    ) -> usize;
    fn emane_bentpipe_receivemanager_cxx_msg_get_fragment_sequence(
        msg_ptr: *mut c_void,
        index: usize,
    ) -> u64;
    fn emane_bentpipe_receivemanager_cxx_msg_is_more_fragments(
        msg_ptr: *mut c_void,
        index: usize,
    ) -> bool;
    fn emane_bentpipe_receivemanager_cxx_msg_get_data(
        msg_ptr: *mut c_void,
        index: usize,
        out_len: *mut usize,
    ) -> *const u8;
    fn emane_bentpipe_receivemanager_cxx_pktinfo_get_src(pkt_info_ptr: *mut c_void) -> u16;
    fn emane_bentpipe_receivemanager_cxx_bpm_get_pcr_curve_index(msg_ptr: *mut c_void) -> u16;
    fn emane_bentpipe_receivemanager_cxx_freq_get_frequency_hz(freq_ptr: *mut c_void) -> u64;
    fn emane_bentpipe_receivemanager_cxx_freq_get_rx_power_dbm(freq_ptr: *mut c_void) -> f64;
    fn emane_bentpipe_receivemanager_cxx_forward_upstream(
        cpp_this: *mut c_void,
        pkt_info_ptr: *mut c_void,
        dst: u16,
        data: *const u8,
        len: usize,
    );
    fn emane_bentpipe_receivemanager_cxx_bend_downstream(
        cpp_this: *mut c_void,
        pkt_info_ptr: *mut c_void,
        dst: u16,
        data: *const u8,
        len: usize,
    );
    fn emane_bentpipe_receivemanager_cxx_forward_upstream_parts(
        cpp_this: *mut c_void,
        pkt_info_ptr: *mut c_void,
        dst: u16,
        data_ptrs: *const *const u8,
        data_lens: *const usize,
        num_parts: usize,
    );
    fn emane_bentpipe_receivemanager_cxx_bend_downstream_parts(
        cpp_this: *mut c_void,
        pkt_info_ptr: *mut c_void,
        dst: u16,
        data_ptrs: *const *const u8,
        data_lens: *const usize,
        num_parts: usize,
    );
    fn emane_bentpipe_receivemanager_cxx_schedule_process(cpp_this: *mut c_void, eor: u64);
    fn emane_bentpipe_receivemanager_cxx_drop_miss_fragment(
        cpp_this: *mut c_void,
        src: u16,
        dst: u16,
        total_bytes: usize,
    );
    fn emane_bentpipe_receivemanager_cxx_delete_msg(ptr: *mut c_void);
    fn emane_bentpipe_receivemanager_cxx_delete_pkt_info(ptr: *mut c_void);
    fn emane_bentpipe_receivemanager_cxx_delete_freq_segments(ptr: *mut c_void);
}

#[no_mangle]
pub extern "C" fn emane_rs_bentpipe_receive_manager_new(
    cpp_this: *mut c_void,
    id: u16,
    transponder_index: u16,
    b_process: bool,
    rx_antenna_index: u16,
    fragment_check_threshold: u64,
    fragment_timeout_threshold: u64,
) -> *mut ReceiveManager {
    let rm = Box::new(ReceiveManager {
        cpp_this,
        id,
        transponder_index,
        b_process,
        rx_antenna_index,
        fragment_check_threshold,
        fragment_timeout_threshold,
        pending_queue: BTreeMap::new(),
        last_end_of_reception: 0,
        fragment_store: HashMap::new(),
        last_fragment_check_time: 0,
        next_eor_check_time: u64::MAX,
    });
    Box::into_raw(rm)
}

#[no_mangle]
pub extern "C" fn emane_rs_bentpipe_receive_manager_free(rm: *mut ReceiveManager) {
    if !rm.is_null() {
        unsafe {
            let _ = Box::from_raw(rm);
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_bentpipe_receive_manager_enqueue(
    rm: *mut ReceiveManager,
    msg_ptr: *mut c_void,
    pkt_info_ptr: *mut c_void,
    length: usize,
    sor: u64,
    freq_segments_ptr: *mut c_void,
    span: u64,
    begin_time: u64,
    seq: u64,
) {
    let rm = unsafe { &mut *rm };

    if sor >= rm.last_end_of_reception {
        rm.pending_queue.insert(
            sor,
            FrameInfo {
                msg_ptr,
                pkt_info_ptr,
                length,
                sor,
                freq_segments_ptr,
                span,
                begin_time,
                seq,
            },
        );
    } else {
        unsafe {
            emane_bentpipe_receivemanager_cxx_drop_lock(rm.cpp_this, pkt_info_ptr, msg_ptr);
            emane_bentpipe_receivemanager_cxx_delete_msg(msg_ptr);
            emane_bentpipe_receivemanager_cxx_delete_pkt_info(pkt_info_ptr);
            emane_bentpipe_receivemanager_cxx_delete_freq_segments(freq_segments_ptr);
        }
    }

    emane_rs_bentpipe_receive_manager_process(rm as *mut ReceiveManager);
}

fn cleanup_frame(frame: FrameInfo) {
    unsafe {
        emane_bentpipe_receivemanager_cxx_delete_msg(frame.msg_ptr);
        emane_bentpipe_receivemanager_cxx_delete_pkt_info(frame.pkt_info_ptr);
        emane_bentpipe_receivemanager_cxx_delete_freq_segments(frame.freq_segments_ptr);
    }
}

fn process_frame(rm: &mut ReceiveManager, frame: FrameInfo, now: u64) {
    let cpp_this = rm.cpp_this;
    unsafe {
        let mut d_noise_floor = 0.0;
        let mut b_signal_in_noise = false;
        let freq_hz =
            emane_bentpipe_receivemanager_cxx_freq_get_frequency_hz(frame.freq_segments_ptr);
        let rx_power_dbm =
            emane_bentpipe_receivemanager_cxx_freq_get_rx_power_dbm(frame.freq_segments_ptr);

        let res = emane_bentpipe_receivemanager_cxx_check_spectrum(
            cpp_this,
            rm.rx_antenna_index,
            freq_hz,
            frame.span,
            frame.sor,
            rx_power_dbm,
            &mut d_noise_floor,
            &mut b_signal_in_noise,
        );

        if res != 0 {
            emane_bentpipe_receivemanager_cxx_drop_spectrum_service(
                cpp_this,
                frame.pkt_info_ptr,
                frame.msg_ptr,
            );
            cleanup_frame(frame);
            return;
        }

        let sinr = rx_power_dbm - d_noise_floor;
        let pcr_curve_index =
            emane_bentpipe_receivemanager_cxx_bpm_get_pcr_curve_index(frame.msg_ptr);

        let mut por = 0.0;
        if emane_bentpipe_receivemanager_cxx_get_por(
            cpp_this,
            pcr_curve_index,
            sinr,
            frame.length,
            &mut por,
        ) == 0
        {
            let f_random = emane_bentpipe_receivemanager_cxx_get_random(cpp_this);
            if por < f_random {
                emane_bentpipe_receivemanager_cxx_drop_sinr(
                    cpp_this,
                    frame.pkt_info_ptr,
                    frame.msg_ptr,
                );
                cleanup_frame(frame);
                return;
            }
        } else {
            emane_bentpipe_receivemanager_cxx_drop_bad_curve(
                cpp_this,
                frame.pkt_info_ptr,
                frame.msg_ptr,
            );
            cleanup_frame(frame);
            return;
        }

        emane_bentpipe_receivemanager_cxx_update_neighbor_metrics(
            cpp_this,
            frame.pkt_info_ptr,
            rm.transponder_index,
            sinr,
            d_noise_floor,
            frame.sor,
        );

        let msg_count = emane_bentpipe_receivemanager_cxx_get_messages_count(frame.msg_ptr);
        let src = emane_bentpipe_receivemanager_cxx_pktinfo_get_src(frame.pkt_info_ptr);
        let broadcast = 0xFFFF; // NEM_BROADCAST_MAC_ADDRESS

        for i in 0..msg_count {
            let dst = emane_bentpipe_receivemanager_cxx_msg_get_dst(frame.msg_ptr, i);

            if (rm.b_process && (dst == rm.id || dst == broadcast))
                || (!rm.b_process && dst != rm.id)
            {
                let mut data_len = 0;
                let data_ptr =
                    emane_bentpipe_receivemanager_cxx_msg_get_data(frame.msg_ptr, i, &mut data_len);
                let data_slice = std::slice::from_raw_parts(data_ptr, data_len);

                if emane_bentpipe_receivemanager_cxx_msg_is_fragment(frame.msg_ptr, i) {
                    let f_index =
                        emane_bentpipe_receivemanager_cxx_msg_get_fragment_index(frame.msg_ptr, i);
                    let f_offset =
                        emane_bentpipe_receivemanager_cxx_msg_get_fragment_offset(frame.msg_ptr, i);
                    let f_seq = emane_bentpipe_receivemanager_cxx_msg_get_fragment_sequence(
                        frame.msg_ptr,
                        i,
                    );
                    let f_more =
                        emane_bentpipe_receivemanager_cxx_msg_is_more_fragments(frame.msg_ptr, i);

                    let key = (src, f_seq);
                    let frag = rm
                        .fragment_store
                        .entry(key)
                        .or_insert_with(|| FragmentInfo {
                            indices: HashSet::new(),
                            parts: BTreeMap::new(),
                            last_fragment_time: now,
                            dst,
                            total_num_fragments: 0,
                        });

                    if frag.indices.insert(f_index) {
                        frag.parts.insert(f_offset, data_slice.to_vec());
                        frag.last_fragment_time = now;
                        if !f_more {
                            frag.total_num_fragments = f_index + 1;
                        }

                        if frag.total_num_fragments != 0
                            && frag.indices.len() == frag.total_num_fragments
                        {
                            let mut ptrs = Vec::new();
                            let mut lens = Vec::new();
                            for part in frag.parts.values() {
                                ptrs.push(part.as_ptr());
                                lens.push(part.len());
                            }

                            if rm.b_process {
                                emane_bentpipe_receivemanager_cxx_publish_accept_good_len(
                                    cpp_this,
                                    frame.pkt_info_ptr,
                                    dst,
                                    lens.iter().sum(),
                                );
                                emane_bentpipe_receivemanager_cxx_forward_upstream_parts(
                                    cpp_this,
                                    frame.pkt_info_ptr,
                                    dst,
                                    ptrs.as_ptr(),
                                    lens.as_ptr(),
                                    ptrs.len(),
                                );
                            } else {
                                emane_bentpipe_receivemanager_cxx_bend_downstream_parts(
                                    cpp_this,
                                    frame.pkt_info_ptr,
                                    dst,
                                    ptrs.as_ptr(),
                                    lens.as_ptr(),
                                    ptrs.len(),
                                );
                            }
                            rm.fragment_store.remove(&key);
                        }
                    }
                } else {
                    if rm.b_process {
                        emane_bentpipe_receivemanager_cxx_publish_accept_good(
                            cpp_this,
                            frame.pkt_info_ptr,
                            frame.msg_ptr,
                            i,
                        );
                        emane_bentpipe_receivemanager_cxx_forward_upstream(
                            cpp_this,
                            frame.pkt_info_ptr,
                            dst,
                            data_ptr,
                            data_len,
                        );
                    } else {
                        emane_bentpipe_receivemanager_cxx_bend_downstream(
                            cpp_this,
                            frame.pkt_info_ptr,
                            dst,
                            data_ptr,
                            data_len,
                        );
                    }
                    rm.last_end_of_reception = frame.sor + frame.span;
                }
            } else {
                emane_bentpipe_receivemanager_cxx_publish_drop_destination_mac(
                    cpp_this,
                    frame.pkt_info_ptr,
                    frame.msg_ptr,
                    i,
                );
            }
        }
        cleanup_frame(frame);
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_bentpipe_receive_manager_process(rm: *mut ReceiveManager) {
    let rm = unsafe { &mut *rm };
    let now = unsafe { emane_bentpipe_receivemanager_cxx_now(rm.cpp_this) };

    loop {
        let (sor, eor) = {
            if let Some((&sor, frame)) = rm.pending_queue.iter().next() {
                (sor, sor + frame.span)
            } else {
                break;
            }
        };

        if sor >= rm.last_end_of_reception {
            if now >= eor {
                let frame = rm.pending_queue.remove(&sor).unwrap();
                process_frame(rm, frame, now);
            } else {
                break;
            }
        } else {
            let frame = rm.pending_queue.remove(&sor).unwrap();
            unsafe {
                emane_bentpipe_receivemanager_cxx_drop_lock(
                    rm.cpp_this,
                    frame.pkt_info_ptr,
                    frame.msg_ptr,
                );
            }
            cleanup_frame(frame);
        }
    }

    if let Some((&sor, frame)) = rm.pending_queue.iter().next() {
        let eor = sor + frame.span;
        rm.next_eor_check_time = eor;
        unsafe {
            emane_bentpipe_receivemanager_cxx_schedule_process(rm.cpp_this, eor);
        }
    }

    if rm.last_fragment_check_time + rm.fragment_check_threshold <= now {
        let mut abandoned = Vec::new();
        for (key, frag) in rm.fragment_store.iter() {
            if frag.last_fragment_time + rm.fragment_timeout_threshold <= now {
                abandoned.push(*key);
            }
        }
        for key in abandoned {
            let frag = rm.fragment_store.remove(&key).unwrap();
            let mut total_bytes = 0;
            for part in frag.parts.values() {
                total_bytes += part.len();
            }
            unsafe {
                emane_bentpipe_receivemanager_cxx_drop_miss_fragment(
                    rm.cpp_this,
                    key.0,
                    frag.dst,
                    total_bytes,
                );
            }
        }
        rm.last_fragment_check_time = now;
    }
}
