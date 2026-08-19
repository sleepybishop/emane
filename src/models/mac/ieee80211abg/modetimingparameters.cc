
#include "modetimingparameters.h"
#include "macconfig.h"
#include "macstatistics.h"
#include "maclayer.h"
#include "emane/packetinfo.h"
#include "emane/constants.h"
#include "utils.h"

extern "C" {
    void* emane_ieee80211abg_modetimingparameters_new(const void* config_ptr);
    void emane_ieee80211abg_modetimingparameters_free(void* ptr);
    uint64_t emane_ieee80211abg_modetimingparameters_getSlotSizeMicroseconds(void* ptr);
    uint64_t emane_ieee80211abg_modetimingparameters_getOverheadMicroseconds(void* ptr, uint8_t category);
    uint64_t emane_ieee80211abg_modetimingparameters_getDeferIntervalMicroseconds(void* ptr, uint8_t category);
    uint64_t emane_ieee80211abg_modetimingparameters_getMessageDurationMicroseconds(void* ptr, uint8_t category, size_t len);
    uint64_t emane_ieee80211abg_modetimingparameters_getBroadcastMessageDurationMicroseconds(void* ptr, size_t len);
    uint64_t emane_ieee80211abg_modetimingparameters_getUnicastMessageDurationMicroseconds(void* ptr, size_t len);
    uint64_t emane_ieee80211abg_modetimingparameters_getCtsMessageDurationMicroseconds(void* ptr);
    uint64_t emane_ieee80211abg_modetimingparameters_getRtsMessageDurationMicroseconds(void* ptr);
}

EMANE::Models::IEEE80211ABG::ModeTimingParameters::ModeTimingParameters(const MACConfig & macConfig):
  macConfig_(macConfig)
{
    rs_state_ = emane_ieee80211abg_modetimingparameters_new(&macConfig_);
}

EMANE::Models::IEEE80211ABG::ModeTimingParameters::~ModeTimingParameters()
{
    emane_ieee80211abg_modetimingparameters_free(rs_state_);
}

EMANE::Microseconds
EMANE::Models::IEEE80211ABG::ModeTimingParameters::getSlotSizeMicroseconds() const
{
    return EMANE::Microseconds{emane_ieee80211abg_modetimingparameters_getSlotSizeMicroseconds(rs_state_)};
}

EMANE::Microseconds 
EMANE::Models::IEEE80211ABG::ModeTimingParameters::getOverheadMicroseconds(std::uint8_t u8Category) const
{
    return EMANE::Microseconds{emane_ieee80211abg_modetimingparameters_getOverheadMicroseconds(rs_state_, u8Category)};
}

EMANE::Microseconds 
EMANE::Models::IEEE80211ABG::ModeTimingParameters::getDeferIntervalMicroseconds(std::uint8_t u8Category) const
{
    return EMANE::Microseconds{emane_ieee80211abg_modetimingparameters_getDeferIntervalMicroseconds(rs_state_, u8Category)};
}

EMANE::Microseconds 
EMANE::Models::IEEE80211ABG::ModeTimingParameters::getMessageDurationMicroseconds(std::uint8_t u8Category, size_t length) const
{
    return EMANE::Microseconds{emane_ieee80211abg_modetimingparameters_getMessageDurationMicroseconds(rs_state_, u8Category, length)};
}

EMANE::Microseconds
EMANE::Models::IEEE80211ABG::ModeTimingParameters::getBroadcastMessageDurationMicroseconds(size_t length) const
{
    return EMANE::Microseconds{emane_ieee80211abg_modetimingparameters_getBroadcastMessageDurationMicroseconds(rs_state_, length)};
}

EMANE::Microseconds
EMANE::Models::IEEE80211ABG::ModeTimingParameters::getUnicastMessageDurationMicroseconds(size_t length) const
{
    return EMANE::Microseconds{emane_ieee80211abg_modetimingparameters_getUnicastMessageDurationMicroseconds(rs_state_, length)};
}

EMANE::Microseconds 
EMANE::Models::IEEE80211ABG::ModeTimingParameters::getCtsMessageDurationMicroseconds() const
{
    return EMANE::Microseconds{emane_ieee80211abg_modetimingparameters_getCtsMessageDurationMicroseconds(rs_state_)};
}

EMANE::Microseconds 
EMANE::Models::IEEE80211ABG::ModeTimingParameters::getRtsMessageDurationMicroseconds() const
{
    return EMANE::Microseconds{emane_ieee80211abg_modetimingparameters_getRtsMessageDurationMicroseconds(rs_state_)};
}

// These are left in C++ because they use TimePoint or CWRatioVector
bool EMANE::Models::IEEE80211ABG::ModeTimingParameters::packetTimedOut(const Microseconds & txOpMicroseconds, const TimePoint & txTime) const
{
  const TimePoint now = Clock::now();

  return (txTime > now) ? false : (std::chrono::duration_cast<Microseconds>(now - txTime) > txOpMicroseconds);
}

EMANE::TimePoint EMANE::Models::IEEE80211ABG::ModeTimingParameters::getSotTime() const
{
  return Clock::now();
}

int EMANE::Models::IEEE80211ABG::ModeTimingParameters::getContentionWindow(std::uint8_t u8Category, std::uint8_t numRetries) const
{
  const auto ratios = macConfig_.getCWMinRatioVector(u8Category);
  int cwMin = macConfig_.getCWMin(u8Category);
  
  if(!ratios.empty() && numRetries < ratios.size())
    {
      cwMin = ratios[numRetries] * macConfig_.getCWMin(u8Category);
    }
  else if(numRetries)
    {
      cwMin = macConfig_.getCWMax(u8Category);
    }

  return cwMin;
}

