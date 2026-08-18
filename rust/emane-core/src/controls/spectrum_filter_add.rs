use std::os::raw::c_void;

#[repr(C)]
pub struct SpectrumFilterAddControlMessage {
    pub filter_index: u16,
    pub antenna_index: u8,
    pub frequency_hz: u64,
    pub bandwidth_hz: u64,
    pub sub_band_bin_size_hz: u64,
    pub filter_match_criterion: *mut c_void,
}

impl SpectrumFilterAddControlMessage {
    pub fn new(
        filter_index: u16,
        antenna_index: u8,
        frequency_hz: u64,
        bandwidth_hz: u64,
        sub_band_bin_size_hz: u64,
        filter_match_criterion: *mut c_void,
    ) -> Self {
        Self {
            filter_index,
            antenna_index,
            frequency_hz,
            bandwidth_hz,
            sub_band_bin_size_hz,
            filter_match_criterion,
        }
    }
}

impl Clone for SpectrumFilterAddControlMessage {
    fn clone(&self) -> Self {
        Self {
            filter_index: self.filter_index,
            antenna_index: self.antenna_index,
            frequency_hz: self.frequency_hz,
            bandwidth_hz: self.bandwidth_hz,
            sub_band_bin_size_hz: self.sub_band_bin_size_hz,
            filter_match_criterion: unsafe {
                if self.filter_match_criterion.is_null() {
                    std::ptr::null_mut()
                } else {
                    emane_filter_match_criterion_clone(self.filter_match_criterion)
                }
            },
        }
    }
}

impl Drop for SpectrumFilterAddControlMessage {
    fn drop(&mut self) {
        if !self.filter_match_criterion.is_null() {
            unsafe {
                emane_filter_match_criterion_destroy(self.filter_match_criterion);
            }
        }
    }
}

extern "C" {
    fn emane_filter_match_criterion_clone(ptr: *const c_void) -> *mut c_void;
    fn emane_filter_match_criterion_destroy(ptr: *mut c_void);
}

#[no_mangle]
pub unsafe extern "C" fn emane_spectrum_filter_add_control_message_create(
    filter_index: u16,
    antenna_index: u8,
    frequency_hz: u64,
    bandwidth_hz: u64,
    sub_band_bin_size_hz: u64,
    filter_match_criterion: *mut c_void,
) -> *mut SpectrumFilterAddControlMessage {
    Box::into_raw(Box::new(SpectrumFilterAddControlMessage::new(
        filter_index,
        antenna_index,
        frequency_hz,
        bandwidth_hz,
        sub_band_bin_size_hz,
        filter_match_criterion,
    )))
}

#[no_mangle]
pub unsafe extern "C" fn emane_spectrum_filter_add_control_message_clone(
    msg: *const SpectrumFilterAddControlMessage,
) -> *mut SpectrumFilterAddControlMessage {
    if msg.is_null() {
        return std::ptr::null_mut();
    }
    Box::into_raw(Box::new((*msg).clone()))
}

#[no_mangle]
pub unsafe extern "C" fn emane_spectrum_filter_add_control_message_destroy(
    msg: *mut SpectrumFilterAddControlMessage,
) {
    if !msg.is_null() {
        let _ = Box::from_raw(msg);
    }
}

#[no_mangle]
pub unsafe extern "C" fn emane_spectrum_filter_add_control_message_get_filter_index(
    msg: *const SpectrumFilterAddControlMessage,
) -> u16 {
    (*msg).filter_index
}

#[no_mangle]
pub unsafe extern "C" fn emane_spectrum_filter_add_control_message_get_antenna_index(
    msg: *const SpectrumFilterAddControlMessage,
) -> u8 {
    (*msg).antenna_index
}

#[no_mangle]
pub unsafe extern "C" fn emane_spectrum_filter_add_control_message_get_frequency_hz(
    msg: *const SpectrumFilterAddControlMessage,
) -> u64 {
    (*msg).frequency_hz
}

#[no_mangle]
pub unsafe extern "C" fn emane_spectrum_filter_add_control_message_get_bandwidth_hz(
    msg: *const SpectrumFilterAddControlMessage,
) -> u64 {
    (*msg).bandwidth_hz
}

#[no_mangle]
pub unsafe extern "C" fn emane_spectrum_filter_add_control_message_get_sub_band_bin_size_hz(
    msg: *const SpectrumFilterAddControlMessage,
) -> u64 {
    (*msg).sub_band_bin_size_hz
}

#[no_mangle]
pub unsafe extern "C" fn emane_spectrum_filter_add_control_message_get_filter_match_criterion(
    msg: *const SpectrumFilterAddControlMessage,
) -> *const c_void {
    (*msg).filter_match_criterion
}

