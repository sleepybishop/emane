#include <cstdint>
/*
 * Copyright (c) 2014 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "emane/events/antennaprofileeventformatter.h"

extern "C" {
    void emane_rs_format_antenna_profile_element(uint16_t nem_id, uint16_t profile_id, double azimuth, double elevation, void* ctx, void (*add_string)(void*, const char*));
}

EMANE::Events::AntennaProfileEventFormatter::
AntennaProfileEventFormatter(const AntennaProfileEvent & event):
  event_(event)
{}
      
EMANE::Strings EMANE::Events::AntennaProfileEventFormatter::operator()() const
{
  Strings strings;

  for(const auto & profile : event_.getAntennaProfiles())
    {
        emane_rs_format_antenna_profile_element(
            profile.getNEMId(),
            profile.getAntennaProfileId(),
            profile.getAntennaAzimuthDegrees(),
            profile.getAntennaElevationDegrees(),
            &strings,
            [](void* ctx, const char* s) {
                static_cast<Strings*>(ctx)->push_back(s);
            }
        );
    }

  return strings;
}
