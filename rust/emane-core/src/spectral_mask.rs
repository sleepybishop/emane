
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use roxmltree::Document;

pub fn frequency_overlap_ratio(freq1: u64, bw1: u64, freq2: u64, bw2: u64) -> (f64, u64, u64) {
    let upper1 = freq1 + bw1 / 2;
    let lower1 = freq1 - bw1 / 2;

    let upper2 = freq2 + bw2 / 2;
    let lower2 = freq2 - bw2 / 2;

    let mut lower_overlap = 0;
    let mut upper_overlap = 0;
    let mut ratio = 0.0;

    if lower2 < upper1 && upper2 > lower1 {
        if lower2 >= lower1 {
            lower_overlap = lower2;
            if upper2 <= upper1 {
                upper_overlap = upper2;
                ratio = 1.0;
            } else {
                upper_overlap = upper1;
                ratio = (upper1 - lower2) as f64 / bw2 as f64;
            }
        } else {
            lower_overlap = lower1;
            if upper2 <= upper1 {
                upper_overlap = upper2;
                ratio = (upper2 - lower1) as f64 / bw2 as f64;
            } else {
                upper_overlap = upper1;
                ratio = (upper1 - lower1) as f64 / bw2 as f64;
            }
        }
    }

    (ratio, lower_overlap, upper_overlap)
}

#[repr(C)]
pub struct FfiSpectralSegment {
    overlap_ratio: f64,
    modifier_mw: f64,
    lower_hz: u64,
    upper_hz: u64,
}

#[repr(C)]
pub struct FfiSpectralOverlap {
    segments: *mut FfiSpectralSegment,
    segments_len: usize,
    lower_hz: u64,
    upper_hz: u64,
}

#[repr(C)]
pub struct FfiMaskOverlap {
    overlaps: *mut FfiSpectralOverlap,
    overlaps_len: usize,
    lower_hz: u64,
    upper_hz: u64,
    total: u64,
}

#[derive(Default)]
pub struct SpectralMaskManager {
    // mask_id -> Vec<(offset, bandwidth, Vec<(width, dBr)>)>
    masks: HashMap<u16, Vec<(i64, u64, Vec<(u64, f64)>)>>,
}

impl SpectralMaskManager {
    pub fn new() -> Self {
        Self {
            masks: HashMap::new(),
        }
    }

    pub fn load(&mut self, uri: &str) -> Result<(), String> {
        let content = std::fs::read_to_string(uri).map_err(|e| format!("Failed to read {}: {}", uri, e))?;
        let doc = Document::parse(&content).map_err(|e| format!("Failed to parse XML: {}", e))?;

        let root = doc.root_element();
        if root.tag_name().name() != "spectral-mask-manifest" {
            return Err("Invalid document root".to_string());
        }

        for node in root.children().filter(|n| n.is_element() && n.tag_name().name() == "mask") {
            let id_str = node.attribute("id").ok_or("mask missing id attribute")?;
            let id: u16 = id_str.parse().map_err(|_| "invalid mask id")?;

            let mut mask_entries = Vec::new();

            for child in node.children().filter(|n| n.is_element()) {
                if child.tag_name().name() == "primary" {
                    let mut shape = Vec::new();
                    let mut bandwidth = 0;
                    for width_node in child.children().filter(|n| n.is_element() && n.tag_name().name() == "width") {
                        let hz_str = width_node.attribute("hz").ok_or("primary width missing hz")?;
                        let hz = parse_frequency(hz_str)?;
                        
                        let dbr = if let Some(dbr_str) = width_node.attribute("dBr") {
                            dbr_str.parse().unwrap_or(0.0)
                        } else {
                            0.0
                        };

                        shape.push((hz, db_to_milliwatt(dbr)));
                        bandwidth += hz;
                    }
                    mask_entries.push((0, bandwidth, shape));
                } else if child.tag_name().name() == "spurs" {
                    for spur_node in child.children().filter(|n| n.is_element() && n.tag_name().name() == "spur") {
                        let offset_str = spur_node.attribute("offset_from_center_hz").ok_or("spur missing offset")?;
                        let offset = parse_frequency_i64(offset_str)?;

                        let mut shape = Vec::new();
                        let mut bandwidth = 0;
                        for width_node in spur_node.children().filter(|n| n.is_element() && n.tag_name().name() == "width") {
                            let hz_str = width_node.attribute("hz").ok_or("spur width missing hz")?;
                            let hz = parse_frequency(hz_str)?;
                            
                            let dbr = width_node.attribute("dBr").ok_or("spur width missing dBr")?.parse().unwrap_or(0.0);

                            shape.push((hz, db_to_milliwatt(dbr)));
                            bandwidth += hz;
                        }
                        mask_entries.push((offset, bandwidth, shape));
                    }
                }
            }
            self.masks.insert(id, mask_entries);
        }

        Ok(())
    }

