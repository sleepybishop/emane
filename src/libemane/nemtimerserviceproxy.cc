#include <cstdint>
/*
 * Copyright (c) 2016 - Adjacent Link LLC, Bridgewater, New Jersey
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

#include "nemtimerserviceproxy.h"

extern "C" {
    size_t emane_rs_timer_schedule(uint64_t expire_micros, uint64_t interval_micros, const void* arg, void* p_user, void (*callback)(size_t, uint64_t, uint64_t, uint64_t, const void*, void*), void (*free_callback)(void*));
    bool emane_rs_timer_cancel(size_t event_id);
}

extern "C" void nem_timer_callback(size_t event_id, uint64_t expire_time_micros, uint64_t schedule_time_micros, uint64_t fire_time_micros, const void* arg, void* pTimerServiceUser) {
    auto proxy = static_cast<EMANE::NEMTimerServiceProxy*>(pTimerServiceUser);
    EMANE::TimePoint expire{std::chrono::duration_cast<EMANE::Clock::duration>(std::chrono::microseconds{expire_time_micros})};
    EMANE::TimePoint schedule{std::chrono::duration_cast<EMANE::Clock::duration>(std::chrono::microseconds{schedule_time_micros})};
    EMANE::TimePoint fire{std::chrono::duration_cast<EMANE::Clock::duration>(std::chrono::microseconds{fire_time_micros})};
    proxy->processTimedEvent(event_id, expire, schedule, fire, arg);
}

struct WrappedCallback {
    EMANE::NEMQueuedLayer* layer;
    std::function<void(const EMANE::TimePoint&, const EMANE::TimePoint&, const EMANE::TimePoint&)> callback;
};

extern "C" void nem_schedule_callback(size_t event_id, uint64_t expire_time_micros, uint64_t schedule_time_micros, uint64_t fire_time_micros, const void* arg, void* p_user) {
    auto wrapped = static_cast<WrappedCallback*>(p_user);
    EMANE::TimePoint expire{std::chrono::duration_cast<EMANE::Clock::duration>(std::chrono::microseconds{expire_time_micros})};
    EMANE::TimePoint schedule{std::chrono::duration_cast<EMANE::Clock::duration>(std::chrono::microseconds{schedule_time_micros})};
    EMANE::TimePoint fire{std::chrono::duration_cast<EMANE::Clock::duration>(std::chrono::microseconds{fire_time_micros})};
    wrapped->layer->processTimer(wrapped->callback, expire, schedule, fire);
}

extern "C" void nem_schedule_free(void* p_user) {
    delete static_cast<WrappedCallback*>(p_user);
}

EMANE::NEMTimerServiceProxy::NEMTimerServiceProxy():
  pNEMQueuedLayer_{}{}

EMANE::NEMTimerServiceProxy::~NEMTimerServiceProxy()
{}

void EMANE::NEMTimerServiceProxy::setNEMLayer(NEMQueuedLayer * pNEMQueuedLayer)
{
  pNEMQueuedLayer_ = pNEMQueuedLayer;
}

bool EMANE::NEMTimerServiceProxy::cancelTimedEvent(TimerEventId eventId)
{
  return emane_rs_timer_cancel(eventId);
}

EMANE::TimerEventId EMANE::NEMTimerServiceProxy::scheduleTimedEvent(const TimePoint & timeout,
                                                                    const void *arg,
                                                                    const Duration & interval)
{
  uint64_t expire_micros = std::chrono::duration_cast<std::chrono::microseconds>(timeout.time_since_epoch()).count();
  uint64_t interval_micros = std::chrono::duration_cast<std::chrono::microseconds>(interval).count();
  return emane_rs_timer_schedule(expire_micros, interval_micros, arg, this, nem_timer_callback, nullptr);
}

void EMANE::NEMTimerServiceProxy::processTimedEvent(TimerEventId eventId,
                                                    const TimePoint & expireTime,
                                                    const TimePoint & scheduleTime,
                                                    const TimePoint & fireTime,
                                                    const void * arg)
{
  pNEMQueuedLayer_->processTimedEvent(eventId,
                                      expireTime,
                                      scheduleTime,
                                      fireTime,
                                      arg);
}

EMANE::TimerEventId EMANE::NEMTimerServiceProxy::schedule_i(TimerCallback callback,
                                                            const TimePoint & timePoint,
                                                            const Duration & interval)
{
  auto wrapped = new WrappedCallback{pNEMQueuedLayer_, callback};
  uint64_t expire_micros = std::chrono::duration_cast<std::chrono::microseconds>(timePoint.time_since_epoch()).count();
  uint64_t interval_micros = std::chrono::duration_cast<std::chrono::microseconds>(interval).count();
  return emane_rs_timer_schedule(expire_micros, interval_micros, nullptr, wrapped, nem_schedule_callback, nem_schedule_free);
}
