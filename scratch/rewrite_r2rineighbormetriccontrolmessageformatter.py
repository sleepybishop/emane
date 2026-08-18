with open("src/libemane/r2rineighbormetriccontrolmessageformatter.cc", "w") as f:
    f.write("""#include <cstdint>
/*
 * Copyright (c) 2014 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "emane/controls/r2rineighbormetriccontrolmessageformatter.h"

extern "C" {
    void emane_rs_format_r2ri_neighbor_metric(uint16_t nem_id, uint64_t num_rx_frames, uint64_t num_tx_frames, uint64_t num_missed_frames, double bandwidth_consumption, float sinr_avg, float sinr_stdv, float noise_floor_avg, float noise_floor_stdv, uint64_t rx_avg_data_rate, uint64_t tx_avg_data_rate, void* ctx, void (*add_string)(void*, const char*));
}

EMANE::Controls::R2RINeighborMetricControlMessageFormatter::
R2RINeighborMetricControlMessageFormatter(const R2RINeighborMetricControlMessage * pMsg):
  pMsg_{pMsg}{}

EMANE::Strings EMANE::Controls::R2RINeighborMetricControlMessageFormatter::operator()() const
{
  Strings strings;
  
  for(const auto & metric : pMsg_->getNeighborMetrics())
    {
      emane_rs_format_r2ri_neighbor_metric(
          metric.getId(),
          metric.getNumRxFrames(),
          metric.getNumTxFrames(),
          metric.getNumMissedFrames(),
          metric.getBandwidthConsumption().count(),
          metric.getSINRAvgdBm(),
          metric.getSINRStddev(),
          metric.getNoiseFloorAvgdBm(),
          metric.getNoiseFloorStddev(),
          metric.getRxAvgDataRatebps(),
          metric.getTxAvgDataRatebps(),
          &strings,
          [](void* ctx, const char* s) {
              static_cast<Strings*>(ctx)->push_back(s);
          }
      );
    }
  
  return strings;
}
""")
