
use std::os::raw::c_void;

#[derive(Clone)]
pub struct FrequencySegmentRs {
    pub frequency_hz: u64,
    pub power_dbm: f64,
    pub duration_microsec: u64,
    pub offset_microsec: u64,
}

#[derive(Clone)]
pub struct AntennaSelfInterferenceRs {
    pub frequency_group_index: usize,
    pub power_milliwatts: Vec<f64>,
}

#[derive(Clone)]
pub struct MimoTxWhileRxInterferenceControlMessageRs {
    pub frequency_groups: Vec<Vec<FrequencySegmentRs>>,
    pub rx_antenna_interferences: Vec<(u16, Vec<AntennaSelfInterferenceRs>)>,
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_tx_rx_create() -> *mut c_void {
    let msg = Box::new(MimoTxWhileRxInterferenceControlMessageRs {
        frequency_groups: Vec::new(),
        rx_antenna_interferences: Vec::new(),
    });
    Box::into_raw(msg) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_tx_rx_add_frequency_group(
    ptr: *mut c_void,
) -> usize {
    let msg = unsafe { &mut *(ptr as *mut MimoTxWhileRxInterferenceControlMessageRs) };
    msg.frequency_groups.push(Vec::new());
    msg.frequency_groups.len() - 1
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_tx_rx_add_frequency_segment(
    ptr: *mut c_void,
    group_idx: usize,
    frequency_hz: u64,
    power_dbm: f64,
    duration_microsec: u64,
    offset_microsec: u64,
) {
    let msg = unsafe { &mut *(ptr as *mut MimoTxWhileRxInterferenceControlMessageRs) };
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
pub extern "C" fn emane_rs_controls_mimo_tx_rx_add_rx_antenna(
    ptr: *mut c_void,
    antenna_index: u16,
) -> usize {
    let msg = unsafe { &mut *(ptr as *mut MimoTxWhileRxInterferenceControlMessageRs) };
    msg.rx_antenna_interferences.push((antenna_index, Vec::new()));
    msg.rx_antenna_interferences.len() - 1
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_tx_rx_add_interference(
    ptr: *mut c_void,
    map_idx: usize,
    frequency_group_index: usize,
) -> usize {
    let msg = unsafe { &mut *(ptr as *mut MimoTxWhileRxInterferenceControlMessageRs) };
    if let Some(entry) = msg.rx_antenna_interferences.get_mut(map_idx) {
        entry.1.push(AntennaSelfInterferenceRs {
            frequency_group_index,
            power_milliwatts: Vec::new(),
        });
        return entry.1.len() - 1;
    }
    0
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_tx_rx_add_interference_power(
    ptr: *mut c_void,
    map_idx: usize,
    interference_idx: usize,
    power_mw: f64,
) {
    let msg = unsafe { &mut *(ptr as *mut MimoTxWhileRxInterferenceControlMessageRs) };
    if let Some(entry) = msg.rx_antenna_interferences.get_mut(map_idx) {
        if let Some(interference) = entry.1.get_mut(interference_idx) {
            interference.power_milliwatts.push(power_mw);
        }
    }
}


#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_tx_rx_clone(ptr: *const c_void) -> *mut c_void {
    let msg = unsafe { &*(ptr as *const MimoTxWhileRxInterferenceControlMessageRs) };
    let cloned = Box::new(msg.clone());
    Box::into_raw(cloned) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_tx_rx_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe { let _ = Box::from_raw(ptr as *mut MimoTxWhileRxInterferenceControlMessageRs); }
    }
}
