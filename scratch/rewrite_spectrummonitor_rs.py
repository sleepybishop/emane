import os
rust_code = """
use std::collections::{HashMap, HashSet, BTreeMap};
use std::sync::{Arc, Mutex};
use std::os::raw::c_void;

use crate::noise_recorder::NoiseRecorder;
use crate::spectral_mask::get_manager;

pub type FilterMatchCallback = extern "C" fn(
    *mut c_void, 
    u64, 
    u64, 
    u16, 
    *const u8, 
    usize
) -> bool;

#[repr(C)]
pub struct FfiSpectrumUpdate {
    pub tx_time: i64,
    pub propagation_delay: i64,
    pub duration: i64,
    pub segments: *mut FfiReportableFrequencySegment,
    pub segments_len: usize,
    pub b_report_as_in_band: bool,
    pub d_receiver_sensitivity_milli_watt: f64,
}

#[repr(C)]
pub struct FfiReportableFrequencySegment {
    pub frequency_hz: u64,
    pub rx_power_dbm: f64,
    pub duration: i64,
    pub offset: i64,
}

#[repr(C)]
pub struct FfiFrequencySegment {
    pub frequency_hz: u64,
    pub duration: i64,
    pub offset: i64,
}

#[repr(C)]
pub struct FfiSpectrumWindow {
    pub noise_data: *mut f64,
    pub noise_data_len: usize,
    pub start_of_window_time: i64,
    pub bin_size: i64,
    pub rx_sensitivity_milli_watt: f64,
    pub is_all_mode: bool,
}

#[repr(C)]
pub struct FfiSpectrumFilterWindow {
    pub noise_data: *mut f64,
    pub noise_data_len: usize,
    pub start_of_window_time: i64,
    pub bin_size: i64,
    pub rx_sensitivity_milli_watt: f64,
    pub sub_band_bin_count: usize,
}

#[repr(C)]
pub struct FfiFilterData {
    pub data: *const u8,
    pub data_len: usize,
    pub is_valid: bool,
}

const NOISE_MODE_NONE: i32 = 0;
const NOISE_MODE_ALL: i32 = 1;
const NOISE_MODE_OUTOFBAND: i32 = 2;
const NOISE_MODE_PASSTHROUGH: i32 = 3;

// implement the SpectrumMonitor struct and logic here...
"""

with open("rust/emane-core/src/spectrum_monitor.rs", "w") as f:
    f.write(rust_code)
