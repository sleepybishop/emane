use std::ffi::c_void;
use std::os::raw::c_char;
use std::time::{SystemTime, UNIX_EPOCH};

extern "C" {
    fn emane_ieee80211abg_maclayer_setDelayTime(maclayer: *mut c_void, entry: *mut c_void);

    fn emane_ieee80211abg_maclayer_getStatistics(maclayer: *mut c_void) -> *mut c_void;
    fn emane_ieee80211abg_maclayer_getModeTiming(maclayer: *mut c_void) -> *mut c_void;

    fn emane_ieee80211abg_entry_get_destination(entry: *mut c_void) -> u16;
    fn emane_ieee80211abg_entry_is_txop_timeout(entry: *mut c_void, beginTimeMicro: u64) -> bool;
    fn emane_ieee80211abg_entry_get_bRtsCtsEnable(entry: *mut c_void) -> bool;
    fn emane_ieee80211abg_entry_get_bCollisionOccured(entry: *mut c_void) -> bool;
    fn emane_ieee80211abg_entry_get_numRetries(entry: *mut c_void) -> u8;
    fn emane_ieee80211abg_entry_set_numRetries(entry: *mut c_void, numRetries: u8);
    fn emane_ieee80211abg_entry_get_maxRetries(entry: *mut c_void) -> u8;
    fn emane_ieee80211abg_entry_get_length(entry: *mut c_void) -> usize;
    fn emane_ieee80211abg_entry_set_duration_and_tx_time_now(
        entry: *mut c_void,
        durationMicroseconds: u64,
    );
    fn emane_ieee80211abg_entry_get_preTxDelayTime_micro(entry: *mut c_void) -> u64;
    fn emane_ieee80211abg_entry_get_postTxWaitTime_micro(entry: *mut c_void) -> u64;

    fn emane_ieee80211abg_modetimingparameters_getMessageDurationMicroseconds(
        ptr: *mut c_void,
        category: u8,
        len: usize,
    ) -> u64;
}

const NEM_BROADCAST_MAC_ADDRESS: u16 = 0xFFFF;

const MSG_TYPE_BROADCAST_DATA: u8 = 1;
const MSG_TYPE_UNICAST_DATA: u8 = 2;
const MSG_TYPE_UNICAST_RTS_CTS_DATA: u8 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxState {
    Idle,
    BroadcastPreTx,
    BroadcastTx,
    BroadcastPostTx,
    UnicastPreTx,
    UnicastTx,
    UnicastPostTx,
    UnicastRtsCtsPreTx,
    UnicastRtsCtsTx,
    UnicastRtsCtsPostTx,
}

pub struct TxStateMachine {
    state: TxState,
}

impl TxStateMachine {
    pub fn new() -> Self {
        Self {
            state: TxState::Idle,
        }
    }

