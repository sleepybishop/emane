use std::os::raw::c_void;

#[derive(Clone)]
pub struct FrequencySegmentRs {
    pub frequency_hz: u64,
    pub power_dbm: f64,
    pub duration_microsec: u64,
    pub offset_microsec: u64,
}

#[derive(Clone)]
pub struct AntennaRs {
    pub frequency_group_index: usize,
    pub index: u16,
    pub bandwidth_hz: u64,
    pub spectral_mask_index: u16,
}

#[derive(Clone)]
pub struct MimoTransmitPropertiesControlMessageRs {
    pub frequency_groups: Vec<Vec<FrequencySegmentRs>>,
    pub transmit_antennas: Vec<AntennaRs>,
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_tx_props_create() -> *mut c_void {
    let msg = Box::new(MimoTransmitPropertiesControlMessageRs {
        frequency_groups: Vec::new(),
        transmit_antennas: Vec::new(),
    });
    Box::into_raw(msg) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_tx_props_add_frequency_group(ptr: *mut c_void) -> usize {
    let msg = unsafe { &mut *(ptr as *mut MimoTransmitPropertiesControlMessageRs) };
    msg.frequency_groups.push(Vec::new());
    msg.frequency_groups.len() - 1
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_tx_props_add_frequency_segment(
    ptr: *mut c_void,
    group_idx: usize,
    frequency_hz: u64,
    power_dbm: f64,
    duration_microsec: u64,
    offset_microsec: u64,
) {
    let msg = unsafe { &mut *(ptr as *mut MimoTransmitPropertiesControlMessageRs) };
    if let Some(group) = msg.frequency_groups.get_mut(group_idx) {
        group.push(FrequencySegmentRs {
            frequency_hz,
            power_dbm,
            duration_microsec,
            offset_microsec,
        });
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_tx_props_add_antenna(
    ptr: *mut c_void,
    frequency_group_index: usize,
    index: u16,
    bandwidth_hz: u64,
    spectral_mask_index: u16,
) {
    let msg = unsafe { &mut *(ptr as *mut MimoTransmitPropertiesControlMessageRs) };
    msg.transmit_antennas.push(AntennaRs {
        frequency_group_index,
        index,
        bandwidth_hz,
        spectral_mask_index,
    });
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_tx_props_clone(ptr: *const c_void) -> *mut c_void {
    let msg = unsafe { &*(ptr as *const MimoTransmitPropertiesControlMessageRs) };
    let cloned = Box::new(msg.clone());
    Box::into_raw(cloned) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_tx_props_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr as *mut MimoTransmitPropertiesControlMessageRs);
        }
    }
}
