#include <cstdint>
/*
 * Copyright (c) 2019-2020 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 *
 * * Redistributions of source code must retain the above copyright
 *   notice, this list of conditions and the following disclaimer.
 * * Redistributions in binary form must reproduce the above copyright
 *   notice, this list of conditions and the following disclaimer in
 *   the documentation and/or other materials provided with the
 *   distribution.
 * * Neither the name of Adjacent Link LLC nor the names of its
 *   contributors may be used to endorse or promote products derived
 *   from this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
 * "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
 * LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS
 * FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE
 * COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
 * BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
 * LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
 * CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN
 * ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 */

#include "emane/controls/spectrumfilteraddcontrolmessage.h"

extern "C" {
  void * emane_spectrum_filter_add_control_message_create(std::uint16_t filter_index,
                                                          std::uint8_t antenna_index,
                                                          std::uint64_t frequency_hz,
                                                          std::uint64_t bandwidth_hz,
                                                          std::uint64_t sub_band_bin_size_hz,
                                                          void * filter_match_criterion);

  void * emane_spectrum_filter_add_control_message_clone(const void * msg);
  void emane_spectrum_filter_add_control_message_destroy(void * msg);

  std::uint16_t emane_spectrum_filter_add_control_message_get_filter_index(const void * msg);
  std::uint8_t emane_spectrum_filter_add_control_message_get_antenna_index(const void * msg);
  std::uint64_t emane_spectrum_filter_add_control_message_get_frequency_hz(const void * msg);
  std::uint64_t emane_spectrum_filter_add_control_message_get_bandwidth_hz(const void * msg);
  std::uint64_t emane_spectrum_filter_add_control_message_get_sub_band_bin_size_hz(const void * msg);
  const void * emane_spectrum_filter_add_control_message_get_filter_match_criterion(const void * msg);

  void * emane_filter_match_criterion_clone(const void * ptr) {
    if (!ptr) return nullptr;
    const EMANE::FilterMatchCriterion * criterion = static_cast<const EMANE::FilterMatchCriterion*>(ptr);
    return criterion->clone();
  }

  void emane_filter_match_criterion_destroy(void * ptr) {
    if (ptr) {
      delete static_cast<const EMANE::FilterMatchCriterion*>(ptr);
    }
  }
}

class EMANE::Controls::SpectrumFilterAddControlMessage::Implementation
{
public:
  Implementation(FilterIndex filterIndex,
                 AntennaIndex antennaIndex,
                 std::uint64_t u64FrequencyHz,
                 std::uint64_t u64BandwidthHz,
                 std::uint64_t u64SubBandBinSizeHz,
                 const FilterMatchCriterion * pFilterMatchCriterion):
    rust_obj_{emane_spectrum_filter_add_control_message_create(
        filterIndex,
        antennaIndex,
        u64FrequencyHz,
        u64BandwidthHz,
        u64SubBandBinSizeHz,
        const_cast<void*>(static_cast<const void*>(pFilterMatchCriterion))
    )}
  {}

  Implementation(const Implementation & impl):
    rust_obj_{emane_spectrum_filter_add_control_message_clone(impl.rust_obj_)}
  {}

  ~Implementation()
  {
    emane_spectrum_filter_add_control_message_destroy(rust_obj_);
  }

  FilterIndex getFilterIndex() const
  {
    return emane_spectrum_filter_add_control_message_get_filter_index(rust_obj_);
  }

  AntennaIndex getAntennaIndex() const
  {
    return emane_spectrum_filter_add_control_message_get_antenna_index(rust_obj_);
  }

  std::uint64_t getBandwidthHz() const
  {
    return emane_spectrum_filter_add_control_message_get_bandwidth_hz(rust_obj_);
  }

  std::uint64_t getFrequencyHz() const
  {
    return emane_spectrum_filter_add_control_message_get_frequency_hz(rust_obj_);
  }

  std::size_t getSubBandBinSizeHz() const
  {
    return emane_spectrum_filter_add_control_message_get_sub_band_bin_size_hz(rust_obj_);
  }

