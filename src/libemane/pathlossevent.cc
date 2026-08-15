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

#include "emane/events/pathlossevent.h"
#include <cstring>

extern "C" {
    struct EmaneRsPathloss {
        uint32_t nem_id;
        float forward_pathloss_db;
        float reverse_pathloss_db;
    };

    uint8_t* emane_rs_pathloss_event_serialize(
        const EmaneRsPathloss* pathlosses,
        size_t num_pathlosses,
        size_t* out_len
    );

    void emane_rs_pathloss_event_free_serialize(uint8_t* ptr, size_t len);

    bool emane_rs_pathloss_event_deserialize(
        const uint8_t* buf,
        size_t len,
        EmaneRsPathloss** out_pathlosses,
        size_t* out_num
    );

    void emane_rs_pathloss_event_free_deserialize(EmaneRsPathloss* ptr, size_t len);
}

class EMANE::Events::PathlossEvent::Implementation
{
public:
  Implementation(const Pathlosses & pathlosses):
    pathlosses_{pathlosses}{}

  const Pathlosses & getPathlosses() const
  {
    return pathlosses_;
  }

private:
  Pathlosses pathlosses_;
};

EMANE::Events::PathlossEvent::PathlossEvent(const Serialization & serialization):
  Event(IDENTIFIER)
{
  EmaneRsPathloss* out_pathlosses = nullptr;
  size_t out_num = 0;

  if(!emane_rs_pathloss_event_deserialize(
      reinterpret_cast<const uint8_t*>(serialization.c_str()),
      serialization.size(),
      &out_pathlosses,
      &out_num))
    {
      throw SerializationException("unable to deserialize PathlossEvent");
    }

  Pathlosses pathlosses;

  if (out_pathlosses && out_num > 0)
    {
      for(size_t i = 0; i < out_num; ++i)
        {
          pathlosses.push_back({static_cast<EMANE::NEMId>(out_pathlosses[i].nem_id),
                out_pathlosses[i].forward_pathloss_db,
                out_pathlosses[i].reverse_pathloss_db});
        }
      emane_rs_pathloss_event_free_deserialize(out_pathlosses, out_num);
    }

  pImpl_.reset(new Implementation{pathlosses});
}

EMANE::Events::PathlossEvent::PathlossEvent(const Pathlosses & pathlosses):
  Event{IDENTIFIER},
  pImpl_{new Implementation{pathlosses}}{}

EMANE::Events::PathlossEvent::PathlossEvent(const PathlossEvent & rhs):
  Event{IDENTIFIER},
  pImpl_{new Implementation{rhs.getPathlosses()}}{}

EMANE::Events::PathlossEvent & EMANE::Events::PathlossEvent::operator=(const PathlossEvent & rhs)
{
  pImpl_.reset(new Implementation{rhs.getPathlosses()});
  return *this;
}

EMANE::Events::PathlossEvent::PathlossEvent(PathlossEvent && rval):
  Event{IDENTIFIER},
  pImpl_{new Implementation{{}}}
{
  rval.pImpl_.swap(pImpl_);
}

EMANE::Events::PathlossEvent & EMANE::Events::PathlossEvent::operator=(PathlossEvent && rval)
{
  rval.pImpl_.swap(pImpl_);
  return *this;
}

EMANE::Events::PathlossEvent::~PathlossEvent(){}

const EMANE::Events::Pathlosses & EMANE::Events::PathlossEvent::getPathlosses() const
{
  return pImpl_->getPathlosses();
}

EMANE::Serialization EMANE::Events::PathlossEvent::serialize() const
{
  const auto & pathlosses = pImpl_->getPathlosses();
  std::vector<EmaneRsPathloss> rs_pathlosses;
  rs_pathlosses.reserve(pathlosses.size());
  
  for(auto & pathloss : pathlosses)
    {
      EmaneRsPathloss rs_pathloss;
      rs_pathloss.nem_id = pathloss.getNEMId();
      rs_pathloss.forward_pathloss_db = pathloss.getForwardPathlossdB();
      rs_pathloss.reverse_pathloss_db = pathloss.getReversePathlossdB();
      rs_pathlosses.push_back(rs_pathloss);
    }

  size_t out_len = 0;
  uint8_t* ptr = emane_rs_pathloss_event_serialize(
      rs_pathlosses.empty() ? nullptr : rs_pathlosses.data(),
      rs_pathlosses.size(),
      &out_len
  );

  if(!ptr)
    {
      throw SerializationException("unable to serialize PathlossEvent");
    }

  Serialization serialization(reinterpret_cast<const char*>(ptr), out_len);
  emane_rs_pathloss_event_free_serialize(ptr, out_len);

  return serialization;
}
