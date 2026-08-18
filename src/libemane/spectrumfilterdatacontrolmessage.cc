/*
 * Copyright (c) 2020 - Adjacent Link LLC, Bridgewater, New Jersey
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

#include "emane/controls/spectrumfilterdatacontrolmessage.h"
#include <cstdint>

extern "C" {
  void* emane_rs_controls_spectrum_filter_data_create(const uint8_t* data, size_t len);
  void* emane_rs_controls_spectrum_filter_data_clone(const void* ptr);
  void emane_rs_controls_spectrum_filter_data_destroy(void* ptr);
  const uint8_t* emane_rs_controls_spectrum_filter_data_get_filter_data(const void* ptr, size_t* len);
}

class EMANE::Controls::SpectrumFilterDataControlMessage::Implementation
{
public:
  Implementation(const FilterData & filterData)
  {
    pImpl_ = emane_rs_controls_spectrum_filter_data_create(
      reinterpret_cast<const uint8_t*>(filterData.data()),
      filterData.size()
    );
    filterDataCached_ = false;
  }

  Implementation(const Implementation & impl)
  {
    pImpl_ = emane_rs_controls_spectrum_filter_data_clone(impl.pImpl_);
    filterDataCached_ = false;
  }

  ~Implementation()
  {
    emane_rs_controls_spectrum_filter_data_destroy(pImpl_);
  }

  const FilterData & getFilterData() const
  {
    if(!filterDataCached_)
      {
        size_t len = 0;
        const uint8_t* data = emane_rs_controls_spectrum_filter_data_get_filter_data(pImpl_, &len);
        if(data && len > 0)
          {
            filterData_.assign(reinterpret_cast<const char*>(data), len);
          }
        filterDataCached_ = true;
      }
    return filterData_;
  }

private:
  void* pImpl_;
  mutable FilterData filterData_;
  mutable bool filterDataCached_;
};

EMANE::Controls::SpectrumFilterDataControlMessage::
SpectrumFilterDataControlMessage(const SpectrumFilterDataControlMessage & msg):
  ControlMessage{IDENTIFIER},
  pImpl_{new Implementation{*msg.pImpl_}}
{}

EMANE::Controls::SpectrumFilterDataControlMessage::
SpectrumFilterDataControlMessage(const FilterData & filterData):
  ControlMessage{IDENTIFIER},
  pImpl_{new Implementation{filterData}}{}

EMANE::Controls::SpectrumFilterDataControlMessage::~SpectrumFilterDataControlMessage(){}


EMANE::Controls::SpectrumFilterDataControlMessage *
EMANE::Controls::SpectrumFilterDataControlMessage::create(const FilterData & filterData)
{
  return new SpectrumFilterDataControlMessage{filterData};
}

const EMANE::FilterData &
EMANE::Controls::SpectrumFilterDataControlMessage::getFilterData() const
{
  return pImpl_->getFilterData();
}

EMANE::Controls::SpectrumFilterDataControlMessage *
EMANE::Controls::SpectrumFilterDataControlMessage::clone() const
{
  return new SpectrumFilterDataControlMessage{*this};
}
