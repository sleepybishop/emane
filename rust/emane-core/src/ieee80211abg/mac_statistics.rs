extern "C" {
    fn emane_ieee80211abg_macstatistics_incrementDownstreamUnicastDataDiscardDueToRetries(ptr: *mut std::ffi::c_void);
    fn emane_ieee80211abg_macstatistics_incrementDownstreamUnicastRtsCtsDataDiscardDueToRetries(ptr: *mut std::ffi::c_void);
    fn emane_ieee80211abg_macstatistics_incrementDownstreamUnicastDataDiscardDueToTxop(ptr: *mut std::ffi::c_void);
    fn emane_ieee80211abg_macstatistics_incrementDownstreamBroadcastDataDiscardDueToTxop(ptr: *mut std::ffi::c_void);
    fn emane_ieee80211abg_macstatistics_incrementUpstreamUnicastDataDiscardDueToSinr(ptr: *mut std::ffi::c_void);
    fn emane_ieee80211abg_macstatistics_incrementUpstreamBroadcastDataDiscardDueToSinr(ptr: *mut std::ffi::c_void);
    fn emane_ieee80211abg_macstatistics_incrementUpstreamUnicastDataDiscardDueToClobberRxDuringTx(ptr: *mut std::ffi::c_void);
    fn emane_ieee80211abg_macstatistics_incrementUpstreamBroadcastDataDiscardDueToClobberRxDuringTx(ptr: *mut std::ffi::c_void);
    fn emane_ieee80211abg_macstatistics_incrementUpstreamUnicastDataDiscardDueToClobberRxHiddenBusy(ptr: *mut std::ffi::c_void);
    fn emane_ieee80211abg_macstatistics_incrementUpstreamBroadcastDataDiscardDueToClobberRxHiddenBusy(ptr: *mut std::ffi::c_void);
    fn emane_ieee80211abg_macstatistics_incrementUpstreamBroadcastNoiseHiddenRx(ptr: *mut std::ffi::c_void);
    fn emane_ieee80211abg_macstatistics_incrementUpstreamUnicastNoiseHiddenRx(ptr: *mut std::ffi::c_void);
    fn emane_ieee80211abg_macstatistics_incrementUpstreamBroadcastNoiseRxCommon(ptr: *mut std::ffi::c_void);
    fn emane_ieee80211abg_macstatistics_incrementUpstreamUnicastNoiseRxCommon(ptr: *mut std::ffi::c_void);
    fn emane_ieee80211abg_macstatistics_incrementUpstreamUnicastRtsCtsDataRxFromPhy(ptr: *mut std::ffi::c_void);
    fn emane_ieee80211abg_macstatistics_incrementUpstreamUnicastCtsRxFromPhy(ptr: *mut std::ffi::c_void);
    fn emane_ieee80211abg_macstatistics_incrementRxOneHopNbrListEventCount(ptr: *mut std::ffi::c_void);
    fn emane_ieee80211abg_macstatistics_incrementRxOneHopNbrListInvalidEventCount(ptr: *mut std::ffi::c_void);
    fn emane_ieee80211abg_macstatistics_incrementTxOneHopNbrListEventCount(ptr: *mut std::ffi::c_void);
    fn emane_ieee80211abg_macstatistics_updateOneHopNbrHighWaterMark(ptr: *mut std::ffi::c_void, num: usize);
    fn emane_ieee80211abg_macstatistics_updateTwoHopNbrHighWaterMark(ptr: *mut std::ffi::c_void, num: usize);
}

pub struct MACStatistics {
    ptr: *mut std::ffi::c_void,
}

impl MACStatistics {
    pub fn new(ptr: *mut std::ffi::c_void) -> Self {
        Self { ptr }
    }

