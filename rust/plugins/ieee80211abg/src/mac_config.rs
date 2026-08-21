extern "C" {
    fn emane_ieee80211abg_macconfig_getPromiscuosEnable(ptr: *mut std::ffi::c_void) -> bool;
    fn emane_ieee80211abg_macconfig_getWmmEnable(ptr: *mut std::ffi::c_void) -> bool;
    fn emane_ieee80211abg_macconfig_getModulationType(ptr: *mut std::ffi::c_void) -> i32;
    fn emane_ieee80211abg_macconfig_getUnicastDataRateIndex(ptr: *mut std::ffi::c_void) -> u8;
    fn emane_ieee80211abg_macconfig_getBroadcastDataRateIndex(ptr: *mut std::ffi::c_void) -> u8;
    fn emane_ieee80211abg_macconfig_getUnicastDataRateKbps(ptr: *mut std::ffi::c_void) -> u32;
    fn emane_ieee80211abg_macconfig_getBroadcastDataRateKbps(ptr: *mut std::ffi::c_void) -> u32;
    fn emane_ieee80211abg_macconfig_getMaxDataRateKbps(ptr: *mut std::ffi::c_void) -> u32;
    fn emane_ieee80211abg_macconfig_getUnicastDataRateKbps_by_category(
        ptr: *mut std::ffi::c_void,
        arg0: u8,
    ) -> u32;
    fn emane_ieee80211abg_macconfig_getBroadcastDataRateKbps_by_category(
        ptr: *mut std::ffi::c_void,
        arg0: u8,
    ) -> u32;
    fn emane_ieee80211abg_macconfig_getMaxP2pDistance(ptr: *mut std::ffi::c_void) -> u32;
    fn emane_ieee80211abg_macconfig_getNumAccessCategories(ptr: *mut std::ffi::c_void) -> u8;
    fn emane_ieee80211abg_macconfig_getRtsThreshold(ptr: *mut std::ffi::c_void) -> u16;
    fn emane_ieee80211abg_macconfig_getQueueSize(ptr: *mut std::ffi::c_void, arg0: u8) -> u8;
    fn emane_ieee80211abg_macconfig_getQueueEntrySize(ptr: *mut std::ffi::c_void, arg0: u8) -> u16;
    fn emane_ieee80211abg_macconfig_getCWMin(ptr: *mut std::ffi::c_void, arg0: u8) -> u16;
    fn emane_ieee80211abg_macconfig_getCWMax(ptr: *mut std::ffi::c_void, arg0: u8) -> u16;
    fn emane_ieee80211abg_macconfig_getAifsMicroseconds(
        ptr: *mut std::ffi::c_void,
        arg0: u8,
    ) -> u64;
    fn emane_ieee80211abg_macconfig_getTxOpMicroseconds(
        ptr: *mut std::ffi::c_void,
        arg0: u8,
    ) -> u64;
    fn emane_ieee80211abg_macconfig_getRetryLimit(ptr: *mut std::ffi::c_void, arg0: u8) -> u8;
    fn emane_ieee80211abg_macconfig_getFlowControlTokens(ptr: *mut std::ffi::c_void) -> u16;
    fn emane_ieee80211abg_macconfig_getFlowControlEnable(ptr: *mut std::ffi::c_void) -> bool;
    fn emane_ieee80211abg_macconfig_getNeighborTimeoutMicroseconds(
        ptr: *mut std::ffi::c_void,
    ) -> u64;
    fn emane_ieee80211abg_macconfig_getChannelActivityIntervalMicroseconds(
        ptr: *mut std::ffi::c_void,
    ) -> u64;
    fn emane_ieee80211abg_macconfig_getNeighborMetricDeleteTimeMicroseconds(
        ptr: *mut std::ffi::c_void,
    ) -> u64;
    fn emane_ieee80211abg_macconfig_getRadioMetricReportIntervalMicroseconds(
        ptr: *mut std::ffi::c_void,
    ) -> u64;
    fn emane_ieee80211abg_macconfig_getRadioMetricEnable(ptr: *mut std::ffi::c_void) -> bool;
}

pub struct MACConfig {
    pub ptr: *mut std::ffi::c_void,
}

impl MACConfig {
    pub fn new(ptr: *mut std::ffi::c_void) -> Self {
        Self { ptr }
    }

