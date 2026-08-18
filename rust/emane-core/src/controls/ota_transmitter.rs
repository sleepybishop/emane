use std::collections::BTreeSet;

#[derive(Clone)]
pub struct OtaTransmitterControlMessage {
    transmitters: BTreeSet<u16>,
}

impl OtaTransmitterControlMessage {
    pub fn new(transmitters: BTreeSet<u16>) -> Self {
        Self { transmitters }
    }
}

#[no_mangle]
pub unsafe extern "C" fn emane_ota_transmitter_control_message_create(
    nem_ids: *const u16,
    num_nem_ids: usize,
) -> *mut OtaTransmitterControlMessage {
    let mut transmitters = BTreeSet::new();
    if !nem_ids.is_null() && num_nem_ids > 0 {
        let slice = std::slice::from_raw_parts(nem_ids, num_nem_ids);
        for &id in slice {
            transmitters.insert(id);
        }
    }
    Box::into_raw(Box::new(OtaTransmitterControlMessage::new(transmitters)))
}

#[no_mangle]
pub unsafe extern "C" fn emane_ota_transmitter_control_message_clone(
    msg: *const OtaTransmitterControlMessage,
) -> *mut OtaTransmitterControlMessage {
    if msg.is_null() {
        return std::ptr::null_mut();
    }
    Box::into_raw(Box::new((*msg).clone()))
}

#[no_mangle]
pub unsafe extern "C" fn emane_ota_transmitter_control_message_destroy(
    msg: *mut OtaTransmitterControlMessage,
) {
    if !msg.is_null() {
        drop(Box::from_raw(msg));
    }
}

#[no_mangle]
pub unsafe extern "C" fn emane_ota_transmitter_control_message_get_transmitters(
    msg: *const OtaTransmitterControlMessage,
    out_nem_ids: *mut *mut u16,
    out_num_nem_ids: *mut usize,
) {
    if msg.is_null() || out_nem_ids.is_null() || out_num_nem_ids.is_null() {
        return;
    }
    let transmitters: Vec<u16> = (*msg).transmitters.iter().copied().collect();
    *out_num_nem_ids = transmitters.len();
    if transmitters.is_empty() {
        *out_nem_ids = std::ptr::null_mut();
    } else {
        let mut boxed_slice = transmitters.into_boxed_slice();
        *out_nem_ids = boxed_slice.as_mut_ptr();
        std::mem::forget(boxed_slice);
    }
}

#[no_mangle]
pub unsafe extern "C" fn emane_ota_transmitter_control_message_free_transmitters(
    nem_ids: *mut u16,
    num_nem_ids: usize,
) {
    if !nem_ids.is_null() {
        drop(Vec::from_raw_parts(nem_ids, num_nem_ids, num_nem_ids));
    }
}
