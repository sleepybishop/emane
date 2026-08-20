#include "emane/maclayerimpl.h"
#include "emane/mactypes.h"
#include "emane/configureexception.h"
#include "emane/utils/commonlayerstatistics.h"

// Forward declare the Rust FFI functions that currently exist in libemane/rust_ffi.h
// Wait, they exist in src/libemane/rust_ffi.h!
#include "../../../src/libemane/rust_ffi.h"

namespace EMANE {
  namespace Models {
    namespace Bypass {
      class MACLayer : public MACLayerImplementor {
      public:
        MACLayer(NEMId id, PlatformServiceProvider* pPlatformService, RadioServiceProvider* pRadioServiceProvider)
          : MACLayerImplementor(id, pPlatformService, pRadioServiceProvider),
            rs_state_(emane_rs_bypass_mac_new(REGISTERED_EMANE_MAC_BYPASS)),
            commonLayerStatistics_({"Reg Id"}) {}

        ~MACLayer() {
          emane_rs_bypass_mac_free(rs_state_);
        }

        void initialize(Registrar& registrar) override {
          commonLayerStatistics_.registerStatistics(registrar.statisticRegistrar());
        }

        void configure(const ConfigurationUpdate& update) override {
          if (!update.empty()) {
            throw ConfigureException("Models::Bypass::MACLayer: Unexpected configuration items.");
          }
        }

        void start() override {}
        void stop() override {}
        void destroy() throw() override {}

        void processUpstreamControl(const ControlMessages&) override {}
        void processDownstreamControl(const ControlMessages&) override {}

        void processUpstreamPacket(const CommonMACHeader& hdr,
                                   UpstreamPacket& pkt,
                                   const ControlMessages&) override {
          auto beginTime = Clock::now();
          commonLayerStatistics_.processInbound(pkt);

          if (!emane_rs_bypass_mac_process_upstream(rs_state_, hdr.getRegistrationId())) {
            commonLayerStatistics_.processOutbound(pkt,
              std::chrono::duration_cast<Microseconds>(Clock::now() - beginTime), 1);
            return; // drop
          }

          commonLayerStatistics_.processOutbound(pkt,
            std::chrono::duration_cast<Microseconds>(Clock::now() - beginTime));
          sendUpstreamPacket(pkt);
        }

        void processDownstreamPacket(DownstreamPacket& pkt,
                                     const ControlMessages&) override {
          auto beginTime = Clock::now();
          commonLayerStatistics_.processInbound(pkt);
          commonLayerStatistics_.processOutbound(pkt,
            std::chrono::duration_cast<Microseconds>(Clock::now() - beginTime));

          uint16_t seq = emane_rs_bypass_mac_process_downstream(rs_state_);
          sendDownstreamPacket(CommonMACHeader{REGISTERED_EMANE_MAC_BYPASS, seq}, pkt);
        }

        void processEvent(const EventId&, const Serialization&) override {}
        void processTimedEvent(TimerEventId, const TimePoint&, const TimePoint&, const TimePoint&, const void*) override {}

      private:
        ::FfiBypassMac* rs_state_;
        Utils::CommonLayerStatistics commonLayerStatistics_;
      };
    }
  }
}

extern "C" void* bypass_mac_create(EMANE::NEMId id, EMANE::PlatformServiceProvider* pPlatformService, EMANE::RadioServiceProvider* pRadioServiceProvider) {
  return new EMANE::Models::Bypass::MACLayer(id, pPlatformService, pRadioServiceProvider);
}

extern "C" void bypass_mac_destroy(void* p) {
  delete static_cast<EMANE::Models::Bypass::MACLayer*>(p);
}
