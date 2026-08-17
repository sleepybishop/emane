use std::collections::HashMap;
use std::os::raw::c_void;

pub struct GainManagerImpl {
    gain_cache: HashMap<u16, HashMap<u16, (u64, u64, f64, f64)>>,
    antenna_update_seq: u64,
}

#[no_mangle]
pub extern "C" fn emane_rs_gain_manager_create() -> *mut c_void {
    let mgr = Box::new(GainManagerImpl {
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
pub extern "C" fn emane_rs_gain_manager_set_cache(
    ptr: *mut c_void, tx_nem_id: u16, tx_antenna_idx: u16,
    tx_antenna_seq: u64, location_seq: u64, remote_gain: f64, local_gain: f64
) {
    let mgr = unsafe { &mut *(ptr as *mut GainManagerImpl) };
    let inner = mgr.gain_cache.entry(tx_nem_id).or_insert_with(HashMap::new);
    inner.insert(tx_antenna_idx, (tx_antenna_seq, location_seq, remote_gain, local_gain));
}

#[no_mangle]
pub extern "C" fn emane_rs_gain_manager_get_cache(
    ptr: *mut c_void, tx_nem_id: u16, tx_antenna_idx: u16,
    tx_antenna_seq: u64, location_seq: u64,
    rx_antenna_seq: u64,
    out_remote_gain: *mut f64, out_local_gain: *mut f64
) -> bool {
    let mgr = unsafe { &mut *(ptr as *mut GainManagerImpl) };
    if rx_antenna_seq != mgr.antenna_update_seq {
        mgr.gain_cache.clear();
        mgr.antenna_update_seq = rx_antenna_seq;
        return false;
    }
    if let Some(inner) = mgr.gain_cache.get(&tx_nem_id) {
        if let Some(&(c_tx_seq, c_loc_seq, r_gain, l_gain)) = inner.get(&tx_antenna_idx) {
            if c_tx_seq == tx_antenna_seq && c_loc_seq == location_seq {
                if !out_remote_gain.is_null() {
                    unsafe { *out_remote_gain = r_gain; }
                }
                if !out_local_gain.is_null() {
                    unsafe { *out_local_gain = l_gain; }
                }
                return true;
            }
        }
    }
    false
}
