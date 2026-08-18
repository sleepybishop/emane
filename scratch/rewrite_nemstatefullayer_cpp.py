cpp_code = """
#include "nemstatefullayer.h"
#include "logservice.h"

extern "C" {
    void* emane_rs_nem_stateful_layer_new(void* inner_layer);
    void emane_rs_nem_stateful_layer_destroy(void* ptr);
    void emane_rs_nem_stateful_layer_initialize(void* ptr, void* registrar);
    void emane_rs_nem_stateful_layer_configure(void* ptr, const void* update);
    void emane_rs_nem_stateful_layer_start(void* ptr);
    void emane_rs_nem_stateful_layer_post_start(void* ptr);
    void emane_rs_nem_stateful_layer_stop(void* ptr);
    void emane_rs_nem_stateful_layer_handle_destroy(void* ptr);
    void emane_rs_nem_stateful_layer_process_configuration(void* ptr, const void* update);
    void emane_rs_nem_stateful_layer_process_downstream_control(void* ptr, const void* msgs);
    void emane_rs_nem_stateful_layer_process_downstream_packet(void* ptr, void* pkt, const void* msgs);
    void emane_rs_nem_stateful_layer_process_upstream_packet(void* ptr, void* pkt, const void* msgs);
    void emane_rs_nem_stateful_layer_process_upstream_control(void* ptr, const void* msgs);
    void emane_rs_nem_stateful_layer_process_event(void* ptr, const void* id, const void* serialization);
    void emane_rs_nem_stateful_layer_process_timed_event(void* ptr, uint32_t timer_id, const void* expire, const void* schedule, const void* fire, const void* arg);
    void emane_rs_nem_stateful_layer_set_upstream_transport(void* ptr, void* t);
    void emane_rs_nem_stateful_layer_set_downstream_transport(void* ptr, void* t);

    // Callbacks to inner layer
    void emane_c_nem_layer_initialize(void* layer, void* registrar) {
        static_cast<EMANE::NEMLayer*>(layer)->initialize(*static_cast<EMANE::Registrar*>(registrar));
    }
    void emane_c_nem_layer_configure(void* layer, const void* update) {
        static_cast<EMANE::NEMLayer*>(layer)->configure(*static_cast<const EMANE::ConfigurationUpdate*>(update));
    }
    void emane_c_nem_layer_start(void* layer) {
        static_cast<EMANE::NEMLayer*>(layer)->start();
    }
    void emane_c_nem_layer_post_start(void* layer) {
        static_cast<EMANE::NEMLayer*>(layer)->postStart();
    }
    void emane_c_nem_layer_stop(void* layer) {
        static_cast<EMANE::NEMLayer*>(layer)->stop();
    }
    void emane_c_nem_layer_destroy(void* layer) {
        static_cast<EMANE::NEMLayer*>(layer)->destroy();
    }
    void emane_c_nem_layer_process_configuration(void* layer, const void* update) {
        static_cast<EMANE::NEMLayer*>(layer)->processConfiguration(*static_cast<const EMANE::ConfigurationUpdate*>(update));
    }
    void emane_c_nem_layer_process_downstream_control(void* layer, const void* msgs) {
        static_cast<EMANE::NEMLayer*>(layer)->processDownstreamControl(*static_cast<const EMANE::ControlMessages*>(msgs));
    }
    void emane_c_nem_layer_process_downstream_packet(void* layer, void* pkt, const void* msgs) {
        static_cast<EMANE::NEMLayer*>(layer)->processDownstreamPacket(*static_cast<EMANE::DownstreamPacket*>(pkt), *static_cast<const EMANE::ControlMessages*>(msgs));
    }
    void emane_c_nem_layer_process_upstream_packet(void* layer, void* pkt, const void* msgs) {
        static_cast<EMANE::NEMLayer*>(layer)->processUpstreamPacket(*static_cast<EMANE::UpstreamPacket*>(pkt), *static_cast<const EMANE::ControlMessages*>(msgs));
    }
    void emane_c_nem_layer_process_upstream_control(void* layer, const void* msgs) {
        static_cast<EMANE::NEMLayer*>(layer)->processUpstreamControl(*static_cast<const EMANE::ControlMessages*>(msgs));
    }
    void emane_c_nem_layer_process_event(void* layer, const void* id, const void* serialization) {
        static_cast<EMANE::NEMLayer*>(layer)->processEvent(*static_cast<const EMANE::EventId*>(id), *static_cast<const EMANE::Serialization*>(serialization));
    }
    void emane_c_nem_layer_process_timed_event(void* layer, uint32_t timer_id, const void* expire, const void* schedule, const void* fire, const void* arg) {
        static_cast<EMANE::NEMLayer*>(layer)->processTimedEvent(
            timer_id,
            *static_cast<const EMANE::TimePoint*>(expire),
            *static_cast<const EMANE::TimePoint*>(schedule),
            *static_cast<const EMANE::TimePoint*>(fire),
            arg
        );
    }
    void emane_c_nem_layer_set_upstream_transport(void* layer, void* t) {
        static_cast<EMANE::NEMLayer*>(layer)->setUpstreamTransport(static_cast<EMANE::UpstreamTransport*>(t));
    }
    void emane_c_nem_layer_set_downstream_transport(void* layer, void* t) {
        static_cast<EMANE::NEMLayer*>(layer)->setDownstreamTransport(static_cast<EMANE::DownstreamTransport*>(t));
    }
    
    void emane_c_log_error(const char* msg) {
        LOGGER_STANDARD_LOGGING(*EMANE::LogServiceSingleton::instance(), EMANE::ERROR_LEVEL, "%s", msg);
    }
}

EMANE::NEMStatefulLayer::NEMStatefulLayer(NEMId id, 
                                          NEMLayer * pLayer, 
                                          EMANE::PlatformServiceProvider *pPlatformService):
  NEMLayer(id, pPlatformService),
  pLayer_(pLayer)
{
    // Initialize Rust state machine
    pState_ = reinterpret_cast<NEMLayerState*>(emane_rs_nem_stateful_layer_new(pLayer_.get()));
}

EMANE::NEMStatefulLayer::~NEMStatefulLayer(){
    emane_rs_nem_stateful_layer_destroy(pState_);
}
    
void EMANE::NEMStatefulLayer::initialize(Registrar & registrar) {
    emane_rs_nem_stateful_layer_initialize(pState_, &registrar);
}

void EMANE::NEMStatefulLayer::configure(const ConfigurationUpdate & update) {
    emane_rs_nem_stateful_layer_configure(pState_, &update);
}

void EMANE::NEMStatefulLayer::start() {
    emane_rs_nem_stateful_layer_start(pState_);
}

void EMANE::NEMStatefulLayer::postStart() {
    emane_rs_nem_stateful_layer_post_start(pState_);
}

void EMANE::NEMStatefulLayer::stop() {
    emane_rs_nem_stateful_layer_stop(pState_);
}

void EMANE::NEMStatefulLayer::destroy() throw() {
    emane_rs_nem_stateful_layer_handle_destroy(pState_);
}

void EMANE::NEMStatefulLayer::processConfiguration(const ConfigurationUpdate & update) {
    emane_rs_nem_stateful_layer_process_configuration(pState_, &update);
}

void EMANE::NEMStatefulLayer::processDownstreamControl(const ControlMessages & msgs) {
    emane_rs_nem_stateful_layer_process_downstream_control(pState_, &msgs);
}

void EMANE::NEMStatefulLayer::processDownstreamPacket(DownstreamPacket & pkt, const ControlMessages & msgs) {
    emane_rs_nem_stateful_layer_process_downstream_packet(pState_, &pkt, &msgs);
}

void EMANE::NEMStatefulLayer::processUpstreamPacket(UpstreamPacket & pkt, const ControlMessages & msgs) {
    emane_rs_nem_stateful_layer_process_upstream_packet(pState_, &pkt, &msgs);
}

void EMANE::NEMStatefulLayer::processUpstreamControl(const ControlMessages & msgs) {
    emane_rs_nem_stateful_layer_process_upstream_control(pState_, &msgs);
}

void EMANE::NEMStatefulLayer::processEvent(const EventId & id, const Serialization & serialization) {
    emane_rs_nem_stateful_layer_process_event(pState_, &id, &serialization);
}

void EMANE::NEMStatefulLayer::setUpstreamTransport(UpstreamTransport * pUpstreamTransport) {
    emane_rs_nem_stateful_layer_set_upstream_transport(pState_, pUpstreamTransport);
}

void EMANE::NEMStatefulLayer::setDownstreamTransport(DownstreamTransport * pDownstreamTransport) {
    emane_rs_nem_stateful_layer_set_downstream_transport(pState_, pDownstreamTransport);
}

void EMANE::NEMStatefulLayer::changeState(NEMLayerState * pState) {
    // Rust handles state internally
}

void EMANE::NEMStatefulLayer::processTimedEvent(TimerEventId eventId,
                                                const TimePoint & expireTime,
                                                const TimePoint & scheduleTime,
                                                const TimePoint & fireTime,
                                                const void * arg) {
    emane_rs_nem_stateful_layer_process_timed_event(pState_, eventId, &expireTime, &scheduleTime, &fireTime, arg);
}
"""

with open("scratch/nemstatefullayer_ffi.cc", "w") as f:
    f.write(cpp_code)
