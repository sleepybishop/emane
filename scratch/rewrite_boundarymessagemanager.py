import re

with open("src/libemane/boundarymessagemanager.cc", "w") as f:
    f.write("""#include <cstdint>
/*
 * Copyright (c) 2013-2017 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "boundarymessagemanager.h"
#include "boundarymessagemanagerexception.h"
#include "logservice.h"
#include "controlmessageserializer.h"
#include "netadaptermessage.h"

#include "emane/utils/threadutils.h"
#include "emane/controls/serializedcontrolmessage.h"
#include "emane/upstreampacket.h"

#include <sys/uio.h>

extern "C" {
    void* emane_rs_boundary_manager_new(std::uint16_t id, void* manager_ptr);
    void emane_rs_boundary_manager_free(void* ptr);
    void emane_rs_boundary_manager_open(void* ptr, const char* local_addr, const char* remote_addr, int protocol);
    void emane_rs_boundary_manager_close(void* ptr);
    void emane_rs_boundary_manager_send(void* ptr, const struct iovec* iov, size_t iov_len);
}

EMANE::BoundaryMessageManager::BoundaryMessageManager(NEMId id):
  id_{id},
  rs_manager_{emane_rs_boundary_manager_new(id, this)}
{}

EMANE::BoundaryMessageManager::~BoundaryMessageManager()
{
  emane_rs_boundary_manager_free(rs_manager_);
}

void EMANE::BoundaryMessageManager::open(const INETAddr & localAddress,
                                         const INETAddr & remoteAddress,
                                         Protocol protocol)
{
  int proto_int = 0;
  if(protocol == Protocol::PROTOCOL_TCP_SERVER) proto_int = 1;
  else if(protocol == Protocol::PROTOCOL_TCP_CLIENT) proto_int = 2;

  emane_rs_boundary_manager_open(rs_manager_, localAddress.str().c_str(), remoteAddress.str().c_str(), proto_int);
}

void EMANE::BoundaryMessageManager::close()
{
  emane_rs_boundary_manager_close(rs_manager_);
}

void EMANE::BoundaryMessageManager::processBoundaryMessage(const void * pData, size_t length)
{
  handleNetworkMessage(const_cast<void*>(pData), length);
}

void EMANE::BoundaryMessageManager::sendPacketMessage(const PacketInfo & packetInfo,
                                                      const void * pPacketData,
                                                      size_t packetLength,
                                                      const ControlMessages & msgs)
{
  Utils::VectorIO vectorIO;
  vectorIO.push_back({const_cast<void *>(pPacketData), packetLength});
  sendPacketMessage(packetInfo, vectorIO, packetLength, msgs);
}

void EMANE::BoundaryMessageManager::sendPacketMessage(const PacketInfo & packetInfo,
                                                      const Utils::VectorIO & packetIO,
                                                      size_t packetDataLength,
                                                      const ControlMessages & msgs)
{
  ControlMessageSerializer controlMessageSerializer(msgs);

  NetAdapterHeader header;
  memset(&header,0,sizeof(header));
  header.u16Id_ = NETADAPTER_DATA_MSG;
  header.u32Length_ = sizeof(header) + sizeof(NetAdapterDataMessage) + packetDataLength + controlMessageSerializer.getLength();
  NetAdapterHeaderToNet(&header);

  NetAdapterDataMessage dataMessage;
  memset(&dataMessage,0,sizeof(NetAdapterDataMessage));
  dataMessage.u16Src_ = packetInfo.getSource();
  dataMessage.u16Dst_ = packetInfo.getDestination();
  if(dataMessage.u16Dst_ == NEM_BROADCAST_MAC_ADDRESS) {
      dataMessage.u16Dst_ = NETADAPTER_BROADCAST_ADDRESS;
  }
  dataMessage.u32DataLen_ = packetDataLength;
  dataMessage.u32CtrlLen_ = controlMessageSerializer.getLength();
  dataMessage.u8Priority_ = packetInfo.getPriority();
  NetAdapterDataMessageToNet(&dataMessage);

  Utils::VectorIO vectorIO;
  vectorIO.push_back({&header,sizeof(header)});
  vectorIO.push_back({&dataMessage,sizeof(dataMessage)});

  vectorIO.insert(vectorIO.end(), packetIO.begin(), packetIO.end());

  const Utils::VectorIO & controlVectorIO = controlMessageSerializer.getVectorIO();
  vectorIO.insert(vectorIO.end(), controlVectorIO.begin(), controlVectorIO.end());

  emane_rs_boundary_manager_send(rs_manager_, reinterpret_cast<const struct iovec*>(vectorIO.data()), vectorIO.size());

  std::for_each(msgs.begin(),msgs.end(),[](const ControlMessage * p){delete p;});
}

void EMANE::BoundaryMessageManager::sendControlMessage(const ControlMessages & msgs)
{
  ControlMessageSerializer controlMessageSerializer(msgs);

  NetAdapterHeader header;
  memset(&header,0,sizeof(header));
  header.u16Id_ = NETADAPTER_CTRL_MSG;
  header.u32Length_ = sizeof(header) + sizeof(NetAdapterControlMessage) + controlMessageSerializer.getLength();
  NetAdapterHeaderToNet(&header);

  NetAdapterControlMessage controlMessage;
  memset(&controlMessage,0,sizeof(NetAdapterControlMessage));
  controlMessage.u32CtrlLen_ = controlMessageSerializer.getLength();
  NetAdapterControlMessageToNet(&controlMessage);

  Utils::VectorIO vectorIO;
  vectorIO.push_back({&header,sizeof(header)});
  vectorIO.push_back({&controlMessage,sizeof(controlMessage)});

  const Utils::VectorIO & controlVectorIO = controlMessageSerializer.getVectorIO();
  vectorIO.insert(vectorIO.end(), controlVectorIO.begin(), controlVectorIO.end());

  emane_rs_boundary_manager_send(rs_manager_, reinterpret_cast<const struct iovec*>(vectorIO.data()), vectorIO.size());

  std::for_each(msgs.begin(),msgs.end(),[](const ControlMessage * p){delete p;});
}

void EMANE::BoundaryMessageManager::handleNetworkMessage(void * buf,size_t len)
{
  NEMId dstNemId{};

  if(len >= sizeof(NetAdapterHeader)) {
      NetAdapterHeader * pHeader = reinterpret_cast<NetAdapterHeader *>(buf);
      NetAdapterHeaderToHost(pHeader);

      if(len == pHeader->u32Length_) {
          len -= sizeof(NetAdapterHeader);

          switch(pHeader->u16Id_) {
            case NETADAPTER_DATA_MSG:
              if(len >= sizeof(NetAdapterDataMessage)) {
                  NetAdapterDataMessage * pMsg = reinterpret_cast<NetAdapterDataMessage *>(pHeader->data_);
                  NetAdapterDataMessageToHost(pMsg);
                  len -= sizeof(NetAdapterDataMessage);

                  if(pMsg->u16Dst_ == NETADAPTER_BROADCAST_ADDRESS) {
                      dstNemId = NEM_BROADCAST_MAC_ADDRESS;
                  } else {
                      dstNemId = pMsg->u16Dst_;
                  }

                  if(len >= pMsg->u32DataLen_) {
                      PacketInfo pinfo{pMsg->u16Src_, dstNemId, pMsg->u8Priority_, Clock::now()};
                      UpstreamPacket pkt(pinfo, pMsg->data_, pMsg->u32DataLen_);
                      len -= pMsg->u32DataLen_;

                      if(len == pMsg->u32CtrlLen_) {
                          doProcessPacketMessage(pinfo, pMsg->data_, pMsg->u32DataLen_,
                                                 ControlMessageSerializer::create(pMsg->data_ + pMsg->u32DataLen_, pMsg->u32CtrlLen_));
                      }
                  }
              }
              break;

            case NETADAPTER_CTRL_MSG:
              if(len >= sizeof(NetAdapterControlMessage)) {
                  NetAdapterControlMessage * pMsg = reinterpret_cast<NetAdapterControlMessage *>(pHeader->data_);
                  NetAdapterControlMessageToHost(pMsg);
                  len -= sizeof(NetAdapterControlMessage);

                  if(len == pMsg->u32CtrlLen_) {
                      doProcessControlMessage(ControlMessageSerializer::create(pMsg->data_, pMsg->u32CtrlLen_));
                  }
              }
              break;
          }
      }
  }
}

extern "C" {
    void emane_c_boundary_manager_handle_message(void* manager_ptr, void* buf, size_t len) {
        auto manager = static_cast<EMANE::BoundaryMessageManager*>(manager_ptr);
        manager->processBoundaryMessage(buf, len);
    }
}
""")
