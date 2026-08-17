#include <cstdint>
#include "locationmanager.h"
#include "positionutils.h"

extern "C" {
    void* emane_rs_location_manager_create(uint16_t nemId);
    void emane_rs_location_manager_destroy(void* ptr);
    void* emane_rs_location_manager_get_local_pov(void* ptr);
    void emane_rs_location_manager_set_local_pov(void* ptr, void* pov);
    void* emane_rs_location_manager_get_pov(void* ptr, uint16_t nemId);
    void emane_rs_location_manager_insert_pov(void* ptr, uint16_t nemId, void* pov);
    void* emane_rs_location_manager_get_cache(void* ptr, uint16_t nemId);
    void emane_rs_location_manager_set_cache(void* ptr, uint16_t nemId, void* loc_info);
    void emane_rs_location_manager_clear_cache(void* ptr);
    void emane_rs_location_manager_erase_cache(void* ptr, uint16_t nemId);
    uint64_t emane_rs_location_manager_get_seq(void* ptr);
    uint64_t emane_rs_location_manager_inc_seq(void* ptr);

    void emane_rs_ffi_free_pov(void* ptr) {
        delete static_cast<EMANE::PositionOrientationVelocity*>(ptr);
    }
    void emane_rs_ffi_free_loc_info(void* ptr) {
        delete static_cast<EMANE::LocationInfo*>(ptr);
    }
}

EMANE::LocationManager::LocationManager(NEMId nemId):
  nemId_{nemId},
  rs_ptr_{emane_rs_location_manager_create(nemId)}
{
    auto pov = new PositionOrientationVelocity();
    emane_rs_location_manager_set_local_pov(rs_ptr_, pov);
}

EMANE::LocationManager::~LocationManager()
{
    if (rs_ptr_) {
        emane_rs_location_manager_destroy(rs_ptr_);
    }
}

void EMANE::LocationManager::update(const Events::Locations & locations)
{
  for(const auto & location : locations)
    {
      EMANE::NEMId targetNEMId{location.getNEMId()};

      if(nemId_ == targetNEMId)
        {
          auto pov = static_cast<PositionOrientationVelocity*>(emane_rs_location_manager_get_local_pov(rs_ptr_));
          if(pov->update(location.getPosition(),
                              location.getOrientation(),
                              location.getVelocity()))
            {
              emane_rs_location_manager_clear_cache(rs_ptr_);
            }
        }
      else
        {
          auto iter = emane_rs_location_manager_get_pov(rs_ptr_, targetNEMId);

          if(iter != nullptr)
            {
              auto pov = static_cast<PositionOrientationVelocity*>(iter);
              if(pov->update(location.getPosition(),
                                     location.getOrientation(),
                                     location.getVelocity()))
                {
                  emane_rs_location_manager_erase_cache(rs_ptr_, targetNEMId);
                }
            }
          else
            {
              auto pov = new PositionOrientationVelocity(location.getPosition(),
                                                  location.getOrientation(),
                                                  location.getVelocity());
              emane_rs_location_manager_insert_pov(rs_ptr_, targetNEMId, pov);
            }
        }
    }
}

std::pair<EMANE::LocationInfo,bool> EMANE::LocationManager::getLocationInfo(NEMId remoteNEMId)
{
  auto localPOV = static_cast<PositionOrientationVelocity*>(emane_rs_location_manager_get_local_pov(rs_ptr_));
  if(localPOV && localPOV->isValid())
    {
      auto cacheIter = emane_rs_location_manager_get_cache(rs_ptr_, remoteNEMId);

      if(cacheIter != nullptr)
        {
          return {*static_cast<LocationInfo*>(cacheIter),true};
        }
      else
        {
          auto iter = emane_rs_location_manager_get_pov(rs_ptr_, remoteNEMId);

          if(iter != nullptr)
            {
              auto pov = static_cast<PositionOrientationVelocity*>(iter);
              uint64_t seq = emane_rs_location_manager_inc_seq(rs_ptr_);
              LocationInfo* locationInfo = new LocationInfo(*localPOV, *pov, seq);

              emane_rs_location_manager_set_cache(rs_ptr_, remoteNEMId, locationInfo);

              return {*locationInfo,true};
            }
        }
    }

  return {LocationInfo{},false};
}

const EMANE::PositionOrientationVelocity & EMANE::LocationManager::getLocalPOV() const
{
  auto pov = static_cast<PositionOrientationVelocity*>(emane_rs_location_manager_get_local_pov(rs_ptr_));
  return *pov;
}
