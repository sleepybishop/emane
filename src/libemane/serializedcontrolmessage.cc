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

#include "emane/controls/serializedcontrolmessage.h"

extern "C" {
    void* emane_controls_serialized_create(std::uint16_t id, const void* data, size_t length);
    void emane_controls_serialized_destroy(void* msg);
    void* emane_controls_serialized_clone(const void* msg);
    std::uint16_t emane_controls_serialized_get_id(const void* msg);
    const void* emane_controls_serialized_get_serialization_data(const void* msg);
    size_t emane_controls_serialized_get_serialization_length(const void* msg);
}

class EMANE::Controls::SerializedControlMessage::Implementation
{
public:
  Implementation(ControlMessageId id, const void * pData, size_t length)
  {
    msg_ = emane_controls_serialized_create(id, pData, length);
  }

  Implementation(const Implementation & other)
  {
    msg_ = emane_controls_serialized_clone(other.msg_);
  }
  
  ~Implementation()
  {
    emane_controls_serialized_destroy(msg_);
  }

  ControlMessageId getSerializedId() const
  {
    return emane_controls_serialized_get_id(msg_);
  }

  std::string getSerialization() const
  {
    const char * data = reinterpret_cast<const char *>(emane_controls_serialized_get_serialization_data(msg_));
    size_t len = emane_controls_serialized_get_serialization_length(msg_);
    return std::string(data, len);
  }

private:
  void* msg_;
};

EMANE::Controls::SerializedControlMessage::
SerializedControlMessage(const SerializedControlMessage & msg):
  ControlMessage{IDENTIFIER},
  pImpl_{new Implementation{*msg.pImpl_}}
{}

EMANE::Controls::SerializedControlMessage::SerializedControlMessage(ControlMessageId id,
                                                          const void * pData,
                                                          size_t length):
  ControlMessage(IDENTIFIER),
  pImpl_(new Implementation(id,pData,length))
{}

EMANE::Controls::SerializedControlMessage::~SerializedControlMessage()
{}

EMANE::ControlMessageId EMANE::Controls::SerializedControlMessage::getSerializedId() const
{
  return pImpl_->getSerializedId();
}

  
std::string EMANE::Controls::SerializedControlMessage::getSerialization() const
{
  return pImpl_->getSerialization();
}


EMANE::Controls::SerializedControlMessage * 
EMANE::Controls::SerializedControlMessage::create(ControlMessageId id,
                                                  const void * pData,
                                                  size_t length)
{
  return new SerializedControlMessage(id,pData,length);
}

EMANE::Controls::SerializedControlMessage *
EMANE::Controls::SerializedControlMessage::clone() const
{
  return new SerializedControlMessage{*this};
}
