
#include "wmmmanager.h"
#include "maclayer.h"

extern "C" {
    FfiWmmManager* emane_rs_ieee80211abg_wmmmanager_new();
    void emane_rs_ieee80211abg_wmmmanager_drop(FfiWmmManager* mgr);
    void emane_rs_ieee80211abg_wmmmanager_update_total_activity(FfiWmmManager* mgr, std::uint8_t category, std::uint64_t duration_microseconds);
    void emane_rs_ieee80211abg_wmmmanager_update_local_activity(FfiWmmManager* mgr, std::uint8_t category, std::uint64_t duration_microseconds);
    void emane_rs_ieee80211abg_wmmmanager_set_num_categories(FfiWmmManager* mgr, std::uint8_t num_categories);
    
    struct UtilizationRatioPairC {
        float first;
        float second;
    };
    std::size_t emane_rs_ieee80211abg_wmmmanager_get_utilization_ratios(FfiWmmManager* mgr, std::uint64_t delta_t_microseconds, UtilizationRatioPairC* out_ratios);
}

EMANE::Models::IEEE80211ABG::WMMManager::WMMManager(NEMId id, PlatformServiceProvider * pPlatformService, MACLayer *pMACLayer):
  id_{id},
  pPlatformService_{pPlatformService},
  pMACLayler_{pMACLayer},
  rs_state_{emane_rs_ieee80211abg_wmmmanager_new()}
{
}

EMANE::Models::IEEE80211ABG::WMMManager::~WMMManager()
{
    if (rs_state_) {
        emane_rs_ieee80211abg_wmmmanager_drop(rs_state_);
        rs_state_ = nullptr;
    }
}

void EMANE::Models::IEEE80211ABG::WMMManager::updateTotalActivity(const std::uint8_t u8Category, const Microseconds & durationMicroseconds)
{
    emane_rs_ieee80211abg_wmmmanager_update_total_activity(rs_state_, u8Category, durationMicroseconds.count());
}

void EMANE::Models::IEEE80211ABG::WMMManager::updateLocalActivity(const std::uint8_t u8Category, const Microseconds & durationMicroseconds)
{
    emane_rs_ieee80211abg_wmmmanager_update_local_activity(rs_state_, u8Category, durationMicroseconds.count());
}

void EMANE::Models::IEEE80211ABG::WMMManager::setNumCategories(const std::uint8_t u8NumCategories)
{
    emane_rs_ieee80211abg_wmmmanager_set_num_categories(rs_state_, u8NumCategories);
}

EMANE::Models::IEEE80211ABG::WMMManager::UtilizationRatioVector EMANE::Models::IEEE80211ABG::WMMManager::getUtilizationRatios(const Microseconds & deltaTMicroseconds)
{
    UtilizationRatioPairC arr[256];
    std::size_t len = emane_rs_ieee80211abg_wmmmanager_get_utilization_ratios(rs_state_, deltaTMicroseconds.count(), arr);
    
    UtilizationRatioVector vec;
    vec.reserve(len);
    for (std::size_t i = 0; i < len; ++i) {
        vec.push_back(std::make_pair(arr[i].first, arr[i].second));
    }
    return vec;
}
