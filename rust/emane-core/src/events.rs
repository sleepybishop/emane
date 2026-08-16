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

#[repr(C)]
pub struct EmaneRsAntennaProfile {
    pub nem_id: u32,
    pub profile_id: u32,
    pub antenna_azimuth_degrees: f64,
    pub antenna_elevation_degrees: f64,
}

#[no_mangle]
pub extern "C" fn emane_rs_antennaprofile_event_serialize(
    items: *const EmaneRsAntennaProfile,
    num_items: usize,
    out_len: *mut usize,
) -> *mut u8 {
    let mut msg = emane_message::AntennaProfileEvent::default();
    
    if num_items > 0 && !items.is_null() {
        let slice = unsafe { slice::from_raw_parts(items, num_items) };
        for p in slice {
            msg.profiles.push(emane_message::antenna_profile_event::Profile {
                nem_id: p.nem_id,
                profile_id: p.profile_id,
                antenna_azimuth_degrees: p.antenna_azimuth_degrees,
                antenna_elevation_degrees: p.antenna_elevation_degrees,
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
pub extern "C" fn emane_rs_antennaprofile_event_free_serialize(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)));
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_antennaprofile_event_deserialize(
    buf: *const u8,
    len: usize,
    out_items: *mut *mut EmaneRsAntennaProfile,
    out_num: *mut usize,
) -> bool {
    if buf.is_null() || len == 0 {
        return false;
    }
    
    let slice = unsafe { slice::from_raw_parts(buf, len) };
    if let Ok(msg) = emane_message::AntennaProfileEvent::decode(slice) {
        let mut vec = Vec::with_capacity(msg.profiles.len());
        for p in msg.profiles {
            vec.push(EmaneRsAntennaProfile {
                nem_id: p.nem_id,
                profile_id: p.profile_id,
                antenna_azimuth_degrees: p.antenna_azimuth_degrees,
                antenna_elevation_degrees: p.antenna_elevation_degrees,
            });
        }
        
        let mut boxed = vec.into_boxed_slice();
        unsafe { 
            *out_num = boxed.len();
            *out_items = boxed.as_mut_ptr();
        }
        std::mem::forget(boxed);
        true
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_antennaprofile_event_free_deserialize(ptr: *mut EmaneRsAntennaProfile, len: usize) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)));
        }
    }
}

#[repr(C)]
pub struct EmaneRsCommEffect {
    pub nem_id: u32,
    pub latency_seconds: f32,
    pub jitter_seconds: f32,
    pub probability_loss: f32,
    pub probability_duplicate: f32,
    pub unicast_bit_rate_bps: u64,
    pub broadcast_bit_rate_bps: u64,
}

#[no_mangle]
pub extern "C" fn emane_rs_commeffect_event_serialize(
    items: *const EmaneRsCommEffect,
    num_items: usize,
    out_len: *mut usize,
) -> *mut u8 {
    let mut msg = emane_message::CommEffectEvent::default();
    
    if num_items > 0 && !items.is_null() {
        let slice = unsafe { slice::from_raw_parts(items, num_items) };
        for p in slice {
            msg.comm_effects.push(emane_message::comm_effect_event::CommEffect {
                nem_id: p.nem_id,
                latency_seconds: p.latency_seconds,
                jitter_seconds: p.jitter_seconds,
                probability_loss: p.probability_loss,
                probability_duplicate: p.probability_duplicate,
                unicast_bit_ratebps: p.unicast_bit_rate_bps,
                broadcast_bit_ratebps: p.broadcast_bit_rate_bps,
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
pub extern "C" fn emane_rs_commeffect_event_free_serialize(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)));
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_commeffect_event_deserialize(
    buf: *const u8,
    len: usize,
    out_items: *mut *mut EmaneRsCommEffect,
    out_num: *mut usize,
) -> bool {
    if buf.is_null() || len == 0 {
        return false;
    }
    
    let slice = unsafe { slice::from_raw_parts(buf, len) };
    if let Ok(msg) = emane_message::CommEffectEvent::decode(slice) {
        let mut vec = Vec::with_capacity(msg.comm_effects.len());
        for p in msg.comm_effects {
            vec.push(EmaneRsCommEffect {
                nem_id: p.nem_id,
                latency_seconds: p.latency_seconds,
                jitter_seconds: p.jitter_seconds,
                probability_loss: p.probability_loss,
                probability_duplicate: p.probability_duplicate,
                unicast_bit_rate_bps: p.unicast_bit_ratebps,
                broadcast_bit_rate_bps: p.broadcast_bit_ratebps,
            });
        }
        
        let mut boxed = vec.into_boxed_slice();
        unsafe { 
            *out_num = boxed.len();
            *out_items = boxed.as_mut_ptr();
        }
        std::mem::forget(boxed);
        true
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_commeffect_event_free_deserialize(ptr: *mut EmaneRsCommEffect, len: usize) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)));
        }
    }
}

#[repr(C)]
pub struct EmaneRsFadingSelection {
    pub nem_id: u32,
    pub model: i32,
}

#[no_mangle]
pub extern "C" fn emane_rs_fadingselection_event_serialize(
    items: *const EmaneRsFadingSelection,
    num_items: usize,
    out_len: *mut usize,
) -> *mut u8 {
    let mut msg = emane_message::FadingSelectionEvent::default();
    
    if num_items > 0 && !items.is_null() {
        let slice = unsafe { slice::from_raw_parts(items, num_items) };
        for p in slice {
            msg.entries.push(emane_message::fading_selection_event::Entry {
                nem_id: p.nem_id,
                model: p.model,
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
pub extern "C" fn emane_rs_fadingselection_event_free_serialize(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)));
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_fadingselection_event_deserialize(
    buf: *const u8,
    len: usize,
    out_items: *mut *mut EmaneRsFadingSelection,
    out_num: *mut usize,
) -> bool {
    if buf.is_null() || len == 0 {
        return false;
    }
    
    let slice = unsafe { slice::from_raw_parts(buf, len) };
    if let Ok(msg) = emane_message::FadingSelectionEvent::decode(slice) {
        let mut vec = Vec::with_capacity(msg.entries.len());
        for p in msg.entries {
            vec.push(EmaneRsFadingSelection {
                nem_id: p.nem_id,
                model: p.model,
            });
        }
        
        let mut boxed = vec.into_boxed_slice();
        unsafe { 
            *out_num = boxed.len();
            *out_items = boxed.as_mut_ptr();
        }
        std::mem::forget(boxed);
        true
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_fadingselection_event_free_deserialize(ptr: *mut EmaneRsFadingSelection, len: usize) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)));
        }
    }
}
