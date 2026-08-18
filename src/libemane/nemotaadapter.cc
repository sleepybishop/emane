#include <cstdint>
/*
 * Copyright (c) 2013-2014,2016 - Adjacent Link LLC, Bridgewater, New Jersey
 * Copyright (c) 2008-2009 - DRS CenGen, LLC, Columbia, Maryland
 * All rights reserved.
 */

#include "nemotaadapter.h"
#include "logservice.h"

#include "emane/utils/threadutils.h"
#include "emane/upstreamtransport.h"

extern "C" {
    void emane_c_ota_manager_register_user(std::uint16_t, void*);
    void emane_c_ota_manager_unregister_user(std::uint16_t);
    void emane_c_ota_manager_send_packet_cpp(std::uint16_t, const void*, const void*);

    void* emane_rs_nem_ota_adapter_new(std::uint16_t id);
    void emane_rs_nem_ota_adapter_free(void* ptr);
    void emane_rs_nem_ota_adapter_open(void* ptr);
    void emane_rs_nem_ota_adapter_close(void* ptr);
    void emane_rs_nem_ota_adapter_push(void* ptr, void* item);
    void emane_c_nem_ota_adapter_process_item(std::uint16_t id, void* item);
}

void emane_c_nem_ota_adapter_process_item(std::uint16_t id, void* item) {
    auto pair = static_cast<std::pair<EMANE::DownstreamPacket, EMANE::ControlMessages>*>(item);
    try {
        emane_c_ota_manager_send_packet_cpp(id, &pair->first, &pair->second);
    } catch(std::exception & exp) {
        LOGGER_STANDARD_LOGGING(*EMANE::LogServiceSingleton::instance(), EMANE::ERROR_LEVEL, "NEMOTAAdapter::processPacketQueue Exception caught: %s", exp.what());
    } catch(...) {
        LOGGER_STANDARD_LOGGING(*EMANE::LogServiceSingleton::instance(), EMANE::ERROR_LEVEL, "NEMOTAAdapter::processPacketQueue Exception caught");
    }
    delete pair;
}

EMANE::NEMOTAAdapter::NEMOTAAdapter(NEMId id):
  id_{id},
  rs_state_{emane_rs_nem_ota_adapter_new(id)}
{}

EMANE::NEMOTAAdapter::~NEMOTAAdapter()
{
  emane_rs_nem_ota_adapter_free(rs_state_);
}

void EMANE::NEMOTAAdapter::open()
{
  emane_c_ota_manager_register_user(id_, this);
  emane_rs_nem_ota_adapter_open(rs_state_);
}

void EMANE::NEMOTAAdapter::close()
{
  try {
      emane_c_ota_manager_unregister_user(id_);
  } catch(...) {}

  emane_rs_nem_ota_adapter_close(rs_state_);
}

void EMANE::NEMOTAAdapter::processOTAPacket(UpstreamPacket & pkt, const ControlMessages & msgs)
{
  sendUpstreamPacket(pkt, msgs);
}

void EMANE::NEMOTAAdapter::processDownstreamPacket(DownstreamPacket & pkt,
                                                   const ControlMessages & msgs)
{
  auto item = new std::pair<DownstreamPacket, ControlMessages>(pkt, msgs);
  emane_rs_nem_ota_adapter_push(rs_state_, item);
}

void EMANE::NEMOTAAdapter::processPacketQueue() {}
