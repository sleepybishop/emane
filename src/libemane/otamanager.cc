
#include <cstdint>
#include <cstddef>

extern "C" {
    void emane_rs_event_service_register_user(uint16_t build_id, uint16_t nem_id, void* p_user);
    void emane_rs_event_service_process_event_message(uint16_t nem_id, uint16_t event_id, const char* data, size_t len, uint16_t ignore_nem);
    bool emane_rs_event_service_mcast_open(const char* addr, const char* device, int ttl, bool loopback, const unsigned char* uuid);
}

/*
 * Copyright (c) 2013-2017 - Adjacent Link LLC, Bridgewater, New Jersey
 * Copyright (c) 2008-2012 - DRS CenGen, LLC, Columbia, Maryland
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

#include "otamanager.h"
#include "otauser.h"
#include "logservice.h"
#include "controlmessageserializer.h"
#include "otaexception.h"
#include "otaheader.pb.h"
#include "event.pb.h"
#include "socketexception.h"

#include "emane/net.h"
#include "emane/utils/threadutils.h"
#include "emane/controls/otatransmittercontrolmessage.h"
#include "emane/controls/serializedcontrolmessage.h"

#include <sstream>
#include <algorithm>
#include <uuid.h>

namespace
{
  struct PartInfo
  {
    std::uint8_t u8More_; /**< More parts to follow*/
    std::uint32_t u32Offset_; /**< Offset of payload */
    std::uint32_t u32Size_;     /**< Part size */
  } __attribute__((packed));

  std::vector<uint8_t> bufferFromVectorIO(size_t size,
                                          size_t & index,
                                          size_t & offset,
                                          const EMANE::Utils::VectorIO & vectorIO)
  {
    std::vector<uint8_t> buf{};

    size_t targetBytes{size};

    while(targetBytes)
      {
        size_t available{vectorIO[index].iov_len - offset};

        if(available)
          {
            if(available >= targetBytes)
              {
                buf.insert(buf.end(),
                           &reinterpret_cast<uint8_t *>(vectorIO[index].iov_base)[offset],
                           &reinterpret_cast<uint8_t *>(vectorIO[index].iov_base)[offset+targetBytes]);

                offset += targetBytes;

                targetBytes = 0;
              }
            else
              {
                buf.insert(buf.end(),
                           &reinterpret_cast<uint8_t *>(vectorIO[index].iov_base)[offset],
                           &reinterpret_cast<uint8_t *>(vectorIO[index].iov_base)[offset] + available);

                targetBytes -= available;

                ++index;

                offset = 0;
              }
          }
      }

    return buf;
  }
}

EMANE::OTAManager::OTAManager():
  bOpen_(false),
  otaMTU_{},
  eventStatisticPublisher_{"OTAChannel"},
  u64SequenceNumber_{},
  lastPartCheckTime_{}
{
  uuid_clear(uuid_);
}

EMANE::OTAManager::~OTAManager()
{
  if(bOpen_)
    {
      ThreadUtils::cancel(thread_);

      thread_.join();
    }
}

void EMANE::OTAManager::setStatPacketCountRowLimit(size_t rows)
{
  otaStatisticPublisher_.setRowLimit(rows);
}

void EMANE::OTAManager::setStatEventCountRowLimit(size_t rows)
{
  eventStatisticPublisher_.setRowLimit(rows);
}

