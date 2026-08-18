use std::collections::HashMap;
use std::os::raw::{c_void, c_int};

extern "C" {
    fn emane_c_location_info_get_distance(loc: *const c_void) -> f64;
    fn emane_c_location_info_is_valid(loc: *const c_void) -> bool;
    fn emane_c_location_info_get_altitude(loc: *const c_void, is_local: bool) -> f64;
    fn emane_c_location_info_get_sequence_number(loc: *const c_void) -> u64;

    fn emane_c_utils_calculate_direction_by_neu(
        loc: *const c_void, 
        local_north: f64, local_east: f64, local_up: f64,
        remote_north: f64, remote_east: f64, remote_up: f64
    ) -> EmaneDirection;

    fn emane_c_utils_calculate_lookup_angles(
        az_ref: f64, az_pointing: f64, el_ref: f64, el_pointing: f64
    ) -> EmaneLookupAngles;

    fn emane_c_utils_check_horizon(height1: f64, height2: f64, dist: f64) -> bool;

    fn emane_c_antenna_manager_get_info(
        am_ptr: *mut c_void, nem_id: u16, index: u16,
        out_is_omni: *mut bool, out_fixed_gain: *mut f64,
        out_has_pointing: *mut bool, out_profile_id: *mut u16,
        out_pointing_azimuth: *mut f64, out_pointing_elevation: *mut f64,
        out_seq: *mut u64
    ) -> bool;
    
    fn emane_rs_antenna_pattern_get_gain(
        pattern: *const c_void, bearing: i16, elevation: i16
    ) -> f64;
}

#[repr(C)]
struct EmaneDirection {
    azimuth: f64,
    elevation: f64,
    distance: f64,
}

#[repr(C)]
struct EmaneLookupAngles {
    azimuth: f64,
    elevation: f64,
}

#[repr(C)]
pub struct EmaneGainResult {
    remote_gain: f64,
    local_gain: f64,
    status: c_int,
    is_cache: bool,
}

struct AntennaInfo {
    is_omni: bool,
    fixed_gain: f64,
    has_pointing: bool,
    profile_id: u16,
    pointing_azimuth: f64,
    pointing_elevation: f64,
    seq: u64,
}

fn get_antenna_info(am_ptr: *mut c_void, nem_id: u16, index: u16) -> Option<AntennaInfo> {
    let mut is_omni = false;
    let mut fixed_gain = 0.0;
    let mut has_pointing = false;
    let mut profile_id = 0;
    let mut pointing_azimuth = 0.0;
    let mut pointing_elevation = 0.0;
    let mut seq = 0;
    
    let ok = unsafe {
        emane_c_antenna_manager_get_info(
            am_ptr, nem_id, index,
            &mut is_omni, &mut fixed_gain,
            &mut has_pointing, &mut profile_id,
            &mut pointing_azimuth, &mut pointing_elevation,
            &mut seq
        )
    };
    if ok {
        Some(AntennaInfo {
            is_omni, fixed_gain, has_pointing, profile_id,
            pointing_azimuth, pointing_elevation, seq
        })
    } else {
        None
    }
}

pub struct GainManagerImpl {
    nem_id: u16,
    rx_antenna_index: u16,
    am_ptr: *mut c_void,
    gain_cache: HashMap<u16, HashMap<u16, (u64, u64, f64, f64)>>,
    antenna_update_seq: u64,
}

