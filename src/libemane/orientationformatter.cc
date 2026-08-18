#include <cstdint>
/*
 * Copyright (c) 2014 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "emane/orientationformatter.h"

extern "C" {
    void emane_rs_format_orientation(double, double, double, void* ctx, void (*add_string)(void*, const char*));
}

EMANE::OrientationFormatter::OrientationFormatter(const Orientation & orientation):
  orientation_(orientation)
{}
      
EMANE::Strings EMANE::OrientationFormatter::operator()() const
{
  Strings strings;
  emane_rs_format_orientation(
    orientation_.getPitchDegrees(), orientation_.getRollDegrees(), orientation_.getYawDegrees(),
    &strings,
    [](void* ctx, const char* s) {
        static_cast<Strings*>(ctx)->push_back(s);
    }
  );
  return strings;
}
