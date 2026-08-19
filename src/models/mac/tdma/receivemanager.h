#ifndef EMANEMODELSTDMARECEIVEMANAGER_HEADER_
#define EMANEMODELSTDMARECEIVEMANAGER_HEADER_

#include "emane/types.h"
#include "emane/upstreampacket.h"
#include "emane/downstreamtransport.h"
#include "emane/radioserviceprovider.h"
#include "emane/logserviceprovider.h"
#include "emane/frequencysegment.h"
#include "emane/neighbormetricmanager.h"
#include "emane/models/tdma/scheduler.h"
#include "basemodelmessage.h"
#include "emane/models/tdma/packetstatuspublisher.h"

namespace EMANE
{
  namespace Models
  {
    namespace TDMA
    {
      class ReceiveManager
      {
      public:
        ReceiveManager(NEMId id,
                       DownstreamTransport * pDownstreamTransport,
                       LogServiceProvider * pLogService,
                       RadioServiceProvider * pRadioService,
                       Scheduler * pScheduler,
                       PacketStatusPublisher * pPacketStatusPublisher,
                       NeighborMetricManager *pNeighborMetricManager);

        ~ReceiveManager();

        void setFragmentCheckThreshold(const std::chrono::seconds & threshold);

        void setFragmentTimeoutThreshold(const std::chrono::seconds & threshold);

        void setPromiscuousMode(bool bEnable);

        void loadCurves(const std::string & sPCRFileName);

        bool enqueue(BaseModelMessage && baseModelMessage,
                     const PacketInfo & pktInfo,
                     size_t length,
                     const TimePoint & startOfReception,
                     const FrequencySegments & frequencySegments,
                     const Microseconds & span,
                     const TimePoint & beginTime,
                     std::uint64_t u64PacketSequence);

        void process(std::uint64_t u64AbsoluteSlotIndex);

        // Accessors for callbacks
        LogServiceProvider* getLogService() { return pLogService_; }
        RadioServiceProvider* getRadioService() { return pRadioService_; }
        PacketStatusPublisher* getPacketStatusPublisher() { return pPacketStatusPublisher_; }
        NeighborMetricManager* getNeighborMetricManager() { return pNeighborMetricManager_; }
        DownstreamTransport* getDownstreamTransport() { return pDownstreamTransport_; }
        Scheduler* getScheduler() { return pScheduler_; }
        NEMId getId() const { return id_; }

      private:
        NEMId id_;
        DownstreamTransport * pDownstreamTransport_;
        LogServiceProvider * pLogService_;
        RadioServiceProvider * pRadioService_;
        Scheduler * pScheduler_;
        PacketStatusPublisher * pPacketStatusPublisher_;
        NeighborMetricManager * pNeighborMetricManager_;

        void* rm_;

        ReceiveManager(const ReceiveManager &) = delete;
        ReceiveManager & operator=(const ReceiveManager &) = delete;
      };
    }
  }
}
#endif // EMANEMODELSTDMARECEIVEMANAGER_HEADER_