    pub fn get_promiscuosenable(&self) -> bool {
        unsafe { emane_ieee80211abg_macconfig_getPromiscuosEnable(self.ptr) }
    }
    pub fn get_wmmenable(&self) -> bool {
        unsafe { emane_ieee80211abg_macconfig_getWmmEnable(self.ptr) }
    }
    pub fn get_modulationtype(&self) -> i32 {
        unsafe { emane_ieee80211abg_macconfig_getModulationType(self.ptr) }
    }
    pub fn get_unicastdatarateindex(&self) -> u8 {
        unsafe { emane_ieee80211abg_macconfig_getUnicastDataRateIndex(self.ptr) }
    }
    pub fn get_broadcastdatarateindex(&self) -> u8 {
        unsafe { emane_ieee80211abg_macconfig_getBroadcastDataRateIndex(self.ptr) }
    }
    pub fn get_unicastdataratekbps(&self) -> u32 {
        unsafe { emane_ieee80211abg_macconfig_getUnicastDataRateKbps(self.ptr) }
    }
    pub fn get_broadcastdataratekbps(&self) -> u32 {
        unsafe { emane_ieee80211abg_macconfig_getBroadcastDataRateKbps(self.ptr) }
    }
    pub fn get_maxdataratekbps(&self) -> u32 {
        unsafe { emane_ieee80211abg_macconfig_getMaxDataRateKbps(self.ptr) }
    }
    pub fn get_unicastdataratekbps_by_category(&self, arg0: u8) -> u32 {
        unsafe { emane_ieee80211abg_macconfig_getUnicastDataRateKbps_by_category(self.ptr, arg0) }
    }
    pub fn get_broadcastdataratekbps_by_category(&self, arg0: u8) -> u32 {
        unsafe { emane_ieee80211abg_macconfig_getBroadcastDataRateKbps_by_category(self.ptr, arg0) }
    }
    pub fn get_maxp2pdistance(&self) -> u32 {
        unsafe { emane_ieee80211abg_macconfig_getMaxP2pDistance(self.ptr) }
    }
    pub fn get_numaccesscategories(&self) -> u8 {
        unsafe { emane_ieee80211abg_macconfig_getNumAccessCategories(self.ptr) }
    }
    pub fn get_rtsthreshold(&self) -> u16 {
        unsafe { emane_ieee80211abg_macconfig_getRtsThreshold(self.ptr) }
    }
    pub fn get_queuesize(&self, arg0: u8) -> u8 {
        unsafe { emane_ieee80211abg_macconfig_getQueueSize(self.ptr, arg0) }
    }
    pub fn get_queueentrysize(&self, arg0: u8) -> u16 {
        unsafe { emane_ieee80211abg_macconfig_getQueueEntrySize(self.ptr, arg0) }
    }
    pub fn get_cwmin(&self, arg0: u8) -> u16 {
        unsafe { emane_ieee80211abg_macconfig_getCWMin(self.ptr, arg0) }
    }
    pub fn get_cwmax(&self, arg0: u8) -> u16 {
        unsafe { emane_ieee80211abg_macconfig_getCWMax(self.ptr, arg0) }
    }
    pub fn get_aifsmicroseconds(&self, arg0: u8) -> u64 {
        unsafe { emane_ieee80211abg_macconfig_getAifsMicroseconds(self.ptr, arg0) }
    }
    pub fn get_txopmicroseconds(&self, arg0: u8) -> u64 {
        unsafe { emane_ieee80211abg_macconfig_getTxOpMicroseconds(self.ptr, arg0) }
    }
    pub fn get_retrylimit(&self, arg0: u8) -> u8 {
        unsafe { emane_ieee80211abg_macconfig_getRetryLimit(self.ptr, arg0) }
    }
    pub fn get_flowcontroltokens(&self) -> u16 {
        unsafe { emane_ieee80211abg_macconfig_getFlowControlTokens(self.ptr) }
    }
    pub fn get_flowcontrolenable(&self) -> bool {
        unsafe { emane_ieee80211abg_macconfig_getFlowControlEnable(self.ptr) }
    }
    pub fn get_neighbortimeoutmicroseconds(&self) -> u64 {
        unsafe { emane_ieee80211abg_macconfig_getNeighborTimeoutMicroseconds(self.ptr) }
    }
    pub fn get_channelactivityintervalmicroseconds(&self) -> u64 {
        unsafe { emane_ieee80211abg_macconfig_getChannelActivityIntervalMicroseconds(self.ptr) }
    }
    pub fn get_neighbormetricdeletetimemicroseconds(&self) -> u64 {
        unsafe { emane_ieee80211abg_macconfig_getNeighborMetricDeleteTimeMicroseconds(self.ptr) }
    }
    pub fn get_radiometricreportintervalmicroseconds(&self) -> u64 {
        unsafe { emane_ieee80211abg_macconfig_getRadioMetricReportIntervalMicroseconds(self.ptr) }
    }
    pub fn get_radiometricenable(&self) -> bool {
        unsafe { emane_ieee80211abg_macconfig_getRadioMetricEnable(self.ptr) }
    }
}
unsafe impl Send for MACConfig {}
unsafe impl Sync for MACConfig {}
