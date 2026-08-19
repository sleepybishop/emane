#include <stddef.h>
void emane_ieee80211abg_macstatistics_incrementDownstreamUnicastDataDiscardDueToRetries(void* pStat) __attribute__((weak));
void emane_ieee80211abg_macstatistics_incrementDownstreamUnicastDataDiscardDueToRetries(void* pStat) {}
void emane_ieee80211abg_macstatistics_incrementDownstreamUnicastRtsCtsDataDiscardDueToRetries(void* pStat) __attribute__((weak));
void emane_ieee80211abg_macstatistics_incrementDownstreamUnicastRtsCtsDataDiscardDueToRetries(void* pStat) {}
void emane_ieee80211abg_macstatistics_incrementDownstreamUnicastDataDiscardDueToTxop(void* pStat) __attribute__((weak));
void emane_ieee80211abg_macstatistics_incrementDownstreamUnicastDataDiscardDueToTxop(void* pStat) {}
void emane_ieee80211abg_macstatistics_incrementDownstreamBroadcastDataDiscardDueToTxop(void* pStat) __attribute__((weak));
void emane_ieee80211abg_macstatistics_incrementDownstreamBroadcastDataDiscardDueToTxop(void* pStat) {}
void emane_ieee80211abg_macstatistics_incrementUpstreamUnicastDataDiscardDueToSinr(void* pStat) __attribute__((weak));
void emane_ieee80211abg_macstatistics_incrementUpstreamUnicastDataDiscardDueToSinr(void* pStat) {}
void emane_ieee80211abg_macstatistics_incrementUpstreamBroadcastDataDiscardDueToSinr(void* pStat) __attribute__((weak));
void emane_ieee80211abg_macstatistics_incrementUpstreamBroadcastDataDiscardDueToSinr(void* pStat) {}
void emane_ieee80211abg_macstatistics_incrementUpstreamUnicastDataDiscardDueToClobberRxDuringTx(void* pStat) __attribute__((weak));
void emane_ieee80211abg_macstatistics_incrementUpstreamUnicastDataDiscardDueToClobberRxDuringTx(void* pStat) {}
void emane_ieee80211abg_macstatistics_incrementUpstreamBroadcastDataDiscardDueToClobberRxDuringTx(void* pStat) __attribute__((weak));
void emane_ieee80211abg_macstatistics_incrementUpstreamBroadcastDataDiscardDueToClobberRxDuringTx(void* pStat) {}
void emane_ieee80211abg_macstatistics_incrementUpstreamUnicastDataDiscardDueToClobberRxHiddenBusy(void* pStat) __attribute__((weak));
void emane_ieee80211abg_macstatistics_incrementUpstreamUnicastDataDiscardDueToClobberRxHiddenBusy(void* pStat) {}
void emane_ieee80211abg_macstatistics_incrementUpstreamBroadcastDataDiscardDueToClobberRxHiddenBusy(void* pStat) __attribute__((weak));
void emane_ieee80211abg_macstatistics_incrementUpstreamBroadcastDataDiscardDueToClobberRxHiddenBusy(void* pStat) {}
void emane_ieee80211abg_macstatistics_incrementUpstreamBroadcastNoiseHiddenRx(void* pStat) __attribute__((weak));
void emane_ieee80211abg_macstatistics_incrementUpstreamBroadcastNoiseHiddenRx(void* pStat) {}
void emane_ieee80211abg_macstatistics_incrementUpstreamUnicastNoiseHiddenRx(void* pStat) __attribute__((weak));
void emane_ieee80211abg_macstatistics_incrementUpstreamUnicastNoiseHiddenRx(void* pStat) {}
void emane_ieee80211abg_macstatistics_incrementUpstreamBroadcastNoiseRxCommon(void* pStat) __attribute__((weak));
void emane_ieee80211abg_macstatistics_incrementUpstreamBroadcastNoiseRxCommon(void* pStat) {}
void emane_ieee80211abg_macstatistics_incrementUpstreamUnicastNoiseRxCommon(void* pStat) __attribute__((weak));
void emane_ieee80211abg_macstatistics_incrementUpstreamUnicastNoiseRxCommon(void* pStat) {}
void emane_ieee80211abg_macstatistics_incrementUpstreamUnicastRtsCtsDataRxFromPhy(void* pStat) __attribute__((weak));
void emane_ieee80211abg_macstatistics_incrementUpstreamUnicastRtsCtsDataRxFromPhy(void* pStat) {}
void emane_ieee80211abg_macstatistics_incrementUpstreamUnicastCtsRxFromPhy(void* pStat) __attribute__((weak));
void emane_ieee80211abg_macstatistics_incrementUpstreamUnicastCtsRxFromPhy(void* pStat) {}
void emane_ieee80211abg_macstatistics_incrementRxOneHopNbrListEventCount(void* pStat) __attribute__((weak));
void emane_ieee80211abg_macstatistics_incrementRxOneHopNbrListEventCount(void* pStat) {}
void emane_ieee80211abg_macstatistics_incrementRxOneHopNbrListInvalidEventCount(void* pStat) __attribute__((weak));
void emane_ieee80211abg_macstatistics_incrementRxOneHopNbrListInvalidEventCount(void* pStat) {}
void emane_ieee80211abg_macstatistics_incrementTxOneHopNbrListEventCount(void* pStat) __attribute__((weak));
void emane_ieee80211abg_macstatistics_incrementTxOneHopNbrListEventCount(void* pStat) {}

