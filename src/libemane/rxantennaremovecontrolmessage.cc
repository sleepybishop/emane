#include <cstdint>
/*
 * Copyright (c) 2020 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "emane/controls/rxantennaremovecontrolmessage.h"

extern "C" {
    void* emane_rs_controls_rx_antenna_remove_create(uint16_t antenna_index);
    void* emane_rs_controls_rx_antenna_remove_clone(const void* ptr);
    void emane_rs_controls_rx_antenna_remove_destroy(void* ptr);
    uint16_t emane_rs_controls_rx_antenna_remove_get_antenna_index(const void* ptr);
}

class EMANE::Controls::RxAntennaRemoveControlMessage::Implementation
{
public:
  Implementation(AntennaIndex antennaIndex):
    rs_ptr_{emane_rs_controls_rx_antenna_remove_create(antennaIndex)}
  {}

  Implementation(const Implementation& other):
    rs_ptr_{emane_rs_controls_rx_antenna_remove_clone(other.rs_ptr_)}
  {}

  ~Implementation()
  {
    if(rs_ptr_) emane_rs_controls_rx_antenna_remove_destroy(rs_ptr_);
  }

  AntennaIndex getAntennaIndex() const
  {
    return emane_rs_controls_rx_antenna_remove_get_antenna_index(rs_ptr_);
  }

private:
  void* rs_ptr_;
};

EMANE::Controls::RxAntennaRemoveControlMessage::
RxAntennaRemoveControlMessage(const RxAntennaRemoveControlMessage & msg):
  ControlMessage{IDENTIFIER},
  pImpl_{new Implementation{*msg.pImpl_}}
{}

EMANE::Controls::RxAntennaRemoveControlMessage::RxAntennaRemoveControlMessage(AntennaIndex antennaIndex):
  ControlMessage{IDENTIFIER},
  pImpl_{new Implementation{antennaIndex}}
{}

EMANE::Controls::RxAntennaRemoveControlMessage::~RxAntennaRemoveControlMessage(){}

EMANE::AntennaIndex
EMANE::Controls::RxAntennaRemoveControlMessage::getAntennaIndex() const
{
  return pImpl_->getAntennaIndex();
}

EMANE::Controls::RxAntennaRemoveControlMessage *
EMANE::Controls::RxAntennaRemoveControlMessage::create(AntennaIndex antennaIndex)
{
  return new RxAntennaRemoveControlMessage{antennaIndex};
}

EMANE::Controls::RxAntennaRemoveControlMessage *
EMANE::Controls::RxAntennaRemoveControlMessage::clone() const
{
  return new RxAntennaRemoveControlMessage{*this};
}
