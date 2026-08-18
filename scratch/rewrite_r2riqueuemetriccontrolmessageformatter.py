with open("src/libemane/r2riqueuemetriccontrolmessageformatter.cc", "w") as f:
    f.write("""#include <cstdint>
/*
 * Copyright (c) 2014 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "emane/controls/r2riqueuemetriccontrolmessageformatter.h"

extern "C" {
    void emane_rs_format_r2ri_queue_metric(uint8_t queue_id, uint32_t max_size, uint32_t current_depth, uint32_t num_discards, double avg_delay, void* ctx, void (*add_string)(void*, const char*));
}

EMANE::Controls::R2RIQueueMetricControlMessageFormatter::
R2RIQueueMetricControlMessageFormatter(const R2RIQueueMetricControlMessage * pMsg):
  pMsg_{pMsg}{}

EMANE::Strings EMANE::Controls::R2RIQueueMetricControlMessageFormatter::operator()() const
{
  Strings strings;
  
  for(const auto & metric : pMsg_->getQueueMetrics())
    {
      emane_rs_format_r2ri_queue_metric(
          metric.getQueueId(),
          metric.getMaxSize(),
          metric.getCurrentDepth(),
          metric.getNumDiscards(),
          std::chrono::duration_cast<EMANE::DoubleSeconds>(metric.getAvgDelay()).count(),
          &strings,
          [](void* ctx, const char* s) {
              static_cast<Strings*>(ctx)->push_back(s);
          }
      );
    }
  
  return strings;
}
""")
