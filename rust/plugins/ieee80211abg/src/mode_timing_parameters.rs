use crate::mac_config::MACConfig;
use std::ffi::c_void;

pub struct ModeTimingParameters {
    pub mac_config: MACConfig,
    pub timing_params: [TimingParams; 4],
}

#[derive(Clone, Copy, Default)]
pub struct TimingParams {
    pub rts_bit_length: u16,
    pub cts_bit_length: u16,
    pub ack_bit_length: u16,
    pub slot_microseconds: u64,
    pub sifs_microseconds: u64,
    pub preamble_microseconds: u64,
}

const MODULATION_TYPE_DEFAULT: usize = 0;
const MODULATION_TYPE_80211A: usize = 1;
const MODULATION_TYPE_80211B: usize = 2;
const MODULATION_TYPE_80211BG: usize = 3;

impl ModeTimingParameters {
    pub fn new(mac_config: MACConfig) -> Self {
        let mut timing_params = [TimingParams::default(); 4];

        timing_params[MODULATION_TYPE_DEFAULT] = TimingParams {
            rts_bit_length: 160,
            cts_bit_length: 112,
            ack_bit_length: 112,
            slot_microseconds: 9,
            sifs_microseconds: 10,
            preamble_microseconds: 192,
        };
        timing_params[MODULATION_TYPE_80211A] = TimingParams {
            rts_bit_length: 160,
            cts_bit_length: 112,
            ack_bit_length: 112,
            slot_microseconds: 9,
            sifs_microseconds: 16,
            preamble_microseconds: 20,
        };
        timing_params[MODULATION_TYPE_80211B] = TimingParams {
            rts_bit_length: 160,
            cts_bit_length: 112,
            ack_bit_length: 112,
            slot_microseconds: 20,
            sifs_microseconds: 10,
            preamble_microseconds: 192,
        };
        timing_params[MODULATION_TYPE_80211BG] = TimingParams {
            rts_bit_length: 160,
            cts_bit_length: 112,
            ack_bit_length: 112,
            slot_microseconds: 20,
            sifs_microseconds: 16,
            preamble_microseconds: 192,
        };

        Self {
            mac_config,
            timing_params,
        }
    }

    pub fn get_sifs_microseconds(&self, mode: i32) -> u64 {
        self.timing_params[mode as usize].sifs_microseconds
    }

    pub fn get_slot_size_microseconds(&self) -> u64 {
        let mut slot =
            self.timing_params[self.mac_config.get_modulationtype() as usize].slot_microseconds;
        if self.mac_config.get_maxp2pdistance() > 0 {
            slot += self.get_propagation_microseconds(self.mac_config.get_maxp2pdistance());
        }
        slot
    }

    pub fn get_propagation_microseconds(&self, distance: u32) -> u64 {
        let time = (distance as f32 / 299792458.0f32) * 1_000_000.0;
        time.ceil() as u64
    }

    pub fn get_overhead_microseconds(&self, category: u8) -> u64 {
        let mode = self.mac_config.get_modulationtype();
        self.timing_params[mode as usize].preamble_microseconds
            + self.timing_params[mode as usize].sifs_microseconds
            + self.get_aifs_microseconds(category)
    }

    pub fn get_defer_interval_microseconds(&self, category: u8) -> u64 {
        self.timing_params[self.mac_config.get_modulationtype() as usize].sifs_microseconds
            + self.get_aifs_microseconds(category)
    }

    pub fn get_aifs_microseconds(&self, category: u8) -> u64 {
        if self.mac_config.get_wmmenable() {
            self.mac_config.get_aifsmicroseconds(category)
        } else {
            self.mac_config.get_aifsmicroseconds(0)
        }
    }

    pub fn get_message_duration_microseconds(&self, category: u8, len: usize) -> u64 {
        let mode = self.mac_config.get_modulationtype();
        let rate_kbps = self
            .mac_config
            .get_unicastdataratekbps_by_category(category);
        let bits = (len as u64 * 8) + 272; // IEEE_80211MAC_DATAHEADER_BITLEN

        let tx_time = if rate_kbps > 0 {
            let usec = (bits as f64 / rate_kbps as f64) * 1000.0;
            usec.ceil() as u64
        } else {
            0
        };

        self.timing_params[mode as usize].preamble_microseconds + tx_time
    }

    pub fn get_broadcast_message_duration_microseconds(&self, len: usize) -> u64 {
        let mode = self.mac_config.get_modulationtype();
        let rate_kbps = self.mac_config.get_broadcastdataratekbps();
        let bits = (len as u64 * 8) + 272;

        let tx_time = if rate_kbps > 0 {
            let usec = (bits as f64 / rate_kbps as f64) * 1000.0;
            usec.ceil() as u64
        } else {
            0
        };

        self.timing_params[mode as usize].preamble_microseconds + tx_time
    }

