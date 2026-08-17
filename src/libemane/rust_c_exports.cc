#include "emane/eventserviceuser.h"
#include "otamanager.h"
#include <cstddef>
#include <cstdint>
#include <vector>

extern "C" {
    void emane_rs_event_service_process_event_message(uint16_t nem_id, uint16_t event_id, const char* data, size_t len, uint16_t ignore_nem);

    // EventService C exports
    void emane_c_event_service_user_process_event(void* p_user, uint16_t event_id, const char* data, size_t len) {
        auto user = static_cast<EMANE::EventServiceUser*>(p_user);
        EMANE::Serialization const serialization(data, len);
        user->processEvent(event_id, serialization);
    }

    void emane_c_event_service_update_stat(int type, const unsigned char* uuid, uint16_t event_id) {
        // No-op for now since EventServiceSingleton is removed
    }

    void emane_c_event_service_register_user(uint16_t build_id, uint16_t nem_id) {
        // No-op since EventService is in Rust
    }

    // OTAManager C exports
    void emane_c_ota_manager_update_stat(const uuid_t * uuid_ptr, uint16_t src_nem, uint32_t stat_type) {
        if(auto p = EMANE::OTAManagerSingleton::instance()) {
            p->updateStat(uuid_ptr, src_nem, stat_type);
        }
    }

    void emane_c_ota_manager_deliver_event(uint16_t src_nem, uint16_t event_id, const uint8_t * data, size_t data_len) {
        emane_rs_event_service_process_event_message(src_nem, event_id, reinterpret_cast<const char*>(data), data_len, 0);
    }

    void emane_c_ota_manager_deliver_upstream(
        uint16_t source, uint16_t destination, uint8_t priority,
        const uuid_t * uuid_ptr,
        const uint8_t * data, size_t data_len,
        const uint8_t * controls, size_t controls_len
    ) {
        if(auto p = EMANE::OTAManagerSingleton::instance()) {
            p->deliverUpstream(source, destination, priority, uuid_ptr, data, data_len, controls, controls_len);
        }
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
