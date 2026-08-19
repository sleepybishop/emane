
use std::collections::HashMap;
use std::ffi::{c_char, CStr, c_void};
use std::fs;
use roxmltree;

pub struct PCREntry {
    pub sinr: f32,
    pub por: f32,
}

pub struct PCRPOR {
    pub pcr: Vec<PCREntry>,
    pub por: Vec<f32>,
}

pub struct PCRManager {
    pcr_por_map: HashMap<u16, PCRPOR>,
    table_packet_size: u32,
    precision_factor: i32,
}

impl PCRManager {
    pub fn new() -> Self {
        Self {
            pcr_por_map: HashMap::new(),
            table_packet_size: 0,
            precision_factor: 100,
        }
    }

    pub fn load(&mut self, uri: &str) -> Result<(), String> {
        let content = fs::read_to_string(uri).map_err(|e| format!("Failed to read file: {}", e))?;
        let doc = roxmltree::Document::parse(&content).map_err(|e| format!("Failed to parse XML: {}", e))?;

        self.pcr_por_map.clear();

        for node in doc.descendants() {
            if node.has_tag_name("pcr") {
                for child in node.children() {
                    if child.has_tag_name("table") {
                        if let Some(pkt_size) = child.attribute("pktsize") {
                            self.table_packet_size = pkt_size.parse().unwrap_or(0);
                        }

                        for datarate_node in child.children() {
                            if datarate_node.has_tag_name("datarate") {
                                let index: u16 = datarate_node.attribute("index").unwrap_or("0").parse().unwrap_or(0);
                                let mut pcr_entry_vector: Vec<PCREntry> = Vec::new();

                                for row in datarate_node.children() {
                                    if row.has_tag_name("row") {
                                        let sinr: f32 = row.attribute("sinr").unwrap_or("0").parse().unwrap_or(0.0);
                                        let por_percent: f32 = row.attribute("por").unwrap_or("0").parse().unwrap_or(0.0);
                                        let por = por_percent / 100.0;

                                        if let Some(last) = pcr_entry_vector.last() {
                                            if sinr == last.sinr {
                                                return Err(format!("Duplicate sinr value {}", sinr));
                                            } else if sinr < last.sinr {
                                                return Err(format!("Out of order sinr value {}", sinr));
                                            }
                                        }

                                        pcr_entry_vector.push(PCREntry { sinr, por });
                                    }
                                }

                                if self.pcr_por_map.insert(index, PCRPOR { pcr: pcr_entry_vector, por: Vec::new() }).is_some() {
                                    return Err(format!("Duplicate datarate index value {}", index));
                                }
                            }
                        }
                    }
                }
            }
        }

        for pcr_por in self.pcr_por_map.values() {
            if pcr_por.pcr.is_empty() {
                return Err("Need at least 1 point to define a pcr curve".to_string());
            }
        }

        self.interpolate();
        Ok(())
    }

    fn interpolate(&mut self) {
        for pcr_por in self.pcr_por_map.values_mut() {
            for i in 0..pcr_por.pcr.len() - 1 {
                let x1 = (pcr_por.pcr[i].sinr * self.precision_factor as f32) as i32;
                let y1 = pcr_por.pcr[i].por;
                let x2 = (pcr_por.pcr[i + 1].sinr * self.precision_factor as f32) as i32;
                let y2 = pcr_por.pcr[i + 1].por;

                let slope = (y2 - y1) / (x2 - x1) as f32;

                for dx in x1..x2 {
                    pcr_por.por.push(((dx - x1) as f32) * slope + y1);
                }
            }

            pcr_por.por.push(pcr_por.pcr.last().unwrap().por);
        }
    }

    pub fn get_pcr(&self, sinr: f32, packet_len: usize, data_rate_index: u16) -> f32 {
        if let Some(pcr_por) = self.pcr_por_map.get(&data_rate_index) {
            if pcr_por.pcr.is_empty() {
                return 0.0;
            }

            let first = pcr_por.pcr.first().unwrap();
            let last = pcr_por.pcr.last().unwrap();

            if sinr < first.sinr {
                return 0.0;
            } else if sinr > last.sinr {
                return 1.0;
            }

            let mut por = if first.sinr == last.sinr {
                first.por
            } else {
                let sinr_scaled = (sinr * self.precision_factor as f32) as i32;
                let sinr_offset = (first.sinr * self.precision_factor as f32) as i32;

                let mut idx = (sinr_scaled - sinr_offset) as usize;
                if idx >= pcr_por.por.len() {
                    idx = pcr_por.por.len() - 1;
                }

                pcr_por.por[idx]
            };

            if self.table_packet_size > 0 {
                por = por.powf(packet_len as f32 / self.table_packet_size as f32);
            }

            por
        } else {
            0.0
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_ieee80211abg_pcrmanager_new() -> *mut c_void {
    Box::into_raw(Box::new(PCRManager::new())) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_ieee80211abg_pcrmanager_drop(mgr: *mut c_void) {
    if !mgr.is_null() {
        unsafe {
            let _ = Box::from_raw(mgr as *mut PCRManager);
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_ieee80211abg_pcrmanager_load(
    mgr: *mut c_void,
    uri: *const c_char,
) -> bool {
    let mgr = unsafe { &mut *(mgr as *mut PCRManager) };
    if uri.is_null() {
        return false;
    }
    let uri_str = unsafe { CStr::from_ptr(uri).to_string_lossy().into_owned() };
    mgr.load(&uri_str).is_ok()
}

#[no_mangle]
pub extern "C" fn emane_rs_ieee80211abg_pcrmanager_get_pcr(
    mgr: *mut c_void,
    sinr: f32,
    packet_len: usize,
    data_rate_index: u16,
) -> f32 {
    let mgr = unsafe { &*(mgr as *mut PCRManager) };
    mgr.get_pcr(sinr, packet_len, data_rate_index)
}