void emane_ieee80211abg_macstatistics_updateOneHopNbrHighWaterMark(void* pStat, size_t num) __attribute__((weak));
void emane_ieee80211abg_macstatistics_updateOneHopNbrHighWaterMark(void* pStat, size_t num) {}
void emane_ieee80211abg_macstatistics_updateTwoHopNbrHighWaterMark(void* pStat, size_t num) __attribute__((weak));
void emane_ieee80211abg_macstatistics_updateTwoHopNbrHighWaterMark(void* pStat, size_t num) {}

void* emane_ieee80211abg_collisiontable_new() __attribute__((weak));
void* emane_ieee80211abg_collisiontable_new() { return 0; }
void emane_ieee80211abg_collisiontable_free(void* ptr) __attribute__((weak));
void emane_ieee80211abg_collisiontable_free(void* ptr) {}
float emane_ieee80211abg_collisiontable_getCollisionFactor(void* ptr, int num, int cw) __attribute__((weak));
float emane_ieee80211abg_collisiontable_getCollisionFactor(void* ptr, int num, int cw) { return 0.0f; }

bool emane_ieee80211abg_macconfig_getPromiscuosEnable(void* pConfig) __attribute__((weak));
bool emane_ieee80211abg_macconfig_getPromiscuosEnable(void* pConfig) { return 0; }
bool emane_ieee80211abg_macconfig_getWmmEnable(void* pConfig) __attribute__((weak));
bool emane_ieee80211abg_macconfig_getWmmEnable(void* pConfig) { return 0; }
int emane_ieee80211abg_macconfig_getModulationType(void* pConfig) __attribute__((weak));
int emane_ieee80211abg_macconfig_getModulationType(void* pConfig) { return 0; }
uint8_t emane_ieee80211abg_macconfig_getUnicastDataRateIndex(void* pConfig) __attribute__((weak));
uint8_t emane_ieee80211abg_macconfig_getUnicastDataRateIndex(void* pConfig) { return 0; }
uint8_t emane_ieee80211abg_macconfig_getBroadcastDataRateIndex(void* pConfig) __attribute__((weak));
uint8_t emane_ieee80211abg_macconfig_getBroadcastDataRateIndex(void* pConfig) { return 0; }
uint32_t emane_ieee80211abg_macconfig_getUnicastDataRateKbps(void* pConfig) __attribute__((weak));
uint32_t emane_ieee80211abg_macconfig_getUnicastDataRateKbps(void* pConfig) { return 0; }
uint32_t emane_ieee80211abg_macconfig_getBroadcastDataRateKbps(void* pConfig) __attribute__((weak));
uint32_t emane_ieee80211abg_macconfig_getBroadcastDataRateKbps(void* pConfig) { return 0; }
uint32_t emane_ieee80211abg_macconfig_getMaxDataRateKbps(void* pConfig) __attribute__((weak));
uint32_t emane_ieee80211abg_macconfig_getMaxDataRateKbps(void* pConfig) { return 0; }
uint32_t emane_ieee80211abg_macconfig_getUnicastDataRateKbps_by_category(void* pConfig, uint8_t arg0) __attribute__((weak));
uint32_t emane_ieee80211abg_macconfig_getUnicastDataRateKbps_by_category(void* pConfig, uint8_t arg0) { return 0; }
uint32_t emane_ieee80211abg_macconfig_getBroadcastDataRateKbps_by_category(void* pConfig, uint8_t arg0) __attribute__((weak));
uint32_t emane_ieee80211abg_macconfig_getBroadcastDataRateKbps_by_category(void* pConfig, uint8_t arg0) { return 0; }
uint32_t emane_ieee80211abg_macconfig_getMaxP2pDistance(void* pConfig) __attribute__((weak));
uint32_t emane_ieee80211abg_macconfig_getMaxP2pDistance(void* pConfig) { return 0; }
uint8_t emane_ieee80211abg_macconfig_getNumAccessCategories(void* pConfig) __attribute__((weak));
uint8_t emane_ieee80211abg_macconfig_getNumAccessCategories(void* pConfig) { return 0; }
uint16_t emane_ieee80211abg_macconfig_getRtsThreshold(void* pConfig) __attribute__((weak));
uint16_t emane_ieee80211abg_macconfig_getRtsThreshold(void* pConfig) { return 0; }
uint8_t emane_ieee80211abg_macconfig_getQueueSize(void* pConfig, uint8_t arg0) __attribute__((weak));
uint8_t emane_ieee80211abg_macconfig_getQueueSize(void* pConfig, uint8_t arg0) { return 0; }
uint16_t emane_ieee80211abg_macconfig_getQueueEntrySize(void* pConfig, uint8_t arg0) __attribute__((weak));
uint16_t emane_ieee80211abg_macconfig_getQueueEntrySize(void* pConfig, uint8_t arg0) { return 0; }
uint16_t emane_ieee80211abg_macconfig_getCWMin(void* pConfig, uint8_t arg0) __attribute__((weak));
uint16_t emane_ieee80211abg_macconfig_getCWMin(void* pConfig, uint8_t arg0) { return 0; }
uint16_t emane_ieee80211abg_macconfig_getCWMax(void* pConfig, uint8_t arg0) __attribute__((weak));
uint16_t emane_ieee80211abg_macconfig_getCWMax(void* pConfig, uint8_t arg0) { return 0; }
uint64_t emane_ieee80211abg_macconfig_getAifsMicroseconds(void* pConfig, uint8_t arg0) __attribute__((weak));
uint64_t emane_ieee80211abg_macconfig_getAifsMicroseconds(void* pConfig, uint8_t arg0) { return 0; }
uint64_t emane_ieee80211abg_macconfig_getTxOpMicroseconds(void* pConfig, uint8_t arg0) __attribute__((weak));
uint64_t emane_ieee80211abg_macconfig_getTxOpMicroseconds(void* pConfig, uint8_t arg0) { return 0; }
uint8_t emane_ieee80211abg_macconfig_getRetryLimit(void* pConfig, uint8_t arg0) __attribute__((weak));
uint8_t emane_ieee80211abg_macconfig_getRetryLimit(void* pConfig, uint8_t arg0) { return 0; }
uint16_t emane_ieee80211abg_macconfig_getFlowControlTokens(void* pConfig) __attribute__((weak));
uint16_t emane_ieee80211abg_macconfig_getFlowControlTokens(void* pConfig) { return 0; }
bool emane_ieee80211abg_macconfig_getFlowControlEnable(void* pConfig) __attribute__((weak));
bool emane_ieee80211abg_macconfig_getFlowControlEnable(void* pConfig) { return 0; }
uint64_t emane_ieee80211abg_macconfig_getNeighborTimeoutMicroseconds(void* pConfig) __attribute__((weak));
uint64_t emane_ieee80211abg_macconfig_getNeighborTimeoutMicroseconds(void* pConfig) { return 0; }
uint64_t emane_ieee80211abg_macconfig_getChannelActivityIntervalMicroseconds(void* pConfig) __attribute__((weak));
uint64_t emane_ieee80211abg_macconfig_getChannelActivityIntervalMicroseconds(void* pConfig) { return 0; }
uint64_t emane_ieee80211abg_macconfig_getNeighborMetricDeleteTimeMicroseconds(void* pConfig) __attribute__((weak));
uint64_t emane_ieee80211abg_macconfig_getNeighborMetricDeleteTimeMicroseconds(void* pConfig) { return 0; }
uint64_t emane_ieee80211abg_macconfig_getRadioMetricReportIntervalMicroseconds(void* pConfig) __attribute__((weak));
uint64_t emane_ieee80211abg_macconfig_getRadioMetricReportIntervalMicroseconds(void* pConfig) { return 0; }
bool emane_ieee80211abg_macconfig_getRadioMetricEnable(void* pConfig) __attribute__((weak));
bool emane_ieee80211abg_macconfig_getRadioMetricEnable(void* pConfig) { return 0; }


