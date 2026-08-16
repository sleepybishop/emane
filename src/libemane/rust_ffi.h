#ifndef EMANE_RUST_FFI_H
#define EMANE_RUST_FFI_H

#include <cstdint>
#include <cstddef>

extern "C" {

    struct FfiAny {
        int32_t any_type;
        int64_t i64_value;
        uint64_t u64_value;
        double d_value;
        const char* s_value;
    };

    struct FfiAnyArray {
        const FfiAny* data;
        size_t len;
    };

    struct FfiStringArray {
        const char** data;
        size_t len;
    };

    // Configuration FFI
    struct FfiConfigItemUpdate {
        const char* name;
        FfiAnyArray values;
    };

    struct FfiConfigUpdate {
        const FfiConfigItemUpdate* data;
        size_t len;
    };
    
    struct FfiConfigUpdateReqItem {
        const char* name;
        FfiStringArray values;
    };

    struct FfiConfigUpdateReq {
        const FfiConfigUpdateReqItem* data;
        size_t len;
    };

    struct FfiConfigInfo {
        const char* name;
        int32_t any_type;
        uint64_t properties;
        FfiAnyArray values;
        const char* usage;
        bool has_min_max;
        FfiAny min_value;
        FfiAny max_value;
        size_t min_occurs;
        size_t max_occurs;
        const char* regex_pattern;
    };

    struct FfiConfigManifest {
        FfiConfigInfo* data;
        size_t len;
    };

    FfiConfigManifest emane_rs_config_get_manifest(uint16_t build_id);
    void emane_rs_config_free_manifest(FfiConfigManifest manifest);

    FfiConfigUpdate emane_rs_config_query(uint16_t build_id, FfiStringArray names);
    void emane_rs_config_free_update(FfiConfigUpdate update);
    
    FfiConfigUpdate emane_rs_config_build_updates(uint16_t buildId, FfiConfigUpdateReq parameters, char* error_buf, size_t error_buf_len);
    bool emane_rs_config_update(uint16_t buildId, FfiConfigUpdate updates, char* error_buf, size_t error_buf_len);

    // Statistic FFI
    struct FfiStatisticQueryItem {
        const char* name;
        FfiAny value;
    };

    struct FfiStatisticQueryResult {
        FfiStatisticQueryItem* data;
        size_t len;
    };

    struct FfiTableRow {
        FfiAnyArray values;
    };

    struct FfiStatisticTableQueryItem {
        const char* name;
        FfiStringArray labels;
        FfiTableRow* rows;
        size_t rows_len;
    };

    struct FfiStatisticTableQueryResult {
        FfiStatisticTableQueryItem* data;
        size_t len;
    };

    FfiStatisticQueryResult emane_rs_statistic_query(uint16_t build_id, FfiStringArray names, char* err_buf, size_t err_len);
    void emane_rs_statistic_free_query_result(FfiStatisticQueryResult res);

    FfiStatisticTableQueryResult emane_rs_statistic_query_table(uint16_t build_id, FfiStringArray names, char* err_buf, size_t err_len);
    void emane_rs_statistic_free_table_query_result(FfiStatisticTableQueryResult res);

    // BuildId FFI
    struct FfiNEMLayerComponent {
        uint16_t build_id;
        int32_t layer_type;
        const char* plugin_name;
    };

    struct FfiNEMLayerComponentList {
        uint16_t nem_id;
        FfiNEMLayerComponent* components;
        size_t len;
    };

    struct FfiNEMLayerComponentMap {
        FfiNEMLayerComponentList* nems;
        size_t len;
    };

    uint16_t emane_rs_buildid_assign();
    void emane_rs_buildid_register_nem_manager(uint16_t build_id, char* err_buf, size_t err_len);
    void emane_rs_buildid_register_transport_manager(uint16_t build_id, char* err_buf, size_t err_len);
    void emane_rs_buildid_register_event_generator_manager(uint16_t build_id, char* err_buf, size_t err_len);
    void emane_rs_buildid_register_event_agent_manager(uint16_t build_id, char* err_buf, size_t err_len);
    void emane_rs_buildid_register_layer(uint16_t nem_id, uint16_t build_id, int32_t layer_type, const char* plugin_name);
    void emane_rs_buildid_register_transport(uint16_t nem_id, uint16_t build_id);
    void emane_rs_buildid_register_nem(uint16_t nem_id, uint16_t build_id);
    void emane_rs_buildid_register_transport_adapter(uint16_t nem_id, uint16_t build_id);
    void emane_rs_buildid_register_event_generator(uint16_t build_id);
    void emane_rs_buildid_register_event_agent(uint16_t build_id);
    
    FfiNEMLayerComponentMap emane_rs_buildid_get_nem_layer_component_map();
    void emane_rs_buildid_free_nem_layer_component_map(FfiNEMLayerComponentMap map);

    // Bypass MAC FFI
    struct FfiBypassMac;
    FfiBypassMac* emane_rs_bypass_mac_new(uint16_t type);
    void emane_rs_bypass_mac_free(FfiBypassMac* ptr);
    bool emane_rs_bypass_mac_process_upstream(FfiBypassMac* ptr, uint16_t hdr_type);
    uint16_t emane_rs_bypass_mac_process_downstream(FfiBypassMac* ptr);

    // Rfpipe MAC FFI
    struct FfiRfpipeMac;

    struct FfiRfpipeUpstreamAction {
        uint8_t action; // 0 = drop, 1 = send_upstream
        uint16_t drop_code;
    };

    struct FfiRfpipeDownstreamAction {
        uint8_t action; // 0 = drop, 1 = enqueue
        uint16_t drop_code;
        uint64_t duration_microseconds;
        uint64_t delay_microseconds;
    };

    FfiRfpipeMac* emane_rs_rfpipe_mac_new(uint16_t id);
    void emane_rs_rfpipe_mac_free(FfiRfpipeMac* ptr);

    void emane_rs_rfpipe_mac_configure(
        FfiRfpipeMac* ptr,
        bool promiscuous_mode,
        uint64_t data_rate_bps,
        uint64_t delay_microseconds,
        bool flow_control_enable,
        bool radio_metric_enable,
        uint16_t flow_control_tokens,
        const char* pcr_curve_uri,
        uint64_t radio_metric_report_interval_microseconds,
        uint64_t neighbor_metric_delete_time_microseconds
    );

    void emane_rs_rfpipe_mac_start(FfiRfpipeMac* ptr);
    
    FfiRfpipeUpstreamAction emane_rs_rfpipe_mac_process_upstream(
        FfiRfpipeMac* ptr,
        double sinr,
        size_t pkt_length,
        uint16_t src,
        uint16_t dst,
        uint64_t sequence_number,
        double noise_floor_db,
        uint64_t start_of_reception_microseconds,
        uint64_t duration_microseconds,
        uint64_t data_rate,
        uint64_t frequency_hz,
        double rx_power_dbm,
        double receiver_sensitivity_db
    );

    FfiRfpipeDownstreamAction emane_rs_rfpipe_mac_process_downstream(
        FfiRfpipeMac* ptr,
        size_t pkt_length
    );

    void emane_rs_rfpipe_mac_downstream_dequeue(
        FfiRfpipeMac* ptr,
        uint16_t dst,
        uint64_t delay_microseconds,
        uint32_t queue_size,
        uint32_t queue_depth,
        uint32_t queue_discards
    );

    bool emane_rs_rfpipe_mac_flow_control_add_token(FfiRfpipeMac* ptr);
    bool emane_rs_rfpipe_mac_process_flow_control_message(FfiRfpipeMac* ptr, uint16_t msg_tokens);

    // Tdma MAC FFI
    struct FfiTdmaMac;
    FfiTdmaMac* emane_rs_tdma_mac_new(uint16_t id);
    void emane_rs_tdma_mac_free(FfiTdmaMac* ptr);

    void emane_rs_tdma_mac_set_flow_control(
        FfiTdmaMac* ptr,
        bool enable,
        uint16_t tokens
    );

    bool emane_rs_tdma_mac_remove_token(FfiTdmaMac* ptr);
    bool emane_rs_tdma_mac_add_token(FfiTdmaMac* ptr);
    bool emane_rs_tdma_mac_process_flow_control_message(FfiTdmaMac* ptr, uint16_t msg_tokens);

}

