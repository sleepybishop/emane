#include <cstdint>
/*
 * Copyright (c) 2019-2020 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "emane/controls/spectrumfilteraddcontrolmessageformatter.h"

extern "C" {
    void emane_rs_format_spectrum_filter_add(uint16_t filter_index, uint16_t antenna_index, uint64_t freq, uint64_t bandwidth, uint64_t subband_bin_size, void* ctx, void (*add_string)(void*, const char*));
}

EMANE::Controls::SpectrumFilterAddControlMessageFormatter::
SpectrumFilterAddControlMessageFormatter(const SpectrumFilterAddControlMessage * pMsg):
  pMsg_{pMsg}{}

EMANE::Strings EMANE::Controls::SpectrumFilterAddControlMessageFormatter::operator()() const
{
  Strings strings;
  
  emane_rs_format_spectrum_filter_add(
      pMsg_->getFilterIndex(),
      pMsg_->getAntennaIndex(),
      pMsg_->getFrequencyHz(),
      pMsg_->getBandwidthHz(),
      pMsg_->getSubBandBinSizeHz(),
      &strings,
      [](void* ctx, const char* s) {
          static_cast<Strings*>(ctx)->push_back(s);
      }
  );

  return strings;
}
