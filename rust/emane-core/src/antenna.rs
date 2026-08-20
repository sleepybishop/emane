use roxmltree::Document;
use std::collections::{BTreeMap, HashMap};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

pub struct AntennaPattern {
    elevation_bearing_gain: BTreeMap<i16, Option<BTreeMap<i16, f64>>>,
    missing_value: f64,
}

impl AntennaPattern {
    pub fn new(uri: &str, sub_root_name: &str, missing_value: f64) -> Result<Self, String> {
        let content =
            std::fs::read_to_string(uri).map_err(|e| format!("Unable to read {}: {}", uri, e))?;
        let doc =
            Document::parse(&content).map_err(|e| format!("Validation failure {}: {}", uri, e))?;

        let root = doc.root_element();

        let mut pattern = AntennaPattern {
            elevation_bearing_gain: BTreeMap::new(),
            missing_value,
        };

        for node in root
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == sub_root_name)
        {
            let mut elevation_range_max: i16 = 0;
            let mut first_elevation = true;

            for elevation_node in node
                .children()
                .filter(|n| n.is_element() && n.tag_name().name() == "elevation")
            {
                let min_str = elevation_node
                    .attribute("min")
                    .ok_or_else(|| format!("Bad entry {}: elevation missing min", uri))?;
                let elevation_min: i16 = min_str
                    .parse()
                    .map_err(|_| format!("Bad entry {}: elevation min parse", uri))?;

                let max_str = elevation_node
                    .attribute("max")
                    .ok_or_else(|| format!("Bad entry {}: elevation missing max", uri))?;
                let elevation_max: i16 = max_str
                    .parse()
                    .map_err(|_| format!("Bad entry {}: elevation max parse", uri))?;

                if elevation_min > elevation_max {
                    return Err(format!(
                        "Bad entry {}: elevation [{},{}] min > max",
                        uri, elevation_min, elevation_max
                    ));
                }

                if !first_elevation
                    && elevation_range_max >= elevation_min
                    && !pattern.elevation_bearing_gain.is_empty()
                {
                    return Err(format!("Bad entry {}: elevation [{},{}] may be out of order or overlap with a previous elevation", uri, elevation_min, elevation_max));
                } else {
                    elevation_range_max = elevation_max;
                    first_elevation = false;
                }

                let mut bearing_gain_map = BTreeMap::new();
                let mut bearing_range_max: i16 = 0;
                let mut first_bearing = true;

                for bearing_node in elevation_node
                    .children()
                    .filter(|n| n.is_element() && n.tag_name().name() == "bearing")
                {
                    let b_min_str = bearing_node
                        .attribute("min")
                        .ok_or_else(|| format!("Bad entry {}: bearing missing min", uri))?;
                    let bearing_min: i16 = b_min_str
                        .parse()
                        .map_err(|_| format!("Bad entry {}: bearing min parse", uri))?;

                    let b_max_str = bearing_node
                        .attribute("max")
                        .ok_or_else(|| format!("Bad entry {}: bearing missing max", uri))?;
                    let bearing_max: i16 = b_max_str
                        .parse()
                        .map_err(|_| format!("Bad entry {}: bearing max parse", uri))?;

                    if bearing_min > bearing_max {
                        return Err(format!(
                            "Bad entry {}: elevation [{},{}] bearing [{},{}] min > max",
                            uri, elevation_min, elevation_max, bearing_min, bearing_max
                        ));
                    }

                    if !first_bearing
                        && bearing_range_max >= bearing_min
                        && !bearing_gain_map.is_empty()
                    {
                        return Err(format!("Bad entry {}: elevation [{},{}] bearing [{},{}] may be out of order or overlap", uri, elevation_min, elevation_max, bearing_min, bearing_max));
                    } else {
                        bearing_range_max = bearing_max;
                        first_bearing = false;
                    }

                    for gain_node in bearing_node
                        .children()
                        .filter(|n| n.is_element() && n.tag_name().name() == "gain")
                    {
                        let gain_str = gain_node
                            .attribute("value")
                            .ok_or_else(|| format!("Bad entry {}: gain missing value", uri))?;
                        let gain: f64 = gain_str
                            .parse()
                            .map_err(|_| format!("Bad entry {}: gain value parse", uri))?;

                        if !bearing_gain_map.contains_key(&(bearing_min - 1)) {
                            bearing_gain_map.insert(bearing_min - 1, missing_value);
                        }
                        bearing_gain_map.insert(bearing_max, gain);
                    }
                }

                if !pattern
                    .elevation_bearing_gain
                    .contains_key(&(elevation_min - 1))
                {
                    pattern
                        .elevation_bearing_gain
                        .insert(elevation_min - 1, None);
                }
                pattern
                    .elevation_bearing_gain
                    .insert(elevation_max, Some(bearing_gain_map));
            }
        }

        Ok(pattern)
    }

    pub fn get_gain(&self, mut bearing: i16, elevation: i16) -> f64 {
        let mut gain = self.missing_value;

        if bearing == 360 {
            bearing = 0;
        }

        if let Some((_, Some(bearing_map))) = self.elevation_bearing_gain.range(elevation..).next()
        {
            if let Some((_, val)) = bearing_map.range(bearing..).next() {
                gain = *val;
            }
        }

        gain
    }
}