#[no_mangle]
pub extern "C" fn emane_rs_gain_manager_create(nem_id: u16, rx_antenna_index: u16, am_ptr: *mut c_void) -> *mut c_void {
    let mgr = Box::new(GainManagerImpl {
        nem_id,
        rx_antenna_index,
        am_ptr,
        gain_cache: HashMap::new(),
        antenna_update_seq: 0,
    });
    Box::into_raw(mgr) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_gain_manager_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        let _ = unsafe { Box::from_raw(ptr as *mut GainManagerImpl) };
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_gain_manager_determine_gain(
    ptr: *mut c_void, 
    transmitter_id: u16, 
    tx_antenna_index: u16, 
    location_pair_info: *const c_void
) -> EmaneGainResult {
    let mgr = unsafe { &mut *(ptr as *mut GainManagerImpl) };
    let am_ptr = mgr.am_ptr;
    
    let remote_ant_info = match get_antenna_info(am_ptr, transmitter_id, tx_antenna_index) {
        Some(info) => info,
        None => return EmaneGainResult { remote_gain: 0.0, local_gain: 0.0, status: 2, is_cache: false }
    };
    let local_ant_info = match get_antenna_info(am_ptr, mgr.nem_id, mgr.rx_antenna_index) {
        Some(info) => info,
        None => return EmaneGainResult { remote_gain: 0.0, local_gain: 0.0, status: 2, is_cache: false }
    };

    let loc_seq = unsafe { emane_c_location_info_get_sequence_number(location_pair_info) };

    if local_ant_info.seq != mgr.antenna_update_seq {
        mgr.gain_cache.clear();
        mgr.antenna_update_seq = local_ant_info.seq;
    } else {
        if let Some(inner) = mgr.gain_cache.get(&transmitter_id) {
            if let Some(&(c_tx_seq, c_loc_seq, r_gain, l_gain)) = inner.get(&tx_antenna_index) {
                if c_tx_seq == remote_ant_info.seq && c_loc_seq == loc_seq {
                    return EmaneGainResult { remote_gain: r_gain, local_gain: l_gain, status: 0, is_cache: true };
                }
            }
        }
    }

    let mut remote_gain_dbi = 0.0;
    let mut local_gain_dbi = 0.0;

    let is_loc_valid = unsafe { emane_c_location_info_is_valid(location_pair_info) };

    if !remote_ant_info.is_omni {
        if !is_loc_valid {
            return EmaneGainResult { remote_gain: 0.0, local_gain: 0.0, status: 1, is_cache: false };
        }
        if remote_ant_info.has_pointing {
            if let Some((p_pattern, p_blockage, p_north, p_east, p_up)) = crate::antenna::get_manager().get_profile_info(remote_ant_info.profile_id) {
                let local_n = if let Some(p) = crate::antenna::get_manager().get_profile_info(local_ant_info.profile_id) { p.2 } else { 0.0 };
                let local_e = if let Some(p) = crate::antenna::get_manager().get_profile_info(local_ant_info.profile_id) { p.3 } else { 0.0 };
                let local_u = if let Some(p) = crate::antenna::get_manager().get_profile_info(local_ant_info.profile_id) { p.4 } else { 0.0 };
                
                let direction = unsafe {
                    emane_c_utils_calculate_direction_by_neu(location_pair_info, p_north, p_east, p_up, local_n, local_e, local_u)
                };
                
                let lookup = unsafe {
                    emane_c_utils_calculate_lookup_angles(direction.azimuth, remote_ant_info.pointing_azimuth, direction.elevation, remote_ant_info.pointing_elevation)
                };
                
                let tx_gain = unsafe { emane_rs_antenna_pattern_get_gain(p_pattern as *const c_void, lookup.azimuth.round() as i16, lookup.elevation.round() as i16) };
                
                let tx_blockage = if !p_blockage.is_null() {
                    unsafe { emane_rs_antenna_pattern_get_gain(p_blockage as *const c_void, direction.azimuth.round() as i16, direction.elevation.round() as i16) }
                } else {
                    0.0
                };
                
                remote_gain_dbi = tx_gain + tx_blockage;
            } else {
                return EmaneGainResult { remote_gain: 0.0, local_gain: 0.0, status: 2, is_cache: false };
            }
        } else {
            return EmaneGainResult { remote_gain: 0.0, local_gain: 0.0, status: 2, is_cache: false };
        }
    } else {
        remote_gain_dbi = remote_ant_info.fixed_gain;
    }

    if !local_ant_info.is_omni {
        if !is_loc_valid {
            return EmaneGainResult { remote_gain: 0.0, local_gain: 0.0, status: 1, is_cache: false };
        }
        if local_ant_info.has_pointing {
            if let Some((p_pattern, p_blockage, p_north, p_east, p_up)) = crate::antenna::get_manager().get_profile_info(local_ant_info.profile_id) {
                let remote_n = if let Some(p) = crate::antenna::get_manager().get_profile_info(remote_ant_info.profile_id) { p.2 } else { 0.0 };
                let remote_e = if let Some(p) = crate::antenna::get_manager().get_profile_info(remote_ant_info.profile_id) { p.3 } else { 0.0 };
                let remote_u = if let Some(p) = crate::antenna::get_manager().get_profile_info(remote_ant_info.profile_id) { p.4 } else { 0.0 };
                
                let direction = unsafe {
                    emane_c_utils_calculate_direction_by_neu(location_pair_info, p_north, p_east, p_up, remote_n, remote_e, remote_u)
                };
                
                let lookup = unsafe {
                    emane_c_utils_calculate_lookup_angles(direction.azimuth, local_ant_info.pointing_azimuth, direction.elevation, local_ant_info.pointing_elevation)
                };
                
                let rx_gain = unsafe { emane_rs_antenna_pattern_get_gain(p_pattern as *const c_void, lookup.azimuth.round() as i16, lookup.elevation.round() as i16) };
                
                let rx_blockage = if !p_blockage.is_null() {
                    unsafe { emane_rs_antenna_pattern_get_gain(p_blockage as *const c_void, direction.azimuth.round() as i16, direction.elevation.round() as i16) }
                } else {
                    0.0
                };
                
                local_gain_dbi = rx_gain + rx_blockage;
            } else {
                return EmaneGainResult { remote_gain: 0.0, local_gain: 0.0, status: 2, is_cache: false };
            }
        } else {
            return EmaneGainResult { remote_gain: 0.0, local_gain: 0.0, status: 2, is_cache: false };
        }
    } else {
        local_gain_dbi = local_ant_info.fixed_gain;
    }

    let dist = unsafe { emane_c_location_info_get_distance(location_pair_info) };
    
    if is_loc_valid && dist > 10.0 {
        let local_alt = unsafe { emane_c_location_info_get_altitude(location_pair_info, true) };
        let remote_alt = unsafe { emane_c_location_info_get_altitude(location_pair_info, false) };
        let local_u = if let Some(p) = crate::antenna::get_manager().get_profile_info(local_ant_info.profile_id) { p.4 } else { 0.0 };
        let remote_u = if let Some(p) = crate::antenna::get_manager().get_profile_info(remote_ant_info.profile_id) { p.4 } else { 0.0 };
        
        let ok = unsafe { emane_c_utils_check_horizon(local_alt + local_u, remote_alt + remote_u, dist) };
        if !ok {
            return EmaneGainResult { remote_gain: 0.0, local_gain: 0.0, status: 3, is_cache: false };
        }
    }

    let inner = mgr.gain_cache.entry(transmitter_id).or_insert_with(HashMap::new);
    inner.insert(tx_antenna_index, (remote_ant_info.seq, loc_seq, remote_gain_dbi, local_gain_dbi));

    EmaneGainResult {
        remote_gain: remote_gain_dbi,
        local_gain: local_gain_dbi,
        status: 0,
        is_cache: false,
    }
}
