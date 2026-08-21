use crate::noise_recorder::NoiseRecorder;
use crate::spectral_mask::get_manager as get_spectral_mask_manager;
use std::collections::{HashMap, HashSet};
use std::ffi::c_void;

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

#[repr(C)]
pub struct FfiFilterMatchCriterion {
    pub context: *mut c_void,
    pub matches: extern "C" fn(
        context: *mut c_void,
        frequency_hz: u64,
        bandwidth_hz: u64,
        sub_id: u16,
        filter_data: *const u8,
        filter_data_len: usize,
    ) -> bool,
}

type MaskOverlap = (Vec<(Vec<(f64, f64, u64, u64)>, u64, u64)>, u64, u64, u64);

type NoiseRecord = (
    u64, // rx freq
    MaskOverlap,
    u64, // start rx freq
    u64, // end rx freq
);

type FilterRecord = (
    u16, // filter index
    MaskOverlap,
    u64,           // tx freq
    *const c_void, // filter match criterion
    u64,           // start rx freq
    u64,           // end rx freq
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
    filter_transmitter_bandwidth_cache:
        HashMap<u64, (HashMap<u64, Vec<FilterRecord>>, HashSet<u64>)>,
    filter_transmitter_spectral_mask_cache:
        HashMap<u64, (HashMap<u64, Vec<FilterRecord>>, HashSet<u64>)>,
}

fn doppler_shift(freq: u64, doppler_factor: f64) -> i64 {
    let shift = (freq as f64 * doppler_factor).round();
    if !shift.is_finite() {
        0
    } else {
        shift.clamp(i64::MIN as f64, i64::MAX as f64) as i64
    }
}

fn milliwatt_to_dbm(mw: f64) -> f64 {
    10.0 * mw.log10()
}

impl Default for SpectrumMonitor {
    fn default() -> Self {
        Self::new()
    }
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
        self.transmitter_bandwidth_cache
            .insert(u64_bandwidth_hz, (HashMap::new(), HashSet::new()));

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

