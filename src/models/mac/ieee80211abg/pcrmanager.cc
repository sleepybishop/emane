
#include "pcrmanager.h"
#include <sstream>

extern "C" {
    FfiPcrManager* emane_rs_ieee80211abg_pcrmanager_new();
    void emane_rs_ieee80211abg_pcrmanager_drop(FfiPcrManager* mgr);
    bool emane_rs_ieee80211abg_pcrmanager_load(FfiPcrManager* mgr, const char* uri);
    float emane_rs_ieee80211abg_pcrmanager_get_pcr(FfiPcrManager* mgr, float sinr, std::size_t packet_len, std::uint16_t data_rate_index);
}

EMANE::Models::IEEE80211ABG::PCRManager::PCRManager(EMANE::NEMId id, EMANE::PlatformServiceProvider * pPlatformService):
id_{id}, 
pPlatformService_{pPlatformService}, 
rs_state_{emane_rs_ieee80211abg_pcrmanager_new()}
{ }

EMANE::Models::IEEE80211ABG::PCRManager::~PCRManager()
{
    if (rs_state_) {
        emane_rs_ieee80211abg_pcrmanager_drop(rs_state_);
        rs_state_ = nullptr;
    }
}

void EMANE::Models::IEEE80211ABG::PCRManager::load(const std::string & uri)
{
    if (!emane_rs_ieee80211abg_pcrmanager_load(rs_state_, uri.c_str())) {
        std::stringstream excString;
        excString << "IEEE80211ABG::PCRManager::load: failed to load " << uri << std::ends;
        throw EMANE::ConfigurationException(excString.str());
    }
}

float EMANE::Models::IEEE80211ABG::PCRManager::getPCR(float fSINR, size_t packetLen, std::uint16_t u16DataRateIndex)
{
    return emane_rs_ieee80211abg_pcrmanager_get_pcr(rs_state_, fSINR, packetLen, u16DataRateIndex);
}
