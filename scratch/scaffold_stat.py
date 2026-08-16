import os

cc_content = """#include "statisticservice.h"
#include "emane/registrarexception.h"
#include "emane/any.h"
#include <cstring>
#include <vector>

// Forward declarations for FFI conversions (assuming they exist from config)
extern "C" {
    struct FfiAny {
        int32_t any_type;
        int64_t i64_value;
        uint64_t u64_value;
        double d_value;
        const char* s_value;
    };
    
    struct FfiStringArray {
        const char** data;
        size_t len;
    };

    struct FfiStatisticInfo {
        const char* name;
        int32_t any_type;
        uint64_t properties;
        const char* description;
        bool is_clearable;
    };

    struct FfiStatisticManifest {
        FfiStatisticInfo* data;
        size_t len;
    };

    struct FfiStatisticTableInfo {
        const char* name;
        uint64_t properties;
        const char* description;
        bool is_clearable;
    };

    struct FfiStatisticTableManifest {
        FfiStatisticTableInfo* data;
        size_t len;
    };

    struct FfiStatisticQueryItem {
        const char* name;
        FfiAny value;
    };

    struct FfiStatisticQueryResult {
        FfiStatisticQueryItem* data;
        size_t len;
    };
    
    struct FfiAnyArray {
        const FfiAny* data;
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

    // Rust exported functions
    void emane_rs_statistic_register(uint16_t build_id, const char* name, int32_t type, uint64_t properties, const char* desc, void* p_statistic, char* err_buf, size_t err_len);
    void emane_rs_statistic_register_table(uint16_t build_id, const char* name, uint64_t properties, const char* desc, void* p_table, void* p_clear_func, char* err_buf, size_t err_len);
    
    FfiStatisticQueryResult emane_rs_statistic_query(uint16_t build_id, FfiStringArray names, char* err_buf, size_t err_len);
    void emane_rs_statistic_free_query_result(FfiStatisticQueryResult res);
    
    FfiStatisticTableQueryResult emane_rs_statistic_query_table(uint16_t build_id, FfiStringArray names, char* err_buf, size_t err_len);
    void emane_rs_statistic_free_table_query_result(FfiStatisticTableQueryResult res);
    
    bool emane_rs_statistic_clear(uint16_t build_id, FfiStringArray names, char* err_buf, size_t err_len);
    bool emane_rs_statistic_clear_table(uint16_t build_id, FfiStringArray names, char* err_buf, size_t err_len);
    
    FfiStatisticManifest emane_rs_statistic_get_manifest(uint16_t build_id);
    void emane_rs_statistic_free_manifest(FfiStatisticManifest res);
    
    FfiStatisticTableManifest emane_rs_statistic_get_table_manifest(uint16_t build_id);
    void emane_rs_statistic_free_table_manifest(FfiStatisticTableManifest res);
}

// C-to-C++ callback exports
extern "C" {
    FfiAny emane_c_statistic_as_any(void* p_statistic) {
        auto stat = static_cast<EMANE::Statistic*>(p_statistic);
        EMANE::Any val = stat->asAny();
        FfiAny ffi{};
        ffi.any_type = static_cast<int32_t>(val.getType());
        if (ffi.any_type == static_cast<int32_t>(EMANE::Any::Type::INT64)) ffi.i64_value = val.asINT64();
        else if (ffi.any_type == static_cast<int32_t>(EMANE::Any::Type::UINT64)) ffi.u64_value = val.asUINT64();
        else if (ffi.any_type == static_cast<int32_t>(EMANE::Any::Type::DOUBLE)) ffi.d_value = val.asDouble();
        else if (ffi.any_type == static_cast<int32_t>(EMANE::Any::Type::STRING)) ffi.s_value = strdup(val.asString().c_str());
        return ffi;
    }
    
    void emane_c_statistic_clear(void* p_statistic) {
        auto stat = static_cast<EMANE::Statistic*>(p_statistic);
        stat->clear();
    }
    
    void emane_c_statistic_table_clear(void* p_clear_func, void* p_table) {
        auto func = static_cast<std::function<void(EMANE::StatisticTablePublisher*)>*>(p_clear_func);
        auto table = static_cast<EMANE::StatisticTablePublisher*>(p_table);
        (*func)(table);
    }
    
    // We will leak the clear_func for now, or we can add a delete function.
    // In EMANE, StatisticService is a singleton that never unregisters, so memory leaks at teardown are acceptable.
}

namespace {
    EMANE::Any convertFfiToAny(const FfiAny& ffi) {
        switch (static_cast<EMANE::Any::Type>(ffi.any_type)) {
            case EMANE::Any::Type::INT64: return EMANE::Any(ffi.i64_value);
            case EMANE::Any::Type::UINT64: return EMANE::Any(ffi.u64_value);
            case EMANE::Any::Type::DOUBLE: return EMANE::Any(ffi.d_value);
            case EMANE::Any::Type::STRING: return EMANE::Any(std::string(ffi.s_value ? ffi.s_value : ""));
            default: return EMANE::Any(ffi.i64_value); // fallback
        }
    }
}

void EMANE::StatisticService::registerStatistic(BuildId buildId, const std::string & sName, Any::Type type, const StatisticProperties & properties, const std::string & sDescription, Statistic * pStatistic) {
    char err_buf[256] = {0};
    emane_rs_statistic_register(buildId, sName.c_str(), static_cast<int32_t>(type), static_cast<uint64_t>(properties), sDescription.c_str(), pStatistic, err_buf, sizeof(err_buf));
    if (err_buf[0] != '\0') {
        throw makeException<RegistrarException>("%s", err_buf);
    }
}

void EMANE::StatisticService::registerTable(BuildId buildId, const std::string & sName, const StatisticProperties & properties, const std::string & sDescription, StatisticTablePublisher * pStatisticTablePublisher, std::function<void(StatisticTablePublisher *)> clearFunc) {
    char err_buf[256] = {0};
    auto pFunc = new std::function<void(StatisticTablePublisher *)>(clearFunc);
    emane_rs_statistic_register_table(buildId, sName.c_str(), static_cast<uint64_t>(properties), sDescription.c_str(), pStatisticTablePublisher, pFunc, err_buf, sizeof(err_buf));
    if (err_buf[0] != '\0') {
        throw makeException<RegistrarException>("%s", err_buf);
    }
}

std::map<std::string, EMANE::Any> EMANE::StatisticService::queryStatistic(BuildId buildId, const std::vector<std::string> & names) const {
    std::vector<const char*> name_ptrs;
    for (const auto& n : names) name_ptrs.push_back(n.c_str());
    FfiStringArray req{name_ptrs.empty() ? nullptr : name_ptrs.data(), name_ptrs.size()};
    
    char err_buf[256] = {0};
    auto res = emane_rs_statistic_query(buildId, req, err_buf, sizeof(err_buf));
    if (err_buf[0] != '\0') {
        emane_rs_statistic_free_query_result(res);
        throw makeException<RegistrarException>("%s", err_buf);
    }
    
    std::map<std::string, EMANE::Any> ret;
    for (size_t i = 0; i < res.len; ++i) {
        ret.insert({std::string(res.data[i].name), convertFfiToAny(res.data[i].value)});
    }
    emane_rs_statistic_free_query_result(res);
    return ret;
}

void EMANE::StatisticService::clearStatistic(BuildId buildId, const std::vector<std::string> & names) const {
    std::vector<const char*> name_ptrs;
    for (const auto& n : names) name_ptrs.push_back(n.c_str());
    FfiStringArray req{name_ptrs.empty() ? nullptr : name_ptrs.data(), name_ptrs.size()};
    
    char err_buf[256] = {0};
    emane_rs_statistic_clear(buildId, req, err_buf, sizeof(err_buf));
    if (err_buf[0] != '\0') {
        throw makeException<RegistrarException>("%s", err_buf);
    }
}

// NOTE: queryTable is omitted from this snippet but will be generated properly in the real code.
// I will implement queryTable by getting values from the C++ object directly via FFI, but actually
// it's easier to just call the C++ object's method from Rust! Wait, Rust calling C++ object methods
// is what emane_c_statistic_table_get_values does.

"""

rs_content = """use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::os::raw::c_char;
use std::ffi::{CStr, CString};
use crate::config::{FfiAny, FfiAnyOwned, VoidPtr};

#[repr(C)]
pub struct FfiStringArray {
    pub data: *const *const c_char,
    pub len: usize,
}
// more Rust structures will be here
"""

with open("/home/joe/src/sleepybishop/emane/scratch/codegen_stat.py", "w") as f:
    f.write('print("Scaffold saved.")')
"""
