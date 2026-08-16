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
