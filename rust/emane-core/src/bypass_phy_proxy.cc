#include "emane/phylayerimpl.h"
#include "emane/phytypes.h"
#include "emane/utils/commonlayerstatistics.h"
#include "emane/configureexception.h"

extern "C" void* emane_rs_bypass_phy_create(uint16_t id, EMANE::PlatformServiceProvider* pPlatformService, void* cpp_this);
extern "C" void emane_rs_bypass_phy_destroy(void* state);
extern "C" void emane_rs_bypass_phy_process_downstream(void* state, EMANE::DownstreamPacket* pkt);
extern "C" void emane_rs_bypass_phy_process_upstream(void* state, EMANE::UpstreamPacket* pkt);

namespace EMANE {
  namespace Models {
    namespace Bypass {
      class PHYLayer : public PHYLayerImplementor {
      public:
        PHYLayer(NEMId id, PlatformServiceProvider* pPlatformService) : PHYLayerImplementor(id, pPlatformService), commonLayerStatistics_{{}} {
            rs_state_ = emane_rs_bypass_phy_create(id, pPlatformService, this);
        }
        ~PHYLayer() override {
            emane_rs_bypass_phy_destroy(rs_state_);
        }

        void initialize(Registrar & registrar) override {
            commonLayerStatistics_.registerStatistics(registrar.statisticRegistrar());
        }

        void configure(const ConfigurationUpdate & update) override {
            if(!update.empty()) {
                throw ConfigureException("Models::Bypass::PHYLayer: Unexpected configuration items.");
            }
        }

        void start() override {}
        void stop() override {}
        void destroy() throw() override {}

        void processUpstreamPacket(const CommonPHYHeader &, UpstreamPacket & pkt, const ControlMessages &) override {
            emane_rs_bypass_phy_process_upstream(rs_state_, &pkt);
        }

        void processDownstreamControl(const ControlMessages &) override {}

        void processDownstreamPacket(DownstreamPacket & pkt, const ControlMessages &) override {
            emane_rs_bypass_phy_process_downstream(rs_state_, &pkt);
        }

        void processEvent(const EventId &, const Serialization &) override {}
        void processTimedEvent(TimerEventId, const TimePoint &, const TimePoint &, const TimePoint &, const void *) override {}

        void processDownstream_proxy(DownstreamPacket & pkt) {
            TimePoint beginTime{Clock::now()};
            commonLayerStatistics_.processInbound(pkt);
            CommonPHYHeader hdr{REGISTERED_EMANE_PHY_BYPASS, 0, 0, Clock::now(), {FrequencySegments{{0,Microseconds::zero()}}}, {}, Transmitters{{id_,0}}, {}};
            commonLayerStatistics_.processOutbound(pkt, std::chrono::duration_cast<Microseconds>(Clock::now() - beginTime));
            sendDownstreamPacket(hdr, pkt);
        }

        void processUpstream_proxy(UpstreamPacket & pkt) {
            TimePoint beginTime{Clock::now()};
            commonLayerStatistics_.processInbound(pkt);
            commonLayerStatistics_.processOutbound(pkt, std::chrono::duration_cast<Microseconds>(Clock::now() - beginTime));
            sendUpstreamPacket(pkt);
        }

      private:
        void* rs_state_;
        Utils::CommonLayerStatistics commonLayerStatistics_;
      };
    }
  }
}

// These are exported so the EMANE plugin loader can find them natively when dlopen-ing the Rust library!
extern "C" EMANE::PHYLayerImplementor* emane_bypass_phy_create_shim(EMANE::NEMId id, EMANE::PlatformServiceProvider* pPlatformService) {
    return new EMANE::Models::Bypass::PHYLayer(id, pPlatformService);
}

extern "C" void emane_bypass_phy_destroy_shim(EMANE::PHYLayerImplementor* p) {
    delete p;
}

// Called by Rust
extern "C" void emane_bypass_phy_proxy_process_downstream(void* impl, void* pkt) {
    auto layer = static_cast<EMANE::Models::Bypass::PHYLayer*>(impl);
    layer->processDownstream_proxy(*static_cast<EMANE::DownstreamPacket*>(pkt));
}

extern "C" void emane_bypass_phy_proxy_process_upstream(void* impl, void* pkt) {
    auto layer = static_cast<EMANE::Models::Bypass::PHYLayer*>(impl);
    layer->processUpstream_proxy(*static_cast<EMANE::UpstreamPacket*>(pkt));
}
