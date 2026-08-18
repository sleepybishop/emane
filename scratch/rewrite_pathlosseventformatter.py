with open("src/libemane/pathlosseventformatter.cc", "w") as f:
    f.write("""#include <cstdint>
/*
 * Copyright (c) 2014 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "emane/events/pathlosseventformatter.h"

extern "C" {
    void emane_rs_format_pathloss_element(uint16_t nem_id, float fwd, float rev, void* ctx, void (*add_string)(void*, const char*));
}

EMANE::Events::PathlossEventFormatter::
PathlossEventFormatter(const PathlossEvent & event):
  event_(event)
{}
      
EMANE::Strings EMANE::Events::PathlossEventFormatter::operator()() const
{
  Strings strings;

  for(const auto & entry : event_.getPathlosses())
    {
        emane_rs_format_pathloss_element(
            entry.getNEMId(),
            entry.getForwardPathlossdB(),
            entry.getReversePathlossdB(),
            &strings,
            [](void* ctx, const char* s) {
                static_cast<Strings*>(ctx)->push_back(s);
            }
        );
    }

  return strings;
}
""")
