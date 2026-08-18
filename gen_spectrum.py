import os

code = """
use std::collections::{HashMap, HashSet};
use std::ffi::c_void;
use crate::noise_recorder::NoiseRecorder;
use crate::spectral_mask::get_manager;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NoiseMode {
    None = 0,
    OutOfBand = 1,
    Passthrough = 2,
    All = 3,
}

#[repr(C)]
pub struct FfiSegment {
    pub frequency_hz: u64,
    pub offset: i64,
    pub duration: i64,
}

#[repr(C)]
pub struct FfiFrequencySegment {
    pub frequency_hz: u64,
    pub rx_power_dbm: f64,
    pub duration_micros: i64,
    pub offset_micros: i64,
}

#[repr(C)]
pub struct FfiSpectrumUpdate {
    pub tx_time: i64,
    pub propagation_delay: i64,
    pub duration: i64,
    pub segments: *mut FfiFrequencySegment,
    pub segments_len: usize,
    pub b_report_as_in_band: bool,
    pub receiver_sensitivity_milli_watt: f64,
}

#[repr(C)]
pub struct FfiSpectrumWindow {
    pub data: *mut f64,
    pub length: usize,
    pub start_of_window_time: i64,
    pub bin_size_micros: i64,
    pub receiver_sensitivity_milli_watt: f64,
    pub noise_mode_all: bool,
}

#[repr(C)]
pub struct FfiSpectrumFilterWindow {
    pub data: *mut f64,
    pub length: usize,
    pub start_of_window_time: i64,
    pub bin_size_micros: i64,
    pub receiver_sensitivity_milli_watt: f64,
    pub sub_band_bin_count: usize,
}

#[repr(C)]
pub struct FfiDumpFilterResult {
    pub data: *mut f64,
    pub length: usize,
    pub sub_band_bin_count: usize,
}

extern "C" {
    fn emane_rs_filter_match(
        p_filter_match_criterion: *const c_void,
        u64_frequency_hz: u64,
        u64_bandwidth_hz: u64,
        u16_sub_id: u16,
        filter_data_ptr: *const u8,
        filter_data_len: usize,
    ) -> bool;
}

#[derive(Clone)]
struct SpectralSegment {
    overlap_ratio: f64,
    modifier_mw: f64,
    lower_hz: u64,
    upper_hz: u64,
}

#[derive(Clone)]
struct SpectralOverlap {
    segments: Vec<SpectralSegment>,
    lower_hz: u64,
    upper_hz: u64,
}

#[derive(Clone)]
struct MaskOverlap {
    overlaps: Vec<SpectralOverlap>,
    lower_hz: u64,
    upper_hz: u64,
    total: u64,
}

#[derive(Clone)]
struct NoiseRecord {
    p_noise_recorder: *mut NoiseRecorder,
    mask_overlap: MaskOverlap,
    frequency: u64,
    start_rx_freq: u64,
    end_rx_freq: u64,
}

#[derive(Clone)]
struct FilterRecord {
    p_noise_recorder: *mut NoiseRecorder,
    mask_overlap: MaskOverlap,
    tx_freq: u64,
    p_filter_match_criterion: *const c_void,
    start_rx_freq: u64,
    end_rx_freq: u64,
}

pub struct SpectrumMonitor {
    bin_size_micros: i64,
    max_offset_micros: i64,
    max_propagation_micros: i64,
    max_duration_micros: i64,
    b_max_clamp: bool,
    b_exclude_same_sub_id_from_filter: bool,
    time_sync_threshold_micros: i64,
    u64_receiver_bandwidth_hz: u64,
    mode: NoiseMode,
    d_receiver_sensitivity_milli_watt: f64,
    u16_sub_id: u16,
    foi: HashSet<u64>,
    noise_recorder_map: HashMap<u64, Box<NoiseRecorder>>,
    
    // bandwidth Hz or spectral mask -> (Cache, no_overlap_set)
    transmitter_bandwidth_cache: HashMap<u64, (HashMap<u64, Vec<NoiseRecord>>, HashSet<u64>)>,
    transmitter_spectral_mask_cache: HashMap<u64, (HashMap<u64, Vec<NoiseRecord>>, HashSet<u64>)>,

    // filter index -> (frequency, bandwidth, NoiseRecorder, FilterMatchCriterion ptr)
    filter_noise_recorder_map: HashMap<u16, (u64, u64, Box<NoiseRecorder>, *const c_void)>,
    
    filter_transmitter_bandwidth_cache: HashMap<u64, (HashMap<u64, Vec<FilterRecord>>, HashSet<u64>)>,
    filter_transmitter_spectral_mask_cache: HashMap<u64, (HashMap<u64, Vec<FilterRecord>>, HashSet<u64>)>,
}

fn doppler_shift(freq: u64, factor: f64) -> i64 {
    (freq as f64 * factor - freq as f64) as i64
}

fn milliwatt_to_db(mw: f64) -> f64 {
    10.0 * mw.log10()
}

impl SpectrumMonitor {
    pub fn new() -> Self {
        Self {
            bin_size_micros: 0,
            max_offset_micros: 0,
            max_propagation_micros: 0,
            max_duration_micros: 0,
            b_max_clamp: false,
            b_exclude_same_sub_id_from_filter: false,
            time_sync_threshold_micros: 0,
            u64_receiver_bandwidth_hz: 0,
            mode: NoiseMode::None,
            d_receiver_sensitivity_milli_watt: 0.0,
            u16_sub_id: 0,
            foi: HashSet::new(),
            noise_recorder_map: HashMap::new(),
            transmitter_bandwidth_cache: HashMap::new(),
            transmitter_spectral_mask_cache: HashMap::new(),
            filter_noise_recorder_map: HashMap::new(),
            filter_transmitter_bandwidth_cache: HashMap::new(),
            filter_transmitter_spectral_mask_cache: HashMap::new(),
        }
    }

    pub fn initialize(
        &mut self,
        u16_sub_id: u16,
        foi: &[u64],
        u64_bandwidth_hz: u64,
        d_receiver_sensitivity_milli_watt: f64,
        mode: NoiseMode,
        bin_size: i64,
        max_offset: i64,
        max_propagation: i64,
        max_duration: i64,
        time_sync_threshold: i64,
        b_max_clamp: bool,
        b_exclude_same_sub_id_from_filter: bool,
    ) {
        self.u16_sub_id = u16_sub_id;
        self.bin_size_micros = bin_size;
        self.mode = mode;
        self.max_offset_micros = max_offset;
        self.max_propagation_micros = max_propagation;
        self.max_duration_micros = max_duration;
        self.b_max_clamp = b_max_clamp;
        self.b_exclude_same_sub_id_from_filter = b_exclude_same_sub_id_from_filter;
        self.time_sync_threshold_micros = time_sync_threshold;
        self.d_receiver_sensitivity_milli_watt = d_receiver_sensitivity_milli_watt;
        self.u64_receiver_bandwidth_hz = u64_bandwidth_hz;
        
        self.foi.clear();
        for &f in foi {
            self.foi.insert(f);
        }

        self.transmitter_bandwidth_cache.clear();
        self.transmitter_bandwidth_cache.insert(u64_bandwidth_hz, (HashMap::new(), HashSet::new()));
        
        self.noise_recorder_map.clear();
        for &f in foi {
            let nr = NoiseRecorder::new(
                bin_size, max_offset, max_propagation, max_duration,
                d_receiver_sensitivity_milli_watt, f, u64_bandwidth_hz, 0
            );
            self.noise_recorder_map.insert(f, Box::new(nr));
        }
    }

    // (Implementing the rest of the logic shortly)
}
"""

with open("rust/emane-core/src/spectrum_monitor.rs", "w") as f:
    f.write(code)
