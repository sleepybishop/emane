use std::os::raw::c_void;
use std::collections::HashSet;

pub struct FrameworkPhy {
    pub cpp_this: *mut c_void,
    pub p_spectrum_service: *mut c_void,
    pub antenna_manager: *mut c_void,
    pub location_manager: *mut c_void,
    pub u64_bandwidth_hz: u64,
    pub d_tx_power_dbm: f64,
    pub u64_tx_frequency_hz: u64,
    pub d_receiver_sensitivity_dbm: f64,
    pub noise_mode: u32,
    pub u16_sub_id: u16,
    pub u16_tx_sequence_number: u16,
    pub optional_fixed_antenna_gain_dbi: (f64, bool),
    pub p_propagation_model_algorithm: *mut c_void,
    pub common_layer_statistics: *mut c_void,
    pub event_table_publisher: *mut c_void,
    pub receive_power_table_publisher: *mut c_void,
    pub observed_power_table_publisher: *mut c_void,
    pub noise_bin_size: u64,
    pub max_segment_offset: u64,
    pub max_message_propagation: u64,
    pub max_segment_duration: u64,
    pub time_sync_threshold: u64,
    pub b_noise_max_clamp: bool,
    pub d_system_noise_figure_db: f64,
    pub p_num_downstream_packets_radio_silence_enabled_drop: *mut c_void,
    pub p_time_sync_threshold_rewrite: *mut c_void,
    pub p_gain_cache_hit: *mut c_void,
    pub p_gain_cache_miss: *mut c_void,
    pub fading_manager: *mut c_void,
    pub b_exclude_same_sub_id_from_filter: bool,
    pub foi: HashSet<u64>,
    pub compatibility_mode: u32,
    pub receive_processors: *mut c_void,
    pub processing_pool: *mut c_void,
    pub u16_processing_pool_size: u16,
    pub b_stats_receive_power_table_enable: bool,
    pub b_stats_observed_power_table_enable: bool,
    pub b_rx_sensitivity_promiscuous_mode_enable: bool,
    pub b_doppler_shift_enable: bool,
    pub spectral_mask_index: u16,
    pub b_radio_silence_enable: bool,
}

impl FrameworkPhy {
    pub fn new() -> Self {
        Self {
            cpp_this: std::ptr::null_mut(),
            p_spectrum_service: std::ptr::null_mut(),
            antenna_manager: std::ptr::null_mut(),
            location_manager: std::ptr::null_mut(),
            u64_bandwidth_hz: 0,
            d_tx_power_dbm: 0.0,
            u64_tx_frequency_hz: 0,
            d_receiver_sensitivity_dbm: 0.0,
            noise_mode: 0,
            u16_sub_id: 0,
            u16_tx_sequence_number: 0,
            optional_fixed_antenna_gain_dbi: (0.0, false),
            p_propagation_model_algorithm: std::ptr::null_mut(),
            common_layer_statistics: std::ptr::null_mut(),
            event_table_publisher: std::ptr::null_mut(),
            receive_power_table_publisher: std::ptr::null_mut(),
            observed_power_table_publisher: std::ptr::null_mut(),
            noise_bin_size: 0,
            max_segment_offset: 0,
            max_message_propagation: 0,
            max_segment_duration: 0,
            time_sync_threshold: 0,
            b_noise_max_clamp: false,
            d_system_noise_figure_db: 0.0,
            p_num_downstream_packets_radio_silence_enabled_drop: std::ptr::null_mut(),
            p_time_sync_threshold_rewrite: std::ptr::null_mut(),
            p_gain_cache_hit: std::ptr::null_mut(),
            p_gain_cache_miss: std::ptr::null_mut(),
            fading_manager: std::ptr::null_mut(),
            b_exclude_same_sub_id_from_filter: false,
            foi: HashSet::new(),
            compatibility_mode: 0,
            receive_processors: std::ptr::null_mut(),
            processing_pool: std::ptr::null_mut(),
            u16_processing_pool_size: 0,
            b_stats_receive_power_table_enable: false,
            b_stats_observed_power_table_enable: false,
            b_rx_sensitivity_promiscuous_mode_enable: false,
            b_doppler_shift_enable: false,
            spectral_mask_index: 0,
            b_radio_silence_enable: false,
        }
    }

