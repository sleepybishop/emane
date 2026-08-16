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
    nem_id: u16,
    average_all_antennas: bool,
    average_all_frequencies: bool,
    rf_receive_metric_cache: HashMap<String, RFSignalCacheEntry>,
    antenna_tracker: HashMap<u16, HashSet<(u16, u64)>>,
}

impl RFSignalTable {
    pub fn new(nem_id: u16) -> Self {
        Self {
            nem_id,
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
            rx_antenna_id = 0;
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
            rx_antenna_id = 0;
        }

        if self.average_all_frequencies {
            frequency_hz = 0;
        }

        let key = format!("{}:{}:{}", src, rx_antenna_id, frequency_hz);

        let entry = self
            .rf_receive_metric_cache
            .entry(key)
            .or_insert_with(RFSignalCacheEntry::default);

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
}
