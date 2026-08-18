#include <cstdint>
/*
 * Copyright (c) 2014 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "emane/controls/frequencyofinterestcontrolmessageformatter.h"

extern "C" {
    void emane_rs_format_foi_bandwidth(uint64_t bandwidth, void* ctx, void (*add_string)(void*, const char*));
    void emane_rs_format_foi_freq(uint64_t freq, void* ctx, void (*add_string)(void*, const char*));
}

EMANE::Controls::FrequencyOfInterestControlMessageFormatter::
FrequencyOfInterestControlMessageFormatter(const FrequencyOfInterestControlMessage * pMsg):
  pMsg_{pMsg}{}

EMANE::Strings EMANE::Controls::FrequencyOfInterestControlMessageFormatter::operator()() const
{
  Strings strings;
  
  auto add_str = [](void* ctx, const char* s) {
      static_cast<Strings*>(ctx)->push_back(s);
  };
  
  emane_rs_format_foi_bandwidth(pMsg_->getBandwidthHz(), &strings, add_str);

  for(const auto & frequencyHz : pMsg_->getFrequencySet())
    {
      emane_rs_format_foi_freq(frequencyHz, &strings, add_str);
    }
  
  return strings;
}
