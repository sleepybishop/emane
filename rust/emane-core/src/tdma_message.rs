use crate::protobufs::emane_message::tdma_base_model_message::message::{Fragment, MessageType};
use crate::protobufs::emane_message::tdma_base_model_message::Message;
use crate::protobufs::emane_message::TdmaBaseModelMessage;
use prost::Message as ProstMessage;

#[repr(C)]
#[derive(Clone, Copy)]
pub enum FfiTdmaMessageType {
    Data = 0,
    Control = 1,
}

pub struct MessageComponent {
    pub msg_type: FfiTdmaMessageType,
    pub destination: u16,
    pub priority: u8,
    pub data: Vec<u8>,
    pub is_fragment: bool,
    pub fragment_index: u32,
    pub fragment_offset: u32,
    pub fragment_sequence: u64,
    pub more_fragments: bool,
}

pub struct BaseModelMessage {
    pub abs_slot_index: u64,
    pub data_rate_bps: u64,
    pub messages: Vec<MessageComponent>,
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_message_new(
    abs_slot_index: u64,
    data_rate_bps: u64,
) -> *mut BaseModelMessage {
    Box::into_raw(Box::new(BaseModelMessage {
        abs_slot_index,
        data_rate_bps,
        messages: Vec::new(),
    }))
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_message_free(ptr: *mut BaseModelMessage) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr)) }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_message_add_component(
    ptr: *mut BaseModelMessage,
    msg_type: FfiTdmaMessageType,
    destination: u16,
    priority: u8,
    data_ptr: *const u8,
    data_len: usize,
) {
    if ptr.is_null() || data_ptr.is_null() {
        return;
    }
    let msg = unsafe { &mut *ptr };
    let data = unsafe { std::slice::from_raw_parts(data_ptr, data_len) }.to_vec();
    msg.messages.push(MessageComponent {
        msg_type,
        destination,
        priority,
        data,
        is_fragment: false,
        fragment_index: 0,
        fragment_offset: 0,
        fragment_sequence: 0,
        more_fragments: false,
    });
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_message_add_fragment(
    ptr: *mut BaseModelMessage,
    msg_type: FfiTdmaMessageType,
    destination: u16,
    priority: u8,
    data_ptr: *const u8,
    data_len: usize,
    index: u32,
    offset: u32,
    sequence: u64,
    more: bool,
) {
    if ptr.is_null() || data_ptr.is_null() {
        return;
    }
    let msg = unsafe { &mut *ptr };
    let data = unsafe { std::slice::from_raw_parts(data_ptr, data_len) }.to_vec();
    msg.messages.push(MessageComponent {
        msg_type,
        destination,
        priority,
        data,
        is_fragment: true,
        fragment_index: index,
        fragment_offset: offset,
        fragment_sequence: sequence,
        more_fragments: more,
    });
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_message_serialize(
    ptr: *const BaseModelMessage,
    out_len: *mut usize,
) -> *mut u8 {
    if ptr.is_null() || out_len.is_null() {
        return std::ptr::null_mut();
    }
    let msg = unsafe { &*ptr };

    let mut pb = TdmaBaseModelMessage {
        abs_slot_index: msg.abs_slot_index,
        data_ratebps: msg.data_rate_bps,
        ..Default::default()
    };

    for comp in &msg.messages {
        let mut pb_msg = Message {
            r#type: match comp.msg_type {
                FfiTdmaMessageType::Data => MessageType::Data as i32,
                FfiTdmaMessageType::Control => MessageType::Control as i32,
            },
            destination: comp.destination as u32,
            priority: comp.priority as u32,
            data: comp.data.clone(),
            ..Default::default()
        };

        if comp.is_fragment {
            pb_msg.fragment = Some(Fragment {
                index: comp.fragment_index,
                offset: comp.fragment_offset,
                sequence: comp.fragment_sequence,
                more: comp.more_fragments,
            });
        }
        pb.messages.push(pb_msg);
    }

    let mut buf = Vec::new();
    pb.encode(&mut buf).unwrap();

    let len = buf.len();
    unsafe { *out_len = len };

    let out_ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    out_ptr
}

#[no_mangle]
pub extern "C" fn emane_rs_tdma_message_deserialize(
    data_ptr: *const u8,
    data_len: usize,
) -> *mut BaseModelMessage {
    if data_ptr.is_null() {
        return std::ptr::null_mut();
    }
    let data = unsafe { std::slice::from_raw_parts(data_ptr, data_len) };
    if let Ok(pb) = TdmaBaseModelMessage::decode(data) {
        let mut messages = Vec::new();
        for msg in pb.messages {
            let msg_type = if msg.r#type == MessageType::Data as i32 {
                FfiTdmaMessageType::Data
            } else {
                FfiTdmaMessageType::Control
            };
            let mut is_frag = false;
            let mut f_index = 0;
            let mut f_offset = 0;
            let mut f_seq = 0;
            let mut f_more = false;

            if let Some(f) = msg.fragment {
                is_frag = true;
                f_index = f.index;
                f_offset = f.offset;
                f_seq = f.sequence;
                f_more = f.more;
            }

            messages.push(MessageComponent {
                msg_type,
                destination: msg.destination as u16,
                priority: msg.priority as u8,
                data: msg.data,
                is_fragment: is_frag,
                fragment_index: f_index,
                fragment_offset: f_offset,
                fragment_sequence: f_seq,
                more_fragments: f_more,
            });
        }
        Box::into_raw(Box::new(BaseModelMessage {
            abs_slot_index: pb.abs_slot_index,
            data_rate_bps: pb.data_ratebps,
            messages,
        }))
    } else {
        std::ptr::null_mut()
    }
}
