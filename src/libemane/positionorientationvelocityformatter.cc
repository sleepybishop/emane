#include <cstdint>
/*
 * Copyright (c) 2014,2020 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "positionorientationvelocityformatter.h"
#include "emane/positionformatter.h"
#include "emane/orientationformatter.h"
#include "emane/velocityformatter.h"

extern "C" {
    void emane_rs_format_pov_invalid(void* ctx, void (*add_string)(void*, const char*));
    void emane_rs_format_pov_start(void* ctx, void (*add_string)(void*, const char*));
    void emane_rs_format_pov_orientation_none(void* ctx, void (*add_string)(void*, const char*));
    void emane_rs_format_pov_adjusted_orientation_none(void* ctx, void (*add_string)(void*, const char*));
    void emane_rs_format_pov_adjusted_orientation_start(void* ctx, void (*add_string)(void*, const char*));
    void emane_rs_format_pov_velocity_none(void* ctx, void (*add_string)(void*, const char*));
}

static void add_string_to_list(void* ctx, const char* str) {
    auto list = static_cast<EMANE::Strings*>(ctx);
    list->push_back(str);
}

EMANE::PositionOrientationVelocityFormatter::
PositionOrientationVelocityFormatter(const PositionOrientationVelocity & pov):
  pov_(pov)
{}

EMANE::Strings EMANE::PositionOrientationVelocityFormatter::operator()() const
{
  Strings strings;

  if(!pov_.isValid())
    {
      emane_rs_format_pov_invalid(&strings, add_string_to_list);
    }
  else
    {
      emane_rs_format_pov_start(&strings, add_string_to_list);

      strings.splice(strings.end(),PositionFormatter(pov_.getPosition())());

      auto optionalOrientation =  pov_.getOrientation();

      if(optionalOrientation.second)
        {
          strings.splice(strings.end(),OrientationFormatter(optionalOrientation.first)());
        }
      else
        {
          emane_rs_format_pov_orientation_none(&strings, add_string_to_list);
        }
      auto optionalAdjustedOrientation =  pov_.getAdjustedOrientation();

      if(optionalAdjustedOrientation.second)
        {
          emane_rs_format_pov_adjusted_orientation_start(&strings, add_string_to_list);
          strings.splice(strings.end(),OrientationFormatter(optionalAdjustedOrientation.first)());
        }
      else
        {
          emane_rs_format_pov_adjusted_orientation_none(&strings, add_string_to_list);
        }

      auto optionalVelocity = pov_.getVelocity();

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
