
#[derive(Clone, Default)]
pub struct TransponderConfigurationUpdate {
    pub transponder_index: u16,
    pub rx_freq: Option<u64>,
    pub curve: Option<u16>,
    pub tx_freq: Option<u64>,
    pub tx_rate: Option<u64>,
    pub tx_pwr: Option<f64>,
    pub tx_delay: Option<u64>,
    pub tx_jitter: Option<u64>,
    pub tx_slots_per_frame: Option<u16>,
    pub tx_slot_size: Option<u64>,
    pub tx_mtu: Option<u64>,
    pub rx_en: Option<bool>,
    pub tx_en: Option<bool>,
}


#[no_mangle]
pub extern "C" fn rust_bentpipe_tcu_new(index: u16) -> *mut TransponderConfigurationUpdate {
    let mut tcu = Box::new(TransponderConfigurationUpdate::default());
    tcu.transponder_index = index;
    Box::into_raw(tcu)
}

#[no_mangle]
pub extern "C" fn rust_bentpipe_tcu_clone(ptr: *const TransponderConfigurationUpdate) -> *mut TransponderConfigurationUpdate {
    if ptr.is_null() { return std::ptr::null_mut(); }
    let orig = unsafe { &*ptr };
    Box::into_raw(Box::new(orig.clone()))
}

#[no_mangle]
pub extern "C" fn rust_bentpipe_tcu_free(ptr: *mut TransponderConfigurationUpdate) {
    if !ptr.is_null() { unsafe { drop(Box::from_raw(ptr)); } }
}

#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_get_index(ptr: *const TransponderConfigurationUpdate) -> u16 { unsafe { (*ptr).transponder_index } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_set_rx_freq(ptr: *mut TransponderConfigurationUpdate, v: u64) { unsafe { (*ptr).rx_freq = Some(v); } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_has_rx_freq(ptr: *const TransponderConfigurationUpdate) -> bool { unsafe { (*ptr).rx_freq.is_some() } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_get_rx_freq(ptr: *const TransponderConfigurationUpdate) -> u64 { unsafe { (*ptr).rx_freq.unwrap_or_default() } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_set_curve(ptr: *mut TransponderConfigurationUpdate, v: u16) { unsafe { (*ptr).curve = Some(v); } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_has_curve(ptr: *const TransponderConfigurationUpdate) -> bool { unsafe { (*ptr).curve.is_some() } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_get_curve(ptr: *const TransponderConfigurationUpdate) -> u16 { unsafe { (*ptr).curve.unwrap_or_default() } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_set_tx_freq(ptr: *mut TransponderConfigurationUpdate, v: u64) { unsafe { (*ptr).tx_freq = Some(v); } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_has_tx_freq(ptr: *const TransponderConfigurationUpdate) -> bool { unsafe { (*ptr).tx_freq.is_some() } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_get_tx_freq(ptr: *const TransponderConfigurationUpdate) -> u64 { unsafe { (*ptr).tx_freq.unwrap_or_default() } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_set_tx_rate(ptr: *mut TransponderConfigurationUpdate, v: u64) { unsafe { (*ptr).tx_rate = Some(v); } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_has_tx_rate(ptr: *const TransponderConfigurationUpdate) -> bool { unsafe { (*ptr).tx_rate.is_some() } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_get_tx_rate(ptr: *const TransponderConfigurationUpdate) -> u64 { unsafe { (*ptr).tx_rate.unwrap_or_default() } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_set_tx_pwr(ptr: *mut TransponderConfigurationUpdate, v: f64) { unsafe { (*ptr).tx_pwr = Some(v); } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_has_tx_pwr(ptr: *const TransponderConfigurationUpdate) -> bool { unsafe { (*ptr).tx_pwr.is_some() } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_get_tx_pwr(ptr: *const TransponderConfigurationUpdate) -> f64 { unsafe { (*ptr).tx_pwr.unwrap_or_default() } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_set_tx_delay(ptr: *mut TransponderConfigurationUpdate, v: u64) { unsafe { (*ptr).tx_delay = Some(v); } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_has_tx_delay(ptr: *const TransponderConfigurationUpdate) -> bool { unsafe { (*ptr).tx_delay.is_some() } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_get_tx_delay(ptr: *const TransponderConfigurationUpdate) -> u64 { unsafe { (*ptr).tx_delay.unwrap_or_default() } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_set_tx_jitter(ptr: *mut TransponderConfigurationUpdate, v: u64) { unsafe { (*ptr).tx_jitter = Some(v); } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_has_tx_jitter(ptr: *const TransponderConfigurationUpdate) -> bool { unsafe { (*ptr).tx_jitter.is_some() } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_get_tx_jitter(ptr: *const TransponderConfigurationUpdate) -> u64 { unsafe { (*ptr).tx_jitter.unwrap_or_default() } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_set_tx_slots_per_frame(ptr: *mut TransponderConfigurationUpdate, v: u16) { unsafe { (*ptr).tx_slots_per_frame = Some(v); } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_has_tx_slots_per_frame(ptr: *const TransponderConfigurationUpdate) -> bool { unsafe { (*ptr).tx_slots_per_frame.is_some() } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_get_tx_slots_per_frame(ptr: *const TransponderConfigurationUpdate) -> u16 { unsafe { (*ptr).tx_slots_per_frame.unwrap_or_default() } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_set_tx_slot_size(ptr: *mut TransponderConfigurationUpdate, v: u64) { unsafe { (*ptr).tx_slot_size = Some(v); } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_has_tx_slot_size(ptr: *const TransponderConfigurationUpdate) -> bool { unsafe { (*ptr).tx_slot_size.is_some() } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_get_tx_slot_size(ptr: *const TransponderConfigurationUpdate) -> u64 { unsafe { (*ptr).tx_slot_size.unwrap_or_default() } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_set_tx_mtu(ptr: *mut TransponderConfigurationUpdate, v: u64) { unsafe { (*ptr).tx_mtu = Some(v); } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_has_tx_mtu(ptr: *const TransponderConfigurationUpdate) -> bool { unsafe { (*ptr).tx_mtu.is_some() } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_get_tx_mtu(ptr: *const TransponderConfigurationUpdate) -> u64 { unsafe { (*ptr).tx_mtu.unwrap_or_default() } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_set_rx_en(ptr: *mut TransponderConfigurationUpdate, v: bool) { unsafe { (*ptr).rx_en = Some(v); } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_has_rx_en(ptr: *const TransponderConfigurationUpdate) -> bool { unsafe { (*ptr).rx_en.is_some() } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_get_rx_en(ptr: *const TransponderConfigurationUpdate) -> bool { unsafe { (*ptr).rx_en.unwrap_or_default() } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_set_tx_en(ptr: *mut TransponderConfigurationUpdate, v: bool) { unsafe { (*ptr).tx_en = Some(v); } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_has_tx_en(ptr: *const TransponderConfigurationUpdate) -> bool { unsafe { (*ptr).tx_en.is_some() } }
#[no_mangle] pub extern "C" fn rust_bentpipe_tcu_get_tx_en(ptr: *const TransponderConfigurationUpdate) -> bool { unsafe { (*ptr).tx_en.unwrap_or_default() } }
