#include "emane/locationinfo.h"
#include "positionutils.h"

extern "C" {
    struct EMANE_Direction {
        double azimuth;
        double elevation;
        double distance;
    };

    EMANE_Direction emane_c_utils_calculate_direction_by_neu(
        const void* loc, 
        double local_north, double local_east, double local_up,
        double remote_north, double remote_east, double remote_up) 
    {
        auto loc_info = static_cast<const EMANE::LocationInfo*>(loc);
        EMANE::PositionNEU local_ant(local_north, local_east, local_up);
        EMANE::PositionNEU remote_ant(remote_north, remote_east, remote_up);
        
        auto res = EMANE::Utils::calculateDirection(
            loc_info->getLocalPOV(), local_ant,
            loc_info->getRemotePOV(), remote_ant
        );
        return { std::get<0>(res), std::get<1>(res), std::get<2>(res) };
    }
    
    double emane_c_location_info_get_altitude(const void* loc, bool is_local) {
        auto loc_info = static_cast<const EMANE::LocationInfo*>(loc);
        if (is_local) return loc_info->getLocalPOV().getPosition().getAltitudeMeters();
        return loc_info->getRemotePOV().getPosition().getAltitudeMeters();
    }
}