    pub fn get_unicast_message_duration_microseconds(&self, len: usize) -> u64 {
        let mode = self.mac_config.get_modulationtype();
        let rate_kbps = self.mac_config.get_unicastdataratekbps();
        let bits = (len as u64 * 8) + 272;

        let tx_time = if rate_kbps > 0 {
            let usec = (bits as f64 / rate_kbps as f64) * 1000.0;
            usec.ceil() as u64
        } else {
            0
        };

        self.timing_params[mode as usize].preamble_microseconds + tx_time
    }

    pub fn get_cts_message_duration_microseconds(&self) -> u64 {
        let mode = self.mac_config.get_modulationtype();
        let rate_kbps = self.mac_config.get_unicastdataratekbps();
        let bits = self.timing_params[mode as usize].cts_bit_length as u64;

        let tx_time = if rate_kbps > 0 {
            let usec = (bits as f64 / rate_kbps as f64) * 1000.0;
            usec.ceil() as u64
        } else {
            0
        };

        self.timing_params[mode as usize].preamble_microseconds + tx_time
    }

    pub fn get_rts_message_duration_microseconds(&self) -> u64 {
        let mode = self.mac_config.get_modulationtype();
        let rate_kbps = self.mac_config.get_unicastdataratekbps();
        let bits = self.timing_params[mode as usize].rts_bit_length as u64;

        let tx_time = if rate_kbps > 0 {
            let usec = (bits as f64 / rate_kbps as f64) * 1000.0;
            usec.ceil() as u64
        } else {
            0
        };

        self.timing_params[mode as usize].preamble_microseconds + tx_time
    }

    // Since we don't have TimePoint easily, we just do it in C++ for `packetTimedOut` and `getSotTime` for now,
    // or port it if needed. Actually it's just a time delta.
}

#[no_mangle]
pub extern "C" fn emane_ieee80211abg_modetimingparameters_new(
    config_ptr: *mut c_void,
) -> *mut c_void {
    Box::into_raw(Box::new(ModeTimingParameters::new(MACConfig::new(
        config_ptr,
    )))) as *mut _
}

#[no_mangle]
pub extern "C" fn emane_ieee80211abg_modetimingparameters_free(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr as *mut ModeTimingParameters));
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_ieee80211abg_modetimingparameters_getSlotSizeMicroseconds(
    ptr: *mut c_void,
) -> u64 {
    unsafe { &*(ptr as *mut ModeTimingParameters) }.get_slot_size_microseconds()
}

#[no_mangle]
pub extern "C" fn emane_ieee80211abg_modetimingparameters_getOverheadMicroseconds(
    ptr: *mut c_void,
    category: u8,
) -> u64 {
    unsafe { &*(ptr as *mut ModeTimingParameters) }.get_overhead_microseconds(category)
}

#[no_mangle]
pub extern "C" fn emane_ieee80211abg_modetimingparameters_getDeferIntervalMicroseconds(
    ptr: *mut c_void,
    category: u8,
) -> u64 {
    unsafe { &*(ptr as *mut ModeTimingParameters) }.get_defer_interval_microseconds(category)
}

#[no_mangle]
pub extern "C" fn emane_ieee80211abg_modetimingparameters_getMessageDurationMicroseconds(
    ptr: *mut c_void,
    category: u8,
    len: usize,
) -> u64 {
    unsafe { &*(ptr as *mut ModeTimingParameters) }.get_message_duration_microseconds(category, len)
}

#[no_mangle]
pub extern "C" fn emane_ieee80211abg_modetimingparameters_getBroadcastMessageDurationMicroseconds(
    ptr: *mut c_void,
    len: usize,
) -> u64 {
    unsafe { &*(ptr as *mut ModeTimingParameters) }.get_broadcast_message_duration_microseconds(len)
}

#[no_mangle]
pub extern "C" fn emane_ieee80211abg_modetimingparameters_getUnicastMessageDurationMicroseconds(
    ptr: *mut c_void,
    len: usize,
) -> u64 {
    unsafe { &*(ptr as *mut ModeTimingParameters) }.get_unicast_message_duration_microseconds(len)
}

#[no_mangle]
pub extern "C" fn emane_ieee80211abg_modetimingparameters_getCtsMessageDurationMicroseconds(
    ptr: *mut c_void,
) -> u64 {
    unsafe { &*(ptr as *mut ModeTimingParameters) }.get_cts_message_duration_microseconds()
}

#[no_mangle]
pub extern "C" fn emane_ieee80211abg_modetimingparameters_getRtsMessageDurationMicroseconds(
    ptr: *mut c_void,
) -> u64 {
    unsafe { &*(ptr as *mut ModeTimingParameters) }.get_rts_message_duration_microseconds()
}