    pub fn request_i(
        &self,
        now: i64,
        u64_frequency_hz: u64,
        duration: i64,
        timepoint: i64,
    ) -> (Vec<f64>, i64, i64, f64, bool) {
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
            (
                vec,
                start,
                self.bin_size,
                self.d_receiver_sensitivity_milli_watt,
                self.mode == NoiseMode::All,
            )
        } else {
            (
                Vec::new(),
                0,
                self.bin_size,
                self.d_receiver_sensitivity_milli_watt,
                false,
            )
        }
    }

    pub fn request_filter_i(
        &self,
        now: i64,
        filter_index: u16,
        duration: i64,
        timepoint: i64,
    ) -> (Vec<f64>, i64, i64, f64, usize) {
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
            (
                vec,
                start,
                self.bin_size,
                self.d_receiver_sensitivity_milli_watt,
                recorder.get_sub_band_bin_count(),
            )
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
        if self
            .filter_noise_recorder_map
            .remove(&filter_index)
            .is_some()
        {
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
        if segments.len() != rx_powers_milli_watt.len() {
            return (0, 0, 0, Vec::new(), false, 0.0);
        }

        let valid_tx_time = if tx_time.abs_diff(now) > self.time_sync_threshold.max(0) as u64 {
            now
        } else {
            tx_time
        };
        let Some(valid_propagation) =
            clamp_value(propagation_delay, self.max_propagation, self.b_max_clamp)
        else {
            return (0, 0, 0, Vec::new(), false, 0.0);
        };

        let mut report_as_in_band = false;
        let mut reportable = Vec::new();
        let mut minimum_sor = i64::MAX;
        let mut maximum_eor = i64::MIN;
        let simple_mode = self.mode == NoiseMode::None
            || self.mode == NoiseMode::PassThrough
            || (self.mode == NoiseMode::OutOfBand && b_in_band);

        for (segment, rx_power_mw) in segments.iter().zip(rx_powers_milli_watt.iter().copied()) {
            let Some(offset) =
                clamp_value(segment.offset_microsec, self.max_offset, self.b_max_clamp)
            else {
                return (0, 0, 0, Vec::new(), false, 0.0);
            };
            let Some(duration) = clamp_value(
                segment.duration_microsec,
                self.max_duration,
                self.b_max_clamp,
            ) else {
                return (0, 0, 0, Vec::new(), false, 0.0);
            };
            let shifted_frequency = apply_doppler(segment.frequency_hz, d_doppler_factor);

            if simple_mode {
                let frequency_match = self.mode == NoiseMode::PassThrough
                    || self.noise_recorder_map.contains_key(&segment.frequency_hz);
                if !frequency_match {
                    continue;
                }
                report_as_in_band = true;
                let overlap = get_spectral_mask_manager().get_spectral_overlap(
                    shifted_frequency,
                    segment.frequency_hz,
                    self.u64_receiver_bandwidth_hz,
                    u64_segment_bandwidth_hz,
                    spectral_mask_index,
                );
                let overlap_power = overlap
                    .as_ref()
                    .map_or(0.0, |value| overlap_power_mw(value, rx_power_mw));
                if overlap_power >= self.d_receiver_sensitivity_milli_watt {
                    update_reception_bounds(
                        valid_tx_time,
                        offset,
                        valid_propagation,
                        duration,
                        &mut minimum_sor,
                        &mut maximum_eor,
                    );
                    reportable.push(FfiFrequencySegment {
                        frequency_hz: segment.frequency_hz,
                        rx_power_dbm: milliwatt_to_dbm(overlap_power),
                        duration_microsec: duration,
                        offset_microsec: offset,
                    });
                }
                continue;
            }

            let mut matching_power = None;
            let recorder_frequencies: Vec<u64> = self.noise_recorder_map.keys().copied().collect();
            for recorder_frequency in recorder_frequencies {
                let Some(overlap) = get_spectral_mask_manager().get_spectral_overlap(
                    shifted_frequency,
                    recorder_frequency,
                    self.u64_receiver_bandwidth_hz,
                    u64_segment_bandwidth_hz,
                    spectral_mask_index,
                ) else {
                    continue;
                };
                let overlap_power = overlap_power_mw(&overlap, rx_power_mw);
                if overlap_power < self.d_receiver_sensitivity_milli_watt {
                    continue;
                }
                let (_, lower, upper, _) = &overlap;
                if let Some(recorder) = self.noise_recorder_map.get_mut(&recorder_frequency) {
                    let (sor, eor) = recorder.update(
                        now,
                        valid_tx_time,
                        offset,
                        valid_propagation,
                        duration,
                        overlap_power,
                        transmitters,
                        *lower,
                        *upper,
                        tx_antenna_index,
                        false,
                    );
                    if recorder_frequency == segment.frequency_hz {
                        matching_power = Some((overlap_power, sor, eor));
                    }
                }
            }

            if b_in_band && self.foi.contains(&segment.frequency_hz) {
                report_as_in_band = true;
            }
            if let Some((power, sor, eor)) = matching_power {
                minimum_sor = minimum_sor.min(sor);
                maximum_eor = maximum_eor.max(eor);
                reportable.push(FfiFrequencySegment {
                    frequency_hz: segment.frequency_hz,
                    rx_power_dbm: milliwatt_to_dbm(power),
                    duration_microsec: duration,
                    offset_microsec: offset,
                });
            }

            if !self.b_exclude_same_sub_id_from_filter || self.u16_sub_id != u16_sub_id {
                self.apply_energy_to_filters(
                    now,
                    valid_tx_time,
                    valid_propagation,
                    offset,
                    duration,
                    shifted_frequency,
                    segment.frequency_hz,
                    u64_segment_bandwidth_hz,
                    rx_power_mw,
                    transmitters,
                    u16_sub_id,
                    tx_antenna_index,
                    spectral_mask_index,
                    filter_data_ptr,
                    filter_data_len,
                );
            }
        }

        let span = if minimum_sor == i64::MAX || maximum_eor == i64::MIN {
            0
        } else {
            maximum_eor.saturating_sub(minimum_sor)
        };
        (
            valid_tx_time,
            valid_propagation,
            span,
            reportable,
            report_as_in_band,
            self.d_receiver_sensitivity_milli_watt,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_energy_to_filters(
        &mut self,
        now: i64,
        tx_time: i64,
        propagation: i64,
        offset: i64,
        duration: i64,
        shifted_tx_frequency: u64,
        original_tx_frequency: u64,
        tx_bandwidth: u64,
        rx_power_mw: f64,
        transmitters: &[u16],
        sub_id: u16,
        tx_antenna_index: u16,
        spectral_mask_index: u16,
        filter_data_ptr: *const u8,
        filter_data_len: usize,
    ) {
        for (_, (filter_frequency, filter_bandwidth, recorder, criterion)) in
            self.filter_noise_recorder_map.iter_mut()
        {
            let Some(overlap) = get_spectral_mask_manager().get_spectral_overlap(
                shifted_tx_frequency,
                *filter_frequency,
                *filter_bandwidth,
                tx_bandwidth,
                spectral_mask_index,
            ) else {
                continue;
            };
            let (_, lower, upper, total_segments) = &overlap;
            let matches = if criterion.is_null() {
                true
            } else {
                let criterion = unsafe { &*(*criterion as *const FfiFilterMatchCriterion) };
                (criterion.matches)(
                    criterion.context,
                    original_tx_frequency,
                    upper.saturating_sub(*lower),
                    if sub_id == self.u16_sub_id { 0 } else { sub_id },
                    filter_data_ptr,
                    filter_data_len,
                )
            };
            if !matches {
                continue;
            }
            let aggregate_power = overlap_power_mw(&overlap, rx_power_mw);
            if aggregate_power < self.d_receiver_sensitivity_milli_watt {
                continue;
            }
            if recorder.get_sub_band_bin_count() > 1 {
                let mut index = 0u64;
                for (segments, _, _) in &overlap.0 {
                    for (_, modifier, segment_lower, segment_upper) in segments {
                        index += 1;
                        recorder.update(
                            now,
                            tx_time,
                            offset,
                            propagation,
                            duration,
                            rx_power_mw * *modifier,
                            transmitters,
                            *segment_lower,
                            *segment_upper,
                            tx_antenna_index,
                            index != *total_segments,
                        );
                    }
                }
            } else {
                recorder.update(
                    now,
                    tx_time,
                    offset,
                    propagation,
                    duration,
                    aggregate_power,
                    transmitters,
                    *lower,
                    *upper,
                    tx_antenna_index,
                    false,
                );
            }
        }
    }
}

fn clamp_value(value: i64, maximum: i64, clamp: bool) -> Option<i64> {
    if value < 0 {
        None
    } else if value > maximum {
        clamp.then_some(maximum)
    } else {
        Some(value)
    }
}

fn apply_doppler(frequency: u64, factor: f64) -> u64 {
    let shifted = i128::from(frequency) + i128::from(doppler_shift(frequency, factor));
    shifted.clamp(0, i128::from(u64::MAX)) as u64
}

fn overlap_power_mw(overlap: &MaskOverlap, rx_power_mw: f64) -> f64 {
    overlap
        .0
        .iter()
        .flat_map(|(segments, _, _)| segments)
        .map(|(ratio, modifier, _, _)| rx_power_mw * modifier * ratio)
        .sum()
}

fn update_reception_bounds(
    tx_time: i64,
    offset: i64,
    propagation: i64,
    duration: i64,
    minimum_sor: &mut i64,
    maximum_eor: &mut i64,
) {
    let sor = tx_time.saturating_add(offset).saturating_add(propagation);
    let eor = sor.saturating_add(duration);
    *minimum_sor = (*minimum_sor).min(sor);
    *maximum_eor = (*maximum_eor).max(eor);
}

// FFI
#[no_mangle]
pub extern "C" fn emane_rs_spectrum_monitor_new() -> *mut c_void {
    Box::into_raw(Box::new(SpectrumMonitor::new())) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_spectrum_monitor_free(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr as *mut SpectrumMonitor);
        }
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
    if ptr.is_null() || (foi_len != 0 && foi_ptr.is_null()) {
        return;
    }
    let monitor = unsafe { &mut *(ptr as *mut SpectrumMonitor) };
    let foi = if foi_len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(foi_ptr, foi_len) }
    };
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
    if ptr.is_null()
        || segments_len != rx_powers_milli_watt_len
        || (segments_len != 0 && segments_ptr.is_null())
        || (rx_powers_milli_watt_len != 0 && rx_powers_milli_watt_ptr.is_null())
        || (transmitters_len != 0 && transmitters_ptr.is_null())
        || (filter_data_len != 0 && filter_data_ptr.is_null())
    {
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
    let segments = if segments_len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(segments_ptr, segments_len) }
    };
    let rx_powers_milli_watt = if rx_powers_milli_watt_len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(rx_powers_milli_watt_ptr, rx_powers_milli_watt_len) }
    };
    let transmitters = if transmitters_len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(transmitters_ptr, transmitters_len) }
    };

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
pub extern "C" fn emane_rs_spectrum_monitor_free_update_segments(
    segments: *mut FfiFrequencySegment,
    len: usize,
) {
    if !segments.is_null() {
        unsafe {
            let _ = Vec::from_raw_parts(segments, len, len);
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_spectrum_monitor_get_frequencies(
    ptr: *const c_void,
    out_freqs: *mut u64,
    max_len: usize,
) -> usize {
    if ptr.is_null() || (max_len != 0 && out_freqs.is_null()) {
        return 0;
    }
    let monitor = unsafe { &*(ptr as *const SpectrumMonitor) };
    let freqs = monitor.get_frequencies();
    let count = std::cmp::min(freqs.len(), max_len);
    if count != 0 {
        unsafe {
            std::ptr::copy_nonoverlapping(freqs.as_ptr(), out_freqs, count);
        }
    }
    count
}

#[no_mangle]
pub extern "C" fn emane_rs_spectrum_monitor_get_receiver_sensitivity_dbm(
    ptr: *const c_void,
) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
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
    if ptr.is_null() {
        return;
    }
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
    if ptr.is_null() {
        return;
    }
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
    let (mut data, start, bin, sens, is_all) =
        monitor.request_i(now, u64_frequency_hz, duration, timepoint);
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
    if ptr.is_null() {
        return;
    }
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
    if ptr.is_null() {
        return;
    }
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
    let (mut data, start, bin, sens, count) =
        monitor.request_filter_i(now, filter_index, duration, timepoint);
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

#[cfg(test)]
mod tests {
    use super::*;

    const FREQUENCY: u64 = 2_400_000_000;
    const BANDWIDTH: u64 = 1_000_000;

    fn monitor(mode: NoiseMode) -> SpectrumMonitor {
        let mut monitor = SpectrumMonitor::new();
        monitor.initialize(
            7,
            &[FREQUENCY],
            BANDWIDTH,
            0.001,
            mode,
            10,
            1_000,
            1_000,
            10_000,
            100,
            true,
            false,
        );
        monitor
    }

    #[test]
    fn records_in_band_energy_and_reports_reception_span() {
        let mut monitor = monitor(NoiseMode::All);
        let segments = [FfiFrequencySegment {
            frequency_hz: FREQUENCY,
            rx_power_dbm: -10.0,
            duration_microsec: 100,
            offset_microsec: 20,
        }];
        let (tx, propagation, span, report, in_band, sensitivity) = monitor.update(
            1_000,
            1_000,
            30,
            0.0,
            &segments,
            BANDWIDTH,
            &[0.1],
            true,
            &[1],
            1,
            0,
            0,
            std::ptr::null(),
            0,
        );

        assert_eq!(tx, 1_000);
        assert_eq!(propagation, 30);
        assert_eq!(span, 100);
        assert!(in_band);
        assert_eq!(report.len(), 1);
        assert!((report[0].rx_power_dbm + 10.0).abs() < 1e-9);
        assert_eq!(sensitivity, 0.001);
        assert!(monitor.dump(FREQUENCY).iter().any(|power| *power > 0.0));
    }

    #[test]
    fn applies_time_sync_and_configured_clamps() {
        let mut monitor = monitor(NoiseMode::None);
        let segments = [FfiFrequencySegment {
            frequency_hz: FREQUENCY,
            rx_power_dbm: -10.0,
            duration_microsec: 20_000,
            offset_microsec: 2_000,
        }];
        let (tx, propagation, span, report, _, _) = monitor.update(
            50_000,
            1,
            2_000,
            0.0,
            &segments,
            BANDWIDTH,
            &[0.1],
            true,
            &[1],
            1,
            0,
            0,
            std::ptr::null(),
            0,
        );

        assert_eq!(tx, 50_000);
        assert_eq!(propagation, 1_000);
        assert_eq!(span, 10_000);
        assert_eq!(report[0].offset_microsec, 1_000);
        assert_eq!(report[0].duration_microsec, 10_000);
    }

    #[test]
    fn zero_length_ffi_inputs_do_not_create_invalid_slices() {
        let ptr = emane_rs_spectrum_monitor_new();
        emane_rs_spectrum_monitor_initialize(
            ptr,
            0,
            std::ptr::null(),
            0,
            BANDWIDTH,
            0.001,
            NoiseMode::None as u32,
            10,
            100,
            100,
            1_000,
            100,
            true,
            false,
        );
        let result = emane_rs_spectrum_monitor_update(
            ptr,
            0,
            0,
            0,
            0.0,
            std::ptr::null(),
            0,
            BANDWIDTH,
            std::ptr::null(),
            0,
            false,
            std::ptr::null(),
            0,
            0,
            0,
            0,
            std::ptr::null(),
            0,
        );
        assert_eq!(result.segments_len, 0);
        emane_rs_spectrum_monitor_free(ptr);
    }
}