  const FilterMatchCriterion * getFilterMatchCriterion() const
  {
    return static_cast<const FilterMatchCriterion *>(
        emane_spectrum_filter_add_control_message_get_filter_match_criterion(rust_obj_));
  }

private:
  void * rust_obj_;
};

EMANE::Controls::SpectrumFilterAddControlMessage::
SpectrumFilterAddControlMessage(const SpectrumFilterAddControlMessage & msg):
  ControlMessage{IDENTIFIER},
  pImpl_{new Implementation{*msg.pImpl_}}
{}

EMANE::Controls::SpectrumFilterAddControlMessage::
SpectrumFilterAddControlMessage(FilterIndex filterIndex,
                                AntennaIndex antennaIndex,
                                std::uint64_t u64FrequencyHz,
                                std::uint64_t u64BandwidthHz,
                                std::uint64_t u64SubBandBinSizeHz,
                                FilterMatchCriterion * pFilterMatchCriterion):
  ControlMessage{IDENTIFIER},
  pImpl_{new Implementation{filterIndex,
                            antennaIndex,
                            u64FrequencyHz,
                            u64BandwidthHz,
                            u64SubBandBinSizeHz,
                            pFilterMatchCriterion}}{}

EMANE::Controls::SpectrumFilterAddControlMessage::~SpectrumFilterAddControlMessage(){}


EMANE::Controls::SpectrumFilterAddControlMessage *
EMANE::Controls::SpectrumFilterAddControlMessage::create(FilterIndex filterIndex,
                                                         AntennaIndex antennaIndex,
                                                         std::uint64_t u64FrequencyHz,
                                                         std::uint64_t u64BandwidthHz,
                                                         std::uint64_t u64SubBandBinSizeHz,
                                                         FilterMatchCriterion * pFilterMatchCriterion)
{
  return new SpectrumFilterAddControlMessage{filterIndex,
                                               antennaIndex,
                                               u64FrequencyHz,
                                               u64BandwidthHz,
                                               u64SubBandBinSizeHz,
                                               pFilterMatchCriterion};

}

EMANE::Controls::SpectrumFilterAddControlMessage *
EMANE::Controls::SpectrumFilterAddControlMessage::create(FilterIndex filterIndex,
                                                         std::uint64_t u64FrequencyHz,
                                                         std::uint64_t u64BandwidthHz,
                                                         std::uint64_t u64SubBandBinSizeHz,
                                                         FilterMatchCriterion * pFilterMatchCriterion)
{
  return new SpectrumFilterAddControlMessage{filterIndex,
                                               DEFAULT_ANTENNA_INDEX,
                                               u64FrequencyHz,
                                               u64BandwidthHz,
                                               u64SubBandBinSizeHz,
                                               pFilterMatchCriterion};

}

std::uint64_t
EMANE::Controls::SpectrumFilterAddControlMessage::getBandwidthHz() const
{
  return pImpl_->getBandwidthHz();
}

std::uint64_t
EMANE::Controls::SpectrumFilterAddControlMessage::getFrequencyHz() const
{
  return pImpl_->getFrequencyHz();
}

const EMANE::FilterMatchCriterion *
EMANE::Controls::SpectrumFilterAddControlMessage::getFilterMatchCriterion() const
{
  return pImpl_->getFilterMatchCriterion();
}

EMANE::FilterIndex
EMANE::Controls::SpectrumFilterAddControlMessage::getFilterIndex() const
{
  return pImpl_->getFilterIndex();
}

EMANE::AntennaIndex
EMANE::Controls::SpectrumFilterAddControlMessage::getAntennaIndex() const
{
  return pImpl_->getAntennaIndex();
}

std::uint64_t
EMANE::Controls::SpectrumFilterAddControlMessage::getSubBandBinSizeHz() const
{
  return pImpl_->getSubBandBinSizeHz();
}

EMANE::Controls::SpectrumFilterAddControlMessage *
EMANE::Controls::SpectrumFilterAddControlMessage::clone() const
{
  return new SpectrumFilterAddControlMessage{*this};
}
