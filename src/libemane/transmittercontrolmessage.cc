#include <cstdint>
/*
 * Copyright (c) 2013-2014 - Adjacent Link LLC, Bridgewater, New Jersey
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

#include "emane/controls/transmittercontrolmessage.h"

extern "C" {
    void* emane_rs_controls_transmitter_create();
    void emane_rs_controls_transmitter_add(void* ptr, uint16_t nem_id, double power_dbm);
    void* emane_rs_controls_transmitter_clone(const void* ptr);
    void emane_rs_controls_transmitter_destroy(void* ptr);
}

class EMANE::Controls::TransmitterControlMessage::Implementation
{
public:
  Implementation(const Transmitters & transmitters):
    transmitters_{transmitters}
  {
      init_rust();
  }
  
  Implementation(const Implementation& other) :
    transmitters_{other.transmitters_}
  {
      pRsMsg_ = emane_rs_controls_transmitter_clone(other.pRsMsg_);
  }

  ~Implementation() {
      emane_rs_controls_transmitter_destroy(pRsMsg_);
  }

  const Transmitters & getTransmitters() const
  {
    return transmitters_;
  }
  
  Implementation* clone() const {
      return new Implementation(*this);
  }

private:
  void init_rust() {
      pRsMsg_ = emane_rs_controls_transmitter_create();
      for(const auto& t : transmitters_) {
          emane_rs_controls_transmitter_add(pRsMsg_, t.getNEMId(), t.getPowerdBm());
      }
  }

  void* pRsMsg_;
  const Transmitters transmitters_;
};

EMANE::Controls::TransmitterControlMessage::
TransmitterControlMessage(const TransmitterControlMessage & msg):
  ControlMessage{IDENTIFIER},
  pImpl_{msg.pImpl_->clone()}
{}

EMANE::Controls::TransmitterControlMessage::TransmitterControlMessage(const Transmitters & transmitters):
  ControlMessage{IDENTIFIER},
  pImpl_{new Implementation{transmitters}}{}


EMANE::Controls::TransmitterControlMessage::~TransmitterControlMessage()
{}

const EMANE::Transmitters &
EMANE::Controls::TransmitterControlMessage::getTransmitters() const
{
  return pImpl_->getTransmitters();
}

EMANE::Controls::TransmitterControlMessage *
EMANE::Controls::TransmitterControlMessage::create(const Transmitters & transmitters)
{
  return new TransmitterControlMessage{transmitters};
}

EMANE::Controls::TransmitterControlMessage *
EMANE::Controls::TransmitterControlMessage::clone() const
{
  return new TransmitterControlMessage{*this};
}
