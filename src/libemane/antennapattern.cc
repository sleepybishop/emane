#include <string>
#include <cstdint>
#include "antennapattern.h"

extern "C" {
    double emane_rs_antenna_pattern_get_gain(const void* pattern, int16_t bearing, int16_t elevation);
}

EMANE::AntennaPattern::AntennaPattern(const std::string & sAntennaPatternURI,
                                      const std::string sSubRootName,
                                      double dMissingValue):
  dMissingValue_{dMissingValue}
{
    // The Rust layer manages creation and loading now.
    // This constructor shouldn't be called directly by the C++ code anymore.
}

double EMANE::AntennaPattern::getGain(std::int16_t iBearing,std::int16_t iElevation) const
{
    return emane_rs_antenna_pattern_get_gain(this, iBearing, iElevation);
}
