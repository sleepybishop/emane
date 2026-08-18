with open("src/libemane/rxantennaremovecontrolmessageformatter.cc", "w") as f:
    f.write("""#include <cstdint>
/*
 * Copyright (c) 2020 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "emane/controls/rxantennaremovecontrolmessageformatter.h"

extern "C" {
    void emane_rs_format_rx_antenna_remove(uint16_t antenna_index, void* ctx, void (*add_string)(void*, const char*));
}

EMANE::Controls::RxAntennaRemoveControlMessageFormatter::
RxAntennaRemoveControlMessageFormatter(const RxAntennaRemoveControlMessage * pMsg):
  pMsg_{pMsg}{}

EMANE::Strings EMANE::Controls::RxAntennaRemoveControlMessageFormatter::operator()() const
{
  Strings strings;
  emane_rs_format_rx_antenna_remove(pMsg_->getAntennaIndex(), &strings, [](void* ctx, const char* s) {
      static_cast<Strings*>(ctx)->push_back(s);
  });
  return strings;
}
""")
