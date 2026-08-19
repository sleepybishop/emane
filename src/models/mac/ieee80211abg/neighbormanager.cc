#include "neighbormanager.h"
#include "maclayer.h"

extern "C" {
    ::FfiNeighborManager* NeighborManager_create(std::uint16_t id, EMANE::PlatformServiceProvider* platform_service, EMANE::Models::IEEE80211ABG::MACLayer* mac_layer);
    void NeighborManager_destroy(::FfiNeighborManager* ptr);
    void NeighborManager_updateDataChannelActivity(::FfiNeighborManager* ptr, std::uint16_t arg0, std::uint8_t arg1, float arg2, std::int64_t arg3, std::int64_t arg4, std::uint8_t arg5);
    void NeighborManager_updateCtrlChannelActivity(::FfiNeighborManager* ptr, std::uint16_t arg0, std::uint16_t arg1, std::uint8_t arg2, float arg3, std::int64_t arg4, std::int64_t arg5, std::uint8_t arg6);
    void NeighborManager_handleOneHopNeighborsEvent(::FfiNeighborManager* ptr, const void* arg0);
    void NeighborManager_start(::FfiNeighborManager* ptr);
    void NeighborManager_resetStatistics(::FfiNeighborManager* ptr);
    void NeighborManager_setNeighborTimeoutMicroseconds(::FfiNeighborManager* ptr, std::int64_t arg0);
    float NeighborManager_getNumberOfEstimatedOneHopNeighbors(::FfiNeighborManager* ptr);
    float NeighborManager_getNumberOfEstimatedTwoHopNeighbors(::FfiNeighborManager* ptr);
    float NeighborManager_getHiddenChannelActivity(::FfiNeighborManager* ptr, std::uint16_t arg0);
    float NeighborManager_getNumberOfEstimatedCommonNeighbors(::FfiNeighborManager* ptr, std::uint16_t arg0);
    float NeighborManager_getNumberOfEstimatedHiddenNeighbors(::FfiNeighborManager* ptr, std::uint16_t arg0);
    float NeighborManager_getLocalNodeTx(::FfiNeighborManager* ptr);
    std::size_t NeighborManager_getTotalActiveOneHopNeighbors(::FfiNeighborManager* ptr);
    void NeighborManager_setCategories(::FfiNeighborManager* ptr, std::uint8_t arg0);
    std::int64_t NeighborManager_getTotalOneHopUtilizationMicroseconds(::FfiNeighborManager* ptr);
    std::int64_t NeighborManager_getTotalTwoHopUtilizationMicroseconds(::FfiNeighborManager* ptr);
    std::int64_t NeighborManager_getAverageMessageDurationMicroseconds(::FfiNeighborManager* ptr);
    std::int64_t NeighborManager_getAllUtilizationMicroseconds(::FfiNeighborManager* ptr, std::uint16_t arg0);
    float NeighborManager_getAverageRxPowerPerMessageMilliWatts(::FfiNeighborManager* ptr);
    float NeighborManager_getAverageRxPowerPerMessageHiddenNodesMilliWatts(::FfiNeighborManager* ptr);
    float NeighborManager_getAverageRxPowerPerMessageCommonNodesMilliWatts(::FfiNeighborManager* ptr);
    float NeighborManager_getRandomRxPowerCommonNodesMilliWatts(::FfiNeighborManager* ptr, std::uint16_t arg0);
    float NeighborManager_getRandomRxPowerHiddenNodesMilliWatts(::FfiNeighborManager* ptr, std::uint16_t arg0);
    std::int64_t NeighborManager_getLastOneHopNbrListTxTime(::FfiNeighborManager* ptr);
    void* NeighborManager_getUtilizationRatios(::FfiNeighborManager* ptr);
    void NeighborManager_registerStatistics(::FfiNeighborManager* ptr, void* arg0);
}
namespace EMANE {
namespace Models {
namespace IEEE80211ABG {

NeighborManager::NeighborManager(NEMId id, PlatformServiceProvider * pPlatformService, MACLayer *pMgr)
    : id_(id), pPlatformService_(pPlatformService), pMACLayer_(pMgr)
{
    rs_state_ = NeighborManager_create(id, pPlatformService, pMgr);
}

NeighborManager::~NeighborManager()
{
    NeighborManager_destroy(rs_state_);
}

void NeighborManager::updateDataChannelActivity(NEMId src, std::uint8_t type, float fRxPowerMilliWatts, const TimePoint & timePoint, const Microseconds & duration, std::uint8_t u8Category) {
    NeighborManager_updateDataChannelActivity(rs_state_, src, type, fRxPowerMilliWatts, timePoint.time_since_epoch().count(), duration.count(), u8Category);
}

void NeighborManager::updateCtrlChannelActivity(NEMId src, NEMId origin, std::uint8_t type, float fRxPowerMilliWatts, const TimePoint & tvTime, const Microseconds & duration, std::uint8_t u8Category) {
    NeighborManager_updateCtrlChannelActivity(rs_state_, src, origin, type, fRxPowerMilliWatts, tvTime.time_since_epoch().count(), duration.count(), u8Category);
}

void NeighborManager::handleOneHopNeighborsEvent(const Serialization &serialization) {
    NeighborManager_handleOneHopNeighborsEvent(rs_state_, &serialization);
}

void NeighborManager::start() {
    NeighborManager_start(rs_state_);
}

void NeighborManager::resetStatistics() {
    NeighborManager_resetStatistics(rs_state_);
}

void NeighborManager::setNeighborTimeoutMicroseconds(const Microseconds & timeOutMicroseconds) {
    NeighborManager_setNeighborTimeoutMicroseconds(rs_state_, timeOutMicroseconds.count());
}

float NeighborManager::getNumberOfEstimatedOneHopNeighbors() const {
    return NeighborManager_getNumberOfEstimatedOneHopNeighbors(rs_state_);
}

float NeighborManager::getNumberOfEstimatedTwoHopNeighbors() const {
    return NeighborManager_getNumberOfEstimatedTwoHopNeighbors(rs_state_);
}

float NeighborManager::getHiddenChannelActivity(NEMId src) const {
    return NeighborManager_getHiddenChannelActivity(rs_state_, src);
}

float NeighborManager::getNumberOfEstimatedCommonNeighbors(NEMId src) const {
    return NeighborManager_getNumberOfEstimatedCommonNeighbors(rs_state_, src);
}

float NeighborManager::getNumberOfEstimatedHiddenNeighbors(NEMId src) const {
    return NeighborManager_getNumberOfEstimatedHiddenNeighbors(rs_state_, src);
}

float NeighborManager::getLocalNodeTx() const {
    return NeighborManager_getLocalNodeTx(rs_state_);
}

size_t NeighborManager::getTotalActiveOneHopNeighbors() const {
    return NeighborManager_getTotalActiveOneHopNeighbors(rs_state_);
}

void NeighborManager::setCategories(std::uint8_t u8NumCategories) {
    NeighborManager_setCategories(rs_state_, u8NumCategories);
}

Microseconds NeighborManager::getTotalOneHopUtilizationMicroseconds() const {
    return Microseconds(NeighborManager_getTotalOneHopUtilizationMicroseconds(rs_state_));
}

Microseconds NeighborManager::getTotalTwoHopUtilizationMicroseconds() const {
    return Microseconds(NeighborManager_getTotalTwoHopUtilizationMicroseconds(rs_state_));
}

Microseconds NeighborManager::getAverageMessageDurationMicroseconds() const {
    return Microseconds(NeighborManager_getAverageMessageDurationMicroseconds(rs_state_));
}

Microseconds NeighborManager::getAllUtilizationMicroseconds(NEMId src) const {
    return Microseconds(NeighborManager_getAllUtilizationMicroseconds(rs_state_, src));
}

float NeighborManager::getAverageRxPowerPerMessageMilliWatts() const {
    return NeighborManager_getAverageRxPowerPerMessageMilliWatts(rs_state_);
}

float NeighborManager::getAverageRxPowerPerMessageHiddenNodesMilliWatts() const {
    return NeighborManager_getAverageRxPowerPerMessageHiddenNodesMilliWatts(rs_state_);
}

float NeighborManager::getAverageRxPowerPerMessageCommonNodesMilliWatts() const {
    return NeighborManager_getAverageRxPowerPerMessageCommonNodesMilliWatts(rs_state_);
}

float NeighborManager::getRandomRxPowerCommonNodesMilliWatts(NEMId src) {
    return NeighborManager_getRandomRxPowerCommonNodesMilliWatts(rs_state_, src);
}

float NeighborManager::getRandomRxPowerHiddenNodesMilliWatts(NEMId src) {
    return NeighborManager_getRandomRxPowerHiddenNodesMilliWatts(rs_state_, src);
}

TimePoint NeighborManager::getLastOneHopNbrListTxTime() const {
    return TimePoint(Microseconds(NeighborManager_getLastOneHopNbrListTxTime(rs_state_)));
}

WMMManager::UtilizationRatioVector NeighborManager::getUtilizationRatios() {
    return WMMManager::UtilizationRatioVector(); // NeighborManager_getUtilizationRatios(rs_state_);
}

void NeighborManager::registerStatistics(StatisticRegistrar & statisticRegistrar) {
    NeighborManager_registerStatistics(rs_state_, &statisticRegistrar);
}
}
}
}
