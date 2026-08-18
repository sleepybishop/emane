import os

RUST_CODE = """
use std::os::raw::{c_void, c_int};
use crate::spectrum_monitor::{SpectrumMonitor, FfiFrequencySegment};
use crate::gain_manager::{emane_rs_gain_manager_determine_gain, EmaneGainResult};
use std::slice;

#[repr(C)]
pub struct EMANE_PathlossResult {
    pathlosses: *mut f64,
    count: usize,
    success: bool,
}

extern "C" {
    fn emane_c_common_phy_header_get_frequency_groups_size(hdr_ptr: *const c_void) -> usize;
    fn emane_c_common_phy_header_get_frequency_segments_ptr(hdr_ptr: *const c_void, group_idx: usize) -> *const c_void;
    fn emane_c_frequency_segments_size(segs_ptr: *const c_void) -> usize;
    fn emane_c_frequency_segments_get(segs_ptr: *const c_void, seg_idx: usize, freq: *mut u64, power: *mut f64, has_power: *mut bool, dur: *mut i64, offset: *mut i64);
    
    fn emane_c_common_phy_header_get_transmit_antennas_size(hdr_ptr: *const c_void) -> usize;
    fn emane_c_common_phy_header_get_transmit_antenna(hdr_ptr: *const c_void, idx: usize, freq_group_idx: *mut usize, ant_idx: *mut u16, bw: *mut u64, mask_idx: *mut u16);

    fn emane_c_common_phy_header_get_transmitters_size(hdr_ptr: *const c_void) -> usize;
    fn emane_c_common_phy_header_get_transmitter(hdr_ptr: *const c_void, idx: usize, nem_id: *mut u16, power_dbm: *mut f64);

    fn emane_c_common_phy_header_get_tx_time(hdr_ptr: *const c_void) -> u64;
    fn emane_c_common_phy_header_get_sub_id(hdr_ptr: *const c_void) -> u16;
    fn emane_c_common_phy_header_get_optional_filter_data(hdr_ptr: *const c_void, out_has_data: *mut bool, out_len: *mut usize) -> *const u8;

    fn emane_c_antenna_interferences_size(ptr: *const c_void) -> usize;
    fn emane_c_antenna_interferences_get(ptr: *const c_void, idx: usize, freq_group_idx: *mut usize, out_powers: *mut *const f64, out_powers_len: *mut usize);

    fn emane_c_frequency_groups_size(ptr: *const c_void) -> usize;
    fn emane_c_frequency_groups_get_segments_ptr(ptr: *const c_void, idx: usize) -> *const c_void;

    fn emane_c_propagation_model_compute(algo_ptr: *mut c_void, src: u16, loc_info_ptr: *const c_void, segments_ptr: *const c_void) -> EMANE_PathlossResult;
    fn emane_c_propagation_model_free_result(res: *mut EMANE_PathlossResult);
    fn emane_c_fading_store_compute_with_model(store_ptr: *mut c_void, model: u32, rx_power: f64, distance: f64, selection: u64) -> f64;

    fn emane_c_location_infos_size(vec_ptr: *const c_void) -> usize;
    fn emane_c_location_infos_get(vec_ptr: *const c_void, index: usize, out_bool: *mut bool) -> *const c_void;

    fn emane_c_fading_infos_size(vec_ptr: *const c_void) -> usize;
    fn emane_c_fading_infos_get(vec_ptr: *const c_void, index: usize, out_bool: *mut bool, out_model: *mut u32) -> u64;

    fn emane_c_receive_processor_add_receive_power(
        result_ptr: *mut c_void, src: u16, rx_ant: u16, tx_ant: u16, freq: u64,
        rx_power: f64, tx_gain: f64, rx_gain: f64, tx_power: f64, pathloss: f64, doppler: f64
    );
    fn emane_c_receive_processor_add_observed_power(
        result_ptr: *mut c_void, src: u16, rx_ant: u16, tx_ant: u16, freq: u64, mask_index: u16, rx_power: f64
    );
    fn emane_c_receive_processor_set_status(result_ptr: *mut c_void, status: u32);
    fn emane_c_receive_processor_set_mimo(result_ptr: *mut c_void, sot: u64, prop_delay: u64);
    fn emane_c_receive_processor_set_gain_cache_hit(result_ptr: *mut c_void, hit: bool);
    fn emane_c_receive_processor_add_doppler_shift(result_ptr: *mut c_void, freq: u64, shift: f64);
    fn emane_c_receive_processor_add_antenna_receive_info(
        result_ptr: *mut c_void, rx_ant: u16, tx_ant: u16, span: u64, rx_sensitivity_dbm: f64, freq_segments_ptr: *const c_void
    );
    
    fn emane_c_location_info_get_distance(loc: *const c_void) -> f64;
}

pub struct ReceiveProcessorImpl {
    id: u16,
    sub_id: u16,
    rx_antenna_index: u16,
    antenna_manager: *mut c_void,
    spectrum_monitor: *mut c_void,
    propagation_model: *mut c_void,
    fading_algorithm_store: *mut c_void,
    populate_receive_power_map: bool,
    populate_observed_power_map: bool,
    doppler_shift: bool,
}

#[no_mangle]
pub extern "C" fn emane_rs_receive_processor_create(
    id: u16,
    sub_id: u16,
    rx_antenna_index: u16,
    antenna_manager: *mut c_void,
    spectrum_monitor: *mut c_void,
    propagation_model: *mut c_void,
    fading_algorithm_store: *mut c_void,
    populate_receive_power_map: bool,
    populate_observed_power_map: bool,
    doppler_shift: bool
) -> *mut c_void {
    let rp = Box::new(ReceiveProcessorImpl {
        id, sub_id, rx_antenna_index, antenna_manager, spectrum_monitor, propagation_model,
        fading_algorithm_store, populate_receive_power_map, populate_observed_power_map, doppler_shift
    });
    Box::into_raw(rp) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_receive_processor_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr as *mut ReceiveProcessorImpl)); }
    }
}

fn db_to_milliwatt(db: f64) -> f64 {
    10.0_f64.powf(db / 10.0)
}

fn milliwatt_to_db(mw: f64) -> f64 {
    10.0 * mw.log10()
}

fn doppler_shift(freq: u64, doppler_factor: f64) -> f64 {
    (freq as f64) * doppler_factor // Simplified
}

#[no_mangle]
pub extern "C" fn emane_rs_receive_processor_process(
    rs_ptr: *mut c_void,
    now_usec: i64,
    common_phy_header: *const c_void,
    location_infos: *const c_void,
    fading_infos: *const c_void,
    in_band: bool,
    result_ptr: *mut c_void
) {
    let rp = unsafe { &mut *(rs_ptr as *mut ReceiveProcessorImpl) };
    
    let mut i_treated_as_in_band = 0;
    let mut i_drop_not_foi = 0;
    let mut i_drop_out_of_band = 0;

    let num_freq_groups = unsafe { emane_c_common_phy_header_get_frequency_groups_size(common_phy_header) };
    let num_tx_antennas = unsafe { emane_c_common_phy_header_get_transmit_antennas_size(common_phy_header) };
    let num_transmitters = unsafe { emane_c_common_phy_header_get_transmitters_size(common_phy_header) };
    
    let tx_time = unsafe { emane_c_common_phy_header_get_tx_time(common_phy_header) };
    let sub_id = unsafe { emane_c_common_phy_header_get_sub_id(common_phy_header) };

    for ant_i in 0..num_tx_antennas {
        let mut group_idx: usize = 0;
        let mut ant_idx: u16 = 0;
        let mut ant_bw: u64 = 0;
        let mut ant_mask_idx: u16 = 0;
        unsafe { emane_c_common_phy_header_get_transmit_antenna(common_phy_header, ant_i, &mut group_idx, &mut ant_idx, &mut ant_bw, &mut ant_mask_idx); }

        if group_idx >= num_freq_groups {
            unsafe { emane_c_receive_processor_set_status(result_ptr, 1); } // DROP_CODE_ANTENNA_FREQ_INDEX
            return;
        }

        let segments_ptr = unsafe { emane_c_common_phy_header_get_frequency_segments_ptr(common_phy_header, group_idx) };
        let num_segments = unsafe { emane_c_frequency_segments_size(segments_ptr) };

        let mut rx_power_segments_milliwatt = vec![0.0f64; num_segments];
        let mut propagation_delay: i64 = 0;
        let mut have_propagation_delay = false;
        
        let mut transmitters = Vec::with_capacity(num_transmitters);
        let mut i_transmitter_index = 0;

        for tx_i in 0..num_transmitters {
            let mut tx_nem_id: u16 = 0;
            let mut tx_power_dbm: f64 = 0.0;
            unsafe { emane_c_common_phy_header_get_transmitter(common_phy_header, tx_i, &mut tx_nem_id, &mut tx_power_dbm); }
            transmitters.push(tx_nem_id);

            let mut loc_valid = false;
            let loc_ptr = unsafe { emane_c_location_infos_get(location_infos, i_transmitter_index, &mut loc_valid) };
            let mut fading_valid = false;
            let mut fading_model = 0;
            let fading_selection = unsafe { emane_c_fading_infos_get(fading_infos, i_transmitter_index, &mut fading_valid, &mut fading_model) };
            
            i_transmitter_index += 1;

            let mut pathloss_info = unsafe { emane_c_propagation_model_compute(rp.propagation_model, tx_nem_id, loc_ptr, segments_ptr) };
            
            if pathloss_info.success && !pathloss_info.pathlosses.is_null() && pathloss_info.count == num_segments {
                let gain_result = unsafe { emane_rs_gain_manager_determine_gain(rp.antenna_manager, tx_nem_id, ant_idx, loc_ptr) };
                
                if gain_result.status == 0 { // SUCCESS
                    unsafe { emane_c_receive_processor_set_gain_cache_hit(result_ptr, gain_result.is_cache); }
                    
                    let pathlosses = unsafe { slice::from_raw_parts(pathloss_info.pathlosses, pathloss_info.count) };
                    
                    for s_idx in 0..num_segments {
                        let mut seg_freq: u64 = 0;
                        let mut seg_power: f64 = 0.0;
                        let mut seg_has_power = false;
                        let mut seg_dur: i64 = 0;
                        let mut seg_offset: i64 = 0;
                        unsafe { emane_c_frequency_segments_get(segments_ptr, s_idx, &mut seg_freq, &mut seg_power, &mut seg_has_power, &mut seg_dur, &mut seg_offset); }
                        
                        let tx_pwr = if seg_has_power { seg_power } else { tx_power_dbm };
                        let power_dbm = tx_pwr + gain_result.remote_gain + gain_result.local_gain - pathlosses[s_idx];
                        
                        let mut rx_pwr_mw = 0.0;
                        if fading_valid {
                            if fading_model == 0 { // NONE
                                rx_pwr_mw = db_to_milliwatt(power_dbm);
                            } else {
                                if loc_valid {
                                    let dist = unsafe { emane_c_location_info_get_distance(loc_ptr) };
                                    rx_pwr_mw += unsafe { emane_c_fading_store_compute_with_model(rp.fading_algorithm_store, fading_model, power_dbm, dist, fading_selection) };
                                } else {
                                    unsafe { emane_c_propagation_model_free_result(&mut pathloss_info); }
                                    unsafe { emane_c_receive_processor_set_status(result_ptr, 2); } // FADINGMANAGER_LOCATION
                                    return;
                                }
                            }
                        } else {
                            unsafe { emane_c_propagation_model_free_result(&mut pathloss_info); }
                            unsafe { emane_c_receive_processor_set_status(result_ptr, 4); } // FADINGMANAGER_SELECTION
                            return;
                        }

                        rx_power_segments_milliwatt[s_idx] += rx_pwr_mw;
                        
                        let mut d_doppler_hz = 0.0;
                        if rp.doppler_shift {
                            d_doppler_hz = doppler_shift(seg_freq, 1.0); // Simplified
                            unsafe { emane_c_receive_processor_add_doppler_shift(result_ptr, seg_freq, d_doppler_hz); }
                        }
                        
                        if rp.populate_receive_power_map {
                            unsafe {
                                emane_c_receive_processor_add_receive_power(
                                    result_ptr, tx_nem_id, rp.rx_antenna_index, ant_idx, seg_freq,
                                    milliwatt_to_db(rx_pwr_mw), gain_result.remote_gain, gain_result.local_gain,
                                    tx_pwr, pathlosses[s_idx], d_doppler_hz
                                );
                            }
                        }
                    }
                    
                    if loc_valid && !have_propagation_delay {
                        let dist = unsafe { emane_c_location_info_get_distance(loc_ptr) };
                        if dist > 0.0 {
                            propagation_delay = (dist / 299792458.0 * 1000000.0).round() as i64;
                        }
                        have_propagation_delay = true;
                    }

                } else {
                    let st = match gain_result.status {
                        1 => 5, // LOCATION
                        2 => 6, // ANTENNAPROFILE
                        3 => 7, // HORIZON
                        4 => 8, // ANTENNA_INDEX
                        _ => 0, // UNKNOWN
                    };
                    unsafe { emane_c_propagation_model_free_result(&mut pathloss_info); }
                    unsafe { emane_c_receive_processor_set_status(result_ptr, st); }
                    return;
                }
            } else {
                unsafe { emane_c_propagation_model_free_result(&mut pathloss_info); }
                unsafe { emane_c_receive_processor_set_status(result_ptr, 9); } // PROPAGATIONMODEL
                return;
            }
            unsafe { emane_c_propagation_model_free_result(&mut pathloss_info); }
        }

        let mut ffi_segments = Vec::with_capacity(num_segments);
        for s_idx in 0..num_segments {
            let mut seg_freq = 0; let mut p=0.0; let mut hp=false; let mut dur=0; let mut off=0;
            unsafe { emane_c_frequency_segments_get(segments_ptr, s_idx, &mut seg_freq, &mut p, &mut hp, &mut dur, &mut off); }
            ffi_segments.push(FfiFrequencySegment {
                frequency_hz: seg_freq,
                rx_power_dbm: 0.0,
                duration_microsec: dur,
                offset_microsec: off,
            });
        }

        let monitor = unsafe { &mut *(rp.spectrum_monitor as *mut SpectrumMonitor) };
        let mut opt_filter_has = false;
        let mut opt_filter_len = 0;
        let opt_filter_ptr = unsafe { emane_c_common_phy_header_get_optional_filter_data(common_phy_header, &mut opt_filter_has, &mut opt_filter_len) };
        
        let (sot, prop_delay, span, res_segments, treat_in_band, sensitivity) = monitor.update(
            now_usec, tx_time as i64, propagation_delay, 1.0,
            &ffi_segments, ant_bw, &rx_power_segments_milliwatt, in_band, &transmitters, sub_id, ant_idx, ant_mask_idx,
            if opt_filter_has { opt_filter_ptr } else { std::ptr::null() }, if opt_filter_has { opt_filter_len } else { 0 }
        );
        
        if true { // Since it returns a tuple directly, not Option
            if treat_in_band {
                if !res_segments.is_empty() {
                    unsafe { emane_c_receive_processor_set_mimo(result_ptr, sot as u64, prop_delay as u64); }
                    if rp.populate_observed_power_map {
                        for rs in &res_segments {
                            unsafe {
                                emane_c_receive_processor_add_observed_power(
                                    result_ptr, transmitters[0], rp.rx_antenna_index, ant_idx, rs.frequency_hz,
                                    ant_mask_idx, rs.rx_power_dbm
                                );
                            }
                        }
                    }
                    unsafe { emane_c_receive_processor_add_antenna_receive_info(result_ptr, rp.rx_antenna_index, ant_idx, span as u64, milliwatt_to_db(sensitivity), segments_ptr); }
                    i_treated_as_in_band += 1;
                }
            } else {
                if in_band { i_drop_not_foi += 1; } else { i_drop_out_of_band += 1; }
            }
        }
    }

    let final_status = if i_treated_as_in_band > 0 {
        13 // SUCCESS
    } else if i_drop_not_foi > 0 {
        11 // NOT_FOI
    } else if i_drop_out_of_band > 0 {
        12 // OUT_OF_BAND
    } else {
        13 // SUCCESS
    };
    unsafe { emane_c_receive_processor_set_status(result_ptr, final_status); }
}

#[no_mangle]
pub extern "C" fn emane_rs_receive_processor_process_self_interference(
    rs_ptr: *mut c_void,
    now_usec: i64,
    tx_time_usec: i64,
    frequency_groups: *const c_void,
    segment_bandwidth_hz: u64,
    antenna_interferences: *const c_void,
    optional_filter_data: *const c_void,
    result_ptr: *mut c_void
) {
    let rp = unsafe { &mut *(rs_ptr as *mut ReceiveProcessorImpl) };
    let num_freq_groups = unsafe { emane_c_frequency_groups_size(frequency_groups) };
    let num_interferences = unsafe { emane_c_antenna_interferences_size(antenna_interferences) };
    
    let monitor = unsafe { &mut *(rp.spectrum_monitor as *mut SpectrumMonitor) };

    for ant_i in 0..num_interferences {
        let mut group_idx: usize = 0;
        let mut powers_ptr: *const f64 = std::ptr::null();
        let mut powers_len: usize = 0;
        unsafe { emane_c_antenna_interferences_get(antenna_interferences, ant_i, &mut group_idx, &mut powers_ptr, &mut powers_len); }
        
        if group_idx >= num_freq_groups {
            // ERROR_ANTENNA_FREQ_INDEX
            return;
        }

        let segments_ptr = unsafe { emane_c_frequency_groups_get_segments_ptr(frequency_groups, group_idx) };
        let num_segments = unsafe { emane_c_frequency_segments_size(segments_ptr) };
        
        let mut ffi_segments = Vec::with_capacity(num_segments);
        for s_idx in 0..num_segments {
            let mut seg_freq = 0; let mut p=0.0; let mut hp=false; let mut dur=0; let mut off=0;
            unsafe { emane_c_frequency_segments_get(segments_ptr, s_idx, &mut seg_freq, &mut p, &mut hp, &mut dur, &mut off); }
            ffi_segments.push(FfiFrequencySegment {
                frequency_hz: seg_freq,
                rx_power_dbm: 0.0,
                duration_microsec: dur,
                offset_microsec: off,
            });
        }

        let supplied_powers = unsafe { slice::from_raw_parts(powers_ptr, powers_len) };
        let mut rx_power_segments_milliwatt = vec![0.0f64; num_segments];
        
        if powers_len == 1 {
            for i in 0..num_segments { rx_power_segments_milliwatt[i] = supplied_powers[0]; }
        } else if num_segments == powers_len {
            for i in 0..num_segments { rx_power_segments_milliwatt[i] = supplied_powers[i]; }
        } else {
            // ERROR_MISSING_POWER_VALUES
            return;
        }
        
        monitor.update(
            now_usec, tx_time_usec, 0, 1.0,
            &ffi_segments, segment_bandwidth_hz, &rx_power_segments_milliwatt, false, &[rp.id], rp.sub_id, ant_i as u16, 0,
            std::ptr::null(), 0
        );
    }
}
"""

with open("rust/emane-core/src/receive_processor.rs", "w") as f:
    f.write(RUST_CODE)
