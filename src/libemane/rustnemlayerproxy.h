#ifndef EMANERUSTNEMLAYERPROXY_H
#define EMANERUSTNEMLAYERPROXY_H

#include "emane/nemqueuedlayer.h"

extern "C" {
    // Rust exports
    void* emane_rs_mac_layer_new(uint16_t id, void* platform_service, void* plugin_impl);
    void emane_rs_mac_layer_free(void* rs_mac);
    void emane_rs_mac_layer_initialize(void* rs_mac, void* registrar);
    void emane_rs_mac_layer_configure(void* rs_mac, void* update);
    void emane_rs_mac_layer_start(void* rs_mac);
    void emane_rs_mac_layer_post_start(void* rs_mac);
    void emane_rs_mac_layer_stop(void* rs_mac);
    void emane_rs_mac_layer_destroy(void* rs_mac);
    void emane_rs_mac_layer_do_process_upstream_packet(void* rs_mac, EMANE::UpstreamPacket* pkt, const EMANE::ControlMessages* msgs);
    void emane_rs_mac_layer_do_process_downstream_packet(void* rs_mac, EMANE::DownstreamPacket* pkt, const EMANE::ControlMessages* msgs);
    void emane_rs_mac_layer_do_process_upstream_control(void* rs_mac, const EMANE::ControlMessages* msgs);
    void emane_rs_mac_layer_do_process_downstream_control(void* rs_mac, const EMANE::ControlMessages* msgs);
    void emane_rs_mac_layer_do_process_event(void* rs_mac, const EMANE::EventId* eventId, const EMANE::Serialization* serialization);
    void emane_rs_mac_layer_do_process_timed_event(void* rs_mac, EMANE::TimerEventId eventId, const EMANE::TimePoint* reqExpireTime, const EMANE::TimePoint* schedTime, const EMANE::TimePoint* fireTime, const void* arg);
    void emane_rs_mac_layer_do_process_configuration(void* rs_mac, const EMANE::ConfigurationUpdate* update);
    void emane_rs_mac_layer_set_upstream_transport(void* rs_mac, EMANE::UpstreamTransport* transport);
    void emane_rs_mac_layer_set_downstream_transport(void* rs_mac, EMANE::DownstreamTransport* transport);
}

namespace EMANE {
    class RustNemLayerProxy : public NEMQueuedLayer {
    public:
        RustNemLayerProxy(NEMId id, PlatformServiceProvider* pPlatformService, void* plugin_impl) 
            : NEMQueuedLayer{id, pPlatformService}, rs_mac_{emane_rs_mac_layer_new(id, pPlatformService, plugin_impl)} {}
        
        ~RustNemLayerProxy() override {
            emane_rs_mac_layer_free(rs_mac_);
        }

        void initialize(Registrar & registrar) override {
            NEMQueuedLayer::initialize(registrar);
            emane_rs_mac_layer_initialize(rs_mac_, &registrar);
        }

        void configure(const ConfigurationUpdate & update) override {
            emane_rs_mac_layer_configure(rs_mac_, const_cast<void*>(static_cast<const void*>(&update)));
        }

        void start() override {
            NEMQueuedLayer::start();
            emane_rs_mac_layer_start(rs_mac_);
        }

        void postStart() override {
            emane_rs_mac_layer_post_start(rs_mac_);
        }

        void stop() override {
            emane_rs_mac_layer_stop(rs_mac_);
            NEMQueuedLayer::stop();
        }

        void destroy() throw() override {
            emane_rs_mac_layer_destroy(rs_mac_);
        }

        void doProcessUpstreamPacket(UpstreamPacket & pkt, const ControlMessages & msgs) override {
            emane_rs_mac_layer_do_process_upstream_packet(rs_mac_, &pkt, &msgs);
        }

        void doProcessDownstreamPacket(DownstreamPacket & pkt, const ControlMessages & msgs) override {
            emane_rs_mac_layer_do_process_downstream_packet(rs_mac_, &pkt, &msgs);
        }

        void doProcessUpstreamControl(const ControlMessages & msgs) override {
            emane_rs_mac_layer_do_process_upstream_control(rs_mac_, &msgs);
        }

        void doProcessDownstreamControl(const ControlMessages & msgs) override {
            emane_rs_mac_layer_do_process_downstream_control(rs_mac_, &msgs);
        }

        void doProcessEvent(const EventId & eventId, const Serialization & serialization) override {
            emane_rs_mac_layer_do_process_event(rs_mac_, &eventId, &serialization);
        }

        void doProcessTimedEvent(TimerEventId eventId, const TimePoint & reqExpireTime, const TimePoint & schedTime, const TimePoint & fireTime, const void * arg) override {
            emane_rs_mac_layer_do_process_timed_event(rs_mac_, eventId, &reqExpireTime, &schedTime, &fireTime, arg);
        }

        void doProcessConfiguration(const ConfigurationUpdate & update) override {
            emane_rs_mac_layer_do_process_configuration(rs_mac_, &update);
        }

        void setUpstreamTransport(UpstreamTransport * pUpstreamTransport) override {
            emane_rs_mac_layer_set_upstream_transport(rs_mac_, pUpstreamTransport);
        }

        void setDownstreamTransport(DownstreamTransport * pDownstreamTransport) override {
            emane_rs_mac_layer_set_downstream_transport(rs_mac_, pDownstreamTransport);
        }

    private:
        void* rs_mac_;
    };
}

#endif