    pub fn process(&mut self, maclayer: *mut c_void, entry: *mut c_void) -> bool {
        match self.state {
            TxState::Idle => {
                let begin_time_micro = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_micros() as u64;
                let dest = unsafe { emane_ieee80211abg_entry_get_destination(entry) };

                if dest == NEM_BROADCAST_MAC_ADDRESS {
                    if unsafe { emane_ieee80211abg_entry_is_txop_timeout(entry, begin_time_micro) }
                    {
                        let stats = unsafe { emane_ieee80211abg_maclayer_getStatistics(maclayer) };
                        crate::mac_statistics::MACStatistics::new(stats)
                            .incrementDownstreamBroadcastDataDiscardDueToTxop();
                        return false;
                    } else {
                        unsafe { emane_ieee80211abg_maclayer_setDelayTime(maclayer, entry) };
                        self.state = TxState::BroadcastPreTx;
                        return true;
                    }
                } else {
                    if unsafe { emane_ieee80211abg_entry_is_txop_timeout(entry, begin_time_micro) }
                    {
                        let stats = unsafe { emane_ieee80211abg_maclayer_getStatistics(maclayer) };
                        crate::mac_statistics::MACStatistics::new(stats)
                            .incrementDownstreamUnicastDataDiscardDueToTxop();
                        return false;
                    } else {
                        unsafe { emane_ieee80211abg_maclayer_setDelayTime(maclayer, entry) };
                        if unsafe { emane_ieee80211abg_entry_get_bRtsCtsEnable(entry) } {
                            self.state = TxState::UnicastRtsCtsPreTx;
                        } else {
                            self.state = TxState::UnicastPreTx;
                        }
                        return true;
                    }
                }
            }
            TxState::BroadcastPreTx => {
                self.state = TxState::BroadcastTx;
                true
            }
            TxState::BroadcastTx => {
                let timing = unsafe { emane_ieee80211abg_maclayer_getModeTiming(maclayer) };
                let length = unsafe { emane_ieee80211abg_entry_get_length(entry) };
                let duration = unsafe {
                    emane_ieee80211abg_modetimingparameters_getMessageDurationMicroseconds(
                        timing,
                        MSG_TYPE_BROADCAST_DATA,
                        length,
                    )
                };
                unsafe { emane_ieee80211abg_entry_set_duration_and_tx_time_now(entry, duration) };
                unsafe {
                    if let Some(fw) = &crate::FW_SERVICE {
                        (fw.send_downstream_packet)(fw.framework_ctx, 0, std::ptr::null(), std::ptr::null(), 0);
                    }
                };
                self.state = TxState::BroadcastPostTx;
                true
            }
            TxState::BroadcastPostTx => {
                self.state = TxState::Idle;
                false
            }
            TxState::UnicastPreTx => {
                self.state = TxState::UnicastTx;
                true
            }
            TxState::UnicastTx => {
                let timing = unsafe { emane_ieee80211abg_maclayer_getModeTiming(maclayer) };
                let length = unsafe { emane_ieee80211abg_entry_get_length(entry) };
                let duration = unsafe {
                    emane_ieee80211abg_modetimingparameters_getMessageDurationMicroseconds(
                        timing,
                        MSG_TYPE_UNICAST_DATA,
                        length,
                    )
                };
                unsafe { emane_ieee80211abg_entry_set_duration_and_tx_time_now(entry, duration) };
                unsafe {
                    if let Some(fw) = &crate::FW_SERVICE {
                        (fw.send_downstream_packet)(fw.framework_ctx, 0, std::ptr::null(), std::ptr::null(), 0);
                    }
                };
                self.state = TxState::UnicastPostTx;
                true
            }
            TxState::UnicastPostTx => {
                if unsafe { emane_ieee80211abg_entry_get_bCollisionOccured(entry) } {
                    let num_retries = unsafe { emane_ieee80211abg_entry_get_numRetries(entry) };
                    let max_retries = unsafe { emane_ieee80211abg_entry_get_maxRetries(entry) };
                    if num_retries >= max_retries {
                        self.state = TxState::Idle;
                        let stats = unsafe { emane_ieee80211abg_maclayer_getStatistics(maclayer) };
                        crate::mac_statistics::MACStatistics::new(stats)
                            .incrementDownstreamUnicastDataDiscardDueToRetries();
                        false
                    } else {
                        unsafe { emane_ieee80211abg_entry_set_numRetries(entry, num_retries + 1) };
                        unsafe { emane_ieee80211abg_maclayer_setDelayTime(maclayer, entry) };
                        self.state = TxState::UnicastPreTx;
                        true
                    }
                } else {
                    self.state = TxState::Idle;
                    false
                }
            }
            TxState::UnicastRtsCtsPreTx => {
                self.state = TxState::UnicastRtsCtsTx;
                true
            }
            TxState::UnicastRtsCtsTx => {
                let timing = unsafe { emane_ieee80211abg_maclayer_getModeTiming(maclayer) };
                let length = unsafe { emane_ieee80211abg_entry_get_length(entry) };
                let duration = unsafe {
                    emane_ieee80211abg_modetimingparameters_getMessageDurationMicroseconds(
                        timing,
                        MSG_TYPE_UNICAST_RTS_CTS_DATA,
                        length,
                    )
                };
                unsafe { emane_ieee80211abg_entry_set_duration_and_tx_time_now(entry, duration) };
                unsafe {
                    if let Some(fw) = &crate::FW_SERVICE {
                        (fw.send_downstream_packet)(fw.framework_ctx, 0, std::ptr::null(), std::ptr::null(), 0);
                    }
                };
                self.state = TxState::UnicastRtsCtsPostTx;
                true
            }
            TxState::UnicastRtsCtsPostTx => {
                if unsafe { emane_ieee80211abg_entry_get_bCollisionOccured(entry) } {
                    let num_retries = unsafe { emane_ieee80211abg_entry_get_numRetries(entry) };
                    let max_retries = unsafe { emane_ieee80211abg_entry_get_maxRetries(entry) };
                    if num_retries >= max_retries {
                        self.state = TxState::Idle;
                        let stats = unsafe { emane_ieee80211abg_maclayer_getStatistics(maclayer) };
                        crate::mac_statistics::MACStatistics::new(stats)
                            .incrementDownstreamUnicastRtsCtsDataDiscardDueToRetries();
                        false
                    } else {
                        unsafe { emane_ieee80211abg_entry_set_numRetries(entry, num_retries + 1) };
                        unsafe { emane_ieee80211abg_maclayer_setDelayTime(maclayer, entry) };
                        self.state = TxState::UnicastRtsCtsPreTx;
                        true
                    }
                } else {
                    self.state = TxState::Idle;
                    false
                }
            }
        }
    }