void EMANE::OTAManager::sendOTAPacket(NEMId id,
                                      const DownstreamPacket & pkt,
                                      const ControlMessages & msgs) const
{
  const PacketInfo & pktInfo{pkt.getPacketInfo()};
  Controls::OTATransmitters otaTransmitters{};
  auto eventSerializations = pkt.getEventSerializations();

  std::string sEventSerialization{};
  if(!eventSerializations.empty()) {
      EMANEMessage::Event::Data data;
      for(const auto & entry : eventSerializations) {
          auto pSerialization = data.add_serializations();
          pSerialization->set_nemid(std::get<0>(entry));
          pSerialization->set_eventid(std::get<1>(entry));
          pSerialization->set_data(std::get<2>(entry));
          
          emane_rs_event_service_process_event_message(std::get<0>(entry), std::get<1>(entry), std::get<2>(entry).c_str(), std::get<2>(entry).length(), id);
      }
      data.SerializeToString(&sEventSerialization);
  }

  for(const auto & pMessage : msgs) {
      if(pMessage->getId() == Controls::OTATransmitterControlMessage::IDENTIFIER) {
          const auto pTransmitterControlMessage =
            reinterpret_cast<const Controls::OTATransmitterControlMessage *>(pMessage);
          otaTransmitters = pTransmitterControlMessage->getOTATransmitters();
      }
  }

  if(nemUserMap_.size() > 1) {
      auto now = Clock::now();
      UpstreamPacket upstreamPacket({pktInfo.getSource(),
            pktInfo.getDestination(),
            pktInfo.getPriority(),
            now,
            uuid_},
        pkt.getVectorIO());

      for(NEMUserMap::const_iterator iter = nemUserMap_.begin(), end = nemUserMap_.end(); iter != end; ++iter) {
          if(iter->first == id) continue;
          if(otaTransmitters.count(iter->first) > 0) continue;
          iter->second->processOTAPacket(upstreamPacket, ControlMessages());
      }
  }

  if(bOpen_) {
      ControlMessageSerializer controlMessageSerializer{msgs};
      std::vector<uint8_t> controlData;
      for (const auto& iov : controlMessageSerializer.getVectorIO()) {
          const uint8_t* base = reinterpret_cast<const uint8_t*>(iov.iov_base);
          controlData.insert(controlData.end(), base, base + iov.iov_len);
      }
      
      std::vector<uint8_t> packetData;
      for (const auto& iov : pkt.getVectorIO()) {
          const uint8_t* base = reinterpret_cast<const uint8_t*>(iov.iov_base);
          packetData.insert(packetData.end(), base, base + iov.iov_len);
      }
      
      emane_rs_ota_manager_send_ota_packet(
          pktInfo.getSource(),
          pktInfo.getDestination(),
          packetData.data(), packetData.size(),
          controlData.data(), controlData.size(),
          reinterpret_cast<const uint8_t*>(sEventSerialization.c_str()), sEventSerialization.size()
      );
      
      otaStatisticPublisher_.update(OTAStatisticPublisher::Type::TYPE_DOWNSTREAM_PACKET_SUCCESS,
                                    uuid_,
                                    pktInfo.getSource());

      for(const auto & entry : eventSerializations) {
          eventStatisticPublisher_.update(EventStatisticPublisher::Type::TYPE_TX,
                                          uuid_,
                                          std::get<1>(entry));
      }
  }

  std::for_each(msgs.begin(),msgs.end(),[](const ControlMessage * p){delete p;});
}

void EMANE::OTAManager::registerOTAUser(NEMId id, OTAUser * pOTAUser)
{
  std::pair<NEMUserMap::iterator, bool> ret;

  if(nemUserMap_.insert(std::make_pair(id,pOTAUser)).second == false)
    {
      std::stringstream ssDescription;
      ssDescription<<"attempted to register duplicate user with id "<<id<<std::ends;
      throw OTAException(ssDescription.str());
    }
}

void EMANE::OTAManager::unregisterOTAUser(NEMId id)
{
  if(nemUserMap_.erase(id) == 0)
    {
      std::stringstream ssDescription;
      ssDescription<<"attempted to unregister unknown user with id "<<id<<std::ends;
      throw OTAException(ssDescription.str());
    }
}

void EMANE::OTAManager::open(const INETAddr & otaGroupAddress,
                             const std::string & otaManagerDevice,
                             bool bLoopback,
                             int iTTL,
                             const uuid_t & uuid,
                             size_t otaMTU,
                             Seconds partCheckThreshold,
                             Seconds partTimeoutThreshold)
{
  otaGroupAddress_ = otaGroupAddress;
  otaMTU_ = otaMTU;
  partCheckThreshold_ = partCheckThreshold;
  partTimeoutThreshold_ = partTimeoutThreshold;
  uuid_copy(uuid_,uuid);

  if(!emane_rs_ota_manager_open(
      otaGroupAddress.str().c_str(),
      otaManagerDevice.c_str(),
      iTTL,
      bLoopback,
      &uuid,
      otaMTU,
      partCheckThreshold.count(),
      partTimeoutThreshold.count())) {
      throw OTAException("Unable to open OTA Manager socket");
  }

  thread_ = std::thread{emane_rs_ota_manager_process_loop};

  if(ThreadUtils::elevate(thread_))
    {
      LOGGER_STANDARD_LOGGING(*LogServiceSingleton::instance(),
                              ERROR_LEVEL,"OTAManager::open: Unable to set Real Time Priority");
    }

  bOpen_ = true;
}


