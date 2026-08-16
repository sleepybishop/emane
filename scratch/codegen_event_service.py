import sys

cc_code = """/*
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

      const char * device{sDevice.empty() ? nullptr : sDevice.c_str()};

      try
        {
          mcast_.open(eventChannelAddress,true,device,iTTL,loopbackEnable);
        }
      catch(SocketException & exp)
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
            <<exp.what()
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
                                  "unable to send event id:%hu for NEM:%hu\\n",
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

          Utils::VectorIO vectorIO{
            {reinterpret_cast<char *>(&u16Length),sizeof(u16Length)},
              {const_cast<char *>(sSerialization.c_str()),sSerialization.size()}};

          if(mcast_.send(&vectorIO[0],static_cast<int>(vectorIO.size())) == -1)
            {
              LOGGER_STANDARD_LOGGING(*LogServiceSingleton::instance(),
                                      ERROR_LEVEL,
                                      "EventService sendEvent "
                                      "unable to send event id:%hu for NEM:%hu\\n",
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
      if((len = mcast_.recv(buf,sizeof(buf),0)) > 0)
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
"""

rs_code = """use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use std::os::raw::c_char;
use crate::config::VoidPtr;

extern "C" {
    fn emane_c_event_service_user_process_event(p_user: *mut std::ffi::c_void, event_id: u16, data: *const c_char, len: usize);
    // Note: LogServiceSingleton::instance() logger might be accessed from C++ if we really need to log
    // but for now we just omit the debug log inside Rust, or use a C++ callback to log.
    // We will omit the DEBUG_LEVEL logs that were in the loops for simplicity.
    fn emane_c_log_debug(msg: *const c_char);
}

pub struct EventServiceUser {
    pub build_id: u16,
    pub nem_id: u16,
    pub p_user: VoidPtr,
}

pub struct EventServiceRegistry {
    pub users: HashMap<u16, EventServiceUser>,
    pub registrations: HashMap<u16, Vec<u16>>, // event_id -> build_ids
}

fn get_event_service_registry() -> &'static Mutex<EventServiceRegistry> {
    static REGISTRY: OnceLock<Mutex<EventServiceRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(EventServiceRegistry {
        users: HashMap::new(),
        registrations: HashMap::new(),
    }))
}

#[no_mangle]
pub extern "C" fn emane_rs_event_service_register_user(build_id: u16, nem_id: u16, p_user: *mut std::ffi::c_void) {
    let mut reg = get_event_service_registry().lock().unwrap();
    reg.users.insert(build_id, EventServiceUser {
        build_id,
        nem_id,
        p_user: VoidPtr(p_user),
    });
}

#[no_mangle]
pub extern "C" fn emane_rs_event_service_register_event(build_id: u16, event_id: u16) -> bool {
    let mut reg = get_event_service_registry().lock().unwrap();
    if reg.users.contains_key(&build_id) {
        reg.registrations.entry(event_id).or_insert_with(Vec::new).push(build_id);
        true
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_event_service_route_local_event(
    build_id: u16, nem_id: u16, event_id: u16, data: *const c_char, len: usize
) {
    let reg = get_event_service_registry().lock().unwrap();
    if let Some(build_ids) = reg.registrations.get(&event_id) {
        for &registered_build_id in build_ids {
            if let Some(user) = reg.users.get(&registered_build_id) {
                if build_id == 0 || registered_build_id != build_id {
                    if nem_id == 0 || user.nem_id == nem_id {
                        unsafe {
                            emane_c_event_service_user_process_event(user.p_user.0, event_id, data, len);
                        }
                    }
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_event_service_process_event_message(
    nem_id: u16, event_id: u16, data: *const c_char, len: usize, ignore_nem: u16
) {
    let reg = get_event_service_registry().lock().unwrap();
    if let Some(build_ids) = reg.registrations.get(&event_id) {
        for &registered_build_id in build_ids {
            if let Some(user) = reg.users.get(&registered_build_id) {
                if ignore_nem == 0 || ignore_nem != user.nem_id {
                    if nem_id == 0 || user.nem_id == nem_id {
                        unsafe {
                            emane_c_event_service_user_process_event(user.p_user.0, event_id, data, len);
                        }
                    }
                }
            }
        }
    }
}

"""

with open("/home/joe/src/sleepybishop/emane/src/libemane/eventservice.cc", "w") as f:
    f.write(cc_code)
with open("/home/joe/src/sleepybishop/emane/rust/emane-core/src/event_service.rs", "w") as f:
    f.write(rs_code)

