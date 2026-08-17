#include "emane/maclayerimpl.h"
#include "emane/upstreampacket.h"
#include "emane/downstreampacket.h"

extern "C" {

void emane_c_mac_plugin_initialize(void* plugin, void* registrar) {
    auto impl = static_cast<EMANE::MACLayerImplementor*>(plugin);
    impl->initialize(*static_cast<EMANE::Registrar*>(registrar));
}

void emane_c_mac_plugin_configure(void* plugin, void* update) {
    auto impl = static_cast<EMANE::MACLayerImplementor*>(plugin);
    impl->configure(*static_cast<EMANE::ConfigurationUpdate*>(update));
}

void emane_c_mac_plugin_start(void* plugin) {
    auto impl = static_cast<EMANE::MACLayerImplementor*>(plugin);
    impl->start();
}

void emane_c_mac_plugin_post_start(void* plugin) {
    auto impl = static_cast<EMANE::MACLayerImplementor*>(plugin);
    impl->postStart();
}

void emane_c_mac_plugin_stop(void* plugin) {
    auto impl = static_cast<EMANE::MACLayerImplementor*>(plugin);
    impl->stop();
}

void emane_c_mac_plugin_destroy(void* plugin) {
    auto impl = static_cast<EMANE::MACLayerImplementor*>(plugin);
    impl->destroy();
}

void emane_c_mac_plugin_process_upstream_packet(void* plugin, EMANE::UpstreamPacket* pkt, const EMANE::ControlMessages* msgs) {
    auto impl = static_cast<EMANE::MACLayerImplementor*>(plugin);
    // MACLayerImplementor implements UpstreamTransport which has processUpstreamPacket(pkt, msgs)
    static_cast<EMANE::UpstreamTransport*>(impl)->processUpstreamPacket(*pkt, *msgs);
}

void emane_c_mac_plugin_process_downstream_packet(void* plugin, EMANE::DownstreamPacket* pkt, const EMANE::ControlMessages* msgs) {
    auto impl = static_cast<EMANE::MACLayerImplementor*>(plugin);
    impl->processDownstreamPacket(*pkt, *msgs);
}

void emane_c_mac_plugin_process_upstream_control(void* plugin, const EMANE::ControlMessages* msgs) {
    auto impl = static_cast<EMANE::MACLayerImplementor*>(plugin);
    impl->processUpstreamControl(*msgs);
}

void emane_c_mac_plugin_process_downstream_control(void* plugin, const EMANE::ControlMessages* msgs) {
    auto impl = static_cast<EMANE::MACLayerImplementor*>(plugin);
    impl->processDownstreamControl(*msgs);
}

void emane_c_mac_plugin_set_upstream_transport(void* plugin, EMANE::UpstreamTransport* transport) {
    auto impl = static_cast<EMANE::MACLayerImplementor*>(plugin);
    impl->setUpstreamTransport(transport);
}

void emane_c_mac_plugin_set_downstream_transport(void* plugin, EMANE::DownstreamTransport* transport) {
    auto impl = static_cast<EMANE::MACLayerImplementor*>(plugin);
    impl->setDownstreamTransport(transport);
}

void emane_c_mac_plugin_process_event(void* plugin, const EMANE::EventId* eventId, const EMANE::Serialization* serialization) {
    auto impl = static_cast<EMANE::MACLayerImplementor*>(plugin);
    impl->processEvent(*eventId, *serialization);
}

void emane_c_mac_plugin_process_timed_event(void* plugin, EMANE::TimerEventId eventId, 
                                            const EMANE::TimePoint* reqExpireTime, 
                                            const EMANE::TimePoint* schedTime, 
                                            const EMANE::TimePoint* fireTime, 
                                            const void* arg) {
    auto impl = static_cast<EMANE::MACLayerImplementor*>(plugin);
    impl->processTimedEvent(eventId, *reqExpireTime, *schedTime, *fireTime, arg);
}

void emane_c_mac_plugin_process_configuration(void* plugin, void* update) {
    auto impl = static_cast<EMANE::MACLayerImplementor*>(plugin);
    impl->processConfiguration(*static_cast<EMANE::ConfigurationUpdate*>(update));
}

}

#include "layerfactorymanager.h"
#include "layerfactory.h"

extern "C" {
    void* emane_rs_ffi_create_mac_plugin(uint16_t id, const char* sLibraryFile, void* platformService, void* radioService) {
        std::string libFile = sLibraryFile ? sLibraryFile : "";
        auto& factory = EMANE::LayerFactoryManagerSingleton::instance()->getMACLayerFactory(libFile);
        auto* layer = factory.createLayer(id, 
            static_cast<EMANE::PlatformServiceProvider*>(platformService), 
            static_cast<EMANE::RadioServiceProvider*>(radioService));
        return layer;
    }
}