pub struct AntennaProfileManifest {
    profiles: HashMap<u16, (usize, usize, f64, f64, f64)>, // (antenna_idx, blockage_idx, north, east, up)
    patterns: Vec<AntennaPattern>,
    pattern_uri_to_idx: HashMap<String, usize>,
}

impl AntennaProfileManifest {
    pub fn new() -> Self {
        Self {
            profiles: HashMap::new(),
            patterns: Vec::new(),
            pattern_uri_to_idx: HashMap::new(),
        }
    }

    fn get_or_load_pattern(
        &mut self,
        uri: &str,
        sub_root: &str,
        missing: f64,
    ) -> Result<usize, String> {
        if let Some(&idx) = self.pattern_uri_to_idx.get(uri) {
            return Ok(idx);
        }
        let p = AntennaPattern::new(uri, sub_root, missing)?;
        let idx = self.patterns.len();
        self.patterns.push(p);
        self.pattern_uri_to_idx.insert(uri.to_string(), idx);
        Ok(idx)
    }

    pub fn load(&mut self, uri: &str) -> Result<(), String> {
        let content =
            std::fs::read_to_string(uri).map_err(|e| format!("Unable to read {}: {}", uri, e))?;
        let doc =
            Document::parse(&content).map_err(|e| format!("Validation failure {}: {}", uri, e))?;

        let root = doc.root_element();
        if root.tag_name().name() != "antennaprofilemanifest" {
            // It might not be strict about root tag name in C++, but it checks for "profile" children.
        }

        for node in root
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == "profile")
        {
            let id_str = node
                .attribute("id")
                .ok_or_else(|| format!("profile missing id"))?;
            let id: u16 = id_str.parse().map_err(|_| format!("profile id parse"))?;

            let antenna_uri = node
                .attribute("antennapatternuri")
                .ok_or_else(|| format!("profile missing antennapatternuri"))?;
            let blockage_uri = node.attribute("blockagepatternuri");

            let mut north = 0.0;
            let mut east = 0.0;
            let mut up = 0.0;

            for child in node
                .children()
                .filter(|n| n.is_element() && n.tag_name().name() == "placement")
            {
                north = child
                    .attribute("north")
                    .unwrap_or("0")
                    .parse()
                    .unwrap_or(0.0);
                east = child
                    .attribute("east")
                    .unwrap_or("0")
                    .parse()
                    .unwrap_or(0.0);
                up = child.attribute("up").unwrap_or("0").parse().unwrap_or(0.0);
            }

            let ant_idx =
                self.get_or_load_pattern(antenna_uri, "antennapattern", -300.0 /* DBM_MIN */)?;

            let mut blk_idx = usize::MAX;
            if let Some(uri) = blockage_uri {
                if !uri.is_empty() {
                    blk_idx = self.get_or_load_pattern(uri, "blockagepattern", 0.0)?;
                }
            }

            if self.profiles.contains_key(&id) {
                return Err(format!("Duplicate antenna profile id {}", id));
            }
            self.profiles
                .insert(id, (ant_idx, blk_idx, north, east, up));
        }

        Ok(())
    }

    pub fn get_profile_info(
        &self,
        id: u16,
    ) -> Option<(*const AntennaPattern, *const AntennaPattern, f64, f64, f64)> {
        if let Some(&(ant_idx, blk_idx, north, east, up)) = self.profiles.get(&id) {
            let ant_ptr = &self.patterns[ant_idx] as *const AntennaPattern;
            let blk_ptr = if blk_idx == usize::MAX {
                std::ptr::null()
            } else {
                &self.patterns[blk_idx] as *const AntennaPattern
            };
            Some((ant_ptr, blk_ptr, north, east, up))
        } else {
            None
        }
    }
}

static mut MANAGER: Option<AntennaProfileManifest> = None;

pub fn get_manager() -> &'static mut AntennaProfileManifest {
    unsafe {
        if MANAGER.is_none() {
            MANAGER = Some(AntennaProfileManifest::new());
        }
        MANAGER.as_mut().unwrap()
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_antenna_profile_load(
    uri: *const c_char,
    error_buf: *mut c_char,
    error_buf_len: usize,
) {
    let uri_str = unsafe { CStr::from_ptr(uri).to_string_lossy() };
    match get_manager().load(&uri_str) {
        Ok(_) => {
            if !error_buf.is_null() && error_buf_len > 0 {
                unsafe {
                    *error_buf = 0;
                }
            }
        }
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
pub extern "C" fn emane_rs_antenna_profile_get_info(
    id: u16,
    ant_ptr_out: *mut *const AntennaPattern,
    blk_ptr_out: *mut *const AntennaPattern,
    north_out: *mut f64,
    east_out: *mut f64,
    up_out: *mut f64,
) -> bool {
    if let Some((a, b, n, e, u)) = get_manager().get_profile_info(id) {
        unsafe {
            *ant_ptr_out = a;
            *blk_ptr_out = b;
            *north_out = n;
            *east_out = e;
            *up_out = u;
        }
        true
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_antenna_pattern_get_gain(
    pattern: *const AntennaPattern,
    bearing: i16,
    elevation: i16,
) -> f64 {
    if pattern.is_null() {
        return -300.0; // Fallback
    }
    unsafe { (*pattern).get_gain(bearing, elevation) }
}
