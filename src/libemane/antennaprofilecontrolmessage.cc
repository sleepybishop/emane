#include <cstdint>
/*
 * Copyright (c) 2013-2014 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "emane/controls/antennaprofilecontrolmessage.h"

extern "C" {
    void* emane_rs_controls_antenna_profile_create(uint16_t id, double azimuth, double elevation);
    void* emane_rs_controls_antenna_profile_clone(const void* ptr);
    void emane_rs_controls_antenna_profile_destroy(void* ptr);
    uint16_t emane_rs_controls_antenna_profile_get_id(const void* ptr);
    double emane_rs_controls_antenna_profile_get_azimuth(const void* ptr);
    double emane_rs_controls_antenna_profile_get_elevation(const void* ptr);
}

class EMANE::Controls::AntennaProfileControlMessage::Implementation
{
public:
  Implementation(AntennaProfileId id,
                 double dAntennaAzimuthDegrees,
                 double dAntennaElevationDegrees):
    rs_ptr_{emane_rs_controls_antenna_profile_create(id, dAntennaAzimuthDegrees, dAntennaElevationDegrees)}
  {}

  Implementation(const Implementation& other):
    rs_ptr_{emane_rs_controls_antenna_profile_clone(other.rs_ptr_)}
  {}

  ~Implementation()
  {
    if(rs_ptr_) emane_rs_controls_antenna_profile_destroy(rs_ptr_);
  }

  AntennaProfileId getAntennaProfileId() const
  {
    return emane_rs_controls_antenna_profile_get_id(rs_ptr_);
  }
  
  double getAntennaAzimuthDegrees() const
  {
    return emane_rs_controls_antenna_profile_get_azimuth(rs_ptr_);
  }
    
  double getAntennaElevationDegrees() const
  {
    return emane_rs_controls_antenna_profile_get_elevation(rs_ptr_);
  }
  
private:
  void* rs_ptr_;
};

EMANE::Controls::AntennaProfileControlMessage::
AntennaProfileControlMessage(const AntennaProfileControlMessage & msg):
  ControlMessage{IDENTIFIER},
  pImpl_{new Implementation{*msg.pImpl_}}
{}

EMANE::Controls::AntennaProfileControlMessage::AntennaProfileControlMessage(AntennaProfileId id,
                                                                  double dAntennaAzimuthDegrees,
                                                                  double dAntennaElevationDegrees):
  ControlMessage{IDENTIFIER},
  pImpl_{new Implementation{id,dAntennaAzimuthDegrees,dAntennaElevationDegrees}}
{}

EMANE::Controls::AntennaProfileControlMessage::~AntennaProfileControlMessage(){}

EMANE::AntennaProfileId EMANE::Controls::AntennaProfileControlMessage::getAntennaProfileId() const
{
  return pImpl_->getAntennaProfileId();
}

double EMANE::Controls::AntennaProfileControlMessage::getAntennaAzimuthDegrees() const
{
  return pImpl_->getAntennaAzimuthDegrees();
}

double EMANE::Controls::AntennaProfileControlMessage::getAntennaElevationDegrees() const
{
  return pImpl_->getAntennaElevationDegrees();
}


EMANE::Controls::AntennaProfileControlMessage *
EMANE::Controls::AntennaProfileControlMessage::create(AntennaProfileId id,
                                            double dAntennaAzimuthDegrees,
                                            double dAntennaElevationDegrees)
{
  return new AntennaProfileControlMessage{id,dAntennaAzimuthDegrees,dAntennaElevationDegrees};
}

EMANE::Controls::AntennaProfileControlMessage *
EMANE::Controls::AntennaProfileControlMessage::clone() const
{
  return new AntennaProfileControlMessage{*this};
}
