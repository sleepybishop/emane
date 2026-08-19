use std::ffi::c_void;

pub struct WMMManager {
    num_categories: u8,
    local_utilization_vector: Vec<u64>,
    total_utilization_vector: Vec<u64>,
    total_utilization_microseconds: u64,
}

impl WMMManager {
    pub fn new() -> Self {
        Self {
            num_categories: 1,
            local_utilization_vector: vec![0; 1],
            total_utilization_vector: vec![0; 1],
            total_utilization_microseconds: 0,
        }
    }

    pub fn set_num_categories(&mut self, num_categories: u8) {
        if self.num_categories != num_categories {
            self.total_utilization_vector.resize(num_categories as usize, 0);
            self.local_utilization_vector.resize(num_categories as usize, 0);
            self.num_categories = num_categories;
            self.reset_counters();
        }
    }

    pub fn update_total_activity(&mut self, category: u8, duration: u64) {
        if category >= self.num_categories {
            return;
        }
        self.total_utilization_vector[category as usize] += duration;
        self.total_utilization_microseconds += duration;
    }

    pub fn update_local_activity(&mut self, category: u8, duration: u64) {
        if category >= self.num_categories {
            return;
        }
        self.local_utilization_vector[category as usize] += duration;
        self.total_utilization_vector[category as usize] += duration;
        self.total_utilization_microseconds += duration;
    }

    pub fn reset_counters(&mut self) {
        for val in self.total_utilization_vector.iter_mut() {
            *val = 0;
        }
        for val in self.local_utilization_vector.iter_mut() {
            *val = 0;
        }
        self.total_utilization_microseconds = 0;
    }

    pub fn get_utilization_ratios(&mut self, delta_t_microseconds: u64) -> Vec<(f32, f32)> {
        let mut vec = vec![(0.0, 0.0); self.num_categories as usize];

        if self.total_utilization_microseconds > 0 && delta_t_microseconds > 0 {
            let mut activity_ratio = self.total_utilization_microseconds as f32 / delta_t_microseconds as f32;
            if activity_ratio > 1.0 {
                activity_ratio = 1.0;
            }

            for category in 0..(self.num_categories as usize) {
                let total_ratio = (self.total_utilization_vector[category] as f32 / self.total_utilization_microseconds as f32) * activity_ratio;
                vec[category].0 = total_ratio;

                if self.total_utilization_vector[category] > 0 {
                    vec[category].1 = self.local_utilization_vector[category] as f32 / self.total_utilization_vector[category] as f32;
                } else {
                    vec[category].1 = 0.0;
                }
            }
        }

        self.reset_counters();
        vec
    }
}

#[repr(C)]
pub struct UtilizationRatioPair {
    pub first: f32,
    pub second: f32,
}

#[no_mangle]
pub extern "C" fn emane_rs_ieee80211abg_wmmmanager_new() -> *mut c_void {
    Box::into_raw(Box::new(WMMManager::new())) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_ieee80211abg_wmmmanager_drop(mgr: *mut c_void) {
    if !mgr.is_null() {
        unsafe {
            let _ = Box::from_raw(mgr as *mut WMMManager);
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_ieee80211abg_wmmmanager_update_total_activity(
    mgr: *mut c_void,
    category: u8,
    duration_microseconds: u64,
) {
    let mgr = unsafe { &mut *(mgr as *mut WMMManager) };
    mgr.update_total_activity(category, duration_microseconds);
}

#[no_mangle]
pub extern "C" fn emane_rs_ieee80211abg_wmmmanager_update_local_activity(
    mgr: *mut c_void,
    category: u8,
    duration_microseconds: u64,
) {
    let mgr = unsafe { &mut *(mgr as *mut WMMManager) };
    mgr.update_local_activity(category, duration_microseconds);
}

#[no_mangle]
pub extern "C" fn emane_rs_ieee80211abg_wmmmanager_set_num_categories(
    mgr: *mut c_void,
    num_categories: u8,
) {
    let mgr = unsafe { &mut *(mgr as *mut WMMManager) };
    mgr.set_num_categories(num_categories);
}

#[no_mangle]
pub extern "C" fn emane_rs_ieee80211abg_wmmmanager_get_utilization_ratios(
    mgr: *mut c_void,
    delta_t_microseconds: u64,
    out_ratios: *mut UtilizationRatioPair,
) -> usize {
    let mgr = unsafe { &mut *(mgr as *mut WMMManager) };
    let ratios = mgr.get_utilization_ratios(delta_t_microseconds);
    unsafe {
        for (i, (first, second)) in ratios.iter().enumerate() {
            (*out_ratios.add(i)).first = *first;
            (*out_ratios.add(i)).second = *second;
        }
    }
    ratios.len()
}
