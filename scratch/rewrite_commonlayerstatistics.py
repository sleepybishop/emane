import os

# We will create rust/emane-core/src/common_layer_statistics.rs
# And we will replace src/libemane/commonlayerstatistics.cc

rust_code = """
// rust/emane-core/src/common_layer_statistics.rs
use std::os::raw::{c_void, c_char};
use std::collections::HashMap;

#[repr(C)]
pub struct CommonLayerStatisticsState {
    pub p_statistic_unicast_drop_table: *mut c_void,
    pub p_statistic_broadcast_drop_table: *mut c_void,
    pub p_statistic_unicast_accept_table: *mut c_void,
    pub p_statistic_broadcast_accept_table: *mut c_void,
    // Add counters and drop map logic here
}

#[no_mangle]
pub extern "C" fn emane_rs_common_layer_statistics_new() -> *mut c_void {
    let state = Box::new(CommonLayerStatisticsState {
        p_statistic_unicast_drop_table: std::ptr::null_mut(),
        p_statistic_broadcast_drop_table: std::ptr::null_mut(),
        p_statistic_unicast_accept_table: std::ptr::null_mut(),
        p_statistic_broadcast_accept_table: std::ptr::null_mut(),
    });
    Box::into_raw(state) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_common_layer_statistics_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr as *mut CommonLayerStatisticsState)); }
    }
}
"""

cpp_code = """
#include "emane/utils/commonlayerstatistics.h"

extern "C" {
    void* emane_rs_common_layer_statistics_new();
    void emane_rs_common_layer_statistics_destroy(void* ptr);
}

class EMANE::Utils::CommonLayerStatistics::Implementation {
public:
    Implementation(const StatisticTableLabels&, const StatisticTableLabels&, const std::string&) {
        ptr_ = emane_rs_common_layer_statistics_new();
    }
    ~Implementation() {
        emane_rs_common_layer_statistics_destroy(ptr_);
    }
    void registerStatistics(StatisticRegistrar&) {}
    void processInbound(const UpstreamPacket&) {}
    void processInbound(const DownstreamPacket&) {}
    void processOutbound(const UpstreamPacket&, Microseconds, size_t) {}
    void processOutbound(const DownstreamPacket&, Microseconds, size_t, bool) {}
private:
    void* ptr_;
};

EMANE::Utils::CommonLayerStatistics::CommonLayerStatistics(const StatisticTableLabels & unicastDropTableLabels,
                                                           const StatisticTableLabels & broadcastDropTableLabels,
                                                           const std::string & sInstance):
pImpl_{new Implementation{unicastDropTableLabels, broadcastDropTableLabels, sInstance}}
{ }

EMANE::Utils::CommonLayerStatistics::~CommonLayerStatistics() { }

void EMANE::Utils::CommonLayerStatistics::registerStatistics(StatisticRegistrar & statisticRegistrar) { pImpl_->registerStatistics(statisticRegistrar); }
void EMANE::Utils::CommonLayerStatistics::processInbound(const UpstreamPacket & pkt) { pImpl_->processInbound(pkt); }
void EMANE::Utils::CommonLayerStatistics::processOutbound(const UpstreamPacket & pkt, Microseconds delay, size_t dropCode) { pImpl_->processOutbound(pkt, delay, dropCode); }
void EMANE::Utils::CommonLayerStatistics::processInbound(const DownstreamPacket & pkt) { pImpl_->processInbound(pkt); }
void EMANE::Utils::CommonLayerStatistics::processOutbound(const DownstreamPacket & pkt, Microseconds delay, size_t dropCode, bool bSelfGenerated) { pImpl_->processOutbound(pkt, delay, dropCode, bSelfGenerated); }
"""

with open("scratch/common_layer_statistics.rs", "w") as f:
    f.write(rust_code)
with open("scratch/commonlayerstatistics.cc", "w") as f:
    f.write(cpp_code)
