/*
 * Copyright (c) 2013-2016 - Adjacent Link LLC, Bridgewater, New Jersey
 * Copyright (c) 2008-2012 - DRS CenGen, LLC, Columbia, Maryland
 * All rights reserved.
 * ...
 */

#include "eventservice.h"
#include "emane/registrarexception.h"
#include "eventserviceexception.h"
#include "event.pb.h"
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
      EMANEMessage::Event msg;

      auto pData = msg.mutable_data();

      auto pSerialization = pData->add_serializations();

      pSerialization->set_nemid(nemId);

      pSerialization->set_eventid(eventId);

      pSerialization->set_data(serialization);

      msg.set_uuid(reinterpret_cast<const char *>(uuid_),sizeof(uuid_));

      msg.set_sequencenumber(++u64SequenceNumber_);

      std::string sSerialization;

      if(!msg.SerializeToString(&sSerialization))
        {
          LOGGER_STANDARD_LOGGING(*LogServiceSingleton::instance(),
                                  ERROR_LEVEL,
                                  "EventService sendEvent "
                                  "unable to send event id:%hu for NEM:%hu\n",
                                  eventId,
                                  nemId);
        }
      else
        {
          LOGGER_STANDARD_LOGGING(*LogServiceSingleton::instance(),
                                  DEBUG_LEVEL,
                                  "Event %03hu EMANE::EventService::sendEvent",
                                  eventId);

          std::uint16_t u16Length = HTONS(sSerialization.size());

          std::string buf(reinterpret_cast<char*>(&u16Length), sizeof(u16Length));
          buf.append(sSerialization);

          if(emane_rs_event_service_mcast_send(buf.data(), buf.size(), eventChannelAddress_.str().c_str()) == -1)
            {
              LOGGER_STANDARD_LOGGING(*LogServiceSingleton::instance(),
                                      ERROR_LEVEL,
                                      "EventService sendEvent "
                                      "unable to send event id:%hu for NEM:%hu\n",
                                      eventId,
                                      nemId);
            }
          else
            {
              eventStatisticPublisher_.update(EventStatisticPublisher::Type::TYPE_TX,
                                              uuid_,
                                              eventId);

            }
        }
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

void  EMANE::EventService::process()
{
  std::uint8_t buf[65536];
  ssize_t len = 0;

  LOGGER_STANDARD_LOGGING(*LogServiceSingleton::instance(),
                          DEBUG_LEVEL,
                          "EventService::processEventMessage");

  while(1)
    {
      if((len = emane_rs_event_service_mcast_recv(reinterpret_cast<char*>(buf), sizeof(buf))) > 0)
        {
          LOGGER_STANDARD_LOGGING(*LogServiceSingleton::instance(),
                                  DEBUG_LEVEL,
                                  "EventService packet received len: %zd",
                                  len);

          std::uint16_t * pu16Length{reinterpret_cast<std::uint16_t *>(buf)};

          *pu16Length = NTOHS(*pu16Length);

          len -= sizeof(std::uint16_t);

          EMANEMessage::Event msg;

          if(static_cast<size_t>(len) == *pu16Length &&
             msg.ParseFromArray(&buf[2], *pu16Length))
            {
              uuid_t remoteUUID;
              uuid_copy(remoteUUID,reinterpret_cast<const unsigned char *>(msg.uuid().data()));

              // only process multicast events that were not sourced locally
              if(uuid_compare(uuid_,remoteUUID))
                {
                  for(const auto & serialization : msg.data().serializations())
                    {
                      NEMId nemId{static_cast<NEMId>(serialization.nemid())};

                      emane_rs_event_service_process_event_message(nemId,
                                                                   serialization.eventid(),
                                                                   serialization.data().c_str(),
                                                                   serialization.data().length(),
                                                                   0); // 0 means no ignoreNEM

                      eventStatisticPublisher_.update(EventStatisticPublisher::Type::TYPE_RX,
                                                      remoteUUID,
                                                      serialization.eventid());
                    }
                }
            }
          else
            {
              LOGGER_STANDARD_LOGGING(*LogServiceSingleton::instance(),
                                      ERROR_LEVEL,
                                      "EventService unable to deserialize event");
            }
        }
      else
        {
          LOGGER_STANDARD_LOGGING(*LogServiceSingleton::instance(),
                                  ERROR_LEVEL,
                                  "EventService Packet Receive error");
          break;
        }
    }

}