    pub fn get_wait_time(&self, entry: *mut c_void, out_time: *mut u64) -> bool {
        match self.state {
            TxState::Idle => false,
            TxState::BroadcastPreTx | TxState::UnicastPreTx | TxState::UnicastRtsCtsPreTx => {
                unsafe {
                    *out_time = emane_ieee80211abg_entry_get_preTxDelayTime_micro(entry);
                }
                true
            }
            TxState::BroadcastTx | TxState::UnicastTx | TxState::UnicastRtsCtsTx => false,
            TxState::BroadcastPostTx | TxState::UnicastPostTx | TxState::UnicastRtsCtsPostTx => {
                unsafe {
                    *out_time = emane_ieee80211abg_entry_get_postTxWaitTime_micro(entry);
                }
                true
            }
        }
    }

    pub fn statename(&self) -> *const c_char {
        match self.state {
            TxState::Idle => c"IdleTxState".as_ptr(),
            TxState::BroadcastPreTx => c"BroadcastPreTxState".as_ptr(),
            TxState::BroadcastTx => c"BroadcastTxState".as_ptr(),
            TxState::BroadcastPostTx => c"BroadcastPostTxState".as_ptr(),
            TxState::UnicastPreTx => c"UnicastPreTxState".as_ptr(),
            TxState::UnicastTx => c"UnicastTxState".as_ptr(),
            TxState::UnicastPostTx => c"UnicastPostTxState".as_ptr(),
            TxState::UnicastRtsCtsPreTx => c"UnicastRtsCtsPreTxState".as_ptr(),
            TxState::UnicastRtsCtsTx => c"UnicastRtsCtsTxState".as_ptr(),
            TxState::UnicastRtsCtsPostTx => c"UnicastRtsCtsPostTxState".as_ptr(),
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_ieee80211abg_tx_state_machine_new() -> *mut c_void {
    Box::into_raw(Box::new(TxStateMachine::new())) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_ieee80211abg_tx_state_machine_free(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr as *mut TxStateMachine));
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_ieee80211abg_tx_state_machine_process(
    ptr: *mut c_void,
    maclayer: *mut c_void,
    entry: *mut c_void,
) -> bool {
    let sm = unsafe { &mut *(ptr as *mut TxStateMachine) };
    sm.process(maclayer, entry)
}

#[no_mangle]
pub extern "C" fn emane_ieee80211abg_tx_state_machine_getWaitTime(
    ptr: *mut c_void,
    entry: *mut c_void,
    out_time: *mut u64,
) -> bool {
    let sm = unsafe { &*(ptr as *mut TxStateMachine) };
    sm.get_wait_time(entry, out_time)
}

#[no_mangle]
pub extern "C" fn emane_ieee80211abg_tx_state_machine_statename(ptr: *mut c_void) -> *const c_char {
    let sm = unsafe { &*(ptr as *mut TxStateMachine) };
    sm.statename()
}

#[no_mangle]
pub extern "C" fn emane_ieee80211abg_tx_state_machine_update(
    _ptr: *mut c_void,
    _maclayer: *mut c_void,
    _entry: *mut c_void,
) {
    // update is a no-op in C++ class
}
