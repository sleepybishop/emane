with open("src/libemane/spectrumfilterremovecontrolmessage.cc", "w") as f:
    f.write("""#include <cstdint>
/*
 * Copyright (c) 2019-2020 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "emane/controls/spectrumfilterremovecontrolmessage.h"

extern "C" {
    void* emane_rs_controls_spectrum_filter_remove_create(uint16_t filter_index, uint16_t antenna_index);
    void* emane_rs_controls_spectrum_filter_remove_clone(const void* ptr);
    void emane_rs_controls_spectrum_filter_remove_destroy(void* ptr);
    uint16_t emane_rs_controls_spectrum_filter_remove_get_filter_index(const void* ptr);
    uint16_t emane_rs_controls_spectrum_filter_remove_get_antenna_index(const void* ptr);
}

class EMANE::Controls::SpectrumFilterRemoveControlMessage::Implementation
{
public:
  Implementation(FilterIndex filterIndex,
                 AntennaIndex antennaIndex):
    rs_ptr_{emane_rs_controls_spectrum_filter_remove_create(filterIndex, antennaIndex)}
  {}

  Implementation(const Implementation & impl):
    rs_ptr_{emane_rs_controls_spectrum_filter_remove_clone(impl.rs_ptr_)}
  {}

  ~Implementation()
  {
    if(rs_ptr_) emane_rs_controls_spectrum_filter_remove_destroy(rs_ptr_);
  }

  FilterIndex getFilterIndex() const
  {
    return emane_rs_controls_spectrum_filter_remove_get_filter_index(rs_ptr_);
  }

  AntennaIndex getAntennaIndex() const
  {
    return emane_rs_controls_spectrum_filter_remove_get_antenna_index(rs_ptr_);
  }

private:
  void* rs_ptr_;
};

EMANE::Controls::SpectrumFilterRemoveControlMessage::
SpectrumFilterRemoveControlMessage(const SpectrumFilterRemoveControlMessage & msg):
  ControlMessage{IDENTIFIER},
  pImpl_{new Implementation{*msg.pImpl_}}
{}

EMANE::Controls::SpectrumFilterRemoveControlMessage::
SpectrumFilterRemoveControlMessage(FilterIndex filterIndex,
                                   AntennaIndex antennaIndex):
  ControlMessage{IDENTIFIER},
  pImpl_{new Implementation{filterIndex,antennaIndex}}{}

EMANE::Controls::SpectrumFilterRemoveControlMessage::~SpectrumFilterRemoveControlMessage(){}


EMANE::Controls::SpectrumFilterRemoveControlMessage *
EMANE::Controls::SpectrumFilterRemoveControlMessage::create(FilterIndex filterIndex,
                                                            AntennaIndex antennaIndex)
{
  return new SpectrumFilterRemoveControlMessage{filterIndex,antennaIndex};
}

EMANE::Controls::SpectrumFilterRemoveControlMessage *
EMANE::Controls::SpectrumFilterRemoveControlMessage::create(FilterIndex filterIndex)
{
  return new SpectrumFilterRemoveControlMessage{filterIndex,DEFAULT_ANTENNA_INDEX};
}

EMANE::FilterIndex
EMANE::Controls::SpectrumFilterRemoveControlMessage::getFilterIndex() const
{
  return pImpl_->getFilterIndex();
}

EMANE::AntennaIndex
EMANE::Controls::SpectrumFilterRemoveControlMessage::getAntennaIndex() const
{
  return pImpl_->getAntennaIndex();
}

EMANE::Controls::SpectrumFilterRemoveControlMessage *
EMANE::Controls::SpectrumFilterRemoveControlMessage::clone() const
{
  return new SpectrumFilterRemoveControlMessage{*this};
}
""")
