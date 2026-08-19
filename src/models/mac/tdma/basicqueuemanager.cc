
#include "emane/models/tdma/basicqueuemanager.h"
#include "emane/configureexception.h"
#include "queuestatuspublisher.h"

extern "C" {
    void* basic_queue_manager_new();
    void basic_queue_manager_free(void* m);
    void basic_queue_manager_set_config(
        void* m,
        std::uint16_t queue_depth,
        bool aggregation_enable,
        bool fragmentation_enable,
        bool strict_dequeue_enable,
        double aggregation_slot_threshold
    );

    struct EnqueueResult {
        void* dropped_pkt;
        bool dropped;
    };
    EnqueueResult basic_queue_manager_enqueue(
        void* m,
        std::uint8_t u8_queue_index,
        void* pkt_ptr,
        std::uint16_t dest,
        size_t length,
        std::uint8_t priority
    );

    struct DequeueAction {
        std::uint8_t action_type; // 0 = DROP, 1 = DEQUEUE_FULL, 2 = DEQUEUE_FRAGMENT
        std::uint8_t queue_index;
        void* pkt_ptr;
        std::uint16_t dest;
        std::uint8_t priority;
        std::uint64_t seq;
        size_t fragment_index;
        size_t fragment_offset;
        size_t fragment_size;
        bool is_control;
        bool more_fragments;
    };

    struct DequeueResult {
        DequeueAction* actions;
        size_t num_actions;
        size_t total_bytes;
    };

    void basic_queue_manager_dequeue(
        void* m,
        std::uint8_t u8_queue_index,
        size_t requested_bytes,
        std::uint16_t destination,
        DequeueResult* res
    );

    void tdma_queue_free_dequeue_result(DequeueResult* res); // we can use the same free function

    struct QueueStatus {
        size_t packets;
        size_t bytes;
    };

    void basic_queue_manager_get_status(
        const void* m,
        QueueStatus* statuses
    );
}

class EMANE::Models::TDMA::BasicQueueManager::Implementation
{
public:
  void* pRustQueueManager_{};
  QueueStatusPublisher queueStatusPublisher_;
  
  Implementation() {
      pRustQueueManager_ = basic_queue_manager_new();
  }
  
  ~Implementation() {
      if (pRustQueueManager_) {
          basic_queue_manager_free(pRustQueueManager_);
      }
  }
};


EMANE::Models::TDMA::BasicQueueManager::BasicQueueManager(NEMId id,
                                                          PlatformServiceProvider * pPlatformServiceProvider):
  QueueManager{id,pPlatformServiceProvider},
  pImpl_{new Implementation{}}{}

EMANE::Models::TDMA::BasicQueueManager::~BasicQueueManager(){}

void EMANE::Models::TDMA::BasicQueueManager::initialize(Registrar & registrar)
{
  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          DEBUG_LEVEL,
                          "MACI %03hu TDMA::BasicQueueManager::%s",
                          id_,
                          __func__);

  auto & configRegistrar = registrar.configurationRegistrar();

  configRegistrar.registerNumeric<std::uint16_t>("queue.depth",
                                                 ConfigurationProperties::DEFAULT,
                                                 {256},
                                                 "Defines the size of the per service class downstream packet"
                                                 " queues (in packets). Each of the 5 queues (control + 4"
                                                 " service classes) will be 'queuedepth' size.");

  configRegistrar.registerNumeric<bool>("queue.aggregationenable",
                                        ConfigurationProperties::DEFAULT,
                                        {true},
                                        "Defines whether packet aggregation is enabled for transmission. When"
                                        " enabled, multiple packets can be sent in the same transmission when"
                                        " there is additional room within the slot.");


  configRegistrar.registerNumeric<bool>("queue.fragmentationenable",
                                        ConfigurationProperties::DEFAULT,
                                        {true},
                                        "Defines whether packet fragmentation is enabled. When enabled, a single"
                                        " packet will be fragmented into multiple message components to be sent"
                                        " over multiple transmissions when the slot is too small.  When disabled"
                                        " and the packet matches the traffic class for the transmit slot as"
                                        " defined in the TDMA schedule, the packet will be discarded.");


  configRegistrar.registerNumeric<bool>("queue.strictdequeueenable",
                                        ConfigurationProperties::DEFAULT,
                                        {false},
                                        "Defines whether packets will be dequeued from a queue other than what"
                                        " has been specified when there are no eligible packets for dequeue in"
                                        " the specified queue. Queues are dequeued highest priority first.");

  configRegistrar.registerNumeric<double>("queue.aggregationslotthreshold",
                                          ConfigurationProperties::DEFAULT,
                                          {90.0},
                                          "Defines the percentage of a slot that must be filled in order to conclude"
                                          " aggregation when queue.aggregationenable is enabled.",
                                          0,
                                          100.0);

  auto & statisticRegistrar = registrar.statisticRegistrar();

  pImpl_->queueStatusPublisher_.registerStatistics(statisticRegistrar);

}

