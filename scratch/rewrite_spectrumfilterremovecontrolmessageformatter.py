with open("src/libemane/spectrumfilterremovecontrolmessageformatter.cc", "w") as f:
    f.write("""#include <cstdint>
/*
 * Copyright (c) 2019-2020 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "emane/controls/spectrumfilterremovecontrolmessageformatter.h"

extern "C" {
    void emane_rs_format_spectrum_filter_remove(uint16_t filter_index, uint16_t antenna_index, void* ctx, void (*add_string)(void*, const char*));
}

EMANE::Controls::SpectrumFilterRemoveControlMessageFormatter::
SpectrumFilterRemoveControlMessageFormatter(const SpectrumFilterRemoveControlMessage * pMsg):
  pMsg_{pMsg}{}

EMANE::Strings EMANE::Controls::SpectrumFilterRemoveControlMessageFormatter::operator()() const
{
  Strings strings;
  emane_rs_format_spectrum_filter_remove(
      pMsg_->getFilterIndex(),
      pMsg_->getAntennaIndex(),
      &strings,
      [](void* ctx, const char* s) {
          static_cast<Strings*>(ctx)->push_back(s);
      }
  );
  return strings;
}
""")