#ifdef __cplusplus
#include "emane/any.h"
#include <string>
#include <vector>

namespace EMANE {
    inline Any convertFfiToAny(const FfiAny& ffiAny) {
        auto type = static_cast<Any::Type>(ffiAny.any_type);
        switch(type) {
            case Any::Type::TYPE_INT64: return Any(ffiAny.i64_value);
            case Any::Type::TYPE_INT32: return Any(static_cast<int32_t>(ffiAny.i64_value));
            case Any::Type::TYPE_INT16: return Any(static_cast<int16_t>(ffiAny.i64_value));
            case Any::Type::TYPE_INT8: return Any(static_cast<int8_t>(ffiAny.i64_value));
            case Any::Type::TYPE_UINT64: return Any(ffiAny.u64_value);
            case Any::Type::TYPE_UINT32: return Any(static_cast<uint32_t>(ffiAny.u64_value));
            case Any::Type::TYPE_UINT16: return Any(static_cast<uint16_t>(ffiAny.u64_value));
            case Any::Type::TYPE_UINT8: return Any(static_cast<uint8_t>(ffiAny.u64_value));
            case Any::Type::TYPE_FLOAT: return Any(static_cast<float>(ffiAny.d_value));
            case Any::Type::TYPE_DOUBLE: return Any(ffiAny.d_value);
            case Any::Type::TYPE_BOOL: return Any(ffiAny.u64_value != 0);
            case Any::Type::TYPE_STRING:
            case Any::Type::TYPE_INET_ADDR:
                if (ffiAny.s_value) {
                    return Any::create(std::string(ffiAny.s_value), type);
                } else {
                    return Any::create(std::string(""), type);
                }
            default: return Any(ffiAny.i64_value);
        }
    }

