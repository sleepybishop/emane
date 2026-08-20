use std::slice;

#[repr(C)]
pub struct EmaneRsSpectrumCompressedRepresentation {
    pub ptr: *mut EmaneRsSpectrumCompressedEntry,
    pub len: usize,
    pub cap: usize,
}

#[repr(C)]
pub struct EmaneRsSpectrumCompressedEntry {
    pub index: usize,
    pub value: f64,
}

#[repr(C)]
pub struct EmaneRsSpectrumSubBandCompressedRepresentation {
    pub ptr: *mut EmaneRsSpectrumSubBandCompressedEntry,
    pub len: usize,
    pub cap: usize,
}

#[repr(C)]
pub struct EmaneRsSpectrumSubBandCompressedEntry {
    pub index: usize,
    pub ptr: *mut f64,
    pub len: usize,
    pub cap: usize,
}

#[repr(C)]
pub struct EmaneRsMaxBinNoiseFloorResult {
    pub noise_floor_db: f64,
    pub is_signal_in_noise: bool,
    pub error_code: i32,
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_spectrum_compress(
    window_ptr: *const f64,
    window_len: usize,
) -> EmaneRsSpectrumCompressedRepresentation {
    let window = slice::from_raw_parts(window_ptr, window_len);
    let mut ret = Vec::new();
    let mut d_previous = 0.0;

    for (i, &entry) in window.iter().enumerate() {
        if d_previous != entry || (ret.is_empty() && entry != 0.0) {
            ret.push(EmaneRsSpectrumCompressedEntry {
                index: i,
                value: entry,
            });
            d_previous = entry;
        }
    }

    let (ptr, len, cap) = ret.into_raw_parts();
    EmaneRsSpectrumCompressedRepresentation { ptr, len, cap }
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_spectrum_compress_free(
    repr: EmaneRsSpectrumCompressedRepresentation,
) {
    if repr.ptr.is_null() {
        return;
    }
    let _ = Vec::from_raw_parts(repr.ptr, repr.len, repr.cap);
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_spectrum_sub_band_compress(
    window_ptr: *const f64,
    window_len: usize,
    sub_band_bin_count: usize,
) -> EmaneRsSpectrumSubBandCompressedRepresentation {
    let window = slice::from_raw_parts(window_ptr, window_len);
    let mut ret = Vec::new();
    let mut d_previous = vec![0.0; sub_band_bin_count];

    let mut b_write = false;
    let mut i = 0;
    let mut sub_band_bin_index = 0;

    for &entry in window.iter() {
        if d_previous[sub_band_bin_index] != entry || (ret.is_empty() && entry != 0.0) {
            b_write = true;
            d_previous[sub_band_bin_index] = entry;
        }

        sub_band_bin_index += 1;

        if sub_band_bin_index == sub_band_bin_count {
            sub_band_bin_index = 0;

            if b_write {
                let vals = d_previous.clone();
                let (ptr, len, cap) = vals.into_raw_parts();
                ret.push(EmaneRsSpectrumSubBandCompressedEntry {
                    index: i,
                    ptr,
                    len,
                    cap,
                });
                b_write = false;
            }
            i += 1;
        }
    }

    let (ptr, len, cap) = ret.into_raw_parts();
    EmaneRsSpectrumSubBandCompressedRepresentation { ptr, len, cap }
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_spectrum_sub_band_compress_free(
    repr: EmaneRsSpectrumSubBandCompressedRepresentation,
) {
    if repr.ptr.is_null() {
        return;
    }
    let entries = Vec::from_raw_parts(repr.ptr, repr.len, repr.cap);
    for entry in entries {
        if !entry.ptr.is_null() {
            let _ = Vec::from_raw_parts(entry.ptr, entry.len, entry.cap);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_max_bin_noise_floor(
    noise_data_ptr: *const f64,
    noise_data_len: usize,
    rx_sensitivity_milliwatt: f64,
    rx_power_dbm: f64,
    b_signal_in_noise: bool,
    start_bin: usize,
    end_bin: usize,
) -> EmaneRsMaxBinNoiseFloorResult {
    let noise_data = slice::from_raw_parts(noise_data_ptr, noise_data_len);

    if end_bin < start_bin {
        return EmaneRsMaxBinNoiseFloorResult {
            noise_floor_db: 0.0,
            is_signal_in_noise: b_signal_in_noise,
            error_code: 1, // Max bin end index < start index
        };
    }

    if end_bin >= noise_data_len || start_bin >= noise_data_len {
        return EmaneRsMaxBinNoiseFloorResult {
            noise_floor_db: 0.0,
            is_signal_in_noise: b_signal_in_noise,
            error_code: 2, // Out of bounds
        };
    }

    let slice = &noise_data[start_bin..=end_bin];
    let max_val = slice.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    let mut d_noise_floor_milliwatt = max_val;
    if b_signal_in_noise {
        // DB_TO_MILLIWATT is 10^(dbm/10.0)
        let rx_power_mw = 10.0_f64.powf(rx_power_dbm / 10.0);
        d_noise_floor_milliwatt -= rx_power_mw;
    }

    if d_noise_floor_milliwatt < rx_sensitivity_milliwatt {
        d_noise_floor_milliwatt = rx_sensitivity_milliwatt;
    }

    // MILLIWATT_TO_DB is 10 * log10(mw)
    let d_noise_floor_db = 10.0 * d_noise_floor_milliwatt.log10();

    EmaneRsMaxBinNoiseFloorResult {
        noise_floor_db: d_noise_floor_db,
        is_signal_in_noise: b_signal_in_noise,
        error_code: 0,
    }
}
