#include <cstdint>
/*
 * Copyright (c) 2014 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "locationinfoformatter.h"
#include "positionorientationvelocityformatter.h"

extern "C" {
    void emane_rs_format_location_info_start(void* ctx, void (*add_string)(void*, const char*));
    void emane_rs_format_location_info_remote(void* ctx, void (*add_string)(void*, const char*));
    void emane_rs_format_location_info_distance(double distance, void* ctx, void (*add_string)(void*, const char*));
}

static void add_string_to_list(void* ctx, const char* str) {
    auto list = static_cast<EMANE::Strings*>(ctx);
    list->push_back(str);
}

EMANE::LocationInfoFormatter::LocationInfoFormatter(const LocationInfo & info):
  info_(info)
{}

EMANE::Strings EMANE::LocationInfoFormatter::operator()() const
{
  Strings strings;

  emane_rs_format_location_info_start(&strings, add_string_to_list);

  strings.splice(strings.end(),PositionOrientationVelocityFormatter(info_.getLocalPOV())());

  emane_rs_format_location_info_remote(&strings, add_string_to_list);

  strings.splice(strings.end(),PositionOrientationVelocityFormatter(info_.getRemotePOV())());

  emane_rs_format_location_info_distance(info_.getDistanceMeters(), &strings, add_string_to_list);

  return strings;
}
