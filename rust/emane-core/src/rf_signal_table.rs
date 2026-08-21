use std::collections::{HashMap, HashSet};

#[derive(Clone, Default)]
struct RFSignalCacheEntry {
    num_samples: u64,
    rx_power_accum_dbm: f64,
    noise_floor_accum_db: f64,
    sinr_accum_db: f64,
    inr_accum_db: f64,
}

impl RFSignalCacheEntry {
    fn update(
        &mut self,
        rx_power_dbm: f64,
        sinr_db: f64,
        noise_floor_db: f64,
        receiver_sensitivity_db: f64,
    ) -> (u64, f64, f64, f64, f64) {
        self.rx_power_accum_dbm += rx_power_dbm;
        self.noise_floor_accum_db += noise_floor_db;
        self.sinr_accum_db += sinr_db;
        self.inr_accum_db += noise_floor_db - receiver_sensitivity_db;
        self.num_samples += 1;

        let num = self.num_samples as f64;
        (
            self.num_samples,
            self.rx_power_accum_dbm / num,
            self.noise_floor_accum_db / num,
            self.sinr_accum_db / num,
            self.inr_accum_db / num,
        )
    }
}

pub struct RFSignalTable {
    _nem_id: u16,
    average_all_antennas: bool,
    average_all_frequencies: bool,
    rf_receive_metric_cache: HashMap<String, RFSignalCacheEntry>,
    antenna_tracker: HashMap<u16, HashSet<(u16, u64)>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RFSignalRow {
    pub source: u16,
    pub antenna: Option<u16>,
    pub frequency_hz: Option<u64>,
    pub samples: u64,
    pub average_rx_power_dbm: f64,
    pub average_noise_floor_dbm: f64,
    pub average_sinr_db: f64,
    pub average_inr_db: f64,
}

impl RFSignalTable {
    pub fn new(nem_id: u16) -> Self {
        Self {
            _nem_id: nem_id,
            average_all_antennas: false,
            average_all_frequencies: false,
            rf_receive_metric_cache: HashMap::new(),
            antenna_tracker: HashMap::new(),
        }
    }

    pub fn set_average_all_antennas(&mut self, val: bool) {
        self.average_all_antennas = val;
    }

    pub fn set_average_all_frequencies(&mut self, val: bool) {
        self.average_all_frequencies = val;
    }

    pub fn reset(&mut self, mut rx_antenna_id: u16) {
        if self.average_all_antennas {
            rx_antenna_id = u16::MAX;
        }

        if let Some(set) = self.antenna_tracker.remove(&rx_antenna_id) {
            for (src, freq) in set {
                let key = format!("{}:{}:{}", src, rx_antenna_id, freq);
                self.rf_receive_metric_cache.remove(&key);
            }
        }
    }

    pub fn reset_all(&mut self) {
        self.rf_receive_metric_cache.clear();
        self.antenna_tracker.clear();
    }

    pub fn update(
        &mut self,
        src: u16,
        mut rx_antenna_id: u16,
        mut frequency_hz: u64,
        rx_power_dbm: f64,
        sinr_db: f64,
        noise_floor_db: f64,
        receiver_sensitivity_db: f64,
    ) {
        if self.average_all_antennas {
            rx_antenna_id = u16::MAX;
        }

        if self.average_all_frequencies {
            frequency_hz = u64::MAX;
        }

        let key = format!("{}:{}:{}", src, rx_antenna_id, frequency_hz);

        let entry = self.rf_receive_metric_cache.entry(key).or_default();

        entry.update(
            rx_power_dbm,
            sinr_db,
            noise_floor_db,
            receiver_sensitivity_db,
        );

        self.antenna_tracker
            .entry(rx_antenna_id)
            .or_default()
            .insert((src, frequency_hz));
    }

