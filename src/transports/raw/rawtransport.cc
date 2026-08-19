/*
 * Copyright (c) 2013-2014,2016,2023 - Adjacent Link LLC, Bridgewater,
 *  New Jersey
 * Copyright (c) 2008-2010 - DRS CenGen, LLC, Columbia, Maryland
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
 * * Neither the name of DRS CenGen, LLC nor the names of its
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

#include "rawtransport.h"
#include "emane/downstreampacket.h"
#include "emane/downstreamtransport.h"
#include "emane/configureexception.h"
#include "emane/startexception.h"
#include "emane/utils/parameterconvert.h"
#include "emane/controls/serializedcontrolmessage.h"

#include <sstream>

extern "C" {
    void* emane_rs_raw_transport_new(uint16_t id, void* cpp_obj, void (*cb)(void*, const uint8_t*, size_t));
    void emane_rs_raw_transport_free(void* ptr);
    int32_t emane_rs_raw_transport_start(void* ptr, const char* device_name);
    void emane_rs_raw_transport_stop(void* ptr);
    int32_t emane_rs_raw_transport_process_upstream_packet(void* ptr, const uint8_t* buf, size_t len);

    void RawTransport_sendDownstreamPacket_cb(void* obj, const uint8_t* buf, size_t len) {
        auto rt = static_cast<EMANE::Transports::Raw::RawTransport*>(obj);
        rt->sendDownstreamPacket_cb(buf, len);
    }
}

EMANE::Transports::Raw::RawTransport::RawTransport(NEMId id, PlatformServiceProvider * pPlatformService):
  EthernetTransport(id, pPlatformService),
  pBitPool_{},
  u64BitRate_{},
  rust_obj_{nullptr}
{
    rust_obj_ = emane_rs_raw_transport_new(id, this, RawTransport_sendDownstreamPacket_cb);
}

EMANE::Transports::Raw::RawTransport::~RawTransport()
{
  if (rust_obj_) {
      emane_rs_raw_transport_free(rust_obj_);
      rust_obj_ = nullptr;
  }
  if(pBitPool_) {
      delete pBitPool_;
      pBitPool_ = nullptr;
  }
}

void EMANE::Transports::Raw::RawTransport::initialize(Registrar & registrar)
{
  pBitPool_ = new Utils::BitPool(pPlatformService_, id_);

  auto & configRegistrar = registrar.configurationRegistrar();

  configRegistrar.registerNonNumeric<std::string>("device",
                                                  ConfigurationProperties::NONE,
                                                  {},
                                                  "Device to use as the raw packet entry point.");

  configRegistrar.registerNumeric<std::uint64_t>("bitrate",
                                                 ConfigurationProperties::DEFAULT,
                                                 {0},
                                                 "Transport bitrate in bps.");

  configRegistrar.registerNumeric<bool>("broadcastmodeenable",
                                        ConfigurationProperties::DEFAULT,
                                        {false},
                                        "Broadcast all packets to all NEMs.");

  configRegistrar.registerNumeric<bool>("arpcacheenable",
                                        ConfigurationProperties::DEFAULT,
                                        {true},
                                        "Enable ARP request/reply monitoring.");

  configRegistrar.registerNonNumeric<std::string>("ethernet.type.unknown.priority",
                                                  ConfigurationProperties::NONE,
                                                  {},
                                                  "Defines the emulator priority value.",
                                                  0,
                                                  std::numeric_limits<std::uint16_t>::max(),
                                                  "^(0[xX]){0,1}\\d+:\\d+$");

  configRegistrar.registerNumeric<std::uint8_t>("ethernet.type.arp.priority",
                                                ConfigurationProperties::DEFAULT,
                                                {0},
                                                "Defines the emulator priority value ARP.");
}

void EMANE::Transports::Raw::RawTransport::configure(const ConfigurationUpdate & update)
{
  for(const auto & item : update)
    {
      if(item.first == "bitrate")
        {
          u64BitRate_ =  item.second[0].asUINT64();
        }
      else if(item.first == "device")
        {
          sTargetDevice_ = item.second[0].asString();
        }
      else if(item.first == "broadcastmodeenable")
        {
          bBroadcastMode_ = item.second[0].asBool();
        }
      else if(item.first == "arpcacheenable")
        {
          bArpCacheMode_ = item.second[0].asBool();
        }
      else if(item.first == "ethernet.type.arp.priority")
        {
          u8EtherTypeARPPriority_ = item.second[0].asUINT8();
        }
      else if(item.first == "ethernet.type.unknown.priority")
        {
          for(const auto & value : item.second)
            {
              std::string sEntry{value.asString()};
              auto pos = sEntry.find_first_of(':');
              std::uint16_t u16EtherType = Utils::ParameterConvert(sEntry.substr(0,pos)).toUINT16();
              std::int8_t u8Priority = Utils::ParameterConvert(sEntry.substr(pos+1)).toUINT8();
              unknownEtherTypePriorityMap_[u16EtherType] = u8Priority;
            }
        }
      else
        {
          throw makeException<ConfigureException>("RawTransport: Unexpected configuration item %s", item.first.c_str());
        }
    }
}

void EMANE::Transports::Raw::RawTransport::start()
{
  if (emane_rs_raw_transport_start(rust_obj_, sTargetDevice_.c_str()) < 0) {
      throw StartException("Failed to start rust raw transport (pcap)");
  }
  pBitPool_->setMaxSize(u64BitRate_);
}

void EMANE::Transports::Raw::RawTransport::stop()
{
  emane_rs_raw_transport_stop(rust_obj_);
}

void EMANE::Transports::Raw::RawTransport::destroy() throw() {}

void EMANE::Transports::Raw::RawTransport::processUpstreamPacket(UpstreamPacket & pkt, const ControlMessages & msgs)
{
  if(verifyFrame(pkt.get(), pkt.length()) < 0) {}

  handleUpstreamControl(msgs);

  const PacketInfo & pktInfo{pkt.getPacketInfo()};
  const Utils::EtherHeader * pEtherHeader = (const Utils::EtherHeader*) pkt.get();
  updateArpCache(pEtherHeader, pktInfo.getSource());

  if (emane_rs_raw_transport_process_upstream_packet(rust_obj_, reinterpret_cast<const uint8_t*>(pkt.get()), pkt.length()) < 0) {
      // pcap send failed
  } else {
      const size_t sizePending = pBitPool_->get(pkt.length() * 8);
      if(sizePending != 0) {}
  }
}

void EMANE::Transports::Raw::RawTransport::processUpstreamControl(const ControlMessages & msgs)
{
  handleUpstreamControl(msgs);
}

void EMANE::Transports::Raw::RawTransport::handleUpstreamControl(const ControlMessages & msgs)
{
  for(const auto & pMessage : msgs)
    {
      if(pMessage->getId() == Controls::SerializedControlMessage::IDENTIFIER) {}
    }
}

void EMANE::Transports::Raw::RawTransport::sendDownstreamPacket_cb(const uint8_t* buf, size_t len)
{
  if(verifyFrame(buf, len) < 0) return;

  NEMId nemDestination;
  std::uint8_t dscp{};

  if(parseFrame((const Utils::EtherHeader *)buf, nemDestination, dscp) < 0) return;

  DownstreamPacket pkt(PacketInfo (id_, nemDestination, dscp,Clock::now()), buf, len);
  sendDownstreamPacket(pkt);

  const size_t sizePending = pBitPool_->get(len * 8);
  if(sizePending != 0) {}
}

DECLARE_TRANSPORT(EMANE::Transports::Raw::RawTransport);