    pub fn get_primary_signal_bandwidth(&self, mask_id: u16) -> u64 {
        if mask_id != 0 {
            if let Some(mask) = self.masks.get(&mask_id) {
                if !mask.is_empty() {
                    return mask[0].1;
                }
            }
        }
        0
    }

    pub fn get_spectral_overlap(&self, tx_freq: u64, rx_freq: u64, mut rx_bw: u64, tx_bw: u64, mask_id: u16) -> Option<(Vec<(Vec<(f64, f64, u64, u64)>, u64, u64)>, u64, u64, u64)> {
        if mask_id != 0 {
            if let Some(mask_entries) = self.masks.get(&mask_id) {
                let mut lower_mask_overlap = u64::MAX;
                let mut upper_mask_overlap = 0;
                let mut total = 0;
                let mut overlaps = Vec::new();

                for (offset, mask_bw, shape) in mask_entries {
                    if rx_bw == 0 {
                        rx_bw = *mask_bw;
                    }

                    let mut lower_spur_overlap = u64::MAX;
                    let mut upper_spur_overlap = 0;
                    let mut segments = Vec::new();

                    let mut tx_lower = tx_freq as i64 + *offset - (*mask_bw as i64) / 2;

                    for (seg_width, seg_mw) in shape {
                        let seg_width = *seg_width;
                        let tx_upper = tx_lower + seg_width as i64;

                        let tx_center = tx_lower + (seg_width as i64) / 2;

                        let (ratio, lower_seg, upper_seg) = frequency_overlap_ratio(
                            rx_freq, rx_bw, 
                            tx_center as u64, seg_width
                        );

                        if ratio > 0.0 {
                            segments.push((ratio, *seg_mw, tx_lower as u64, tx_upper as u64));
                            if lower_spur_overlap > lower_seg {
                                lower_spur_overlap = lower_seg;
                            }
                            if upper_spur_overlap < upper_seg {
                                upper_spur_overlap = upper_seg;
                            }
                            total += 1;
                        }

                        tx_lower += seg_width as i64;
                    }

                    if !segments.is_empty() {
                        if lower_mask_overlap > lower_spur_overlap {
                            lower_mask_overlap = lower_spur_overlap;
                        }
                        if upper_mask_overlap < upper_spur_overlap {
                            upper_mask_overlap = upper_spur_overlap;
                        }
                        overlaps.push((segments, lower_spur_overlap, upper_spur_overlap));
                    }
                }

                if !overlaps.is_empty() {
                    return Some((overlaps, lower_mask_overlap, upper_mask_overlap, total));
                }
            }
        } else {
            let (ratio, lower, upper) = frequency_overlap_ratio(rx_freq, rx_bw, tx_freq, tx_bw);
            if ratio > 0.0 {
                let segments = vec![(ratio, 1.0, lower, upper)];
                let overlaps = vec![(segments, lower, upper)];
                return Some((overlaps, lower, upper, 1));
            }
        }
        None
    }
}

fn parse_frequency(val: &str) -> Result<u64, String> {
    let mut multiplier = 1.0;
    let mut num_str = val;
    if val.ends_with('G') { multiplier = 1_000_000_000.0; num_str = &val[..val.len()-1]; }
    else if val.ends_with('M') { multiplier = 1_000_000.0; num_str = &val[..val.len()-1]; }
    else if val.ends_with('K') { multiplier = 1_000.0; num_str = &val[..val.len()-1]; }
    
    let num: f64 = num_str.parse().map_err(|_| "invalid frequency")?;
    Ok((num * multiplier) as u64)
}

