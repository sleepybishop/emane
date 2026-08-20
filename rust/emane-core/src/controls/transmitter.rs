#[derive(Clone)]
pub struct Transmitter {
    pub nem_id: u16,
    pub power_dbm: f64,
}

#[derive(Clone)]
pub struct TransmitterControlMessage {
    pub transmitters: Vec<Transmitter>,
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_controls_transmitter_create() -> *mut TransmitterControlMessage {
    Box::into_raw(Box::new(TransmitterControlMessage {
        transmitters: Vec::new(),
    }))
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_controls_transmitter_add(
    msg: *mut TransmitterControlMessage,
    nem_id: u16,
    power_dbm: f64,
) {
    if let Some(msg) = unsafe { msg.as_mut() } {
        msg.transmitters.push(Transmitter { nem_id, power_dbm });
    }
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_controls_transmitter_clone(
    msg: *const TransmitterControlMessage,
) -> *mut TransmitterControlMessage {
    if let Some(msg) = unsafe { msg.as_ref() } {
        Box::into_raw(Box::new(msg.clone()))
    } else {
        std::ptr::null_mut()
    }
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_controls_transmitter_destroy(
    msg: *mut TransmitterControlMessage,
) {
    if !msg.is_null() {
        unsafe { drop(Box::from_raw(msg)) }
    }
}
