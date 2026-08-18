with open("src/libemane/pathlossexeventformatter.cc", "w") as f:
    f.write("""#include <cstdint>
/*
 * Copyright (c) 2025 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "emane/events/pathlossexeventformatter.h"

extern "C" {
    void* emane_rs_format_pathloss_ex_start(uint16_t nem_id);
    void emane_rs_format_pathloss_ex_append(void* ptr, uint64_t freq, float pathloss);
    void emane_rs_format_pathloss_ex_end(void* ptr, void* ctx, void (*add_string)(void*, const char*));
}

EMANE::Events::PathlossExEventFormatter::
PathlossExEventFormatter(const PathlossExEvent & event):
  event_(event)
{}
      
EMANE::Strings EMANE::Events::PathlossExEventFormatter::operator()() const
{
  Strings strings;

  for(const auto & pathloessEx : event_.getPathlossExs())
    {
        void* state = emane_rs_format_pathloss_ex_start(pathloessEx.getNEMId());
        
        for(const auto & entry : pathloessEx.getFrequencyPathlossMap())
        {
            emane_rs_format_pathloss_ex_append(state, entry.first, entry.second);
        }
        
        emane_rs_format_pathloss_ex_end(state, &strings, [](void* ctx, const char* s) {
            static_cast<Strings*>(ctx)->push_back(s);
        });
    }

  return strings;
}
""")
