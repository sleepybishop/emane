
#include "maclayer.h"
#include "emane/upstreampacket.h"
#include "emane/downstreampacket.h"

extern "C" {
    void* emane_rs_rfpipe_mac_new(uint16_t id, void* c_mac_layer);
    void emane_rs_rfpipe_mac_free(void* ptr);
    void emane_rs_rfpipe_mac_initialize(void* ptr, void* registrar);
    void emane_rs_rfpipe_mac_configure(void* ptr, void* update);
    void emane_rs_rfpipe_mac_start(void* ptr);
    void emane_rs_rfpipe_mac_post_start(void* ptr);
    void emane_rs_rfpipe_mac_stop(void* ptr);
    void emane_rs_rfpipe_mac_destroy(void* ptr);
    void emane_rs_rfpipe_mac_process_upstream_control(void* ptr, const void* msgs);
    void emane_rs_rfpipe_mac_process_upstream_packet(void* ptr, const void* hdr, void* pkt, const void* msgs);
    void emane_rs_rfpipe_mac_process_downstream_control(void* ptr, const void* msgs);
    void emane_rs_rfpipe_mac_process_downstream_packet(void* ptr, void* pkt, const void* msgs);
    void emane_rs_rfpipe_mac_process_event(void* ptr, const void* eventId, const void* serialization);
    void emane_rs_rfpipe_mac_process_timed_event(void* ptr, uint32_t eventId, const void* expire, const void* sched, const void* fire, const void* arg);
    
    // Callbacks from Rust back to C++
    void emane_c_rfpipe_send_upstream_packet(void* mac, void* pkt, const void* msgs) {
        auto* layer = static_cast<EMANE::Models::RFPipe::MACLayer*>(mac);
        layer->doSendUpstreamPacket(*static_cast<EMANE::UpstreamPacket*>(pkt), *static_cast<const EMANE::ControlMessages*>(msgs));
    }
    void emane_c_rfpipe_send_downstream_packet(void* mac, const void* hdr, void* pkt, const void* msgs) {
        auto* layer = static_cast<EMANE::Models::RFPipe::MACLayer*>(mac);
        layer->doSendDownstreamPacket(*static_cast<const EMANE::CommonMACHeader*>(hdr), *static_cast<EMANE::DownstreamPacket*>(pkt), *static_cast<const EMANE::ControlMessages*>(msgs));
    }
    void emane_c_rfpipe_send_upstream_control(void* mac, const void* msgs) {
        auto* layer = static_cast<EMANE::Models::RFPipe::MACLayer*>(mac);
        layer->doSendUpstreamControl(*static_cast<const EMANE::ControlMessages*>(msgs));
    }
    void emane_c_rfpipe_send_downstream_control(void* mac, const void* msgs) {
        auto* layer = static_cast<EMANE::Models::RFPipe::MACLayer*>(mac);
        layer->doSendDownstreamControl(*static_cast<const EMANE::ControlMessages*>(msgs));
    }
}

EMANE::Models::RFPipe::MACLayer::MACLayer(NEMId id, PlatformServiceProvider *pPlatformServiceProvider, RadioServiceProvider * pRadioServiceProvider):
  MACLayerImplementor{id, pPlatformServiceProvider, pRadioServiceProvider},
  rs_state_{emane_rs_rfpipe_mac_new(id, this)}
{}

EMANE::Models::RFPipe::MACLayer::~MACLayer() {
    emane_rs_rfpipe_mac_free(rs_state_);
}

void EMANE::Models::RFPipe::MACLayer::initialize(Registrar & registrar) {
    emane_rs_rfpipe_mac_initialize(rs_state_, &registrar);
}

void EMANE::Models::RFPipe::MACLayer::configure(const ConfigurationUpdate & update) {
    emane_rs_rfpipe_mac_configure(rs_state_, const_cast<ConfigurationUpdate*>(&update));
}

void EMANE::Models::RFPipe::MACLayer::start() {
    emane_rs_rfpipe_mac_start(rs_state_);
}

void EMANE::Models::RFPipe::MACLayer::postStart() {
    emane_rs_rfpipe_mac_post_start(rs_state_);
}

void EMANE::Models::RFPipe::MACLayer::stop() {
    emane_rs_rfpipe_mac_stop(rs_state_);
}

void EMANE::Models::RFPipe::MACLayer::destroy() throw() {
    emane_rs_rfpipe_mac_destroy(rs_state_);
}

void EMANE::Models::RFPipe::MACLayer::processUpstreamControl(const ControlMessages & msgs) {
    emane_rs_rfpipe_mac_process_upstream_control(rs_state_, &msgs);
}

void EMANE::Models::RFPipe::MACLayer::processUpstreamPacket(const CommonMACHeader & hdr, UpstreamPacket & pkt, const ControlMessages & msgs) {
    emane_rs_rfpipe_mac_process_upstream_packet(rs_state_, &hdr, &pkt, &msgs);
}

void EMANE::Models::RFPipe::MACLayer::processDownstreamControl(const ControlMessages & msgs) {
    emane_rs_rfpipe_mac_process_downstream_control(rs_state_, &msgs);
}

void EMANE::Models::RFPipe::MACLayer::processDownstreamPacket(DownstreamPacket & pkt, const ControlMessages & msgs) {
    emane_rs_rfpipe_mac_process_downstream_packet(rs_state_, &pkt, &msgs);
}

void EMANE::Models::RFPipe::MACLayer::processConfiguration(const ConfigurationUpdate & update) {
    // Usually ignored in porting unless dynamic config is supported
}

void EMANE::Models::RFPipe::MACLayer::processEvent(const EventId & eventId, const Serialization & serialization) {
    emane_rs_rfpipe_mac_process_event(rs_state_, &eventId, &serialization);
}

void EMANE::Models::RFPipe::MACLayer::processTimedEvent(TimerEventId eventId, const TimePoint & expireTime, const TimePoint & scheduleTime, const TimePoint & fireTime, const void * arg) {
    emane_rs_rfpipe_mac_process_timed_event(rs_state_, eventId, &expireTime, &scheduleTime, &fireTime, arg);
}

void EMANE::Models::RFPipe::MACLayer::doSendUpstreamPacket(UpstreamPacket & pkt, const ControlMessages & msgs) {
    sendUpstreamPacket(pkt, msgs);
}

void EMANE::Models::RFPipe::MACLayer::doSendDownstreamPacket(const CommonMACHeader & hdr, DownstreamPacket & pkt, const ControlMessages & msgs) {
    sendDownstreamPacket(hdr, pkt, msgs);
}

void EMANE::Models::RFPipe::MACLayer::doSendUpstreamControl(const ControlMessages & msgs) {
    sendUpstreamControl(msgs);
}

void EMANE::Models::RFPipe::MACLayer::doSendDownstreamControl(const ControlMessages & msgs) {
    sendDownstreamControl(msgs);
}

DECLARE_MAC_LAYER(EMANE::Models::RFPipe::MACLayer);
