use crate::framework_phy::FrameworkPhy;
use std::os::raw::c_void;

extern "C" {
    fn emane_c_framework_phy_downstream_stub_process_inbound(stats: *mut c_void, pkt: *mut c_void);
    fn emane_c_framework_phy_downstream_stub_process_outbound(stats: *mut c_void, pkt: *mut c_void, duration: u64);
    fn emane_c_framework_phy_downstream_stub_send_downstream_packet(
        phy: *mut c_void,
        header: *mut c_void,
        pkt: *mut c_void,
        controls: *mut c_void,
    );
    fn emane_c_framework_phy_downstream_stub_process_self_interference(
        phy: *mut c_void,
        antenna_index: u16,
        now: u64,
        tx_time_stamp: u64,
        freq_groups: *mut c_void,
        bandwidth: u64,
        rx_power: f64,
        filter_data: *mut c_void,
    );
    fn emane_c_framework_phy_downstream_stub_get_control_messages_len(msgs: *mut c_void) -> usize;
    fn emane_c_framework_phy_downstream_stub_get_control_message(msgs: *mut c_void, index: usize) -> *mut c_void;
    fn emane_c_framework_phy_downstream_stub_get_control_message_id(msg: *mut c_void) -> u16;
}

pub fn process_downstream_packet(phy: &mut FrameworkPhy, pkt: *mut c_void, msgs: *mut c_void) {
    if phy.b_radio_silence_enable {
        // ++*pNumDownstreamPacketsRadioSilenceEnabledDrop_;
        return;
    }

    unsafe {
        emane_c_framework_phy_downstream_stub_process_inbound(phy.common_layer_statistics, pkt);
    }

    let _u64_bandwidth_hz = phy.u64_bandwidth_hz;
    let _transmitters: *mut c_void = std::ptr::null_mut(); // stub
    let _frequency_groups: *mut c_void = std::ptr::null_mut(); // stub
    let _transmit_antennas: *mut c_void = std::ptr::null_mut(); // stub
    let _ota_transmitters: *mut c_void = std::ptr::null_mut(); // stub
    let _tx_time_stamp = 0; // stub now
    let _d_tx_while_rx_interference_rx_power_milli_watt = 0.0;
    let _optional_spectrum_filter_data: *mut c_void = std::ptr::null_mut(); // stub

    let msg_len = unsafe { emane_c_framework_phy_downstream_stub_get_control_messages_len(msgs) };
    for i in 0..msg_len {
        let msg = unsafe { emane_c_framework_phy_downstream_stub_get_control_message(msgs, i) };
        let msg_id = unsafe { emane_c_framework_phy_downstream_stub_get_control_message_id(msg) };
        match msg_id {
            // TransmitterControlMessage
            104 => {
                // stub
            }
            // FrequencyControlMessage
            100 => {
                if phy.compatibility_mode == 1 {
                    // stub
                }
            }
            // AntennaProfileControlMessage
            102 => {
                if phy.compatibility_mode == 1 {
                    // stub
                }
            }
            // TimeStampControlMessage
            106 => {
                // stub
            }
            // TxWhileRxInterferenceControlMessage
            108 => {
                if phy.compatibility_mode == 1 {
                    // stub
                }
            }
            // MIMOTransmitPropertiesControlMessage
            116 => {
                if phy.compatibility_mode == 2 {
                    // stub
                }
            }
            // RxAntennaUpdateControlMessage
            114 => {
                if phy.compatibility_mode == 2 {
                    // stub
                }
            }
            // SpectrumFilterDataControlMessage
            110 => {
                // stub
            }
            // MIMOTxWhileRxInterferenceControlMessage
            118 => {
                if phy.compatibility_mode == 2 {
                    // stub
                }
            }
            _ => {
                // ignore
            }
        }
    }

    // transmitAntennas empty? -> use default antenna based on fixed gain

    // verify transmitters list include this nem

    // CommonPHYHeader phyHeader...
    let phy_header: *mut c_void = std::ptr::null_mut();

    let downstream_control_messages: *mut c_void = std::ptr::null_mut();
    // if !ota_transmitters.is_empty() -> create OTATransmitterControlMessage
    
    unsafe {
        emane_c_framework_phy_downstream_stub_process_outbound(phy.common_layer_statistics, pkt, 0);
        emane_c_framework_phy_downstream_stub_send_downstream_packet(
            phy as *mut _ as *mut c_void,
            phy_header,
            pkt,
            downstream_control_messages,
        );
    }

    if phy.compatibility_mode == 1 && _d_tx_while_rx_interference_rx_power_milli_watt > 0.0 {
        unsafe {
            emane_c_framework_phy_downstream_stub_process_self_interference(
                phy as *mut _ as *mut c_void,
                0, // DEFAULT_ANTENNA_INDEX
                0, // now
                _tx_time_stamp,
                _frequency_groups,
                _u64_bandwidth_hz,
                _d_tx_while_rx_interference_rx_power_milli_watt,
                _optional_spectrum_filter_data,
            );
        }
    } else if phy.compatibility_mode == 2 {
        // handle MIMO process_self_interference
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_framework_phy_process_downstream_packet(
    ptr: *mut c_void,
    pkt: *mut c_void,
    msgs: *mut c_void,
) {
    let phy = unsafe { &mut *(ptr as *mut FrameworkPhy) };
    process_downstream_packet(phy, pkt, msgs);
}
