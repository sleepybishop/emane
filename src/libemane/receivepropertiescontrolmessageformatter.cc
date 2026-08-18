#include <cstdint>
/*
 * Copyright (c) 2013-2014 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "emane/controls/receivepropertiescontrolmessageformatter.h"

extern "C" {
    void emane_rs_format_receive_properties(double sot, int64_t span, int64_t prop_delay, double rx_sens, void* ctx, void (*add_string)(void*, const char*));
}

EMANE::Controls::ReceivePropertiesControlMessageFormatter::
ReceivePropertiesControlMessageFormatter(const ReceivePropertiesControlMessage * pMsg):
  pMsg_{pMsg}{}

EMANE::Strings EMANE::Controls::ReceivePropertiesControlMessageFormatter::operator()() const
{
  Strings strings;
  
  emane_rs_format_receive_properties(
      std::chrono::duration_cast<DoubleSeconds>(pMsg_->getTxTime().time_since_epoch()).count(),
      pMsg_->getSpan().count(),
      pMsg_->getPropagationDelay().count(),
      pMsg_->getReceiverSensitivitydBm(),
      &strings,
      [](void* ctx, const char* s) {
          static_cast<Strings*>(ctx)->push_back(s);
      }
  );

  return strings;
}
