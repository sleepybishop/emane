use std::collections::HashSet;

#[derive(Clone, Default)]
pub struct FrequencyOfInterestControlMessage {
    pub bandwidth_hz: u64,
    pub frequency_set: HashSet<u64>,
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_controls_foi_create(
    bandwidth_hz: u64,
) -> *mut FrequencyOfInterestControlMessage {
    let msg = Box::new(FrequencyOfInterestControlMessage {
        bandwidth_hz,
        frequency_set: HashSet::new(),
    });
    Box::into_raw(msg)
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_controls_foi_add_frequency(
    ptr: *mut FrequencyOfInterestControlMessage,
    freq: u64,
) {
    if let Some(msg) = unsafe { ptr.as_mut() } {
        msg.frequency_set.insert(freq);
    }
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_controls_foi_clone(
    ptr: *const FrequencyOfInterestControlMessage,
) -> *mut FrequencyOfInterestControlMessage {
    if let Some(msg) = unsafe { ptr.as_ref() } {
        Box::into_raw(Box::new(msg.clone()))
    } else {
        std::ptr::null_mut()
    }
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_controls_foi_destroy(
    ptr: *mut FrequencyOfInterestControlMessage,
) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr)) }
    }
}