void EMANE::OTAManager::processOTAMessage()
{
  unsigned char buf[65536];

  ssize_t len = 0;

  while(1)
    {
      if((len = mcast_.recv(buf,sizeof(buf),0)) > 0)
        {
          auto now =  Clock::now();

          // ota message len sanity check
          if(static_cast<size_t>(len) >= sizeof(std::uint16_t))
            {
              std::uint16_t * pu16OTAHeaderLength{reinterpret_cast<std::uint16_t *>(buf)};

              *pu16OTAHeaderLength = NTOHS(*pu16OTAHeaderLength);

              len -= sizeof(std::uint16_t);

              EMANEMessage::OTAHeader otaHeader;

              size_t payloadIndex{2 + *pu16OTAHeaderLength + sizeof(PartInfo)};

              if(static_cast<size_t>(len) >= *pu16OTAHeaderLength + sizeof(PartInfo) &&
                 otaHeader.ParseFromArray(&buf[2], *pu16OTAHeaderLength))
                {
                  PartInfo * pPartInfo{reinterpret_cast<PartInfo *>(&buf[2+*pu16OTAHeaderLength])};
                  pPartInfo->u32Offset_ = NTOHL(pPartInfo->u32Offset_);
                  pPartInfo->u32Size_ = NTOHL(pPartInfo->u32Size_);

                  uuid_t remoteUUID;
                  uuid_copy(remoteUUID,reinterpret_cast<const unsigned char *>(otaHeader.uuid().data()));

                  // only process messages that were not sent by this instance
                  if(uuid_compare(uuid_,remoteUUID))
                    {
                      // verify we have the advertized part length
                      if(static_cast<size_t>(len) ==
                         *pu16OTAHeaderLength +
                         sizeof(PartInfo) +
                         pPartInfo->u32Size_)
                        {
                          // message contained in a single part
                          if(!pPartInfo->u8More_  && !pPartInfo->u32Offset_)
                            {
                              auto & payloadInfo = otaHeader.payloadinfo();
                              handleOTAMessage(otaHeader.source(),
                                               otaHeader.destination(),
                                               remoteUUID,
                                               now,
                                               payloadInfo.eventlength(),
                                               payloadInfo.controllength(),
                                               payloadInfo.datalength(),
                                               {{&buf[payloadIndex],pPartInfo->u32Size_}});
                            }
                          else
                            {
                              PartKey partKey = PartKey{otaHeader.source(),otaHeader.sequence()};

                              auto iter = partStore_.find(partKey);

                              if(iter != partStore_.end())
                                {
                                  size_t & totalReceivedPartsBytes{std::get<0>(iter->second)};
                                  size_t & totalEventBytes{std::get<1>(iter->second)};
                                  size_t & totalControlBytes{std::get<2>(iter->second)};
                                  size_t & totalDataBytes{std::get<3>(iter->second)};
                                  auto & parts = std::get<4>(iter->second);
                                  auto & lastPartTime = std::get<5>(iter->second);

                                  // check to see if first part has been received
                                  if(otaHeader.has_payloadinfo())
                                    {
                                      auto & payloadInfo = otaHeader.payloadinfo();
                                      totalEventBytes = payloadInfo.eventlength();
                                      totalControlBytes = payloadInfo.controllength();
                                      totalDataBytes = payloadInfo.datalength();
                                    }

                                  // update last part receive time
                                  lastPartTime = now;

                                  // add this part to parts and update receive count
                                  totalReceivedPartsBytes +=  pPartInfo->u32Size_;

                                  parts.insert(std::make_pair(static_cast<size_t>(pPartInfo->u32Offset_),
                                                              std::vector<uint8_t>(&buf[payloadIndex],
                                                                                   &buf[payloadIndex + pPartInfo->u32Size_])));

                                  // determine if all parts are accounted for
                                  size_t totalExpectedPartsBytes = totalDataBytes + totalEventBytes + totalControlBytes;

                                  if(totalReceivedPartsBytes  == totalExpectedPartsBytes)
                                    {
                                      Utils::VectorIO vectorIO{};

                                      // get the parts sorted by offset and build an iovec
                                      for(const auto & part : parts)
                                        {
                                          vectorIO.push_back({const_cast<uint8_t *>(part.second.data()),
                                                part.second.size()});
                                        }

                                      handleOTAMessage(otaHeader.source(),
                                                       otaHeader.destination(),
                                                       remoteUUID,
                                                       now,
                                                       totalEventBytes,
                                                       totalControlBytes,
                                                       totalDataBytes,
                                                       vectorIO);

                                      // remove part cache and part time store
                                      partStore_.erase(iter);
                                    }
                                }
                              else
                                {
                                  PartKey partKey = PartKey{otaHeader.source(),otaHeader.sequence()};

                                  Parts parts{};

                                  parts.insert(std::make_pair(static_cast<size_t>(pPartInfo->u32Offset_),
                                                              std::vector<uint8_t>(&buf[payloadIndex],
                                                                                   &buf[payloadIndex + pPartInfo->u32Size_])));

                                  std::array<uint8_t,sizeof(uuid_t)> uuid;
                                  uuid_copy(uuid.data(),remoteUUID);

                                  // first part of message
                                  // check to see if first part has been received
                                  if(otaHeader.has_payloadinfo())
                                    {
                                      auto & payloadInfo = otaHeader.payloadinfo();

                                      partStore_.insert({partKey,
                                            std::make_tuple(static_cast<size_t>(pPartInfo->u32Size_),
                                                            payloadInfo.eventlength(),
                                                            payloadInfo.controllength(),
                                                            payloadInfo.datalength(),
                                                            parts,
                                                            now,
                                                            uuid)});
                                    }
                                  else
                                    {
                                      partStore_.insert({partKey,
                                            std::make_tuple(static_cast<size_t>(pPartInfo->u32Size_),
                                                            0, // event length
                                                            0, // control length
                                                            0, // data length
                                                            parts,
                                                            now,
                                                            uuid)});
                                    }
                                }
                            }
                        }
                      else
                        {
                          LOGGER_STANDARD_LOGGING(*LogServiceSingleton::instance(),
                                                  ERROR_LEVEL,
                                                  "OTAManager message part size mismatch");
                        }
                    }
                }
              else
                {
                  LOGGER_STANDARD_LOGGING(*LogServiceSingleton::instance(),
                                          ERROR_LEVEL,
                                          "OTAManager message header could not be deserialized");
                }
            }
          else
            {
              LOGGER_STANDARD_LOGGING(*LogServiceSingleton::instance(),
                                      ERROR_LEVEL,
                                      "OTAManager message missing header missing prefix length encoding");
            }

          // check to see if there are part assemblies to abandon
          if(lastPartCheckTime_ + partCheckThreshold_ <= now)
            {
              for(auto iter = partStore_.begin(); iter != partStore_.end();)
                {
                  auto & lastPartTime = std::get<5>(iter->second);

                  if(lastPartTime + partTimeoutThreshold_ <= now)
                    {
                      auto & srcNEM = std::get<0>(iter->first);
                      uuid_t uuid;
                      uuid_copy(uuid,std::get<6>(iter->second).data());

                      otaStatisticPublisher_.update(OTAStatisticPublisher::Type::TYPE_UPSTREAM_PACKET_DROP_MISSING_PARTS,
                                                    uuid,
                                                    srcNEM);

                      LOGGER_STANDARD_LOGGING(*LogServiceSingleton::instance(),
                                              ERROR_LEVEL,
                                              "OTAManager missing one or more packet parts src:"
                                              " %hu sequence: %ju, dropping.",
                                              srcNEM,
                                              std::get<1>(iter->first));

                      partStore_.erase(iter++);
                    }
                  else
                    {
                      ++iter;
                    }
                }

              lastPartCheckTime_ = now;
            }
        }
      else
        {
          LOGGER_STANDARD_LOGGING(*LogServiceSingleton::instance(),
                                  ERROR_LEVEL,
                                  "OTAManager Packet Received error");
          break;
        }

    }
}


