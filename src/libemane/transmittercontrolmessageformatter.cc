#include <cstdint>
/*
 * Copyright (c) 2013 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "emane/controls/transmittercontrolmessageformatter.h"

extern "C" {
    void emane_rs_format_transmitter_control(uint16_t nem_id, double power, void* ctx, void (*add_string)(void*, const char*));
}

EMANE::Controls::TransmitterControlMessageFormatter::
TransmitterControlMessageFormatter(const TransmitterControlMessage * pMsg):
  pMsg_{pMsg}{}

EMANE::Strings EMANE::Controls::TransmitterControlMessageFormatter::operator()() const
{
  Strings strings;
  
  for(const auto & transmitter : pMsg_->getTransmitters())
    {
      emane_rs_format_transmitter_control(
          transmitter.getNEMId(),
          transmitter.getPowerdBm(),
          &strings,
          [](void* ctx, const char* s) {
              static_cast<Strings*>(ctx)->push_back(s);
          }
      );
    }
  return strings;
}
