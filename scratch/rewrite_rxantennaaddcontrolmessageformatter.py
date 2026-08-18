with open("src/libemane/rxantennaaddcontrolmessageformatter.cc", "w") as f:
    f.write("""#include <cstdint>
/*
 * Copyright (c) 2020 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "emane/controls/rxantennaaddcontrolmessageformatter.h"

extern "C" {
    void emane_rs_format_rx_antenna_add_info(uint16_t index, double gain, bool gain_b, double azimuth, double elevation, uint16_t profile_id, bool pointing_b, uint64_t bandwidth, void* ctx, void (*add_string)(void*, const char*));
    void emane_rs_format_rx_antenna_add_freq(uint64_t freq, void* ctx, void (*add_string)(void*, const char*));
}

EMANE::Controls::RxAntennaAddControlMessageFormatter::
RxAntennaAddControlMessageFormatter(const RxAntennaAddControlMessage * pMsg):
  pMsg_{pMsg}{}

EMANE::Strings EMANE::Controls::RxAntennaAddControlMessageFormatter::operator()() const
{
  Strings strings;
  
  auto add_str = [](void* ctx, const char* s) {
      static_cast<Strings*>(ctx)->push_back(s);
  };
  
  const auto & antenna = pMsg_->getAntenna();
  auto fixedGaindBi = antenna.getFixedGaindBi();
  auto pointing = antenna.getPointing();
  
  emane_rs_format_rx_antenna_add_info(
      antenna.getIndex(),
      fixedGaindBi.first,
      fixedGaindBi.second,
      pointing.first.getAzimuthDegrees(),
      pointing.first.getElevationDegrees(),
      pointing.first.getProfileId(),
      pointing.second,
      antenna.getBandwidthHz(),
      &strings,
      add_str
  );

  for(const auto & frequency : pMsg_->getFrequencyOfInterestSet())
    {
      emane_rs_format_rx_antenna_add_freq(frequency, &strings, add_str);
    }

  return strings;
}
""")