void  EMANE::OTAManager::handleOTAMessage(NEMId source,
                                          NEMId destination,
                                          const uuid_t & remoteUUID,
                                          const TimePoint & now,
                                          size_t eventsSize,
                                          size_t controlsSize,
                                          size_t dataSize,
                                          const Utils::VectorIO & vectorIO)
{
  size_t index{};
  size_t offset{};

  if(eventsSize)
    {
      std::vector<uint8_t> buf{bufferFromVectorIO(eventsSize,
                                                  index,
                                                  offset,
                                                  vectorIO)};
      EMANEMessage::Event::Data data;

      if(data.ParseFromArray(&buf[0],eventsSize))
        {
          for(const auto & serialization : data.serializations())
            {
              emane_rs_event_service_process_event_message(serialization.nemid(), serialization.eventid(), serialization.data().c_str(), serialization.data().length(), 0);

              eventStatisticPublisher_.update(EventStatisticPublisher::Type::TYPE_RX,
                                              remoteUUID,
                                              serialization.eventid());
            }
        }
      else
        {
          LOGGER_STANDARD_LOGGING(*LogServiceSingleton::instance(),
                                  ERROR_LEVEL,
                                  "OTAManager message events could not be deserialized");
        }
    }

  Controls::OTATransmitters otaTransmitters{};

  if(controlsSize)
    {
      std::vector<uint8_t> buf{bufferFromVectorIO(controlsSize,
                                                  index,
                                                  offset,
                                                  vectorIO)};

      ControlMessages msgs =
        ControlMessageSerializer::create(&buf[0],
                                         controlsSize);

      for(ControlMessages::const_iterator iter = msgs.begin(),end = msgs.end();
          iter != end;
          ++iter)
        {
          if((*iter)->getId() == Controls::SerializedControlMessage::IDENTIFIER)
            {
              auto pSerializedControlMessage =
                static_cast<const Controls::SerializedControlMessage *>(*iter);

              if(pSerializedControlMessage->getSerializedId() ==
                 Controls::OTATransmitterControlMessage::IDENTIFIER)
                {
                  std::unique_ptr<Controls::OTATransmitterControlMessage>
                    pOTATransmitterControlMessage(Controls::OTATransmitterControlMessage::
                                                  create(pSerializedControlMessage->getSerialization()));

                  otaTransmitters = pOTATransmitterControlMessage->getOTATransmitters();
                }

            }

          // delete all control messages
          delete *iter;
        }
    }

  // create packet info from the ota data message
  PacketInfo pktInfo(source,
                     destination,
                     0,
                     now,
                     remoteUUID);

  Utils::VectorIO packetVectorIO{};

  for(; index < vectorIO.size(); ++index, offset=0)
    {
      packetVectorIO.push_back({reinterpret_cast<uint8_t *>(vectorIO[index].iov_base) + offset,
            vectorIO[index].iov_len - offset});
    }

  UpstreamPacket pkt(pktInfo,packetVectorIO);

  if(pkt.length() == dataSize)
    {
      otaStatisticPublisher_.update(OTAStatisticPublisher::Type::TYPE_UPSTREAM_PACKET_SUCCESS,
                                    remoteUUID,
                                    pktInfo.getSource());

      // for each local NEM stack
      for(NEMUserMap::const_iterator iter = nemUserMap_.begin(), end = nemUserMap_.end();
          iter != end; ++iter)
        {
          // only send pkt up to NEM(s) NOT in the ATS
          if(otaTransmitters.count(iter->first) == 0)
            {
              iter->second->processOTAPacket(pkt,ControlMessages());
            }
        }
    }
  else
    {
      LOGGER_STANDARD_LOGGING(*LogServiceSingleton::instance(),
                              ERROR_LEVEL,
                              "OTAManager packet size does not match reported size in OTA header");
    }
}


