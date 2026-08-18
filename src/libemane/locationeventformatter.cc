#include <cstdint>
/*
 * Copyright (c) 2014 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "emane/events/locationeventformatter.h"
#include "emane/positionformatter.h"
#include "emane/orientationformatter.h"
#include "emane/velocityformatter.h"

extern "C" {
    void emane_rs_format_location_event_nem(uint16_t nem_id, void* ctx, void (*add_string)(void*, const char*));
    void emane_rs_format_pov_orientation_none(void* ctx, void (*add_string)(void*, const char*));
    void emane_rs_format_pov_velocity_none(void* ctx, void (*add_string)(void*, const char*));
}

static void add_string_to_list(void* ctx, const char* str) {
    auto list = static_cast<EMANE::Strings*>(ctx);
    list->push_back(str);
}

EMANE::Events::LocationEventFormatter::
LocationEventFormatter(const LocationEvent & event):
  event_(event)
{}

EMANE::Strings EMANE::Events::LocationEventFormatter::operator()() const
{
  Strings strings;

  for(const auto & location : event_.getLocations())
    {
      emane_rs_format_location_event_nem(location.getNEMId(), &strings, add_string_to_list);

      strings.splice(strings.end(),PositionFormatter(location.getPosition())());

      auto optionalOrientation =  location.getOrientation();

      if(optionalOrientation.second)
        {
          strings.splice(strings.end(),OrientationFormatter(optionalOrientation.first)());
        }
      else
        {
          emane_rs_format_pov_orientation_none(&strings, add_string_to_list);
        }

      auto optionalVelocity = location.getVelocity();

      if(optionalVelocity.second)
        {
          strings.splice(strings.end(),VelocityFormatter(optionalVelocity.first)());
        }
      else
        {
          emane_rs_format_pov_velocity_none(&strings, add_string_to_list);
        }
    }

  return strings;
}
