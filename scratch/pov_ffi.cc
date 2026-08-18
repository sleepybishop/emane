#include "emane/locationinfo.h"
#include "positionutils.h"

extern "C" {
    double emane_c_location_info_get_distance(const void* loc) {
        return static_cast<const EMANE::LocationInfo*>(loc)->getDistanceMeters();
    }
    
    bool emane_c_location_info_is_valid(const void* loc) {
        return static_cast<const EMANE::LocationInfo*>(loc)->isValid();
    }

    struct EMANE_Direction {
        double azimuth;
        double elevation;
        double distance;
    };

    EMANE_Direction emane_c_utils_calculate_direction(
        const void* loc, 
        const void* local_ant_placement, 
        const void* remote_ant_placement) 
    {
        auto loc_info = static_cast<const EMANE::LocationInfo*>(loc);
        auto local_ant = static_cast<const EMANE::PositionNEU*>(local_ant_placement);
        auto remote_ant = static_cast<const EMANE::PositionNEU*>(remote_ant_placement);
        
        auto res = EMANE::Utils::calculateDirection(
            loc_info->getLocalPOV(), *local_ant,
            loc_info->getRemotePOV(), *remote_ant
        );
        return { std::get<0>(res), std::get<1>(res), std::get<2>(res) };
    }

    struct EMANE_LookupAngles {
        double azimuth;
        double elevation;
    };

    EMANE_LookupAngles emane_c_utils_calculate_lookup_angles(
        double dAzReference, double dAzPointing,
        double dElReference, double dElPointing) 
    {
        auto res = EMANE::Utils::calculateLookupAngles(dAzReference, dAzPointing, dElReference, dElPointing);
        return { res.first, res.second };
    }

    bool emane_c_utils_check_horizon(double height1, double height2, double dist) {
        return EMANE::Utils::checkHorizon(height1, height2, dist);
    }
}