    pub fn initialize(&mut self, registrar: *mut c_void) {
        // Stubbed FFI call to C++ configRegistrar.registerNumeric/registerNonNumeric
        unsafe {
            emane_c_framework_phy_initialize_stub(registrar);
        }
    }

    pub fn configure(&mut self, update: *mut c_void) {
        // Stubbed FFI call to C++ update loop handling
        unsafe {
            emane_c_framework_phy_configure_stub(update);
        }
    }

    pub fn process_upstream_packet(
        &mut self,
        common_phy_header: *mut c_void,
        pkt: *mut c_void,
        _control_messages: *mut c_void,
    ) {
        unsafe {
            // Emulate processUpstreamPacket_i
            emane_c_framework_phy_common_layer_statistics_process_inbound(self.cpp_this, pkt);
            
            let is_in_band = emane_c_framework_phy_check_in_band(self.cpp_this, common_phy_header);
            
            if self.compatibility_mode == 1 {
                emane_c_framework_phy_create_default_antenna_if_needed(self.cpp_this);
            } else {
                let rx_processors_empty = emane_c_framework_phy_receive_processors_is_empty(self.cpp_this);
                if rx_processors_empty {
                    emane_c_framework_phy_common_layer_statistics_process_outbound_drop(self.cpp_this, pkt, 3); // DROP_CODE_MISSING_CONTROL
                    return;
                }
            }

            if is_in_band || self.noise_mode != 0 /* NONE */ {
                // FFI stub for the large receiveProcessor loop and result processing
                let dropped = emane_c_framework_phy_process_receive_processors(
                    self.cpp_this,
                    common_phy_header,
                    pkt,
                    is_in_band,
                );
                
                if !dropped {
                    // It will handle sendUpstreamPacket internally in the stub
                    // or we can stub the end
                }
            } else {
                emane_c_framework_phy_common_layer_statistics_process_outbound_drop(self.cpp_this, pkt, 4); // DROP_CODE_OUT_OF_BAND
                return;
            }
        }
    }
}

extern "C" {
    fn emane_c_framework_phy_initialize_stub(registrar: *mut c_void);
    fn emane_c_framework_phy_configure_stub(update: *mut c_void);

    fn emane_c_framework_phy_common_layer_statistics_process_inbound(phy: *mut c_void, pkt: *mut c_void);
    fn emane_c_framework_phy_common_layer_statistics_process_outbound_drop(phy: *mut c_void, pkt: *mut c_void, drop_code: u32);
    fn emane_c_framework_phy_check_in_band(phy: *mut c_void, header: *mut c_void) -> bool;
    fn emane_c_framework_phy_create_default_antenna_if_needed(phy: *mut c_void);
    fn emane_c_framework_phy_receive_processors_is_empty(phy: *mut c_void) -> bool;
    fn emane_c_framework_phy_process_receive_processors(phy: *mut c_void, header: *mut c_void, pkt: *mut c_void, in_band: bool) -> bool;
}

#[no_mangle]
pub extern "C" fn emane_rs_framework_phy_create(cpp_this: *mut c_void) -> *mut c_void {
    Box::into_raw(Box::new({ let mut phy = FrameworkPhy::new(); phy.cpp_this = cpp_this; phy })) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_framework_phy_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr as *mut FrameworkPhy));
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_framework_phy_initialize(ptr: *mut c_void, registrar: *mut c_void) {
    let phy = unsafe { &mut *(ptr as *mut FrameworkPhy) };
    phy.initialize(registrar);
}

#[no_mangle]
pub extern "C" fn emane_rs_framework_phy_configure(ptr: *mut c_void, update: *mut c_void) {
    let phy = unsafe { &mut *(ptr as *mut FrameworkPhy) };
    phy.configure(update);
}

#[no_mangle]
pub extern "C" fn emane_rs_framework_phy_process_upstream_packet(
    ptr: *mut c_void,
    common_phy_header: *mut c_void,
    pkt: *mut c_void,
    control_messages: *mut c_void,
) {
    let phy = unsafe { &mut *(ptr as *mut FrameworkPhy) };
    phy.process_upstream_packet(common_phy_header, pkt, control_messages);
}
