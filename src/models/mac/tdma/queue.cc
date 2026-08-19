
#include "queue.h"

extern "C" {
    void* tdma_queue_new();
    void tdma_queue_free(void* q);
    void tdma_queue_initialize(void* q, std::uint16_t queue_depth, bool fragment, bool aggregate, bool is_control);

    struct EnqueueResult {
        void* dropped_pkt;
        bool dropped;
    };
    EnqueueResult tdma_queue_enqueue(void* q, void* pkt_ptr, std::uint16_t dest, size_t length, std::uint8_t priority);

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

    void tdma_queue_dequeue(void* q, size_t requested_bytes, std::uint16_t destination, bool b_drop, DequeueResult* res);
    void tdma_queue_free_dequeue_result(DequeueResult* res);
    void tdma_queue_get_status(const void* q, size_t* packets, size_t* bytes);
}

EMANE::Models::TDMA::Queue::Queue():
  pRustQueue_{tdma_queue_new()} {}

EMANE::Models::TDMA::Queue::~Queue() {
  if (pRustQueue_) {
    tdma_queue_free(pRustQueue_);
    pRustQueue_ = nullptr;
  }
}

void EMANE::Models::TDMA::Queue::initialize(std::uint16_t u16QueueDepth,
                                            bool bFragment,
                                            bool bAggregate,
                                            bool bIsControl)
{
  tdma_queue_initialize(pRustQueue_, u16QueueDepth, bFragment, bAggregate, bIsControl);
}

std::pair<std::unique_ptr<EMANE::DownstreamPacket>,bool>
  EMANE::Models::TDMA::Queue::enqueue(DownstreamPacket && pkt)
{
  DownstreamPacket * pPkt{new DownstreamPacket{std::move(pkt)}};
  NEMId dest{pPkt->getPacketInfo().getDestination()};
  
  EnqueueResult res = tdma_queue_enqueue(pRustQueue_, pPkt, dest, pPkt->length(), pPkt->getPacketInfo().getPriority());
  
  std::unique_ptr<DownstreamPacket> pDroppedPacket;
  if (res.dropped_pkt) {
      pDroppedPacket.reset(reinterpret_cast<DownstreamPacket*>(res.dropped_pkt));
  }
  return {std::move(pDroppedPacket), res.dropped};
}

std::tuple<EMANE::Models::TDMA::MessageComponents,
           size_t,
           std::list<std::unique_ptr<EMANE::DownstreamPacket>>>
  EMANE::Models::TDMA::Queue::dequeue(size_t requestedBytes, NEMId destination,bool bDrop)
{
  MessageComponents components{};
  std::list<std::unique_ptr<DownstreamPacket>> dropped;
  
  DequeueResult res{};
  tdma_queue_dequeue(pRustQueue_, requestedBytes, destination, bDrop, &res);
  
  for (size_t i = 0; i < res.num_actions; ++i) {
      auto& act = res.actions[i];
      DownstreamPacket* pPkt = reinterpret_cast<DownstreamPacket*>(act.pkt_ptr);
      if (act.action_type == 0) {
          // Drop
          dropped.push_back(std::unique_ptr<DownstreamPacket>{pPkt});
      } else {
          // Dequeue
          if (act.action_type == 1) { // FULL
              if (act.fragment_offset > 0) {
                  // Final fragment of a fragmented packet
                  auto ret = fragmentPacket(pPkt, act);
                  components.push_back(std::move(ret.first));
                  delete pPkt;
              } else {
                  components.push_back({act.is_control ?
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
              components.push_back(std::move(ret.first));
          }
      }
  }
  
  size_t totalBytes = res.total_bytes;
  tdma_queue_free_dequeue_result(&res);
  
  return std::make_tuple(std::move(components), totalBytes, std::move(dropped));
}

std::pair<EMANE::Models::TDMA::MessageComponent,size_t>
EMANE::Models::TDMA::Queue::fragmentPacket(DownstreamPacket * pPacket,
                                           const DequeueAction& act)
{
  size_t totalBytesVisited{};
  size_t totalBytesCopied{};
  Utils::VectorIO vectorIOs{};
  
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

              vectorIOs.push_back(Utils::make_iovec(pBuf+cur_offset,amountToCopy));

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

  MessageComponent component{act.is_control ?
      MessageComponent::Type::CONTROL :
      MessageComponent::Type::DATA,
      act.dest,
      act.priority,
      vectorIOs,
      act.fragment_index,
      act.fragment_offset,
      act.seq,
      act.more_fragments};

  return {component,totalBytesCopied};
}

std::tuple<size_t,size_t> EMANE::Models::TDMA::Queue::getStatus() const
{
  size_t packets = 0, bytes = 0;
  tdma_queue_get_status(pRustQueue_, &packets, &bytes);
  return std::make_tuple(packets, bytes);
}
