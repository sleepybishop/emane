
#ifndef RFPIPEMAC_MACLAYER_HEADER_
#define RFPIPEMAC_MACLAYER_HEADER_

#include "emane/maclayerimpl.h"
#include "emane/mactypes.h"

namespace EMANE {
  namespace Models {
    namespace RFPipe {
      class MACLayer : public MACLayerImplementor {
      public:
        MACLayer(NEMId id, PlatformServiceProvider *pPlatformServiceProvider, RadioServiceProvider * pRadioServiceProvider);
        ~MACLayer();

        void initialize(Registrar & registrar) override;
        void configure(const ConfigurationUpdate & update) override;
        void start() override;
        void postStart() override;
        void stop() override;
        void destroy() throw() override;
        void processUpstreamControl(const ControlMessages & msgs) override;
        void processUpstreamPacket(const CommonMACHeader & hdr, UpstreamPacket & pkt, const ControlMessages & msgs) override;
        void processDownstreamControl(const ControlMessages & msgs) override;
        void processDownstreamPacket(DownstreamPacket & pkt, const ControlMessages & msgs) override;
        void processConfiguration(const ConfigurationUpdate & update) override;
        void processEvent(const EventId & eventId, const Serialization & serialization) override;
        void processTimedEvent(TimerEventId eventId, const TimePoint & expireTime, const TimePoint & scheduleTime, const TimePoint & fireTime, const void * arg) override;

        void doSendUpstreamPacket(UpstreamPacket & pkt, const ControlMessages & msgs);
        void doSendDownstreamPacket(const CommonMACHeader & hdr, DownstreamPacket & pkt, const ControlMessages & msgs);
        void doSendUpstreamControl(const ControlMessages & msgs);
        void doSendDownstreamControl(const ControlMessages & msgs);

      private:
        void* rs_state_;
      };
    }
  }
}
#endif //RFPIPEMAC_MACLAYER_HEADER_
