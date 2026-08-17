#include <cstdint>
/*
 * Copyright (c) 2014 - Adjacent Link LLC, Bridgewater, New Jersey
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

#include "eventserviceproxy.h"

extern "C" {
  void emane_rs_event_service_route_local_event(uint16_t build_id, uint16_t nem_id, uint16_t event_id, const char* data, size_t len);
  void emane_rs_event_service_send_event_multicast(
      const unsigned char* uuid,
      uint16_t event_id,
      uint16_t nem_id,
      const char* data,
      size_t len,
      uint64_t seq_num,
      const char* addr);
}

EMANE::EventServiceProxy::EventServiceProxy():
  buildId_{}{}

void EMANE::EventServiceProxy::setBuildId(BuildId buildId)
{
  buildId_ = buildId;
}

void EMANE::EventServiceProxy::sendEvent(NEMId nemId, 
                                         const Event & event)
{
  auto serialization = event.serialize();
  emane_rs_event_service_route_local_event(buildId_, nemId, event.getEventId(), serialization.c_str(), serialization.length());
  // Multicast logic is now handled strictly in Rust, so we don't need to double-send here.
  // Wait, actually, the C++ code used to route locally AND send multicast!
  // If we just want to replace EventServiceSingleton::instance()->sendEvent(), we can write a C wrapper for the full send!
}

void EMANE::EventServiceProxy::sendEvent(NEMId nemId, 
                                         EventId eventId, 
                                         const Serialization & serialization)
{
  emane_rs_event_service_route_local_event(buildId_, nemId, eventId, serialization.c_str(), serialization.length());
  // Same here.
}