    pub fn incrementDownstreamUnicastDataDiscardDueToRetries(&self) {
        unsafe { emane_ieee80211abg_macstatistics_incrementDownstreamUnicastDataDiscardDueToRetries(self.ptr) }
    }
    pub fn incrementDownstreamUnicastRtsCtsDataDiscardDueToRetries(&self) {
        unsafe { emane_ieee80211abg_macstatistics_incrementDownstreamUnicastRtsCtsDataDiscardDueToRetries(self.ptr) }
    }
    pub fn incrementDownstreamUnicastDataDiscardDueToTxop(&self) {
        unsafe { emane_ieee80211abg_macstatistics_incrementDownstreamUnicastDataDiscardDueToTxop(self.ptr) }
    }
    pub fn incrementDownstreamBroadcastDataDiscardDueToTxop(&self) {
        unsafe { emane_ieee80211abg_macstatistics_incrementDownstreamBroadcastDataDiscardDueToTxop(self.ptr) }
    }
    pub fn incrementUpstreamUnicastDataDiscardDueToSinr(&self) {
        unsafe { emane_ieee80211abg_macstatistics_incrementUpstreamUnicastDataDiscardDueToSinr(self.ptr) }
    }
    pub fn incrementUpstreamBroadcastDataDiscardDueToSinr(&self) {
        unsafe { emane_ieee80211abg_macstatistics_incrementUpstreamBroadcastDataDiscardDueToSinr(self.ptr) }
    }
    pub fn incrementUpstreamUnicastDataDiscardDueToClobberRxDuringTx(&self) {
        unsafe { emane_ieee80211abg_macstatistics_incrementUpstreamUnicastDataDiscardDueToClobberRxDuringTx(self.ptr) }
    }
    pub fn incrementUpstreamBroadcastDataDiscardDueToClobberRxDuringTx(&self) {
        unsafe { emane_ieee80211abg_macstatistics_incrementUpstreamBroadcastDataDiscardDueToClobberRxDuringTx(self.ptr) }
    }
    pub fn incrementUpstreamUnicastDataDiscardDueToClobberRxHiddenBusy(&self) {
        unsafe { emane_ieee80211abg_macstatistics_incrementUpstreamUnicastDataDiscardDueToClobberRxHiddenBusy(self.ptr) }
    }
    pub fn incrementUpstreamBroadcastDataDiscardDueToClobberRxHiddenBusy(&self) {
        unsafe { emane_ieee80211abg_macstatistics_incrementUpstreamBroadcastDataDiscardDueToClobberRxHiddenBusy(self.ptr) }
    }
    pub fn incrementUpstreamBroadcastNoiseHiddenRx(&self) {
        unsafe { emane_ieee80211abg_macstatistics_incrementUpstreamBroadcastNoiseHiddenRx(self.ptr) }
    }
    pub fn incrementUpstreamUnicastNoiseHiddenRx(&self) {
        unsafe { emane_ieee80211abg_macstatistics_incrementUpstreamUnicastNoiseHiddenRx(self.ptr) }
    }
    pub fn incrementUpstreamBroadcastNoiseRxCommon(&self) {
        unsafe { emane_ieee80211abg_macstatistics_incrementUpstreamBroadcastNoiseRxCommon(self.ptr) }
    }
    pub fn incrementUpstreamUnicastNoiseRxCommon(&self) {
        unsafe { emane_ieee80211abg_macstatistics_incrementUpstreamUnicastNoiseRxCommon(self.ptr) }
    }
    pub fn incrementUpstreamUnicastRtsCtsDataRxFromPhy(&self) {
        unsafe { emane_ieee80211abg_macstatistics_incrementUpstreamUnicastRtsCtsDataRxFromPhy(self.ptr) }
    }
    pub fn incrementUpstreamUnicastCtsRxFromPhy(&self) {
        unsafe { emane_ieee80211abg_macstatistics_incrementUpstreamUnicastCtsRxFromPhy(self.ptr) }
    }
    pub fn incrementRxOneHopNbrListEventCount(&self) {
        unsafe { emane_ieee80211abg_macstatistics_incrementRxOneHopNbrListEventCount(self.ptr) }
    }
    pub fn incrementRxOneHopNbrListInvalidEventCount(&self) {
        unsafe { emane_ieee80211abg_macstatistics_incrementRxOneHopNbrListInvalidEventCount(self.ptr) }
    }
    pub fn incrementTxOneHopNbrListEventCount(&self) {
        unsafe { emane_ieee80211abg_macstatistics_incrementTxOneHopNbrListEventCount(self.ptr) }
    }
    pub fn updateOneHopNbrHighWaterMark(&self, num: usize) {
        unsafe { emane_ieee80211abg_macstatistics_updateOneHopNbrHighWaterMark(self.ptr, num) }
    }
    pub fn updateTwoHopNbrHighWaterMark(&self, num: usize) {
        unsafe { emane_ieee80211abg_macstatistics_updateTwoHopNbrHighWaterMark(self.ptr, num) }
    }
}
unsafe impl Send for MACStatistics {}
unsafe impl Sync for MACStatistics {}
