use std::os::raw::c_void;
use std::slice;

#[derive(Clone)]
pub struct SerializedControlMessage {
    serialized_id: u16,
    serialization: Vec<u8>,
}

#[no_mangle]
pub unsafe extern "C" fn emane_controls_serialized_create(
    id: u16,
    data: *const c_void,
    length: usize,
) -> *mut SerializedControlMessage {
    let slice = if length > 0 && !data.is_null() {
        slice::from_raw_parts(data as *const u8, length)
    } else {
        &[]
    };
    
    let msg = Box::new(SerializedControlMessage {
        serialized_id: id,
        serialization: slice.to_vec(),
    });
    
    Box::into_raw(msg)
}

#[no_mangle]
pub unsafe extern "C" fn emane_controls_serialized_destroy(msg: *mut SerializedControlMessage) {
    if !msg.is_null() {
        drop(Box::from_raw(msg));
    }
}

#[no_mangle]
pub unsafe extern "C" fn emane_controls_serialized_clone(
    msg: *const SerializedControlMessage,
) -> *mut SerializedControlMessage {
    if msg.is_null() {
        return std::ptr::null_mut();
    }
    let clone = Box::new((*msg).clone());
    Box::into_raw(clone)
}

#[no_mangle]
pub unsafe extern "C" fn emane_controls_serialized_get_id(
    msg: *const SerializedControlMessage,
) -> u16 {
    if msg.is_null() {
        return 0;
    }
    (*msg).serialized_id
}

#[no_mangle]
pub unsafe extern "C" fn emane_controls_serialized_get_serialization_data(
    msg: *const SerializedControlMessage,
) -> *const c_void {
    if msg.is_null() {
        return std::ptr::null();
    }
    (*msg).serialization.as_ptr() as *const c_void
}

#[no_mangle]
pub unsafe extern "C" fn emane_controls_serialized_get_serialization_length(
    msg: *const SerializedControlMessage,
) -> usize {
    if msg.is_null() {
        return 0;
    }
    (*msg).serialization.len()
}
