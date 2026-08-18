import sys
import os

with open("src/spectrum_monitor.rs", "w") as f:
    f.write(r'''
use std::collections::{HashMap, HashSet, BTreeMap};
use std::ffi::c_void;
use crate::noise_recorder::NoiseRecorder;
use crate::spectral_mask::{get_manager, frequency_overlap_ratio};

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NoiseMode {
    None = 0,
    All = 1,
    OutOfBand = 2,
    PassThrough = 3,
}

#[repr(C)]
pub struct FfiFrequencySegment {
    pub frequency_hz: u64,
    pub rx_power_dbm: f64,
    pub duration_microsec: i64,
    pub offset_microsec: i64,
}

#[repr(C)]
pub struct FfiSpectrumUpdate {
    pub tx_time: i64,
    pub propagation_delay: i64,
    pub duration: i64,
    pub segments: *mut FfiFrequencySegment,
    pub segments_len: usize,
    pub report_as_in_band: bool,
    pub receiver_sensitivity_mw: f64,
}

#[repr(C)]
pub struct FfiSpectrumWindow {
    pub data: *mut f64,
    pub length: usize,
    pub start_of_window_time: i64,
    pub bin_size_microsec: i64,
    pub receiver_sensitivity_mw: f64,
    pub is_noise_all: bool,
}

#[repr(C)]
pub struct FfiSpectrumFilterWindow {
    pub data: *mut f64,
    pub length: usize,
    pub start_of_window_time: i64,
    pub bin_size_microsec: i64,
    pub receiver_sensitivity_mw: f64,
    pub sub_band_bin_count: usize,
}

extern "C" {
    fn emane_rs_filter_match(
        criterion: *const c_void,
        freq: u64,
        bw: u64,
        subid: u16,
        filter_data_ptr: *const u8,
        filter_data_len: usize,
    ) -> bool;
}

type MaskOverlap = (Vec<(Vec<(f64, f64, u64, u64)>, u64, u64)>, u64, u64, u64);

type NoiseRecord = (
    u64, // rx freq
    MaskOverlap,
    u64, // start rx freq
    u64  // end rx freq
);

type FilterRecord = (
    u16, // filter index
    MaskOverlap,
    u64, // tx freq
    *const c_void, // filter match criterion
    u64, // start rx freq
    u64  // end rx freq
);

unsafe impl Send for SpectrumMonitor {}
unsafe impl Sync for SpectrumMonitor {}

pub struct SpectrumMonitor {
    bin_size: i64,
    max_offset: i64,
    max_propagation: i64,
    max_duration: i64,
    b_max_clamp: bool,
    b_exclude_same_sub_id_from_filter: bool,
    time_sync_threshold: i64,

    transmitter_bandwidth_cache: HashMap<u64, (HashMap<u64, Vec<NoiseRecord>>, HashSet<u64>)>,
    transmitter_spectral_mask_cache: HashMap<u64, (HashMap<u64, Vec<NoiseRecord>>, HashSet<u64>)>,

    noise_recorder_map: HashMap<u64, Box<NoiseRecorder>>,

    u64_receiver_bandwidth_hz: u64,
    mode: NoiseMode,
    d_receiver_sensitivity_milli_watt: f64,
    u16_sub_id: u16,
    foi: HashSet<u64>,

    filter_noise_recorder_map: HashMap<u16, (u64, u64, Box<NoiseRecorder>, *const c_void)>,
    filter_transmitter_bandwidth_cache: HashMap<u64, (HashMap<u64, Vec<FilterRecord>>, HashSet<u64>)>,
    filter_transmitter_spectral_mask_cache: HashMap<u64, (HashMap<u64, Vec<FilterRecord>>, HashSet<u64>)>,
}

fn doppler_shift(freq: u64, doppler_factor: f64) -> i64 {
    (freq as f64 * doppler_factor).round() as i64
}

fn milliwatt_to_dbm(mw: f64) -> f64 {
    10.0 * mw.log10()
}

impl SpectrumMonitor {
    pub fn new() -> Self {
        Self {
            bin_size: 0,
            max_offset: 0,
            max_propagation: 0,
            max_duration: 0,
            b_max_clamp: false,
            b_exclude_same_sub_id_from_filter: false,
            time_sync_threshold: 0,
            transmitter_bandwidth_cache: HashMap::new(),
            transmitter_spectral_mask_cache: HashMap::new(),
            noise_recorder_map: HashMap::new(),
            u64_receiver_bandwidth_hz: 0,
            mode: NoiseMode::None,
            d_receiver_sensitivity_milli_watt: 0.0,
            u16_sub_id: 0,
            foi: HashSet::new(),
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
        self.bin_size = bin_size;
        self.mode = mode;
        self.max_offset = max_offset;
        self.max_propagation = max_propagation;
        self.max_duration = max_duration;
        self.b_max_clamp = b_max_clamp;
        self.b_exclude_same_sub_id_from_filter = b_exclude_same_sub_id_from_filter;
        self.time_sync_threshold = time_sync_threshold;
        self.d_receiver_sensitivity_milli_watt = d_receiver_sensitivity_milli_watt;
        self.u64_receiver_bandwidth_hz = u64_bandwidth_hz;
        self.foi = foi.iter().copied().collect();

        self.transmitter_bandwidth_cache.clear();
        self.transmitter_bandwidth_cache.insert(u64_bandwidth_hz, (HashMap::new(), HashSet::new()));
        
        self.noise_recorder_map.clear();
        for &freq in &self.foi {
            self.noise_recorder_map.insert(
                freq,
                Box::new(NoiseRecorder::new(
                    bin_size,
                    max_offset,
                    max_propagation,
                    max_duration,
                    d_receiver_sensitivity_milli_watt,
                    freq,
                    u64_bandwidth_hz,
                    0,
                )),
            );
        }
    }

    pub fn get_frequencies(&self) -> Vec<u64> {
        self.foi.iter().copied().collect()
    }

    pub fn get_receiver_sensitivity_dbm(&self) -> f64 {
        milliwatt_to_dbm(self.d_receiver_sensitivity_milli_watt)
    }

    pub fn dump(&self, u64_frequency_hz: u64) -> Vec<f64> {
        if let Some(recorder) = self.noise_recorder_map.get(&u64_frequency_hz) {
            recorder.dump()
        } else {
            Vec::new()
        }
    }

    pub fn dump_filter(&self, filter_index: u16) -> (Vec<f64>, usize) {
        if let Some((_, _, recorder, _)) = self.filter_noise_recorder_map.get(&filter_index) {
            (recorder.dump(), recorder.get_sub_band_bin_count())
        } else {
            (Vec::new(), 0)
        }
    }

    pub fn request_i(&self, now: i64, u64_frequency_hz: u64, duration: i64, timepoint: i64) -> (Vec<f64>, i64, i64, f64, bool) {
        let mut valid_duration = duration;
        if valid_duration > self.max_duration {
            if self.b_max_clamp {
                valid_duration = self.max_duration;
            } else {
                panic!("duration > max_duration");
            }
        }
        if let Some(recorder) = self.noise_recorder_map.get(&u64_frequency_hz) {
            let (vec, start) = recorder.get(now, valid_duration, timepoint);
            (vec, start, self.bin_size, self.d_receiver_sensitivity_milli_watt, self.mode == NoiseMode::All)
        } else {
            (Vec::new(), 0, self.bin_size, self.d_receiver_sensitivity_milli_watt, false)
        }
    }

    pub fn request_filter_i(&self, now: i64, filter_index: u16, duration: i64, timepoint: i64) -> (Vec<f64>, i64, i64, f64, usize) {
        let mut valid_duration = duration;
        if valid_duration > self.max_duration {
            if self.b_max_clamp {
                valid_duration = self.max_duration;
            } else {
                panic!("duration > max_duration");
            }
        }
        if let Some((_, _, recorder, _)) = self.filter_noise_recorder_map.get(&filter_index) {
            let (vec, start) = recorder.get(now, valid_duration, timepoint);
            (vec, start, self.bin_size, self.d_receiver_sensitivity_milli_watt, recorder.get_sub_band_bin_count())
        } else {
            panic!("Unknown filter id");
        }
    }

    pub fn initialize_filter(
        &mut self,
        filter_index: u16,
        u64_frequency_hz: u64,
        u64_bandwidth_hz: u64,
        u64_bandwidth_bin_size_hz: u64,
        p_filter_match_criterion: *const c_void,
    ) {
        if self.filter_noise_recorder_map.contains_key(&filter_index) {
            panic!("Filter id already present");
        }
        self.filter_transmitter_bandwidth_cache.clear();
        self.filter_noise_recorder_map.insert(
            filter_index,
            (
                u64_frequency_hz,
                u64_bandwidth_hz,
                Box::new(NoiseRecorder::new(
                    self.bin_size,
                    self.max_offset,
                    self.max_propagation,
                    self.max_duration,
                    self.d_receiver_sensitivity_milli_watt,
                    u64_frequency_hz,
                    u64_bandwidth_hz,
                    u64_bandwidth_bin_size_hz,
                )),
                p_filter_match_criterion,
            ),
        );
    }

    pub fn remove_filter(&mut self, filter_index: u16) {
        if self.filter_noise_recorder_map.remove(&filter_index).is_some() {
            self.filter_transmitter_bandwidth_cache.clear();
        }
    }

    pub fn update(
        &mut self,
        now: i64,
        tx_time: i64,
        propagation_delay: i64,
        d_doppler_factor: f64,
        segments: &[FfiFrequencySegment],
        u64_segment_bandwidth_hz: u64,
        rx_powers_milli_watt: &[f64],
        b_in_band: bool,
        transmitters: &[u16],
        u16_sub_id: u16,
        tx_antenna_index: u16,
        spectral_mask_index: u16,
        filter_data_ptr: *const u8,
        filter_data_len: usize,
    ) -> (i64, i64, i64, Vec<FfiFrequencySegment>, bool, f64) {
        // dummy update to fix compilation issue while I build the rest.
        (0, 0, 0, Vec::new(), false, 0.0)
    }
}

// TODO update

// FFI 
#[no_mangle]
pub extern "C" fn emane_rs_spectrum_monitor_new() -> *mut c_void {
    Box::into_raw(Box::new(SpectrumMonitor::new())) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_spectrum_monitor_free(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe { let _ = Box::from_raw(ptr as *mut SpectrumMonitor); }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_spectrum_monitor_initialize(
    ptr: *mut c_void,
    u16_sub_id: u16,
    foi_ptr: *const u64,
    foi_len: usize,
    u64_bandwidth_hz: u64,
    d_receiver_sensitivity_milli_watt: f64,
    mode: u32,
    bin_size: i64,
    max_offset: i64,
    max_propagation: i64,
    max_duration: i64,
    time_sync_threshold: i64,
    b_max_clamp: bool,
    b_exclude_same_sub_id_from_filter: bool,
) {
    if ptr.is_null() { return; }
    let monitor = unsafe { &mut *(ptr as *mut SpectrumMonitor) };
    let foi = unsafe { std::slice::from_raw_parts(foi_ptr, foi_len) };
    let m = match mode {
        0 => NoiseMode::None,
        1 => NoiseMode::All,
        2 => NoiseMode::OutOfBand,
        3 => NoiseMode::PassThrough,
        _ => NoiseMode::None,
    };
    monitor.initialize(
        u16_sub_id,
        foi,
        u64_bandwidth_hz,
        d_receiver_sensitivity_milli_watt,
        m,
        bin_size,
        max_offset,
        max_propagation,
        max_duration,
        time_sync_threshold,
        b_max_clamp,
        b_exclude_same_sub_id_from_filter,
    );
}

#[no_mangle]
pub extern "C" fn emane_rs_spectrum_monitor_update(
    ptr: *mut c_void,
    now: i64,
    tx_time: i64,
    propagation_delay: i64,
    d_doppler_factor: f64,
    segments_ptr: *const FfiFrequencySegment,
    segments_len: usize,
    u64_segment_bandwidth_hz: u64,
    rx_powers_milli_watt_ptr: *const f64,
    rx_powers_milli_watt_len: usize,
    b_in_band: bool,
    transmitters_ptr: *const u16,
    transmitters_len: usize,
    u16_sub_id: u16,
    tx_antenna_index: u16,
    spectral_mask_index: u16,
    filter_data_ptr: *const u8,
    filter_data_len: usize,
) -> FfiSpectrumUpdate {
    if ptr.is_null() || segments_len != rx_powers_milli_watt_len {
        return FfiSpectrumUpdate {
            tx_time: 0,
            propagation_delay: 0,
            duration: 0,
            segments: std::ptr::null_mut(),
            segments_len: 0,
            report_as_in_band: false,
            receiver_sensitivity_mw: 0.0,
        };
    }
    let monitor = unsafe { &mut *(ptr as *mut SpectrumMonitor) };
    let segments = unsafe { std::slice::from_raw_parts(segments_ptr, segments_len) };
    let rx_powers_milli_watt = unsafe { std::slice::from_raw_parts(rx_powers_milli_watt_ptr, rx_powers_milli_watt_len) };
    let transmitters = unsafe { std::slice::from_raw_parts(transmitters_ptr, transmitters_len) };
    
    let (t, p, d, mut segs, r, s) = monitor.update(
        now,
        tx_time,
        propagation_delay,
        d_doppler_factor,
        segments,
        u64_segment_bandwidth_hz,
        rx_powers_milli_watt,
        b_in_band,
        transmitters,
        u16_sub_id,
        tx_antenna_index,
        spectral_mask_index,
        filter_data_ptr,
        filter_data_len,
    );
    
    segs.shrink_to_fit();
    let slen = segs.len();
    let sptr = segs.leak().as_mut_ptr();
    
    FfiSpectrumUpdate {
        tx_time: t,
        propagation_delay: p,
        duration: d,
        segments: sptr,
        segments_len: slen,
        report_as_in_band: r,
        receiver_sensitivity_mw: s,
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_spectrum_monitor_free_update_segments(segments: *mut FfiFrequencySegment, len: usize) {
    if !segments.is_null() {
        unsafe { let _ = Vec::from_raw_parts(segments, len, len); }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_spectrum_monitor_get_frequencies(
    ptr: *const c_void,
    out_freqs: *mut u64,
    max_len: usize,
) -> usize {
    if ptr.is_null() { return 0; }
    let monitor = unsafe { &*(ptr as *const SpectrumMonitor) };
    let freqs = monitor.get_frequencies();
    let count = std::cmp::min(freqs.len(), max_len);
    unsafe {
        std::ptr::copy_nonoverlapping(freqs.as_ptr(), out_freqs, count);
    }
    count
}

#[no_mangle]
pub extern "C" fn emane_rs_spectrum_monitor_get_receiver_sensitivity_dbm(ptr: *const c_void) -> f64 {
    if ptr.is_null() { return 0.0; }
    let monitor = unsafe { &*(ptr as *const SpectrumMonitor) };
    monitor.get_receiver_sensitivity_dbm()
}

#[no_mangle]
pub extern "C" fn emane_rs_spectrum_monitor_dump(
    ptr: *const c_void,
    u64_frequency_hz: u64,
    out_data: *mut *mut f64,
    out_len: *mut usize,
) {
    if ptr.is_null() { return; }
    let monitor = unsafe { &*(ptr as *const SpectrumMonitor) };
    let mut data = monitor.dump(u64_frequency_hz);
    data.shrink_to_fit();
    unsafe {
        *out_len = data.len();
        *out_data = data.leak().as_mut_ptr();
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_spectrum_monitor_dump_filter(
    ptr: *const c_void,
    filter_index: u16,
    out_data: *mut *mut f64,
    out_len: *mut usize,
    out_sub_band_bin_count: *mut usize,
) {
    if ptr.is_null() { return; }
    let monitor = unsafe { &*(ptr as *const SpectrumMonitor) };
    let (mut data, count) = monitor.dump_filter(filter_index);
    data.shrink_to_fit();
    unsafe {
        *out_len = data.len();
        *out_sub_band_bin_count = count;
        *out_data = data.leak().as_mut_ptr();
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_spectrum_monitor_request_i(
    ptr: *const c_void,
    now: i64,
    u64_frequency_hz: u64,
    duration: i64,
    timepoint: i64,
) -> FfiSpectrumWindow {
    if ptr.is_null() {
        return FfiSpectrumWindow {
            data: std::ptr::null_mut(),
            length: 0,
            start_of_window_time: 0,
            bin_size_microsec: 0,
            receiver_sensitivity_mw: 0.0,
            is_noise_all: false,
        };
    }
    let monitor = unsafe { &*(ptr as *const SpectrumMonitor) };
    let (mut data, start, bin, sens, is_all) = monitor.request_i(now, u64_frequency_hz, duration, timepoint);
    data.shrink_to_fit();
    let length = data.len();
    let data_ptr = data.leak().as_mut_ptr();
    FfiSpectrumWindow {
        data: data_ptr,
        length,
        start_of_window_time: start,
        bin_size_microsec: bin,
        receiver_sensitivity_mw: sens,
        is_noise_all: is_all,
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_spectrum_monitor_free_window_data(data: *mut f64, len: usize) {
    if !data.is_null() {
        unsafe {
            let _ = Vec::from_raw_parts(data, len, len);
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_spectrum_monitor_initialize_filter(
    ptr: *mut c_void,
    filter_index: u16,
    u64_frequency_hz: u64,
    u64_bandwidth_hz: u64,
    u64_bandwidth_bin_size_hz: u64,
    p_filter_match_criterion: *const c_void,
) {
    if ptr.is_null() { return; }
    let monitor = unsafe { &mut *(ptr as *mut SpectrumMonitor) };
    monitor.initialize_filter(
        filter_index,
        u64_frequency_hz,
        u64_bandwidth_hz,
        u64_bandwidth_bin_size_hz,
        p_filter_match_criterion,
    );
}

#[no_mangle]
pub extern "C" fn emane_rs_spectrum_monitor_remove_filter(ptr: *mut c_void, filter_index: u16) {
    if ptr.is_null() { return; }
    let monitor = unsafe { &mut *(ptr as *mut SpectrumMonitor) };
    monitor.remove_filter(filter_index);
}

#[no_mangle]
pub extern "C" fn emane_rs_spectrum_monitor_request_filter_i(
    ptr: *const c_void,
    now: i64,
    filter_index: u16,
    duration: i64,
    timepoint: i64,
) -> FfiSpectrumFilterWindow {
    if ptr.is_null() {
        return FfiSpectrumFilterWindow {
            data: std::ptr::null_mut(),
            length: 0,
            start_of_window_time: 0,
            bin_size_microsec: 0,
            receiver_sensitivity_mw: 0.0,
            sub_band_bin_count: 0,
        };
    }
    let monitor = unsafe { &*(ptr as *const SpectrumMonitor) };
    let (mut data, start, bin, sens, count) = monitor.request_filter_i(now, filter_index, duration, timepoint);
    data.shrink_to_fit();
    let length = data.len();
    let data_ptr = data.leak().as_mut_ptr();
    FfiSpectrumFilterWindow {
        data: data_ptr,
        length,
        start_of_window_time: start,
        bin_size_microsec: bin,
        receiver_sensitivity_mw: sens,
        sub_band_bin_count: count,
    }
}
''')
