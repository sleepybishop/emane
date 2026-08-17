#include <cstdint>
/*
 * Copyright (c) 2020 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "antennamanager.h"
#include "antennaprofilemanifest.h"

extern "C" {
    void* emane_rs_antenna_manager_create();
    void emane_rs_antenna_manager_destroy(void* ptr);
    void emane_rs_antenna_manager_insert_antenna_info(void* ptr, uint16_t nemId, uint16_t index, void* info);
    void* emane_rs_antenna_manager_get_antenna_info(void* ptr, uint16_t nemId, uint16_t index);
    void emane_rs_antenna_manager_remove_antenna_info(void* ptr, uint16_t nemId, uint16_t index);
    
    void emane_rs_antenna_manager_insert_default_pointing(void* ptr, uint16_t nemId, void* pointing);
    void* emane_rs_antenna_manager_get_default_pointing(void* ptr, uint16_t nemId);
    
    uint64_t emane_rs_antenna_manager_inc_seq(void* ptr);
    uint64_t emane_rs_antenna_manager_get_seq(void* ptr);

    void emane_rs_ffi_free_antenna_info(void* ptr) {
        delete static_cast<EMANE::AntennaManager::AntennaInfo*>(ptr);
    }
    void emane_rs_ffi_free_pointing(void* ptr) {
        delete static_cast<EMANE::Antenna::Pointing*>(ptr);
    }
}

EMANE::AntennaManager::AntennaInfo::AntennaInfo():
  u64UpdateSequence_{},
  antenna_{},
  pPattern_{},
  pBlockage_{},
  placement_{}{}

EMANE::AntennaManager::AntennaManager():
  rs_ptr_{emane_rs_antenna_manager_create()}
{}

EMANE::AntennaManager::~AntennaManager()
{
    if (rs_ptr_) {
        emane_rs_antenna_manager_destroy(rs_ptr_);
    }
}

void EMANE::AntennaManager::update(const Events::AntennaProfiles & antennaProfiles)
{
  emane_rs_antenna_manager_inc_seq(rs_ptr_);

  for(const auto & antennaProfile : antennaProfiles)
    {
      auto target = Antenna::createProfileDefined(DEFAULT_ANTENNA_INDEX,
                                                  {antennaProfile.getAntennaProfileId(),
                                                   antennaProfile.getAntennaAzimuthDegrees(),
                                                   antennaProfile.getAntennaElevationDegrees()});

      auto pointing = new Antenna::Pointing(target.getPointing().first);
      emane_rs_antenna_manager_insert_default_pointing(rs_ptr_, antennaProfile.getNEMId(), pointing);

      update(antennaProfile.getNEMId(),target);
    }
}

void EMANE::AntennaManager::update(NEMId nemId, const Antenna & antenna)
{
  uint64_t u64UpdateSequence_ = emane_rs_antenna_manager_inc_seq(rs_ptr_);

  auto target = antenna;
  
  AntennaInfo* iterAntenna = static_cast<AntennaInfo*>(emane_rs_antenna_manager_get_antenna_info(rs_ptr_, nemId, target.getIndex()));

  if(iterAntenna == nullptr)
    {
      iterAntenna = new AntennaInfo();
      iterAntenna->u64UpdateSequence_ = u64UpdateSequence_;
      emane_rs_antenna_manager_insert_antenna_info(rs_ptr_, nemId, target.getIndex(), iterAntenna);
    }

  if(iterAntenna->antenna_ != target)
    {
      if(target.isProfileDefined())
        {
          if(!target.getPointing().second && !target.getIndex())
            {
              auto iterDefaultPointing = static_cast<Antenna::Pointing*>(emane_rs_antenna_manager_get_default_pointing(rs_ptr_, nemId));

              if(iterDefaultPointing != nullptr)
                {
                  target.setPointing(*iterDefaultPointing);
                }
            }

          const auto & currentPointing = iterAntenna->antenna_.getPointing();
          const auto & targetPointing = target.getPointing();

          if(!currentPointing.second ||
             targetPointing.first.getProfileId() != currentPointing.first.getProfileId())
            {
              const auto ret =
                AntennaProfileManifest::instance()->getProfileInfo(targetPointing.first.getProfileId());

              if(ret.second)
                {
                  iterAntenna->pPattern_ = std::get<0>(ret.first);
                  iterAntenna->pBlockage_ = std::get<1>(ret.first);
                  iterAntenna->placement_ = std::get<2>(ret.first);
                }
              else
                {
                  iterAntenna->pPattern_ = nullptr;
                  iterAntenna->pBlockage_ = nullptr;
                  iterAntenna->placement_ = {};
                }
            }
        }
      else
        {
          iterAntenna->pPattern_ = nullptr;
          iterAntenna->pBlockage_ = nullptr;
          iterAntenna->placement_ = {};
        }

      iterAntenna->u64UpdateSequence_ = u64UpdateSequence_;
      iterAntenna->antenna_ = target;
    }
}

std::pair<const EMANE::AntennaManager::AntennaInfo &, bool>
EMANE::AntennaManager::getAntennaInfo(NEMId nemId,
                                      AntennaIndex antennaIndex) const
{
  static AntennaInfo empty{};

  auto iterAntenna = static_cast<AntennaInfo*>(emane_rs_antenna_manager_get_antenna_info(rs_ptr_, nemId, antennaIndex));

  if(iterAntenna != nullptr)
    {
      return {*iterAntenna,true};
    }

  return {empty,false};
}

void EMANE::AntennaManager::remove(NEMId nemId,
                                   AntennaIndex antennaIndex)
{
  emane_rs_antenna_manager_remove_antenna_info(rs_ptr_, nemId, antennaIndex);
}