void EMANE::Models::TDMA::BasicQueueManager::configure(const ConfigurationUpdate & update)
{
  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          DEBUG_LEVEL,
                          "MACI %03hu TDMA::BasicQueueManager::%s",
                          id_,
                          __func__);

  std::uint16_t u16QueueDepth{};
  bool bAggregationEnable_{true};
  bool bFragmentationEnable_{true};
  bool bStrictDequeueEnable_{false};
  double dAggregationSlotThreshold_{90.0};

  for(const auto & item : update)
    {
      if(item.first == "queue.depth")
        {
          u16QueueDepth = item.second[0].asUINT16();
        }
      else if(item.first == "queue.aggregationenable")
        {
          bAggregationEnable_ = item.second[0].asBool();
        }
      else if(item.first == "queue.fragmentationenable")
        {
          bFragmentationEnable_ = item.second[0].asBool();
        }
      else if(item.first == "queue.strictdequeueenable")
        {
          bStrictDequeueEnable_ = item.second[0].asBool();
        }
      else if(item.first == "queue.aggregationslotthreshold")
        {
          dAggregationSlotThreshold_ = item.second[0].asDouble();
        }
      else
        {
          throw makeException<ConfigureException>("TDMA::BasicQueueManager: "
                                                   "Unexpected configuration item %s",
                                                   item.first.c_str());
        }
    }

  basic_queue_manager_set_config(
      pImpl_->pRustQueueManager_,
      u16QueueDepth,
      bAggregationEnable_,
      bFragmentationEnable_,
      bStrictDequeueEnable_,
      dAggregationSlotThreshold_
  );
}

void EMANE::Models::TDMA::BasicQueueManager::start()
{
  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          DEBUG_LEVEL,
                          "MACI %03hu TDMA::BasicQueueManager::%s",
                          id_,
                          __func__);
}

void EMANE::Models::TDMA::BasicQueueManager::postStart()
{
  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          DEBUG_LEVEL,
                          "MACI %03hu TDMA::BasicQueueManager::%s",
                          id_,
                          __func__);
}

void EMANE::Models::TDMA::BasicQueueManager::stop()
{
  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          DEBUG_LEVEL,
                          "MACI %03hu TDMA::BasicQueueManager::%s",
                          id_,
                          __func__);
}

void EMANE::Models::TDMA::BasicQueueManager::destroy() throw()
{
  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          DEBUG_LEVEL,
                          "MACI %03hu TDMA::BasicQueueManager::%s",
                          id_,
                          __func__);
}

size_t EMANE::Models::TDMA::BasicQueueManager::enqueue(std::uint8_t u8QueueIndex,
                                                     DownstreamPacket && pkt)
{
  size_t packetsDropped{};

  if(u8QueueIndex < 5)
    {
      DownstreamPacket * pPkt{new DownstreamPacket{std::move(pkt)}};
      NEMId dest{pPkt->getPacketInfo().getDestination()};
      
      EnqueueResult res = basic_queue_manager_enqueue(
          pImpl_->pRustQueueManager_,
          u8QueueIndex,
          pPkt,
          dest,
          pPkt->length(),
          pPkt->getPacketInfo().getPriority()
      );

      if(res.dropped)
        {
          packetsDropped = 1;
          pImpl_->queueStatusPublisher_.drop(u8QueueIndex,
                                             QueueStatusPublisher::DropReason::DROP_OVERFLOW,
                                             1);

          DownstreamPacket* pDropped = reinterpret_cast<DownstreamPacket*>(res.dropped_pkt);
          const auto & pktInfo = pDropped->getPacketInfo();

          pPacketStatusPublisher_->outbound(pktInfo.getSource(),
                                            pktInfo.getDestination(),
                                            pktInfo.getPriority(),
                                            pDropped->length(),
                                            PacketStatusPublisher::OutboundAction::DROP_OVERFLOW);
          delete pDropped;
        }

      pImpl_->queueStatusPublisher_.enqueue(u8QueueIndex);
    }

  return packetsDropped;
}