fn parse_frequency_i64(val: &str) -> Result<i64, String> {
    let mut multiplier = 1.0;
    let mut num_str = val;
    let mut sign = 1.0;
    
    if num_str.starts_with('-') { sign = -1.0; num_str = &num_str[1..]; }
    else if num_str.starts_with('+') { num_str = &num_str[1..]; }

    if num_str.ends_with('G') { multiplier = 1_000_000_000.0; num_str = &num_str[..num_str.len()-1]; }
    else if num_str.ends_with('M') { multiplier = 1_000_000.0; num_str = &num_str[..num_str.len()-1]; }
    else if num_str.ends_with('K') { multiplier = 1_000.0; num_str = &num_str[..num_str.len()-1]; }
    
    let num: f64 = num_str.parse().map_err(|_| "invalid frequency")?;
    Ok((sign * num * multiplier) as i64)
}

fn db_to_milliwatt(db: f64) -> f64 {
    10.0f64.powf(db / 10.0)
}

static mut MANAGER: Option<SpectralMaskManager> = None;

fn get_manager() -> &'static mut SpectralMaskManager {
    unsafe {
        if MANAGER.is_none() {
            MANAGER = Some(SpectralMaskManager::new());
        }
        MANAGER.as_mut().unwrap()
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_spectral_mask_load(uri: *const c_char, error_buf: *mut c_char, error_buf_len: usize) {
    let uri_str = unsafe { CStr::from_ptr(uri).to_string_lossy() };
    match get_manager().load(&uri_str) {
        Ok(_) => {
            if !error_buf.is_null() && error_buf_len > 0 {
                unsafe { *error_buf = 0; }
            }
        },
        Err(e) => {
            if !error_buf.is_null() && error_buf_len > 0 {
                let c_msg = CString::new(e).unwrap();
                let bytes = c_msg.as_bytes_with_nul();
                let copy_len = std::cmp::min(bytes.len(), error_buf_len - 1);
                unsafe {
                    std::ptr::copy_nonoverlapping(bytes.as_ptr(), error_buf as *mut u8, copy_len);
                    *error_buf.add(copy_len) = 0;
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_spectral_mask_get_primary_bandwidth(mask_id: u16) -> u64 {
    get_manager().get_primary_signal_bandwidth(mask_id)
}

#[no_mangle]
pub extern "C" fn emane_rs_spectral_mask_get_overlap(
    tx_freq: u64, rx_freq: u64, rx_bw: u64, tx_bw: u64, mask_id: u16,
    out: *mut FfiMaskOverlap
) -> bool {
    if let Some((overlaps, lower, upper, total)) = get_manager().get_spectral_overlap(tx_freq, rx_freq, rx_bw, tx_bw, mask_id) {
        
        let mut ffi_overlaps = Vec::with_capacity(overlaps.len());
        for (segments, spur_lower, spur_upper) in overlaps {
            let mut ffi_segments = Vec::with_capacity(segments.len());
            for (ratio, mw, seg_lower, seg_upper) in segments {
                ffi_segments.push(FfiSpectralSegment {
                    overlap_ratio: ratio,
                    modifier_mw: mw,
                    lower_hz: seg_lower,
                    upper_hz: seg_upper,
                });
            }
            
            let segments_len = ffi_segments.len();
            let segments_ptr = ffi_segments.leak().as_mut_ptr();
            
            ffi_overlaps.push(FfiSpectralOverlap {
                segments: segments_ptr,
                segments_len,
                lower_hz: spur_lower,
                upper_hz: spur_upper,
            });
        }
        
        let overlaps_len = ffi_overlaps.len();
        let overlaps_ptr = ffi_overlaps.leak().as_mut_ptr();
        
        unsafe {
            *out = FfiMaskOverlap {
                overlaps: overlaps_ptr,
                overlaps_len,
                lower_hz: lower,
                upper_hz: upper,
                total,
            };
        }
        true
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_spectral_mask_free_overlap(overlap: *mut FfiMaskOverlap) {
    unsafe {
        if overlap.is_null() { return; }
        
        let overlaps = Vec::from_raw_parts((*overlap).overlaps, (*overlap).overlaps_len, (*overlap).overlaps_len);
        for ov in overlaps {
            if !ov.segments.is_null() {
                Vec::from_raw_parts(ov.segments, ov.segments_len, ov.segments_len);
            }
        }
    }
}
