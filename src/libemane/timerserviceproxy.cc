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

#include "timerserviceproxy.h"
#include "logservice.h"

extern "C" {
    size_t emane_rs_timer_schedule(uint64_t expire_micros, uint64_t interval_micros, const void* arg, void* p_user, void (*callback)(size_t, uint64_t, uint64_t, uint64_t, const void*, void*), void (*free_callback)(void*));
    bool emane_rs_timer_cancel(size_t event_id);
}

extern "C" void timer_callback(size_t event_id, uint64_t expire_time_micros, uint64_t schedule_time_micros, uint64_t fire_time_micros, const void* arg, void* pTimerServiceUser) {
    auto proxy = static_cast<EMANE::TimerServiceProxy*>(pTimerServiceUser);
    EMANE::TimePoint expire{std::chrono::duration_cast<EMANE::Clock::duration>(std::chrono::microseconds{expire_time_micros})};
    EMANE::TimePoint schedule{std::chrono::duration_cast<EMANE::Clock::duration>(std::chrono::microseconds{schedule_time_micros})};
    EMANE::TimePoint fire{std::chrono::duration_cast<EMANE::Clock::duration>(std::chrono::microseconds{fire_time_micros})};
    proxy->processTimedEvent(event_id, expire, schedule, fire, arg);
}

EMANE::TimerServiceProxy::TimerServiceProxy():
  pTimerServiceUser_{}{}

EMANE::TimerServiceProxy::~TimerServiceProxy()
{}

void EMANE::TimerServiceProxy::setTimerServiceUser(TimerServiceUser * pTimerServiceUser)
{
  pTimerServiceUser_ = pTimerServiceUser;
}

bool EMANE::TimerServiceProxy::cancelTimedEvent(TimerEventId eventId)
{
  return emane_rs_timer_cancel(eventId);
}

EMANE::TimerEventId EMANE::TimerServiceProxy::scheduleTimedEvent(const TimePoint & timeout,
                                                                 const void *arg,
                                                                 const Duration & interval)
{
  uint64_t expire_micros = std::chrono::duration_cast<std::chrono::microseconds>(timeout.time_since_epoch()).count();
  uint64_t interval_micros = std::chrono::duration_cast<std::chrono::microseconds>(interval).count();
  return emane_rs_timer_schedule(expire_micros, interval_micros, arg, this, timer_callback, nullptr);
}

void EMANE::TimerServiceProxy::processTimedEvent(TimerEventId eventId,
                                                 const TimePoint & expireTime,
                                                 const TimePoint & scheduleTime,
                                                 const TimePoint & fireTime,
                                                 const void * arg)
{
  pTimerServiceUser_->processTimedEvent(eventId,
                                        expireTime,
                                        scheduleTime,
                                        fireTime,
                                        arg);
}

EMANE::TimerEventId EMANE::TimerServiceProxy::schedule_i(TimerCallback,
                                                         const TimePoint &,
                                                         const Duration &)
{
  LOGGER_STANDARD_LOGGING(*LogServiceSingleton::instance(),
                          ERROR_LEVEL,
                          "TimerService schedule not available to component");

  return 0;
}