namespace {
  std::pair<EMANE::Models::TDMA::MessageComponent,size_t>
  fragmentPacket(EMANE::DownstreamPacket * pPacket, const DequeueAction& act)
  {
    size_t totalBytesVisited{};
    size_t totalBytesCopied{};
    EMANE::Utils::VectorIO vectorIOs{};
    
    size_t offset = act.fragment_offset;
    size_t bytes = act.fragment_size;

    for(const auto & entry : pPacket->getVectorIO())
      {
        if(totalBytesCopied < bytes)
          {
            if(totalBytesVisited + entry.iov_len < offset)
              {
                totalBytesVisited += entry.iov_len;
              }
            else
              {
                char * pBuf{reinterpret_cast<char *>(entry.iov_base)};
                auto cur_offset = offset - totalBytesVisited;
                auto remainder = entry.iov_len - cur_offset;

                if(totalBytesVisited < offset)
                  {
                    totalBytesVisited = offset;
                  }

                size_t amountToCopy{};
                if(remainder > bytes - totalBytesCopied)
                  {
                    amountToCopy =  bytes - totalBytesCopied;
                    remainder -= amountToCopy;
                  }
                else
                  {
                    amountToCopy = remainder;
                  }

                vectorIOs.push_back(EMANE::Utils::make_iovec(pBuf+cur_offset,amountToCopy));

                offset += amountToCopy;
                totalBytesVisited += amountToCopy;
                totalBytesCopied += amountToCopy;
              }
          }
        else
          {
            break;
          }
      }

    EMANE::Models::TDMA::MessageComponent component{act.is_control ?
        EMANE::Models::TDMA::MessageComponent::Type::CONTROL :
        EMANE::Models::TDMA::MessageComponent::Type::DATA,
        act.dest,
        act.priority,
        vectorIOs,
        act.fragment_index,
        act.fragment_offset,
        act.seq,
        act.more_fragments};

    return {component,totalBytesCopied};
  }
}

std::tuple<EMANE::Models::TDMA::MessageComponents,size_t>
EMANE::Models::TDMA::BasicQueueManager::dequeue(std::uint8_t u8QueueIndex,
                                                size_t requestedBytes,
                                                NEMId destination)
{
  MessageComponents components{};
  size_t totalLength{};

  if(u8QueueIndex < 5)
    {
      DequeueResult res{};
      basic_queue_manager_dequeue(pImpl_->pRustQueueManager_, u8QueueIndex, requestedBytes, destination, &res);
      
      totalLength = res.total_bytes;
      
      std::map<std::uint8_t, MessageComponents> queue_parts;
      
      for (size_t i = 0; i < res.num_actions; ++i) {
          auto& act = res.actions[i];
          DownstreamPacket* pPkt = reinterpret_cast<DownstreamPacket*>(act.pkt_ptr);
          if (act.action_type == 0) {
              // DROP
              const auto & pktInfo = pPkt->getPacketInfo();
              pPacketStatusPublisher_->outbound(pktInfo.getSource(),
                                                pktInfo.getDestination(),
                                                pktInfo.getPriority(),
                                                pPkt->length(),
                                                PacketStatusPublisher::OutboundAction::DROP_TOO_BIG);

              pImpl_->queueStatusPublisher_.drop(act.queue_index,
                                                 QueueStatusPublisher::DropReason::DROP_TOOBIG,
                                                 1);
              delete pPkt;
          } else {
              // DEQUEUE
              if (act.action_type == 1) { // FULL
                  if (act.fragment_offset > 0) {
                      auto ret = fragmentPacket(pPkt, act);
                      queue_parts[act.queue_index].push_back(std::move(ret.first));
                      delete pPkt;
                  } else {
                      queue_parts[act.queue_index].push_back({act.is_control ?
                            MessageComponent::Type::CONTROL :
                            MessageComponent::Type::DATA,
                            act.dest,
                            act.priority,
                            pPkt->getVectorIO(),
                            act.fragment_index,
                            act.fragment_offset,
                            act.seq,
                            false});
                      delete pPkt;
                  }
              } else if (act.action_type == 2) { // FRAGMENT
                  auto ret = fragmentPacket(pPkt, act);
                  queue_parts[act.queue_index].push_back(std::move(ret.first));
              }
          }
      }
      
      for (auto& pair : queue_parts) {
          pImpl_->queueStatusPublisher_.dequeue(u8QueueIndex, pair.first, pair.second);
          components.splice(components.end(), pair.second);
      }
      
      tdma_queue_free_dequeue_result(&res);
    }

  pPacketStatusPublisher_->outbound(id_,
                                    components,
                                    PacketStatusPublisher::OutboundAction::ACCEPT_GOOD);

  return std::make_tuple(std::move(components),totalLength);
}

EMANE::Models::TDMA::QueueInfos
EMANE::Models::TDMA::BasicQueueManager::getPacketQueueInfo() const
{
  QueueInfos queueInfos{};
  QueueStatus statuses[5];
  
  basic_queue_manager_get_status(pImpl_->pRustQueueManager_, statuses);
  
  for(int i = 0; i < 5; ++i)
    {
      queueInfos.push_back({static_cast<std::uint8_t>(i),
            statuses[i].packets,
            statuses[i].bytes});
    }

  return queueInfos;
}