    inline void convertAnyToFfi(const Any& any, FfiAny& ffiAny, std::vector<std::string>& stringStorage) {
        ffiAny.any_type = static_cast<int32_t>(any.getType());
        ffiAny.i64_value = 0;
        ffiAny.u64_value = 0;
        ffiAny.d_value = 0.0;
        ffiAny.s_value = nullptr;

        switch(any.getType()) {
            case Any::Type::TYPE_INT64:
            case Any::Type::TYPE_INT32:
            case Any::Type::TYPE_INT16:
            case Any::Type::TYPE_INT8:
                ffiAny.i64_value = any.asINT64();
                break;
            case Any::Type::TYPE_UINT64:
            case Any::Type::TYPE_UINT32:
            case Any::Type::TYPE_UINT16:
            case Any::Type::TYPE_UINT8:
                ffiAny.u64_value = any.asUINT64();
                break;
            case Any::Type::TYPE_FLOAT:
            case Any::Type::TYPE_DOUBLE:
                ffiAny.d_value = any.asDouble();
                break;
            case Any::Type::TYPE_BOOL:
                ffiAny.u64_value = any.asBool() ? 1 : 0;
                break;
            case Any::Type::TYPE_INET_ADDR:
            case Any::Type::TYPE_STRING:
                stringStorage.push_back(any.asString());
                ffiAny.s_value = stringStorage.back().c_str();
                break;
        }
    }
}
#endif // __cplusplus

#endif // EMANE_RUST_FFI_H
