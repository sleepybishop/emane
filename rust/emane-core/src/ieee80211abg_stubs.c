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
