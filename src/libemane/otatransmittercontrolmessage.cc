#include <cstdint>
/*
 * Copyright (c) 2013-2014,2016 - Adjacent Link LLC, Bridgewater,
 * New Jersey
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

#include "emane/controls/otatransmittercontrolmessage.h"
#include "otatransmitter.pb.h"
#include <vector>

extern "C" {
    struct OtaTransmitterControlMessage;

    OtaTransmitterControlMessage* emane_ota_transmitter_control_message_create(const std::uint16_t* nem_ids, size_t num_nem_ids);
    OtaTransmitterControlMessage* emane_ota_transmitter_control_message_clone(const OtaTransmitterControlMessage* msg);
    void emane_ota_transmitter_control_message_destroy(OtaTransmitterControlMessage* msg);
    void emane_ota_transmitter_control_message_get_transmitters(const OtaTransmitterControlMessage* msg, std::uint16_t** out_nem_ids, size_t* out_num_nem_ids);
    void emane_ota_transmitter_control_message_free_transmitters(std::uint16_t* nem_ids, size_t num_nem_ids);
}

class EMANE::Controls::OTATransmitterControlMessage::Implementation
{
public:
  Implementation(const OTATransmitters & otaTransmitters)
  {
      std::vector<std::uint16_t> vec(otaTransmitters.begin(), otaTransmitters.end());
      msg_ = emane_ota_transmitter_control_message_create(vec.data(), vec.size());
  }

  Implementation(const Implementation & impl)
  {
      msg_ = emane_ota_transmitter_control_message_clone(impl.msg_);
  }

  ~Implementation()
  {
      emane_ota_transmitter_control_message_destroy(msg_);
  }

  const OTATransmitters & getOTATransmitters() const
  {
      if (cachedTransmitters_.empty())
      {
          std::uint16_t* ids = nullptr;
          size_t num = 0;
          emane_ota_transmitter_control_message_get_transmitters(msg_, &ids, &num);
          if (ids)
          {
              for (size_t i = 0; i < num; ++i)
              {
                  cachedTransmitters_.insert(ids[i]);
              }
              emane_ota_transmitter_control_message_free_transmitters(ids, num);
          }
      }
      return cachedTransmitters_;
  }

private:
  OtaTransmitterControlMessage* msg_;
  mutable OTATransmitters cachedTransmitters_;
};

EMANE::Controls::OTATransmitterControlMessage::
OTATransmitterControlMessage(const OTATransmitterControlMessage & msg):
  ControlMessage{IDENTIFIER},
  pImpl_{new Implementation{*msg.pImpl_}}
{}

EMANE::Controls::OTATransmitterControlMessage::~OTATransmitterControlMessage(){}

EMANE::Controls::OTATransmitterControlMessage::
OTATransmitterControlMessage(const OTATransmitters & otaTransmitters):
  ControlMessage{IDENTIFIER},
  pImpl_{new Implementation{otaTransmitters}}{}

const EMANE::Controls::OTATransmitters &
EMANE::Controls::OTATransmitterControlMessage::getOTATransmitters() const
{
  return pImpl_->getOTATransmitters();
}

EMANE::Controls::OTATransmitterControlMessage *
EMANE::Controls::OTATransmitterControlMessage::create(const Serialization & serialization)
{
  OTATransmitters otaTransmitters;
  EMANEMessage::OTATransmitterControlMessage msg;

  if(!msg.ParseFromString(serialization))
    {
      throw SerializationException("unable to deserialize : OTATransmitterControlMessage");
    }

  for(int i = 0; i < msg.nemid_size(); ++i)
    {
      otaTransmitters.insert(msg.nemid(i));
    }

  return new OTATransmitterControlMessage{otaTransmitters};
}

EMANE::Controls::OTATransmitterControlMessage *
EMANE::Controls::OTATransmitterControlMessage::create(const OTATransmitters & otaTransmitters)
{
  return new OTATransmitterControlMessage{otaTransmitters};
}

EMANE::Serialization EMANE::Controls::OTATransmitterControlMessage::serialize() const
{
  Serialization serialization;
  EMANEMessage::OTATransmitterControlMessage msg;

  const OTATransmitters & otaTransmitters = pImpl_->getOTATransmitters();

  OTATransmitters::const_iterator iter = otaTransmitters.begin();
  for(; iter != otaTransmitters.end(); ++iter)
    {
      msg.add_nemid(*iter);
    }

  if(!msg.SerializeToString(&serialization))
    {
      throw SerializationException("unable to serialize OTATransmitterControlMessage");
    }

  return serialization;
}

EMANE::Controls::OTATransmitterControlMessage *
EMANE::Controls::OTATransmitterControlMessage::clone() const
{
  return new OTATransmitterControlMessage{*this};
}
