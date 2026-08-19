#ifndef EMANEMODELSTDMAQUEUE_HEADER_
#define EMANEMODELSTDMAQUEUE_HEADER_

#include "emane/types.h"
#include "emane/downstreampacket.h"
#include "emane/models/tdma/messagecomponent.h"

#include <cstdint>
#include <map>

struct DequeueAction;

namespace EMANE
{
  namespace Models
  {
    namespace TDMA
    {
      class Queue
      {
      public:
        Queue();
        ~Queue(); // added destructor for Rust FFI cleanup

        void initialize(std::uint16_t u16QueueDepth,
                        bool bFragment,
                        bool bAggregate,
                        bool bIsControl);

        std::pair<std::unique_ptr<DownstreamPacket>,bool>
        enqueue(DownstreamPacket && pkt);

        std::tuple<MessageComponents,
                   size_t,
                   std::list<std::unique_ptr<DownstreamPacket>>>
          dequeue(size_t requestedBytes,NEMId destination,bool bDrop);

        // packets, bytes
        std::tuple<size_t,size_t> getStatus() const;

      private:
        void* pRustQueue_{nullptr};

        std::pair<MessageComponent,size_t> fragmentPacket(DownstreamPacket * pPacket,
                                                          const DequeueAction& act);
      };
    }
  }
}

#endif // EMANEMODELSTDMAQUEUE_HEADER_
