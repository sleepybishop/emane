/*
 * Copyright (c) 2015,2023 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "queuemanager.h"
#include "rust_queuemanager.h"
#include "emane/configureexception.h"

EMANE::Models::BentPipe::QueueManager::QueueManager(NEMId id,
                                                    PlatformServiceProvider * pPlatformService,
                                                    PacketStatusPublisher * pPacketStatusPublisher):
  id_{id},
  pPlatformService_{pPlatformService},
  pPacketStatusPublisher_{pPacketStatusPublisher},
  u16QueueDepth_{},
  bAggregationEnable_{},
  bFragmentationEnable_{}
{
  pRustQueueManager_ = bentpipe_queue_manager_new();
}

EMANE::Models::BentPipe::QueueManager::~QueueManager()
{
  if (pRustQueueManager_) {
      bentpipe_queue_manager_free(pRustQueueManager_);
  }
}

void EMANE::Models::BentPipe::QueueManager::initialize(Registrar & registrar)
{
  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          DEBUG_LEVEL,
                          "MACI %03hu BentPipe::QueueManager::%s",
                          id_,
                          __func__);

  auto & configRegistrar = registrar.configurationRegistrar();

  std::string sPrefix{CONFIG_PREFIX};

  configRegistrar.registerNumeric<std::uint16_t>(sPrefix + "depth",
                                                 ConfigurationProperties::DEFAULT,
                                                 {256},
                                                 "Defines the size of the per transponder downstream packet"
                                                 " queues in packets.");

  configRegistrar.registerNumeric<bool>(sPrefix + "aggregationenable",
                                        ConfigurationProperties::DEFAULT,
                                        {true},
                                        "Defines whether packet aggregation is enabled for transmission. When"
                                        " enabled, multiple packets up to a specified MTU can be sent in the same"
                                        " transmission.");


  configRegistrar.registerNumeric<bool>(sPrefix + "fragmentationenable",
                                        ConfigurationProperties::DEFAULT,
                                        {true},
                                        "Defines whether packet fragmentation is enabled. When enabled, a single"
                                        " packet larger than a specified MTU will be fragmented into multiple message"
                                        " and sent. When disabled, packets larger than the MTU will be discarded.");

  auto & statisticRegistrar = registrar.statisticRegistrar();

  queueStatusPublisher_.registerStatistics(statisticRegistrar);

}

void EMANE::Models::BentPipe::QueueManager::configure(const ConfigurationUpdate & update)
{
  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          DEBUG_LEVEL,
                          "MACI %03hu BentPipe::QueueManager::%s",
                          id_,
                          __func__);

  std::string sPrefix{CONFIG_PREFIX};

  for(const auto & item : update)
    {
      if(item.first == sPrefix + "depth")
        {
          u16QueueDepth_ = item.second[0].asUINT16();

          LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                  INFO_LEVEL,
                                  "MACI %03hu BentPipe::QueueManager::%s: %s = %hu",
                                  id_,
                                  __func__,
                                  item.first.c_str(),
                                  u16QueueDepth_);
        }
      else if(item.first == sPrefix + "aggregationenable")
        {
          bAggregationEnable_ = item.second[0].asBool();

          LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                  INFO_LEVEL,
                                  "MACI %03hu BentPipe::QueueManager::%s: %s = %s",
                                  id_,
                                  __func__,
                                  item.first.c_str(),
                                  bAggregationEnable_ ? "on" : "off");
        }
      else if(item.first == sPrefix + "fragmentationenable")
        {
          bFragmentationEnable_ = item.second[0].asBool();

          LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                  INFO_LEVEL,
                                  "MACI %03hu BentPipe::QueueManager::%s: %s = %s",
                                  id_,
                                  __func__,
                                  item.first.c_str(),
                                  bFragmentationEnable_ ? "on" : "off");
        }
      else
        {
          throw makeException<ConfigureException>("BentPipe::QueueManager: "
                                                  "Unexpected configuration item %s",
                                                  item.first.c_str());
        }
    }

    bentpipe_queue_manager_set_config(pRustQueueManager_, u16QueueDepth_, bAggregationEnable_, bFragmentationEnable_);
}

void  EMANE::Models::BentPipe::QueueManager::addQueue(TransponderIndex transponderIndex)
{
  bentpipe_queue_manager_add_queue(pRustQueueManager_, transponderIndex);
}

void EMANE::Models::BentPipe::QueueManager::removeQueue(TransponderIndex transponderIndex)
{
  bentpipe_queue_manager_remove_queue(pRustQueueManager_, transponderIndex);
}

void EMANE::Models::BentPipe::QueueManager::start()
{
  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          DEBUG_LEVEL,
                          "MACI %03hu BentPipe::QueueManager::%s",
                          id_,
                          __func__);
}

void EMANE::Models::BentPipe::QueueManager::postStart()
{
  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          DEBUG_LEVEL,
                          "MACI %03hu BentPipe::QueueManager::%s",
                          id_,
                          __func__);
}

void EMANE::Models::BentPipe::QueueManager::stop()
{
  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          DEBUG_LEVEL,
                          "MACI %03hu BentPipe::QueueManager::%s",
                          id_,
                          __func__);
}

void EMANE::Models::BentPipe::QueueManager::destroy() throw()
{
  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          DEBUG_LEVEL,
                          "MACI %03hu BentPipe::QueueManager::%s",
                          id_,
                          __func__);
}

size_t EMANE::Models::BentPipe::QueueManager::enqueue(TransponderIndex transponserIndex,
                                                      DownstreamPacket && pkt)
{
  size_t packetsDropped{};
  
  DownstreamPacket* pPkt = new DownstreamPacket{std::move(pkt)};

  BentPipeEnqueueResult ret = bentpipe_queue_manager_enqueue(pRustQueueManager_, transponserIndex, pPkt, pPkt->length());

  if(ret.dropped)
    {
      packetsDropped = 1;

      queueStatusPublisher_.drop(transponserIndex,
                                 QueueStatusPublisher::DropReason::DROP_OVERFLOW,
                                 1);

      DownstreamPacket* droppedPkt = static_cast<DownstreamPacket*>(ret.dropped_pkt);
      const auto & pktInfo = droppedPkt->getPacketInfo();

      pPacketStatusPublisher_->outbound(pktInfo.getSource(),
                                        pktInfo.getDestination(),
                                        droppedPkt->length(),
                                        PacketStatusPublisher::OutboundAction::DROP_OVERFLOW);
      delete droppedPkt;
    }

  queueStatusPublisher_.enqueue(transponserIndex);

  return packetsDropped;
}

std::tuple<EMANE::Models::BentPipe::MessageComponents,size_t>
EMANE::Models::BentPipe::QueueManager::dequeue(TransponderIndex transponserIndex,
                                               size_t requestedBytes)
{
  MessageComponents components{};
  
  BentPipeDequeueResult res;
  bentpipe_queue_manager_dequeue(pRustQueueManager_, transponserIndex, requestedBytes, &res);
  
  size_t totalLength = res.total_bytes;

  for (size_t i = 0; i < res.num_actions; ++i) {
      const auto& action = res.actions[i];
      DownstreamPacket* pPkt = static_cast<DownstreamPacket*>(action.pkt_ptr);
      
      if (action.action_type == 0) { // DROP
          const auto & pktInfo = pPkt->getPacketInfo();
          pPacketStatusPublisher_->outbound(pktInfo.getSource(),
                                            pktInfo.getDestination(),
                                            pPkt->length(),
                                            PacketStatusPublisher::OutboundAction::DROP_TOO_BIG);

          queueStatusPublisher_.drop(transponserIndex,
                                     QueueStatusPublisher::DropReason::DROP_TOOBIG,
                                     1);
          delete pPkt;
      } else if (action.action_type == 1) { // FULL
          NEMId dst = pPkt->getPacketInfo().getDestination();
          components.push_back({dst,
                                pPkt->getVectorIO(),
                                action.fragment_index,
                                action.fragment_offset,
                                action.seq,
                                false});
          delete pPkt;
      } else if (action.action_type == 2) { // FRAGMENT
          NEMId dst = pPkt->getPacketInfo().getDestination();
          
          size_t totalBytesVisited{};
          size_t totalBytesCopied{};
          Utils::VectorIO vectorIOs{};
          
          for(const auto & entry : pPkt->getVectorIO())
            {
              if(totalBytesCopied < action.fragment_size)
                {
                  if(totalBytesVisited + entry.iov_len < action.fragment_offset)
                    {
                      totalBytesVisited += entry.iov_len;
                    }
                  else
                    {
                      char * pBuf{reinterpret_cast<char *>(entry.iov_base)};
                      auto offset = action.fragment_offset - totalBytesVisited;
                      auto remainder = entry.iov_len - offset;

                      if(totalBytesVisited < action.fragment_offset)
                        {
                          totalBytesVisited = action.fragment_offset;
                        }

                      size_t amountToCopy{};

                      if(remainder > action.fragment_size - totalBytesCopied)
                        {
                          amountToCopy =  action.fragment_size - totalBytesCopied;
                          remainder -= amountToCopy;
                        }
                      else
                        {
                          amountToCopy = remainder;
                        }

                      vectorIOs.push_back(Utils::make_iovec(pBuf+offset,amountToCopy));

                      totalBytesVisited += amountToCopy;
                      totalBytesCopied += amountToCopy;
                    }
                }
              else
                {
                  break;
                }
            }

          components.push_back({dst,
                                vectorIOs,
                                action.fragment_index,
                                action.fragment_offset,
                                action.seq,
                                action.more_fragments});
                                
          if (!action.more_fragments) {
              delete pPkt;
          }
      }
  }
  
  if(totalLength)
    {
      queueStatusPublisher_.dequeue(transponserIndex,
                                    components);
    }
  
  bentpipe_queue_manager_free_dequeue_result(&res);

  pPacketStatusPublisher_->outbound(id_,
                                    components,
                                    PacketStatusPublisher::OutboundAction::ACCEPT_GOOD);

  return std::make_tuple(std::move(components),totalLength);
}

EMANE::Models::BentPipe::QueueInfos
EMANE::Models::BentPipe::QueueManager::getPacketQueueInfo() const
{
  QueueInfos queueInfos{};
  
  BentPipeQueueInfosResult res;
  bentpipe_queue_manager_get_queue_infos(pRustQueueManager_, &res);
  
  for(size_t i = 0; i < res.num_infos; ++i) {
      queueInfos.push_back({res.infos[i].transponder_index, res.infos[i].packets, res.infos[i].bytes});
  }
  
  bentpipe_queue_manager_free_queue_infos_result(&res);

  return queueInfos;
}
