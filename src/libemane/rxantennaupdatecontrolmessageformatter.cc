#include <cstdint>
/*
 * Copyright (c) 2020 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "emane/controls/rxantennaupdatecontrolmessageformatter.h"

extern "C" {
    void emane_rs_format_rx_antenna_update_info(uint16_t index, double gain, bool gain_b, double azimuth, double elevation, bool pointing_b, void* ctx, void (*add_string)(void*, const char*));
}

EMANE::Controls::RxAntennaUpdateControlMessageFormatter::
RxAntennaUpdateControlMessageFormatter(const RxAntennaUpdateControlMessage * pMsg):
  pMsg_{pMsg}{}

EMANE::Strings EMANE::Controls::RxAntennaUpdateControlMessageFormatter::operator()() const
{
  Strings strings;
  
  auto add_str = [](void* ctx, const char* s) {
      static_cast<Strings*>(ctx)->push_back(s);
  };
  
  const auto & antenna = pMsg_->getAntenna();
  auto fixedGaindBi = antenna.getFixedGaindBi();
  auto pointing = antenna.getPointing();
  
  emane_rs_format_rx_antenna_update_info(
      antenna.getIndex(),
      fixedGaindBi.first,
      fixedGaindBi.second,
      pointing.first.getAzimuthDegrees(),
      pointing.first.getElevationDegrees(),
      pointing.second,
      &strings,
      add_str
  );

  return strings;
}
