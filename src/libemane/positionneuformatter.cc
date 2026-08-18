#include <cstdint>
/*
 * Copyright (c) 2014 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "positionneuformatter.h"

extern "C" {
    void emane_rs_format_position_neu(double, double, double, void* ctx, void (*add_string)(void*, const char*));
}

EMANE::PositionNEUFormatter::PositionNEUFormatter(const PositionNEU & positionneu):
  positionNEU_(positionneu)
{}
      
EMANE::Strings EMANE::PositionNEUFormatter::operator()() const
{
  Strings strings;
  emane_rs_format_position_neu(
    positionNEU_.getNorthMeters(), positionNEU_.getEastMeters(), positionNEU_.getUpMeters(),
    &strings,
    [](void* ctx, const char* s) {
        static_cast<Strings*>(ctx)->push_back(s);
    }
  );
  return strings;
}
