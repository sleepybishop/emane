#include <cstdint>
/*
 * Copyright (c) 2014 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "emane/positionformatter.h"

extern "C" {
    void emane_rs_format_position(double, double, double, void* ctx, void (*add_string)(void*, const char*));
}

EMANE::PositionFormatter::PositionFormatter(const Position & position):
  position_(position)
{}
      
EMANE::Strings EMANE::PositionFormatter::operator()() const
{
  Strings strings;
  emane_rs_format_position(
    position_.getLatitudeDegrees(), position_.getLongitudeDegrees(), position_.getAltitudeMeters(),
    &strings,
    [](void* ctx, const char* s) {
        static_cast<Strings*>(ctx)->push_back(s);
    }
  );
  return strings;
}
