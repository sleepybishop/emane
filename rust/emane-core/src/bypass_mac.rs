pub struct BypassMac {
    type_: u16,
    sequence_number: u16,
}

#[no_mangle]
pub extern "C" fn emane_rs_bypass_mac_new(type_: u16) -> *mut BypassMac {
    Box::into_raw(Box::new(BypassMac {
        type_,
        sequence_number: 0,
    }))
}

#[no_mangle]
pub extern "C" fn emane_rs_bypass_mac_free(ptr: *mut BypassMac) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr)) }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_bypass_mac_process_upstream(
    ptr: *mut BypassMac,
    hdr_type: u16,
) -> bool {
    let state = unsafe { &mut *ptr };
    hdr_type == state.type_
}

#[no_mangle]
pub extern "C" fn emane_rs_bypass_mac_process_downstream(
    ptr: *mut BypassMac,
) -> u16 {
    let state = unsafe { &mut *ptr };
    let seq = state.sequence_number;
    state.sequence_number = state.sequence_number.wrapping_add(1);
    seq
}
