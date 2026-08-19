#ifndef EMANETRANSPORTSVIRTUALVIRTUALTRANSPORT_HEADER_
#define EMANETRANSPORTSVIRTUALVIRTUALTRANSPORT_HEADER_

#include "ethernettransport.h"
#include "emane/inetaddr.h"
#include "emane/utils/bitpool.h"
#include "emane/flowcontrolclient.h"
#include "emane/utils/commonlayerstatistics.h"

namespace EMANE {
  namespace Transports {
    namespace Virtual {
      class VirtualTransport : public Ethernet::EthernetTransport {
      public:
        VirtualTransport(NEMId id, PlatformServiceProvider *pPlatformService);
        ~VirtualTransport();

        void initialize(Registrar & registrar) override;
        void configure(const ConfigurationUpdate & update) override;
        void start() override;
        void postStart() override;
        void stop() override;
        void destroy() throw() override;

        void processUpstreamPacket(UpstreamPacket & pkt, const ControlMessages & msgs) override;
        void processUpstreamControl(const ControlMessages & msgs) override;

        void sendDownstreamPacket_cb(const uint8_t* buf, size_t len);

      private:
        INETAddr address_;
        INETAddr mask_;
        std::string sDeviceName_;
        std::string sDevicePath_;
        bool bARPMode_;
        bool bBroadcastMode_;
        bool bArpCacheMode_;
        
        Utils::BitPool * pBitPool_;
        FlowControlClient flowControlClient_;
        bool bFlowControlEnable_;
        std::uint64_t u64BitRate_;
        Utils::CommonLayerStatistics commonLayerStatistics_;

        void* rust_obj_;

        void handleUpstreamControl(const ControlMessages & msgs);
      };
    }
  }
}
#endif // EMANETRANSPORTSVIRTUALVIRTUALTRANSPORT_HEADER_