    pub fn rows(&self) -> Vec<RFSignalRow> {
        let mut rows = self
            .rf_receive_metric_cache
            .iter()
            .filter_map(|(key, entry)| {
                let mut fields = key.split(':');
                let source = fields.next()?.parse().ok()?;
                let antenna = fields.next()?.parse::<u16>().ok()?;
                let frequency_hz = fields.next()?.parse::<u64>().ok()?;
                let samples = entry.num_samples;
                let divisor = samples.max(1) as f64;
                Some(RFSignalRow {
                    source,
                    antenna: (antenna != u16::MAX).then_some(antenna),
                    frequency_hz: (frequency_hz != u64::MAX).then_some(frequency_hz),
                    samples,
                    average_rx_power_dbm: entry.rx_power_accum_dbm / divisor,
                    average_noise_floor_dbm: entry.noise_floor_accum_db / divisor,
                    average_sinr_db: entry.sinr_accum_db / divisor,
                    average_inr_db: entry.inr_accum_db / divisor,
                })
            })
            .collect::<Vec<_>>();
        rows.sort_by_key(|row| (row.source, row.antenna, row.frequency_hz));
        rows
    }
}

use std::ffi::CString;
use std::os::raw::c_char;

#[repr(C)]
pub struct RfSignalUpdateResult {
    pub is_new: bool,
    pub key: *mut c_char,
    pub src: u16,
    pub rx_antenna_id: u16,
    pub frequency_hz: u64,
    pub num_samples: u64,
    pub avg_rx_power: f64,
    pub avg_noise_floor: f64,
    pub avg_sinr: f64,
    pub avg_inr: f64,
}

#[no_mangle]
pub extern "C" fn emane_rs_rf_signal_table_new(nem_id: u16) -> *mut RFSignalTable {
    let table = Box::new(RFSignalTable::new(nem_id));
    Box::into_raw(table)
}

#[no_mangle]
pub extern "C" fn emane_rs_rf_signal_table_free(ptr: *mut RFSignalTable) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_rf_signal_table_configure(
    ptr: *mut RFSignalTable,
    b_average_all_antenna: bool,
    b_average_all_frequencies: bool,
) {
    let table = unsafe { &mut *ptr };
    table.set_average_all_antennas(b_average_all_antenna);
    table.set_average_all_frequencies(b_average_all_frequencies);
}

#[no_mangle]
pub extern "C" fn emane_rs_rf_signal_table_update(
    ptr: *mut RFSignalTable,
    src: u16,
    rx_antenna_id: u16,
    frequency_hz: u64,
    rx_power_dbm: f64,
    sinr_db: f64,
    noise_floor_db: f64,
    receiver_sensitivity_db: f64,
    out_result: *mut RfSignalUpdateResult,
) {
    let table = unsafe { &mut *ptr };
    let mut adj_antenna = rx_antenna_id;
    if table.average_all_antennas {
        adj_antenna = u16::MAX;
    }

    let mut adj_freq = frequency_hz;
    if table.average_all_frequencies {
        adj_freq = u64::MAX;
    }

    let key = format!("{}:{}:{}", src, adj_antenna, adj_freq);
    let is_new = !table.rf_receive_metric_cache.contains_key(&key);

    table.update(
        src,
        rx_antenna_id,
        frequency_hz,
        rx_power_dbm,
        sinr_db,
        noise_floor_db,
        receiver_sensitivity_db,
    );

    let entry = table.rf_receive_metric_cache.get(&key).unwrap();
    let num_samples = entry.num_samples;
    let n = num_samples as f64;
    let avg_rx_power = entry.rx_power_accum_dbm / n;
    let avg_noise_floor = entry.noise_floor_accum_db / n;
    let avg_sinr = entry.sinr_accum_db / n;
    let avg_inr = entry.inr_accum_db / n;

    unsafe {
        (*out_result).is_new = is_new;
        (*out_result).key = CString::new(key).unwrap().into_raw();
        (*out_result).src = src;
        (*out_result).rx_antenna_id = adj_antenna;
        (*out_result).frequency_hz = adj_freq;
        (*out_result).num_samples = num_samples;
        (*out_result).avg_rx_power = avg_rx_power;
        (*out_result).avg_noise_floor = avg_noise_floor;
        (*out_result).avg_sinr = avg_sinr;
        (*out_result).avg_inr = avg_inr;
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_rf_signal_table_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            drop(CString::from_raw(ptr));
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_rf_signal_table_reset(
    ptr: *mut RFSignalTable,
    rx_antenna_id: u16,
    out_keys_len: *mut usize,
) -> *mut *mut c_char {
    let table = unsafe { &mut *ptr };
    let mut adj_antenna = rx_antenna_id;
    if table.average_all_antennas {
        adj_antenna = u16::MAX;
    }

    let mut keys_to_delete = Vec::new();

    if let Some(set) = table.antenna_tracker.remove(&adj_antenna) {
        for (src, freq) in set {
            let key = format!("{}:{}:{}", src, adj_antenna, freq);
            if table.rf_receive_metric_cache.remove(&key).is_some() {
                keys_to_delete.push(CString::new(key).unwrap().into_raw());
            }
        }
    }

    unsafe {
        *out_keys_len = keys_to_delete.len();
    }
    let mut boxed_slice = keys_to_delete.into_boxed_slice();
    let res = boxed_slice.as_mut_ptr();
    std::mem::forget(boxed_slice);
    res
}

#[no_mangle]
pub extern "C" fn emane_rs_rf_signal_table_free_keys(keys_ptr: *mut *mut c_char, len: usize) {
    if keys_ptr.is_null() {
        return;
    }
    let keys = unsafe { Vec::from_raw_parts(keys_ptr, len, len) };
    for k in keys {
        unsafe {
            drop(CString::from_raw(k));
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_rf_signal_table_reset_all(ptr: *mut RFSignalTable) {
    let table = unsafe { &mut *ptr };
    table.reset_all();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn averages_and_collapses_requested_dimensions() {
        let mut table = RFSignalTable::new(1);
        table.set_average_all_antennas(true);
        table.set_average_all_frequencies(true);
        table.update(2, 4, 2_400_000_000, -50.0, 10.0, -60.0, -90.0);
        table.update(2, 7, 2_410_000_000, -40.0, 20.0, -55.0, -90.0);
        let rows = table.rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].source, 2);
        assert_eq!(rows[0].antenna, None);
        assert_eq!(rows[0].frequency_hz, None);
        assert_eq!(rows[0].samples, 2);
        assert_eq!(rows[0].average_rx_power_dbm, -45.0);
        assert_eq!(rows[0].average_sinr_db, 15.0);
        assert_eq!(rows[0].average_inr_db, 32.5);
        table.reset(4);
        assert!(table.rows().is_empty());
    }
}
