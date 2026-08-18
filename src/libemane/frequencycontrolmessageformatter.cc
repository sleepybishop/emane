#include <cstdint>
/*
 * Copyright (c) 2013 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "emane/controls/frequencycontrolmessageformatter.h"

extern "C" {
    void emane_rs_format_foi_bandwidth(uint64_t bandwidth, void* ctx, void (*add_string)(void*, const char*));
    void emane_rs_format_frequency_control_segment(uint64_t freq, int64_t duration, int64_t offset, double power, void* ctx, void (*add_string)(void*, const char*));
}

EMANE::Controls::FrequencyControlMessageFormatter::
FrequencyControlMessageFormatter(const FrequencyControlMessage * pMsg):
  pMsg_{pMsg}{}

EMANE::Strings EMANE::Controls::FrequencyControlMessageFormatter::operator()() const
{
  Strings strings;
  
  auto add_str = [](void* ctx, const char* s) {
      static_cast<Strings*>(ctx)->push_back(s);
  };
  
  emane_rs_format_foi_bandwidth(pMsg_->getBandwidthHz(), &strings, add_str);

  for(const auto & segment : pMsg_->getFrequencySegments())
    {
      emane_rs_format_frequency_control_segment(
          segment.getFrequencyHz(),
          segment.getDuration().count(),
          segment.getOffset().count(),
          segment.getRxPowerdBm(),
          &strings,
          add_str
      );
    }
  
  return strings;
}
