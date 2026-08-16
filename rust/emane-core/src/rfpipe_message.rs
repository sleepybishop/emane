use crate::protobufs::emane_message::RfPipeMacHeader;
use prost::Message as ProstMessage;

pub struct RFPipeMACHeaderMessage {
    pub data_rate: u64,
}

#[no_mangle]
pub extern "C" fn emane_rs_rfpipe_message_new(data_rate: u64) -> *mut RFPipeMACHeaderMessage {
    Box::into_raw(Box::new(RFPipeMACHeaderMessage { data_rate }))
}

#[no_mangle]
pub extern "C" fn emane_rs_rfpipe_message_free(ptr: *mut RFPipeMACHeaderMessage) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr)) }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_rfpipe_message_serialize(
    ptr: *const RFPipeMACHeaderMessage,
    out_len: *mut usize,
) -> *mut u8 {
    if ptr.is_null() || out_len.is_null() {
        return std::ptr::null_mut();
    }
    let msg = unsafe { &*ptr };

    let mut pb = RfPipeMacHeader::default();
    pb.data_rate = msg.data_rate;

    let mut buf = Vec::new();
    pb.encode(&mut buf).unwrap();

    let len = buf.len();
    unsafe { *out_len = len };

    let out_ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    out_ptr
}

#[no_mangle]
pub extern "C" fn emane_rs_rfpipe_message_deserialize(
    data_ptr: *const u8,
    data_len: usize,
) -> *mut RFPipeMACHeaderMessage {
    if data_ptr.is_null() {
        return std::ptr::null_mut();
    }
    let data = unsafe { std::slice::from_raw_parts(data_ptr, data_len) };
    if let Ok(pb) = RfPipeMacHeader::decode(data) {
        Box::into_raw(Box::new(RFPipeMACHeaderMessage {
            data_rate: pb.data_rate,
        }))
    } else {
        std::ptr::null_mut()
    }
}