void* emane_ieee80211abg_modetimingparameters_new(void* config_ptr) __attribute__((weak));
void* emane_ieee80211abg_modetimingparameters_new(void* config_ptr) { return 0; }
void emane_ieee80211abg_modetimingparameters_free(void* ptr) __attribute__((weak));
void emane_ieee80211abg_modetimingparameters_free(void* ptr) {}
uint64_t emane_ieee80211abg_modetimingparameters_getSlotSizeMicroseconds(void* ptr) __attribute__((weak));
uint64_t emane_ieee80211abg_modetimingparameters_getSlotSizeMicroseconds(void* ptr) { return 0; }
uint64_t emane_ieee80211abg_modetimingparameters_getOverheadMicroseconds(void* ptr, uint8_t category) __attribute__((weak));
uint64_t emane_ieee80211abg_modetimingparameters_getOverheadMicroseconds(void* ptr, uint8_t category) { return 0; }
uint64_t emane_ieee80211abg_modetimingparameters_getDeferIntervalMicroseconds(void* ptr, uint8_t category) __attribute__((weak));
uint64_t emane_ieee80211abg_modetimingparameters_getDeferIntervalMicroseconds(void* ptr, uint8_t category) { return 0; }
uint64_t emane_ieee80211abg_modetimingparameters_getMessageDurationMicroseconds(void* ptr, uint8_t category, size_t len) __attribute__((weak));
uint64_t emane_ieee80211abg_modetimingparameters_getMessageDurationMicroseconds(void* ptr, uint8_t category, size_t len) { return 0; }
uint64_t emane_ieee80211abg_modetimingparameters_getBroadcastMessageDurationMicroseconds(void* ptr, size_t len) __attribute__((weak));
uint64_t emane_ieee80211abg_modetimingparameters_getBroadcastMessageDurationMicroseconds(void* ptr, size_t len) { return 0; }
uint64_t emane_ieee80211abg_modetimingparameters_getUnicastMessageDurationMicroseconds(void* ptr, size_t len) __attribute__((weak));
uint64_t emane_ieee80211abg_modetimingparameters_getUnicastMessageDurationMicroseconds(void* ptr, size_t len) { return 0; }
uint64_t emane_ieee80211abg_modetimingparameters_getCtsMessageDurationMicroseconds(void* ptr) __attribute__((weak));
uint64_t emane_ieee80211abg_modetimingparameters_getCtsMessageDurationMicroseconds(void* ptr) { return 0; }
uint64_t emane_ieee80211abg_modetimingparameters_getRtsMessageDurationMicroseconds(void* ptr) __attribute__((weak));
uint64_t emane_ieee80211abg_modetimingparameters_getRtsMessageDurationMicroseconds(void* ptr) { return 0; }



void emane_ieee80211abg_maclayer_sendDownstreamBroadcastData(void* maclayer, void* entry) __attribute__((weak));
void emane_ieee80211abg_maclayer_sendDownstreamBroadcastData(void* maclayer, void* entry) {}
void emane_ieee80211abg_maclayer_sendDownstreamUnicastData(void* maclayer, void* entry) __attribute__((weak));
void emane_ieee80211abg_maclayer_sendDownstreamUnicastData(void* maclayer, void* entry) {}
void emane_ieee80211abg_maclayer_setDelayTime(void* maclayer, void* entry) __attribute__((weak));
void emane_ieee80211abg_maclayer_setDelayTime(void* maclayer, void* entry) {}

