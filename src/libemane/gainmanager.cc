#include <cstdint>
#include "gainmanager.h"
#include "positionutils.h"
#include "locationinfo.h"

extern "C" {
    void* emane_rs_gain_manager_create(uint16_t nemId, uint16_t rxAntennaIndex, void* antennaManager);
    void emane_rs_gain_manager_destroy(void* ptr);
    
    struct EMANE_GainResult {
        double remote_gain;
        double local_gain;
        int status; 
        bool is_cache;
    };
    
    EMANE_GainResult emane_rs_gain_manager_determine_gain(void* ptr, uint16_t tx_nem_id, uint16_t tx_antenna_index, const void* location_info);

    // C FFI trampolines
    double emane_c_location_info_get_distance(const void* loc) {
        return static_cast<const EMANE::LocationInfo*>(loc)->getDistanceMeters();
    }
    bool emane_c_location_info_is_valid(const void* loc) {
        return static_cast<const EMANE::LocationInfo*>(loc)->isValid();
    }
    double emane_c_location_info_get_altitude(const void* loc, bool is_local) {
        auto loc_info = static_cast<const EMANE::LocationInfo*>(loc);
        if (is_local) return loc_info->getLocalPOV().getPosition().getAltitudeMeters();
        return loc_info->getRemotePOV().getPosition().getAltitudeMeters();
    }
    uint64_t emane_c_location_info_get_sequence_number(const void* loc) {
        return static_cast<const EMANE::LocationInfo*>(loc)->getSequenceNumber();
    }

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
    
    bool emane_c_antenna_manager_get_info(
        void* am_ptr, uint16_t nemId, uint16_t index,
        bool* out_is_omni, double* out_fixed_gain,
        bool* out_has_pointing, uint16_t* out_profile_id,
        double* out_pointing_azimuth, double* out_pointing_elevation,
        uint64_t* out_seq)
    {
        auto am = static_cast<EMANE::AntennaManager*>(am_ptr);
        auto info = am->getAntennaInfo(nemId, index);
        if (!info.second) return false;
        
        *out_seq = info.first.u64UpdateSequence_;
        const auto& ant = info.first.antenna_;
        *out_is_omni = ant.isIdealOmni();
        *out_fixed_gain = ant.getFixedGaindBi().first;
        
        auto pointing = ant.getPointing();
        *out_has_pointing = pointing.second;
        if (pointing.second) {
            *out_profile_id = pointing.first.getProfileId();
            *out_pointing_azimuth = pointing.first.getAzimuthDegrees();
            *out_pointing_elevation = pointing.first.getElevationDegrees();
        }
        return true;
    }
}

EMANE::GainManager::AntennaPatternInfo::AntennaPatternInfo():
  pPattern_{}, pBlockage_{}, placement_{}{}

EMANE::GainManager::AntennaPatternInfo::AntennaPatternInfo(AntennaPattern * pPattern,
                                                           AntennaPattern * pBlockage,
                                                           const PositionNEU & placement):
  pPattern_{pPattern}, pBlockage_{pBlockage}, placement_{placement}{}

EMANE::GainManager::GainManager(NEMId nemId, AntennaIndex rxAntennaIndex, AntennaManager & antennaManager):
  id_{nemId}, rxAntennaIndex_{rxAntennaIndex}, antennaManager_{antennaManager},
  rs_ptr_{emane_rs_gain_manager_create(nemId, rxAntennaIndex, &antennaManager)}
{}

EMANE::GainManager::~GainManager() {
    if(rs_ptr_) emane_rs_gain_manager_destroy(rs_ptr_);
}

void EMANE::GainManager::setGainCache(NEMId, const AntennaManager::AntennaInfo &, const LocationInfo &, double, double) {}
std::tuple<double,double,bool> EMANE::GainManager::getGainCache(NEMId, const AntennaManager::AntennaInfo &, const AntennaManager::AntennaInfo &, const LocationInfo &) { return {}; }

EMANE::GainManager::GainInfo EMANE::GainManager::determineGain(NEMId transmitterId,
                                  AntennaIndex txAntennaIndex,
                                  const LocationInfo & locationPairInfo)
{
    auto res = emane_rs_gain_manager_determine_gain(rs_ptr_, transmitterId, txAntennaIndex, &locationPairInfo);
    return std::make_tuple(res.remote_gain, res.local_gain, static_cast<GainStatus>(res.status), res.is_cache);
}
