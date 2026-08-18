#include <cstdint>
/*
 * Copyright (c) 2014 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "emane/velocityformatter.h"

extern "C" {
    void emane_rs_format_velocity(double, double, double, void* ctx, void (*add_string)(void*, const char*));
}

EMANE::VelocityFormatter::VelocityFormatter(const Velocity & velocity):
  velocity_(velocity)
{}
      
EMANE::Strings EMANE::VelocityFormatter::operator()() const
{
  Strings strings;
  emane_rs_format_velocity(
    velocity_.getAzimuthDegrees(), velocity_.getElevationDegrees(), velocity_.getMagnitudeMetersPerSecond(),
    &strings,
    [](void* ctx, const char* s) {
        static_cast<Strings*>(ctx)->push_back(s);
    }
  );
  return strings;
}
