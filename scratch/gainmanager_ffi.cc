#include "gainmanager.h"

extern "C" {
    struct EMANE_GainManager_Result {
        double remote_gain;
        double local_gain;
        int status;
        bool cache_hit;
    };

    EMANE_GainManager_Result emane_c_gain_manager_determine_gain(void* gain_mgr_ptr,
                                                                 uint16_t tx_nem,
                                                                 uint16_t tx_ant_index,
                                                                 const void* loc_info_ptr) {
        auto mgr = static_cast<EMANE::GainManager*>(gain_mgr_ptr);
        auto loc = static_cast<const EMANE::LocationInfo*>(loc_info_ptr);
        auto res = mgr->determineGain(tx_nem, tx_ant_index, *loc);
        
        return {
            std::get<0>(res),
            std::get<1>(res),
            static_cast<int>(std::get<2>(res)),
            std::get<3>(res)
        };
    }
}
