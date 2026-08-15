use crate::protobufs::emane_message;
use prost::Message;
use std::os::raw::c_void;
use std::slice;

#[repr(C)]
pub struct EmaneRsPathloss {
    pub nem_id: u32,
    pub forward_pathloss_db: f32,
    pub reverse_pathloss_db: f32,
}

#[no_mangle]
pub extern "C" fn emane_rs_pathloss_event_serialize(
    pathlosses: *const EmaneRsPathloss,
    num_pathlosses: usize,
    out_len: *mut usize,
) -> *mut u8 {
    let mut msg = emane_message::PathlossEvent::default();
    
    if num_pathlosses > 0 && !pathlosses.is_null() {
        let slice = unsafe { slice::from_raw_parts(pathlosses, num_pathlosses) };
        for p in slice {
            msg.pathlosses.push(emane_message::pathloss_event::Pathloss {
                nem_id: p.nem_id,
                forward_pathlossd_b: p.forward_pathloss_db,
                reverse_pathlossd_b: p.reverse_pathloss_db,
            });
        }
    }
    
    let mut buf = Vec::with_capacity(msg.encoded_len());
    if msg.encode(&mut buf).is_ok() {
        let mut boxed = buf.into_boxed_slice();
        unsafe { *out_len = boxed.len() };
        let ptr = boxed.as_mut_ptr();
        std::mem::forget(boxed);
        ptr
    } else {
        std::ptr::null_mut()
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_pathloss_event_free_serialize(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        unsafe {
            // Reconstruct the boxed slice to drop it
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)));
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_pathloss_event_deserialize(
    buf: *const u8,
    len: usize,
    out_pathlosses: *mut *mut EmaneRsPathloss,
    out_num: *mut usize,
) -> bool {
    if buf.is_null() || len == 0 {
        return false;
    }
    
    let slice = unsafe { slice::from_raw_parts(buf, len) };
    if let Ok(msg) = emane_message::PathlossEvent::decode(slice) {
        let mut vec = Vec::with_capacity(msg.pathlosses.len());
        for p in msg.pathlosses {
            vec.push(EmaneRsPathloss {
                nem_id: p.nem_id,
                forward_pathloss_db: p.forward_pathlossd_b,
                reverse_pathloss_db: p.reverse_pathlossd_b,
            });
        }
        
        let mut boxed = vec.into_boxed_slice();
        unsafe { 
            *out_num = boxed.len();
            *out_pathlosses = boxed.as_mut_ptr();
        }
        std::mem::forget(boxed);
        true
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_pathloss_event_free_deserialize(ptr: *mut EmaneRsPathloss, len: usize) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)));
        }
    }
}
