use std::os::raw::c_void;

pub enum UpstreamPacket {}
pub enum DownstreamPacket {}
pub enum ControlMessages {}

pub unsafe fn emane_bentpipe_upstreampacket_get_src(pkt: *const UpstreamPacket) -> u16 { 0 }
pub unsafe fn emane_bentpipe_upstreampacket_get_dst(pkt: *const UpstreamPacket) -> u16 { 0 }
pub unsafe fn emane_bentpipe_upstreampacket_get_priority(pkt: *const UpstreamPacket) -> u8 { 0 }
pub unsafe fn emane_bentpipe_upstreampacket_length(pkt: *const UpstreamPacket) -> usize { 0 }
pub unsafe fn emane_bentpipe_upstreampacket_stripLengthPrefixFraming( pkt: *mut UpstreamPacket, ) -> usize { 0 }
pub unsafe fn emane_bentpipe_upstreampacket_get(pkt: *const UpstreamPacket) -> *const c_void { std::ptr::null_mut() }
pub unsafe fn emane_bentpipe_downstreampacket_get_priority(pkt: *const DownstreamPacket) -> u8 { 0 }
pub unsafe fn emane_bentpipe_commonmacheader_get_registration_id(hdr: *const c_void) -> u16 { 0 }
pub unsafe fn emane_bentpipe_commonmacheader_get_sequence_number(hdr: *const c_void) -> u64 { 0 }
pub unsafe fn emane_bentpipe_radiomodel_drop_registration_id( radiomodel: *mut c_void, src: u16, dst: u16, length: usize, ) {}
pub unsafe fn emane_bentpipe_radiomodel_drop_rx_off( radiomodel: *mut c_void, src: u16, pkt_data: *const c_void, pkt_len: usize, ) {}
pub unsafe fn emane_bentpipe_radiomodel_find_transponder( radiomodel: *mut c_void, msgs: *const ControlMessages, out_antenna_index: *mut u16, out_freq_hz: *mut u64, out_span: *mut u64, out_start_of_reception: *mut u64, ) -> i32 { 0 }
pub unsafe fn emane_bentpipe_radiomodel_enqueue_receive( radiomodel: *mut c_void, transponder_index: i32, pkt: *mut UpstreamPacket, msgs: *const ControlMessages, seq_num: u64, ) {}
pub unsafe fn emane_bentpipe_radiomodel_get_tos_to_transponder( radiomodel: *mut c_void, tos: u8, ) -> i32 { 0 }
pub unsafe fn emane_bentpipe_radiomodel_queue_manager_enqueue( radiomodel: *mut c_void, transponder_index: i32, pkt: *mut DownstreamPacket, ) {}
pub unsafe fn emane_bentpipe_radiomodel_process(radiomodel: *mut c_void) {}

pub const REGISTERED_EMANE_MAC_BENT_PIPE: u16 = 3;

#[no_mangle]
pub extern "C" fn emane_bentpipe_radiomodel_processUpstreamControl(
    _radiomodel: *mut c_void,
    _msgs: *const ControlMessages,
) {
}

#[no_mangle]
pub extern "C" fn emane_bentpipe_radiomodel_processDownstreamControl(
    _radiomodel: *mut c_void,
    _msgs: *const ControlMessages,
) {
}

#[no_mangle]
pub extern "C" fn emane_bentpipe_radiomodel_processUpstreamPacket(
    radiomodel: *mut c_void,
    hdr: *const c_void,
    pkt: *mut UpstreamPacket,
    msgs: *const ControlMessages,
) {
    unsafe {
        let reg_id = emane_bentpipe_commonmacheader_get_registration_id(hdr);
        if reg_id != REGISTERED_EMANE_MAC_BENT_PIPE {
            let src = emane_bentpipe_upstreampacket_get_src(pkt);
            let dst = emane_bentpipe_upstreampacket_get_dst(pkt);
            let length = emane_bentpipe_upstreampacket_length(pkt);
            emane_bentpipe_radiomodel_drop_registration_id(radiomodel, src, dst, length);
            return;
        }

        let len = emane_bentpipe_upstreampacket_stripLengthPrefixFraming(pkt);
        let pkt_len = emane_bentpipe_upstreampacket_length(pkt);

        if len > 0 && pkt_len >= len {
            let mut antenna_index: u16 = 0;
            let mut freq_hz: u64 = 0;
            let mut span: u64 = 0;
            let mut start_of_reception: u64 = 0;

            let transponder_index = emane_bentpipe_radiomodel_find_transponder(
                radiomodel,
                msgs,
                &mut antenna_index,
                &mut freq_hz,
                &mut span,
                &mut start_of_reception,
            );

            if transponder_index != -1 {
                let seq_num = emane_bentpipe_commonmacheader_get_sequence_number(hdr);
                emane_bentpipe_radiomodel_enqueue_receive(
                    radiomodel,
                    transponder_index,
                    pkt,
                    msgs,
                    seq_num,
                );
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_bentpipe_radiomodel_processDownstreamPacket(
    radiomodel: *mut c_void,
    pkt: *mut DownstreamPacket,
    _msgs: *const ControlMessages,
) {
    unsafe {
        let priority = emane_bentpipe_downstreampacket_get_priority(pkt);
        let transponder_index =
            emane_bentpipe_radiomodel_get_tos_to_transponder(radiomodel, priority);

        if transponder_index != -1 {
            emane_bentpipe_radiomodel_queue_manager_enqueue(radiomodel, transponder_index, pkt);
        }

        emane_bentpipe_radiomodel_process(radiomodel);
    }
}
