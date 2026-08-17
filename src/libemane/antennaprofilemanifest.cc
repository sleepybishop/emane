#include <cstdint>
#include "antennaprofilemanifest.h"
#include "antennaprofileexception.h"

extern "C" {
    void emane_rs_antenna_profile_load(const char* uri, char* error_buf, size_t error_buf_len);
    bool emane_rs_antenna_profile_get_info(uint16_t id, const void** ant_ptr_out, const void** blk_ptr_out, double* north_out, double* east_out, double* up_out);
}

void EMANE::AntennaProfileManifest::load(const std::string & sAntennaProfileURI)
{
    char error_buf[1024];
    error_buf[0] = 0;
    emane_rs_antenna_profile_load(sAntennaProfileURI.c_str(), error_buf, sizeof(error_buf));
    if (error_buf[0] != 0) {
        throw AntennaProfileException(error_buf);
    }
}

std::pair<std::tuple<EMANE::AntennaPattern *,EMANE::AntennaPattern *,EMANE::PositionNEU>,bool>
EMANE::AntennaProfileManifest::getProfileInfo(AntennaProfileId antennaProfileId) const
{
    const void* ant_ptr = nullptr;
    const void* blk_ptr = nullptr;
    double north = 0.0;
    double east = 0.0;
    double up = 0.0;
    
    if (emane_rs_antenna_profile_get_info(antennaProfileId, &ant_ptr, &blk_ptr, &north, &east, &up)) {
        return {std::make_tuple(
            reinterpret_cast<EMANE::AntennaPattern*>(const_cast<void*>(ant_ptr)),
            reinterpret_cast<EMANE::AntennaPattern*>(const_cast<void*>(blk_ptr)),
            PositionNEU{north, east, up}
        ), true};
    }
    
    return {{}, false};
}
