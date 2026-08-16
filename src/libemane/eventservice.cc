/*
 * Copyright (c) 2013-2016 - Adjacent Link LLC, Bridgewater, New Jersey
 * Copyright (c) 2008-2012 - DRS CenGen, LLC, Columbia, Maryland
 * All rights reserved.
 * ...
 */

#include "eventservice.h"
#include "emane/registrarexception.h"
#include "eventserviceexception.h"
#include "logservice.h"
#include "socketexception.h"

#include "emane/utils/vectorio.h"
#include "emane/utils/threadutils.h"
#include "emane/net.h"

#include <sstream>

extern "C" {
    void emane_rs_event_service_register_user(uint16_t build_id, uint16_t nem_id, void* p_user);
    bool emane_rs_event_service_register_event(uint16_t build_id, uint16_t event_id);
    void emane_rs_event_service_process_event_message(uint16_t nem_id, uint16_t event_id, const char* data, size_t len, uint16_t ignore_nem);
    void emane_rs_event_service_route_local_event(uint16_t build_id, uint16_t nem_id, uint16_t event_id, const char* data, size_t len);
    bool emane_rs_event_service_mcast_open(const char* addr, const char* device, int ttl, bool loopback);
    int emane_rs_event_service_mcast_send(const char* data, size_t len, const char* addr);
    int emane_rs_event_service_mcast_recv(char* buf, size_t max_len);

    void emane_c_event_service_user_process_event(void* p_user, uint16_t event_id, const char* data, size_t len) {
        auto user = static_cast<EMANE::EventServiceUser*>(p_user);
        EMANE::Serialization serialization(data, len);
        user->processEvent(event_id, serialization);
    }
    void emane_c_event_service_update_stat(int type, const unsigned char* uuid_data, uint16_t event_id) {
        uuid_t uuid;
        uuid_copy(uuid, uuid_data);
        EMANE::EventServiceSingleton::instance()->updateStat(type, uuid, event_id);
    }
}

EMANE::EventService::EventService():
  bOpen_{false},
  eventStatisticPublisher_{"EventChannel"},
  u64SequenceNumber_{}
{
  uuid_clear(uuid_);
}


EMANE::EventService::~EventService()
{
  if(thread_.joinable())
    {
      ThreadUtils::cancel(thread_);

      thread_.join();
    }
}

void EMANE::EventService::registerEvent(BuildId buildId, EventId eventId)
{
  if (!emane_rs_event_service_register_event(buildId, eventId)) {
      throw RegistrarException{"Component not eligible to register for events"};
  }
}

void EMANE::EventService::registerEventServiceUser(BuildId buildId,
                                                   EventServiceUser * pEventServiceUser,
                                                   NEMId nemId)
{
  emane_rs_event_service_register_user(buildId, nemId, pEventServiceUser);
}

void EMANE::EventService::open(const INETAddr & eventChannelAddress,
                               const std::string & sDevice,
                               int iTTL,
                               bool loopbackEnable,
                               const uuid_t & uuid)
{
  if(bOpen_)
    {
      throw EventServiceException("EventService already open");
    }
  else
    {
      uuid_copy(uuid_,uuid);

      bOpen_ = true;

      eventChannelAddress_ = eventChannelAddress;
      const char * device{sDevice.empty() ? nullptr : sDevice.c_str()};

      if (!emane_rs_event_service_mcast_open(
              eventChannelAddress.str().c_str(),
              device,
              iTTL,
              loopbackEnable))
        {
          std::stringstream sstream;
          sstream
            <<"Platform Event Service: Unable to open Event Service socket: '"
            <<eventChannelAddress.str()
            <<"'."
            <<std::endl
            <<std::endl
            <<"Possible reason(s):"
            <<std::endl
            <<" * No Multicast device specified and routing table non deterministic"
            <<std::endl
            <<"   (no multicast route and no default route)."
            <<std::endl
            <<" * Multicast device "
            <<sDevice
            <<" does not exist or is not up."
            <<std::endl
            <<std::ends;

          throw EventServiceException(sstream.str());
        }

      thread_ = std::thread(&EventService::process,this);

      if(ThreadUtils::elevate(thread_))
        {
          LOGGER_STANDARD_LOGGING(*LogServiceSingleton::instance(),
                                  ERROR_LEVEL,
                                  "EventService::open: Unable to set Real Time Priority");
        }
    }
}


void  EMANE::EventService::sendEvent(BuildId buildId,
                                     NEMId nemId,
                                     const Event & event) const
{
  sendEvent(buildId,nemId,event.getEventId(),event.serialize());
}

extern "C" {
  void emane_rs_event_service_send_event_multicast(
      const unsigned char* uuid,
      uint16_t event_id,
      uint16_t nem_id,
      const char* data,
      size_t len,
      uint64_t seq_num,
      const char* addr);

  void emane_rs_event_service_process_loop(const unsigned char* local_uuid);
}


void EMANE::EventService::sendEvent(BuildId buildId,
                                    NEMId nemId,
                                    EventId eventId,
                                    const Serialization & serialization) const
{
  // route locally via rust
  emane_rs_event_service_route_local_event(buildId, nemId, eventId, serialization.c_str(), serialization.length());

  // send the event out via the multicast channel
  if(bOpen_)
    {
      LOGGER_STANDARD_LOGGING(*LogServiceSingleton::instance(),
                              DEBUG_LEVEL,
                              "Event %03hu EMANE::EventService::sendEvent",
                              eventId);

      emane_rs_event_service_send_event_multicast(
          uuid_,
          eventId,
          nemId,
          serialization.c_str(),
          serialization.length(),
          ++u64SequenceNumber_,
          eventChannelAddress_.str().c_str());
    }
  else
    {
      LOGGER_STANDARD_LOGGING(*LogServiceSingleton::instance(),
                              ERROR_LEVEL,
                              "Event %03hu EMANE::EventService::sendEvent, not open, drop",
                              eventId);
    }
}

void EMANE::EventService::processEventMessage(NEMId nemId,
                                              EventId eventId,
                                              const Serialization & serialization,
                                              NEMId ignoreNEM) const
{
  emane_rs_event_service_process_event_message(nemId, eventId, serialization.c_str(), serialization.length(), ignoreNEM);
}

void EMANE::EventService::setStatEventCountRowLimit(size_t rows)
{
  eventStatisticPublisher_.setRowLimit(rows);
}

void EMANE::EventService::updateStat(int type, const unsigned char* uuid_data, EventId eventId) const
{
  uuid_t uuid;
  uuid_copy(uuid, uuid_data);
  eventStatisticPublisher_.update(static_cast<EventStatisticPublisher::Type>(type), uuid, eventId);
}

void  EMANE::EventService::process()
{
  LOGGER_STANDARD_LOGGING(*LogServiceSingleton::instance(),
                          DEBUG_LEVEL,
                          "EventService::process starting Rust loop");

  emane_rs_event_service_process_loop(uuid_);
}
