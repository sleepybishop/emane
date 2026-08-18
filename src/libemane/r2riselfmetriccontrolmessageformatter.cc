#include <cstdint>
/*
 * Copyright (c) 2014 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "emane/controls/r2riselfmetriccontrolmessageformatter.h"

extern "C" {
    void emane_rs_format_r2ri_self_metric(uint64_t broadcast_bps, uint64_t max_bps, double interval, void* ctx, void (*add_string)(void*, const char*));
}

EMANE::Controls::R2RISelfMetricControlMessageFormatter::
R2RISelfMetricControlMessageFormatter(const R2RISelfMetricControlMessage * pMsg):
  pMsg_{pMsg}{}

EMANE::Strings EMANE::Controls::R2RISelfMetricControlMessageFormatter::operator()() const
{
  Strings strings;
  emane_rs_format_r2ri_self_metric(
      pMsg_->getBroadcastDataRatebps(),
      pMsg_->getMaxDataRatebps(),
      std::chrono::duration_cast<EMANE::DoubleSeconds>(pMsg_->getReportInterval()).count(),
      &strings,
      [](void* ctx, const char* s) {
          static_cast<Strings*>(ctx)->push_back(s);
      }
  );
  return strings;
}
