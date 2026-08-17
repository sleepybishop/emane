#include "emane/eventserviceuser.h"
#include "otauser.h"
#include "emane/downstreampacket.h"
#include "emane/upstreampacket.h"
#include "emane/controlmessage.h"
#include "emane/controls/otatransmittercontrolmessage.h"
#include "controlmessageserializer.h"
#include "event.pb.h"
#include "otastatisticpublisher.h"
#include "eventstatisticpublisher.h"
#include <memory>
#include <cstddef>
#include <cstdint>
#include <vector>
#include <map>

static std::unique_ptr<EMANE::OTAStatisticPublisher> g_ota_publisher;
static std::unique_ptr<EMANE::EventStatisticPublisher> g_event_publisher;
static std::map<uint16_t, EMANE::OTAUser*> g_nemUserMap;
static uuid_t g_local_uuid;

extern "C" {
    void emane_rs_event_service_process_event_message(uint16_t nem_id, uint16_t event_id, const char* data, size_t len, uint16_t ignore_nem);
    void emane_rs_ota_manager_register_user(uint16_t id, void* p_user);
    void emane_rs_ota_manager_unregister_user(uint16_t id);
    bool emane_rs_ota_manager_send_ota_packet(uint16_t, uint16_t, const uint8_t*, size_t, const uint8_t*, size_t, const uint8_t*, size_t);

    void emane_c_ota_manager_init_publishers() {
        if (!g_ota_publisher) g_ota_publisher = std::make_unique<EMANE::OTAStatisticPublisher>();
        if (!g_event_publisher) g_event_publisher = std::make_unique<EMANE::EventStatisticPublisher>("OTA");
    }

    void emane_c_ota_manager_set_stat_packet_count_limit(uint32_t pkt_limit) {
        if (g_ota_publisher) g_ota_publisher->setRowLimit(pkt_limit);
    }

    void emane_c_ota_manager_set_stat_event_count_limit(uint32_t evt_limit) {
        if (g_event_publisher) g_event_publisher->setRowLimit(evt_limit);
    }
    
    void emane_c_ota_manager_set_local_uuid(const unsigned char* u) {
        std::copy(u, u + 16, g_local_uuid);
    }

    void emane_c_ota_manager_register_user(uint16_t id, void* p_user) {
        g_nemUserMap[id] = static_cast<EMANE::OTAUser*>(p_user);
        emane_rs_ota_manager_register_user(id, p_user);
    }

    void emane_c_ota_manager_unregister_user(uint16_t id) {
        g_nemUserMap.erase(id);
        emane_rs_ota_manager_unregister_user(id);
    }

    void emane_c_ota_manager_send_packet_cpp(uint16_t id, const void* pkt_ptr, const void* msgs_ptr) {
        auto& pkt = *static_cast<const EMANE::DownstreamPacket*>(pkt_ptr);
        auto& msgs = *static_cast<const EMANE::ControlMessages*>(msgs_ptr);

        const EMANE::PacketInfo & pktInfo{pkt.getPacketInfo()};
        EMANE::Controls::OTATransmitters otaTransmitters{};
        auto eventSerializations = pkt.getEventSerializations();

        std::string sEventSerialization{};
        if(!eventSerializations.empty()) {
            EMANEMessage::Event::Data data;
            for(const auto & entry : eventSerializations) {
                auto pSerialization = data.add_serializations();
                pSerialization->set_nemid(std::get<0>(entry));
                pSerialization->set_eventid(std::get<1>(entry));
                pSerialization->set_data(std::get<2>(entry));
                
                emane_rs_event_service_process_event_message(std::get<0>(entry), std::get<1>(entry), std::get<2>(entry).c_str(), std::get<2>(entry).length(), id);
            }
            data.SerializeToString(&sEventSerialization);
        }

        for(const auto & pMessage : msgs) {
            if(pMessage->getId() == EMANE::Controls::OTATransmitterControlMessage::IDENTIFIER) {
                const auto pTransmitterControlMessage =
                  reinterpret_cast<const EMANE::Controls::OTATransmitterControlMessage *>(pMessage);
                otaTransmitters = pTransmitterControlMessage->getOTATransmitters();
            }
        }

        if(g_nemUserMap.size() > 1) {
            auto now = EMANE::Clock::now();
            EMANE::UpstreamPacket upstreamPacket({pktInfo.getSource(),
                  pktInfo.getDestination(),
                  pktInfo.getPriority(),
                  now,
                  g_local_uuid},
              pkt.getVectorIO());

            for(auto iter = g_nemUserMap.begin(), end = g_nemUserMap.end(); iter != end; ++iter) {
                if(iter->first == id) continue;
                if(otaTransmitters.count(iter->first) > 0) continue;
                iter->second->processOTAPacket(upstreamPacket, EMANE::ControlMessages());
            }
        }

        EMANE::ControlMessageSerializer controlMessageSerializer{msgs};
        std::vector<uint8_t> controlData;
        for (const auto& iov : controlMessageSerializer.getVectorIO()) {
            const uint8_t* base = reinterpret_cast<const uint8_t*>(iov.iov_base);
            controlData.insert(controlData.end(), base, base + iov.iov_len);
        }
        
        std::vector<uint8_t> packetData;
        for (const auto& iov : pkt.getVectorIO()) {
            const uint8_t* base = reinterpret_cast<const uint8_t*>(iov.iov_base);
            packetData.insert(packetData.end(), base, base + iov.iov_len);
        }
        
        emane_rs_ota_manager_send_ota_packet(
            pktInfo.getSource(),
            pktInfo.getDestination(),
            packetData.data(), packetData.size(),
            controlData.data(), controlData.size(),
            reinterpret_cast<const uint8_t*>(sEventSerialization.c_str()), sEventSerialization.size()
        );
        
        if (g_ota_publisher) {
            g_ota_publisher->update(EMANE::OTAStatisticPublisher::Type::TYPE_DOWNSTREAM_PACKET_SUCCESS,
                                      g_local_uuid,
                                      pktInfo.getSource());
        }

        if (g_event_publisher) {
            for(const auto & entry : eventSerializations) {
                g_event_publisher->update(EMANE::EventStatisticPublisher::Type::TYPE_TX,
                                                g_local_uuid,
                                                std::get<1>(entry));
            }
        }

        std::for_each(msgs.begin(),msgs.end(),[](const EMANE::ControlMessage * p){delete p;});
    }

    // EventService C exports
    void emane_c_event_service_user_process_event(void* p_user, uint16_t event_id, const char* data, size_t len) {
        auto user = static_cast<EMANE::EventServiceUser*>(p_user);
        EMANE::Serialization const serialization(data, len);
        user->processEvent(event_id, serialization);
    }

    void emane_c_event_service_update_stat(int type, const unsigned char* uuid, uint16_t event_id) {
    }

    void emane_c_event_service_register_user(uint16_t build_id, uint16_t nem_id) {
    }

    void emane_c_ota_manager_update_stat(const uuid_t * uuid_ptr, uint16_t src_nem, uint32_t stat_type) {
        if (g_ota_publisher) {
            uuid_t u;
            uuid_copy(u, *uuid_ptr);
            if(stat_type == 2) {
                g_ota_publisher->update(EMANE::OTAStatisticPublisher::Type::TYPE_UPSTREAM_PACKET_SUCCESS, u, src_nem);
            } else if(stat_type == 3) {
                g_ota_publisher->update(EMANE::OTAStatisticPublisher::Type::TYPE_UPSTREAM_PACKET_DROP_MISSING_PARTS, u, src_nem);
            }
        }
    }

    void emane_c_ota_manager_deliver_event(uint16_t src_nem, uint16_t event_id, const uint8_t * data, size_t data_len) {
        if (g_event_publisher) {
            uuid_t u;
            uuid_clear(u);
            g_event_publisher->update(EMANE::EventStatisticPublisher::Type::TYPE_RX, u, event_id);
        }
        emane_rs_event_service_process_event_message(src_nem, event_id, reinterpret_cast<const char*>(data), data_len, 0);
    }

    void emane_c_ota_user_process_packet(
        void* p_user,
        uint16_t source, uint16_t destination, uint8_t priority,
        const uuid_t* uuid_ptr,
        const uint8_t* data, size_t data_len,
        const uint8_t* controls, size_t controls_len
    ) {
        auto user = static_cast<EMANE::OTAUser*>(p_user);
        auto now = EMANE::Clock::now();
        uuid_t remote_uuid;
        uuid_copy(remote_uuid, *uuid_ptr);
        
        EMANE::PacketInfo pktInfo(source, destination, priority, now, remote_uuid);
        
        EMANE::Utils::VectorIO packetVectorIO{};
        if (data_len > 0) {
            packetVectorIO.push_back({const_cast<uint8_t *>(data), data_len});
        }
        
        EMANE::UpstreamPacket pkt(pktInfo, packetVectorIO);
        
        EMANE::ControlMessages msgs;
        if (controls_len > 0) {
            msgs = EMANE::ControlMessageSerializer::create(const_cast<uint8_t *>(controls), controls_len);
        }
        
        user->processOTAPacket(pkt, msgs);
    }
}

#include "emane/application/statisticcontroller.h"

namespace EMANE {
    namespace Application {
        StatisticManifest StatisticController::getStatisticManifest(BuildId) {
            return {};
        }
        
        StatisticTableManifest StatisticController::getTableManifest(BuildId) {
            return {};
        }
    }
}
