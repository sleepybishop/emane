def process(path, type_name, c_func, args_call):
    with open(path, "r") as f:
        content = f.read()
    
    header = f'''#include <cstdint>
/*
 * Copyright (c) 2014 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "emane/{type_name.lower()}formatter.h"

extern "C" {{
    void {c_func}({args_call}, void* ctx, void (*add_string)(void*, const char*));
}}

EMANE::{type_name}Formatter::{type_name}Formatter(const {type_name} & {type_name.lower()}):
  {type_name.lower()}_({type_name.lower()})
{{}}
      
EMANE::Strings EMANE::{type_name}Formatter::operator()() const
{{
  Strings strings;
  {c_func}(
'''
    if type_name == "Orientation":
        args = "orientation_.getPitchDegrees(), orientation_.getRollDegrees(), orientation_.getYawDegrees()"
    elif type_name == "Position":
        args = "position_.getLatitudeDegrees(), position_.getLongitudeDegrees(), position_.getAltitudeMeters()"
    elif type_name == "PositionNEU":
        args = "positionNEU_.getNorthMeters(), positionNEU_.getEastMeters(), positionNEU_.getUpMeters()"
        header = header.replace(f'#include "emane/{type_name.lower()}formatter.h"', '#include "positionneuformatter.h"')
    elif type_name == "Velocity":
        args = "velocity_.getAzimuthDegrees(), velocity_.getElevationDegrees(), velocity_.getMagnitudeMetersPerSecond()"

    header += f'''    {args},
    &strings,
    [](void* ctx, const char* s) {{
        static_cast<Strings*>(ctx)->push_back(s);
    }}
  );
  return strings;
}}
'''
    with open(path, "w") as f:
        f.write(header)

process("src/libemane/orientationformatter.cc", "Orientation", "emane_rs_format_orientation", "double, double, double")
process("src/libemane/positionformatter.cc", "Position", "emane_rs_format_position", "double, double, double")
process("src/libemane/positionneuformatter.cc", "PositionNEU", "emane_rs_format_position_neu", "double, double, double")
process("src/libemane/velocityformatter.cc", "Velocity", "emane_rs_format_velocity", "double, double, double")
