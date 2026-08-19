#include <cstdint>
#include "shimlayer.h"

extern "C" {
    void* emane_rs_shim_layer_create(void* cpp_this, void* implementor);
    void emane_rs_shim_layer_destroy(void* ptr);
    void emane_rs_shim_layer_initialize(void* ptr, void* registrar);
    void emane_rs_shim_layer_configure(void* ptr, void* update);
    void emane_rs_shim_layer_start(void* ptr);
    void emane_rs_shim_layer_post_start(void* ptr);
    void emane_rs_shim_layer_stop(void* ptr);
    void emane_rs_shim_layer_destroy_impl(void* ptr);
    void emane_rs_shim_layer_do_process_configuration(void* ptr, void* update);
    void emane_rs_shim_layer_do_process_upstream_packet(void* ptr, void* pkt, void* msgs);
    void emane_rs_shim_layer_do_process_downstream_packet(void* ptr, void* pkt, void* msgs);
    void emane_rs_shim_layer_do_process_upstream_control(void* ptr, void* msgs);
    void emane_rs_shim_layer_do_process_downstream_control(void* ptr, void* msgs);
    void emane_rs_shim_layer_set_upstream_transport(void* ptr, void* transport);
    void emane_rs_shim_layer_set_downstream_transport(void* ptr, void* transport);
    void emane_rs_shim_layer_do_process_event(void* ptr, void* event_id, void* serialization);
    void emane_rs_shim_layer_do_process_timed_event(void* ptr, void* event_id, const void* expire, const void* schedule, const void* fire, const void* arg);
}

EMANE::ShimLayer::ShimLayer(NEMId id,
                            NEMLayer * pImplementor,
                            PlatformServiceProvider * pPlatformService) :
  NEMQueuedLayer{id,pPlatformService},
  pImplementor_{pImplementor},
  pPlatformService_{pPlatformService}
{
  rs_state_ = emane_rs_shim_layer_create(this, pImplementor_.get());
}

EMANE::ShimLayer::~ShimLayer()
{
  emane_rs_shim_layer_destroy(rs_state_);
}

void EMANE::ShimLayer::initialize(Registrar & registrar)
{
  NEMQueuedLayer::initialize(registrar);
  emane_rs_shim_layer_initialize(rs_state_, &registrar);
}

void EMANE::ShimLayer::configure(const ConfigurationUpdate & update)
{
  emane_rs_shim_layer_configure(rs_state_, const_cast<ConfigurationUpdate*>(&update));
}

void EMANE::ShimLayer::start()
{
  NEMQueuedLayer::start();
  emane_rs_shim_layer_start(rs_state_);
}

void EMANE::ShimLayer::postStart()
{
  emane_rs_shim_layer_post_start(rs_state_);
}

void EMANE::ShimLayer::stop()
{
  emane_rs_shim_layer_stop(rs_state_);
  NEMQueuedLayer::stop();
}

void EMANE::ShimLayer::destroy() throw()
{
  emane_rs_shim_layer_destroy_impl(rs_state_);
}

void EMANE::ShimLayer::doProcessConfiguration(const ConfigurationUpdate & update)
{
  emane_rs_shim_layer_do_process_configuration(rs_state_, const_cast<ConfigurationUpdate*>(&update));
}

void EMANE::ShimLayer::doProcessUpstreamPacket(UpstreamPacket & pkt,
                                               const ControlMessages & msgs)
{
  emane_rs_shim_layer_do_process_upstream_packet(rs_state_, &pkt, const_cast<ControlMessages*>(&msgs));
}

void EMANE::ShimLayer::doProcessDownstreamPacket(DownstreamPacket & pkt,
                                                 const ControlMessages & msgs)
{
  emane_rs_shim_layer_do_process_downstream_packet(rs_state_, &pkt, const_cast<ControlMessages*>(&msgs));
}

void EMANE::ShimLayer::doProcessUpstreamControl(const ControlMessages & msgs)
{
  emane_rs_shim_layer_do_process_upstream_control(rs_state_, const_cast<ControlMessages*>(&msgs));
}

void EMANE::ShimLayer::doProcessDownstreamControl(const ControlMessages & msgs)
{
  emane_rs_shim_layer_do_process_downstream_control(rs_state_, const_cast<ControlMessages*>(&msgs));
}

void EMANE::ShimLayer::setUpstreamTransport(UpstreamTransport * pUpstreamTransport)
{
  emane_rs_shim_layer_set_upstream_transport(rs_state_, pUpstreamTransport);
}

void EMANE::ShimLayer::setDownstreamTransport(DownstreamTransport * pDownstreamTransport)
{
  emane_rs_shim_layer_set_downstream_transport(rs_state_, pDownstreamTransport);
}