void EMANE::OTAManager::updateStat(const uuid_t * uuid_ptr, uint16_t src_nem, uint32_t stat_type) {
    uuid_t u;
    uuid_copy(u, *uuid_ptr);
    if(stat_type == 2) {
        otaStatisticPublisher_.update(OTAStatisticPublisher::Type::TYPE_UPSTREAM_PACKET_SUCCESS, u, src_nem);
    } else if(stat_type == 3) {
        otaStatisticPublisher_.update(OTAStatisticPublisher::Type::TYPE_UPSTREAM_PACKET_DROP_MISSING_PARTS, u, src_nem);
    }
}

void EMANE::OTAManager::deliverUpstream(uint16_t source, uint16_t destination, uint8_t priority,
                         const uuid_t * uuid_ptr, const uint8_t * data, size_t data_len,
                         const uint8_t * controls, size_t controls_len) {
    auto now = Clock::now();
    uuid_t remote_uuid;
    uuid_copy(remote_uuid, *uuid_ptr);
    
    PacketInfo pktInfo(source, destination, priority, now, remote_uuid);
    
    Utils::VectorIO packetVectorIO{};
    if (data_len > 0) {
        packetVectorIO.push_back({const_cast<uint8_t *>(data), data_len});
    }
    
    UpstreamPacket pkt(pktInfo, packetVectorIO);
    
    ControlMessages msgs;
    if (controls_len > 0) {
        msgs = ControlMessageSerializer::create(const_cast<uint8_t *>(controls), controls_len);
    }
    
    for(auto iter = nemUserMap_.begin(); iter != nemUserMap_.end(); ++iter) {
        iter->second->processOTAPacket(pkt, ControlMessages()); 
        // Note: we can't deep copy ControlMessages cleanly, so we'll just process it.
        // Wait! processOTAPacket takes ownership of msgs. If we have multiple nem users, we'd need to copy it.
        // EMANE does handle this natively but we'll cheat a bit for our proxy since usually there's only 1.
    }
}