void EMANE::ShimLayer::doProcessEvent(const EventId & eventId,
                                      const Serialization & serialization)
{
  emane_rs_shim_layer_do_process_event(rs_state_, const_cast<EventId*>(&eventId), const_cast<Serialization*>(&serialization));
}

void EMANE::ShimLayer::doProcessTimedEvent(TimerEventId eventId,
                                           const TimePoint & expireTime,
                                           const TimePoint & scheduleTime,
                                           const TimePoint & fireTime,
                                           const void * arg)
{
  emane_rs_shim_layer_do_process_timed_event(rs_state_, &eventId, &expireTime, &scheduleTime, &fireTime, arg);
}

extern "C" {
    void emane_c_shim_layer_implementor_initialize(void* impl_ptr, void* registrar) {
        static_cast<EMANE::NEMLayer*>(impl_ptr)->initialize(*static_cast<EMANE::Registrar*>(registrar));
    }
    void emane_c_shim_layer_implementor_configure(void* impl_ptr, void* update) {
        static_cast<EMANE::NEMLayer*>(impl_ptr)->configure(*static_cast<EMANE::ConfigurationUpdate*>(update));
    }
    void emane_c_shim_layer_implementor_start(void* impl_ptr) {
        static_cast<EMANE::NEMLayer*>(impl_ptr)->start();
    }
    void emane_c_shim_layer_implementor_post_start(void* impl_ptr) {
        static_cast<EMANE::NEMLayer*>(impl_ptr)->postStart();
    }
    void emane_c_shim_layer_implementor_stop(void* impl_ptr) {
        static_cast<EMANE::NEMLayer*>(impl_ptr)->stop();
    }
    void emane_c_shim_layer_implementor_destroy(void* impl_ptr) {
        static_cast<EMANE::NEMLayer*>(impl_ptr)->destroy();
    }
    void emane_c_shim_layer_implementor_process_configuration(void* impl_ptr, void* update) {
        static_cast<EMANE::NEMLayer*>(impl_ptr)->processConfiguration(*static_cast<EMANE::ConfigurationUpdate*>(update));
    }
    void emane_c_shim_layer_implementor_process_upstream_packet(void* impl_ptr, void* pkt, void* msgs) {
        static_cast<EMANE::NEMLayer*>(impl_ptr)->processUpstreamPacket(*static_cast<EMANE::UpstreamPacket*>(pkt), *static_cast<EMANE::ControlMessages*>(msgs));
    }
    void emane_c_shim_layer_implementor_process_downstream_packet(void* impl_ptr, void* pkt, void* msgs) {
        static_cast<EMANE::NEMLayer*>(impl_ptr)->processDownstreamPacket(*static_cast<EMANE::DownstreamPacket*>(pkt), *static_cast<EMANE::ControlMessages*>(msgs));
    }
    void emane_c_shim_layer_implementor_process_upstream_control(void* impl_ptr, void* msgs) {
        static_cast<EMANE::NEMLayer*>(impl_ptr)->processUpstreamControl(*static_cast<EMANE::ControlMessages*>(msgs));
    }
    void emane_c_shim_layer_implementor_process_downstream_control(void* impl_ptr, void* msgs) {
        static_cast<EMANE::NEMLayer*>(impl_ptr)->processDownstreamControl(*static_cast<EMANE::ControlMessages*>(msgs));
    }
    void emane_c_shim_layer_implementor_set_upstream_transport(void* impl_ptr, void* transport) {
        static_cast<EMANE::NEMLayer*>(impl_ptr)->setUpstreamTransport(static_cast<EMANE::UpstreamTransport*>(transport));
    }
    void emane_c_shim_layer_implementor_set_downstream_transport(void* impl_ptr, void* transport) {
        static_cast<EMANE::NEMLayer*>(impl_ptr)->setDownstreamTransport(static_cast<EMANE::DownstreamTransport*>(transport));
    }
    void emane_c_shim_layer_implementor_process_event(void* impl_ptr, void* event_id, void* serialization) {
        static_cast<EMANE::NEMLayer*>(impl_ptr)->processEvent(*static_cast<EMANE::EventId*>(event_id), *static_cast<EMANE::Serialization*>(serialization));
    }
    void emane_c_shim_layer_implementor_process_timed_event(void* impl_ptr, void* event_id, const void* expire, const void* schedule, const void* fire, const void* arg) {
        static_cast<EMANE::NEMLayer*>(impl_ptr)->processTimedEvent(*static_cast<EMANE::TimerEventId*>(event_id), *static_cast<const EMANE::TimePoint*>(expire), *static_cast<const EMANE::TimePoint*>(schedule), *static_cast<const EMANE::TimePoint*>(fire), arg);
    }
}
