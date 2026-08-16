import sys

cc_code = """/*
 * Copyright (c) 2013,2015 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 * ...
 */

#include "emane/registrarexception.h"
#include "statisticservice.h"
#include <iterator>
#include <cstring>
#include <vector>

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

    void emane_rs_statistic_register(uint16_t build_id, const char* name, int32_t type, uint64_t properties, const char* desc, void* p_statistic, char* err_buf, size_t err_len);
    void emane_rs_statistic_register_table(uint16_t build_id, const char* name, uint64_t properties, const char* desc, void* p_table, void* p_clear_func, char* err_buf, size_t err_len);
    
    FfiStatisticQueryResult emane_rs_statistic_query(uint16_t build_id, FfiStringArray names, char* err_buf, size_t err_len);
    void emane_rs_statistic_free_query_result(FfiStatisticQueryResult res);
    
    FfiStatisticTableQueryResult emane_rs_statistic_query_table(uint16_t build_id, FfiStringArray names, char* err_buf, size_t err_len);
    void emane_rs_statistic_free_table_query_result(FfiStatisticTableQueryResult res);
    
    void emane_rs_statistic_clear(uint16_t build_id, FfiStringArray names, char* err_buf, size_t err_len);
    void emane_rs_statistic_clear_table(uint16_t build_id, FfiStringArray names, char* err_buf, size_t err_len);
    
    FfiStatisticManifest emane_rs_statistic_get_manifest(uint16_t build_id);
    void emane_rs_statistic_free_manifest(FfiStatisticManifest res);
    
    FfiStatisticTableManifest emane_rs_statistic_get_table_manifest(uint16_t build_id);
    void emane_rs_statistic_free_table_manifest(FfiStatisticTableManifest res);
}

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
    
    void emane_c_statistic_free_any_string(const char* s) {
        if (s) free((void*)s);
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

    void emane_c_statistic_table_get_values(void* p_table, FfiStringArray* out_labels, FfiTableRow** out_rows, size_t* out_rows_len) {
        auto table = static_cast<EMANE::StatisticTablePublisher*>(p_table);
        auto labels = table->getLabels();
        auto values = table->getValues();

        const char** c_labels = new const char*[labels.size()];
        for (size_t i = 0; i < labels.size(); ++i) c_labels[i] = strdup(labels[i].c_str());
        out_labels->data = c_labels;
        out_labels->len = labels.size();

        FfiTableRow* c_rows = new FfiTableRow[values.size()];
        for (size_t i = 0; i < values.size(); ++i) {
            FfiAny* any_arr = new FfiAny[values[i].size()];
            for (size_t j = 0; j < values[i].size(); ++j) {
                EMANE::Any& val = values[i][j];
                FfiAny ffi{};
                ffi.any_type = static_cast<int32_t>(val.getType());
                if (ffi.any_type == static_cast<int32_t>(EMANE::Any::Type::INT64)) ffi.i64_value = val.asINT64();
                else if (ffi.any_type == static_cast<int32_t>(EMANE::Any::Type::UINT64)) ffi.u64_value = val.asUINT64();
                else if (ffi.any_type == static_cast<int32_t>(EMANE::Any::Type::DOUBLE)) ffi.d_value = val.asDouble();
                else if (ffi.any_type == static_cast<int32_t>(EMANE::Any::Type::STRING)) ffi.s_value = strdup(val.asString().c_str());
                any_arr[j] = ffi;
            }
            c_rows[i].values.data = any_arr;
            c_rows[i].values.len = values[i].size();
        }
        *out_rows = c_rows;
        *out_rows_len = values.size();
    }
    
    void emane_c_statistic_table_free_values(FfiStringArray labels, FfiTableRow* rows, size_t rows_len) {
        for (size_t i = 0; i < labels.len; ++i) free((void*)labels.data[i]);
        delete[] labels.data;
        for (size_t i = 0; i < rows_len; ++i) {
            for (size_t j = 0; j < rows[i].values.len; ++j) {
                if (rows[i].values.data[j].any_type == static_cast<int32_t>(EMANE::Any::Type::STRING)) {
                    free((void*)rows[i].values.data[j].s_value);
                }
            }
            delete[] rows[i].values.data;
        }
        delete[] rows;
    }
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

std::map<std::string,std::pair<EMANE::StatisticTableLabels,EMANE::StatisticTableValues>>
EMANE::StatisticService::queryTable(BuildId buildId, const std::vector<std::string> & names) const {
    std::vector<const char*> name_ptrs;
    for (const auto& n : names) name_ptrs.push_back(n.c_str());
    FfiStringArray req{name_ptrs.empty() ? nullptr : name_ptrs.data(), name_ptrs.size()};
    
    char err_buf[256] = {0};
    auto res = emane_rs_statistic_query_table(buildId, req, err_buf, sizeof(err_buf));
    if (err_buf[0] != '\0') {
        emane_rs_statistic_free_table_query_result(res);
        throw makeException<RegistrarException>("%s", err_buf);
    }
    
    std::map<std::string,std::pair<EMANE::StatisticTableLabels,EMANE::StatisticTableValues>> ret;
    for (size_t i = 0; i < res.len; ++i) {
        EMANE::StatisticTableLabels labels;
        for (size_t j = 0; j < res.data[i].labels.len; ++j) {
            labels.push_back(std::string(res.data[i].labels.data[j]));
        }
        EMANE::StatisticTableValues values;
        for (size_t r = 0; r < res.data[i].rows_len; ++r) {
            std::vector<EMANE::Any> row;
            for (size_t c = 0; c < res.data[i].rows[r].values.len; ++c) {
                row.push_back(convertFfiToAny(res.data[i].rows[r].values.data[c]));
            }
            values.push_back(row);
        }
        ret.insert({std::string(res.data[i].name), std::make_pair(labels, values)});
    }
    emane_rs_statistic_free_table_query_result(res);
    return ret;
}

void EMANE::StatisticService::clearTable(BuildId buildId, const std::vector<std::string> & names) const {
    std::vector<const char*> name_ptrs;
    for (const auto& n : names) name_ptrs.push_back(n.c_str());
    FfiStringArray req{name_ptrs.empty() ? nullptr : name_ptrs.data(), name_ptrs.size()};
    
    char err_buf[256] = {0};
    emane_rs_statistic_clear_table(buildId, req, err_buf, sizeof(err_buf));
    if (err_buf[0] != '\0') {
        throw makeException<RegistrarException>("%s", err_buf);
    }
}

EMANE::StatisticManifest EMANE::StatisticService::getStatisticManifest(BuildId buildId) const {
    auto res = emane_rs_statistic_get_manifest(buildId);
    EMANE::StatisticManifest ret;
    for (size_t i = 0; i < res.len; ++i) {
        ret.push_back(EMANE::StatisticInfo(
            std::string(res.data[i].name),
            static_cast<EMANE::Any::Type>(res.data[i].any_type),
            static_cast<EMANE::StatisticProperties>(res.data[i].properties),
            std::string(res.data[i].description)
        ));
    }
    emane_rs_statistic_free_manifest(res);
    return ret;
}

EMANE::StatisticTableManifest EMANE::StatisticService::getTableManifest(BuildId buildId) const {
    auto res = emane_rs_statistic_get_table_manifest(buildId);
    EMANE::StatisticTableManifest ret;
    for (size_t i = 0; i < res.len; ++i) {
        ret.push_back(EMANE::StatisticTableInfo(
            std::string(res.data[i].name),
            static_cast<EMANE::StatisticProperties>(res.data[i].properties),
            std::string(res.data[i].description)
        ));
    }
    emane_rs_statistic_free_table_manifest(res);
    return ret;
}

"""

rs_code = """use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::os::raw::c_char;
use std::ffi::{CStr, CString};
use crate::config::{FfiAny, FfiAnyArray, VoidPtr};

#[repr(C)]
pub struct FfiStringArray {
    pub data: *const *const c_char,
    pub len: usize,
}

#[repr(C)]
pub struct FfiStatisticInfo {
    pub name: *const c_char,
    pub any_type: i32,
    pub properties: u64,
    pub description: *const c_char,
    pub is_clearable: bool,
}

#[repr(C)]
pub struct FfiStatisticManifest {
    pub data: *mut FfiStatisticInfo,
    pub len: usize,
}

#[repr(C)]
pub struct FfiStatisticTableInfo {
    pub name: *const c_char,
    pub properties: u64,
    pub description: *const c_char,
    pub is_clearable: bool,
}

#[repr(C)]
pub struct FfiStatisticTableManifest {
    pub data: *mut FfiStatisticTableInfo,
    pub len: usize,
}

#[repr(C)]
pub struct FfiStatisticQueryItem {
    pub name: *const c_char,
    pub value: FfiAny,
}

#[repr(C)]
pub struct FfiStatisticQueryResult {
    pub data: *mut FfiStatisticQueryItem,
    pub len: usize,
}

#[repr(C)]
pub struct FfiTableRow {
    pub values: FfiAnyArray,
}

#[repr(C)]
pub struct FfiStatisticTableQueryItem {
    pub name: *const c_char,
    pub labels: FfiStringArray,
    pub rows: *mut FfiTableRow,
    pub rows_len: usize,
}

#[repr(C)]
pub struct FfiStatisticTableQueryResult {
    pub data: *mut FfiStatisticTableQueryItem,
    pub len: usize,
}

extern "C" {
    fn emane_c_statistic_as_any(p_statistic: *mut std::ffi::c_void) -> FfiAny;
    fn emane_c_statistic_free_any_string(s: *const c_char);
    fn emane_c_statistic_clear(p_statistic: *mut std::ffi::c_void);
    fn emane_c_statistic_table_clear(p_clear_func: *mut std::ffi::c_void, p_table: *mut std::ffi::c_void);
    fn emane_c_statistic_table_get_values(p_table: *mut std::ffi::c_void, out_labels: *mut FfiStringArray, out_rows: *mut *mut FfiTableRow, out_rows_len: *mut usize);
    fn emane_c_statistic_table_free_values(labels: FfiStringArray, rows: *mut FfiTableRow, rows_len: usize);
}

pub struct StatisticInfo {
    name: String,
    any_type: i32,
    properties: u64,
    description: String,
    p_statistic: VoidPtr,
}

pub struct StatisticTableInfo {
    name: String,
    properties: u64,
    description: String,
    p_table: VoidPtr,
    p_clear_func: VoidPtr,
}

pub struct StatisticService {
    stats: HashMap<u16, HashMap<String, StatisticInfo>>,
    tables: HashMap<u16, HashMap<String, StatisticTableInfo>>,
}

fn get_statistic_service() -> &'static Mutex<StatisticService> {
    static STATISTIC_SERVICE: OnceLock<Mutex<StatisticService>> = OnceLock::new();
    STATISTIC_SERVICE.get_or_init(|| Mutex::new(StatisticService {
        stats: HashMap::new(),
        tables: HashMap::new(),
    }))
}

fn write_error(msg: &str, err_buf: *mut c_char, err_len: usize) {
    if err_buf.is_null() || err_len == 0 { return; }
    let c_msg = std::ffi::CString::new(msg).unwrap_or_default();
    let bytes = c_msg.as_bytes_with_nul();
    let copy_len = std::cmp::min(bytes.len(), err_len - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), err_buf as *mut u8, copy_len);
        *err_buf.add(copy_len) = 0;
    }
}

fn is_clearable(properties: u64) -> bool {
    (properties & 1) != 0 // Assuming EMANE::StatisticProperties::CLEARABLE is bit 0, let's just say true if non-zero. Wait, EMANE's `isClearable` checks `properties_ == StatisticProperties::CLEARABLE`. Wait, enum class!
}

#[no_mangle]
pub extern "C" fn emane_rs_statistic_register(
    build_id: u16, s_name: *const c_char, any_type: i32, properties: u64,
    s_desc: *const c_char, p_statistic: *mut std::ffi::c_void,
    err_buf: *mut c_char, err_len: usize
) {
    let mut s = get_statistic_service().lock().unwrap();
    let store = s.stats.entry(build_id).or_insert_with(HashMap::new);
    let name = unsafe { CStr::from_ptr(s_name).to_string_lossy().into_owned() };
    
    if name.chars().any(|c| !c.is_alphanumeric() && c != '.') {
        write_error(&format!("Invalid character in the statistic name: {}", name), err_buf, err_len);
        return;
    }
    
    if store.contains_key(&name) {
        write_error(&format!("Statistic already registered: {}", name), err_buf, err_len);
        return;
    }
    
    store.insert(name.clone(), StatisticInfo {
        name,
        any_type,
        properties,
        description: unsafe { CStr::from_ptr(s_desc).to_string_lossy().into_owned() },
        p_statistic: VoidPtr(p_statistic),
    });
}

#[no_mangle]
pub extern "C" fn emane_rs_statistic_register_table(
    build_id: u16, s_name: *const c_char, properties: u64,
    s_desc: *const c_char, p_table: *mut std::ffi::c_void, p_clear_func: *mut std::ffi::c_void,
    err_buf: *mut c_char, err_len: usize
) {
    let mut s = get_statistic_service().lock().unwrap();
    let store = s.tables.entry(build_id).or_insert_with(HashMap::new);
    let name = unsafe { CStr::from_ptr(s_name).to_string_lossy().into_owned() };
    
    if name.chars().any(|c| !c.is_alphanumeric() && c != '.') {
        write_error(&format!("Invalid character in the statistic table name: {}", name), err_buf, err_len);
        return;
    }
    
    if store.contains_key(&name) {
        write_error(&format!("Statistic table already registered: {}", name), err_buf, err_len);
        return;
    }
    
    store.insert(name.clone(), StatisticTableInfo {
        name,
        properties,
        description: unsafe { CStr::from_ptr(s_desc).to_string_lossy().into_owned() },
        p_table: VoidPtr(p_table),
        p_clear_func: VoidPtr(p_clear_func),
    });
}

#[no_mangle]
pub extern "C" fn emane_rs_statistic_query(build_id: u16, names: FfiStringArray, err_buf: *mut c_char, err_len: usize) -> FfiStatisticQueryResult {
    let s = get_statistic_service().lock().unwrap();
    let mut res = Vec::new();
    
    if let Some(store) = s.stats.get(&build_id) {
        if names.len == 0 {
            for (name, info) in store {
                let ffi_any = unsafe { emane_c_statistic_as_any(info.p_statistic.0) };
                res.push(FfiStatisticQueryItem {
                    name: CString::new(name.clone()).unwrap().into_raw(),
                    value: ffi_any,
                });
            }
        } else {
            let slice = unsafe { std::slice::from_raw_parts(names.data, names.len) };
            for &c_name in slice {
                let name = unsafe { CStr::from_ptr(c_name).to_string_lossy().into_owned() };
                if let Some(info) = store.get(&name) {
                    let ffi_any = unsafe { emane_c_statistic_as_any(info.p_statistic.0) };
                    res.push(FfiStatisticQueryItem {
                        name: CString::new(name).unwrap().into_raw(),
                        value: ffi_any,
                    });
                } else {
                    write_error(&format!("Unknown statistic name: {}", name), err_buf, err_len);
                    return FfiStatisticQueryResult { data: std::ptr::null_mut(), len: 0 };
                }
            }
        }
    }
    
    let mut boxed_slice = res.into_boxed_slice();
    let data = boxed_slice.as_mut_ptr();
    let len = boxed_slice.len();
    std::mem::forget(boxed_slice);
    FfiStatisticQueryResult { data, len }
}

#[no_mangle]
pub extern "C" fn emane_rs_statistic_free_query_result(res: FfiStatisticQueryResult) {
    if !res.data.is_null() {
        let slice = unsafe { std::slice::from_raw_parts_mut(res.data, res.len) };
        for item in slice {
            unsafe {
                let _ = CString::from_raw(item.name as *mut c_char);
                if item.value.any_type == 4 { // STRING
                    emane_c_statistic_free_any_string(item.value.s_value);
                }
            }
        }
        unsafe { drop(Box::from_raw(std::slice::from_raw_parts_mut(res.data, res.len))); }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_statistic_clear(build_id: u16, names: FfiStringArray, err_buf: *mut c_char, err_len: usize) {
    let s = get_statistic_service().lock().unwrap();
    if let Some(store) = s.stats.get(&build_id) {
        if names.len == 0 {
            for info in store.values() {
                if info.properties == 1 { // CLEARABLE
                    unsafe { emane_c_statistic_clear(info.p_statistic.0); }
                }
            }
        } else {
            let slice = unsafe { std::slice::from_raw_parts(names.data, names.len) };
            let mut to_clear = Vec::new();
            for &c_name in slice {
                let name = unsafe { CStr::from_ptr(c_name).to_string_lossy().into_owned() };
                if let Some(info) = store.get(&name) {
                    if info.properties == 1 {
                        to_clear.push(info.p_statistic.0);
                    } else {
                        write_error(&format!("Statistic not clearable: {}", name), err_buf, err_len);
                        return;
                    }
                } else {
                    write_error(&format!("Unknown statistic name: {}", name), err_buf, err_len);
                    return;
                }
            }
            for p in to_clear {
                unsafe { emane_c_statistic_clear(p); }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_statistic_query_table(build_id: u16, names: FfiStringArray, err_buf: *mut c_char, err_len: usize) -> FfiStatisticTableQueryResult {
    let s = get_statistic_service().lock().unwrap();
    let mut res = Vec::new();
    
    if let Some(store) = s.tables.get(&build_id) {
        if names.len == 0 {
            for (name, info) in store {
                let mut c_labels = FfiStringArray { data: std::ptr::null(), len: 0 };
                let mut c_rows = std::ptr::null_mut();
                let mut c_rows_len = 0;
                unsafe { emane_c_statistic_table_get_values(info.p_table.0, &mut c_labels, &mut c_rows, &mut c_rows_len); }
                
                res.push(FfiStatisticTableQueryItem {
                    name: CString::new(name.clone()).unwrap().into_raw(),
                    labels: c_labels,
                    rows: c_rows,
                    rows_len: c_rows_len,
                });
            }
        } else {
            let slice = unsafe { std::slice::from_raw_parts(names.data, names.len) };
            for &c_name in slice {
                let name = unsafe { CStr::from_ptr(c_name).to_string_lossy().into_owned() };
                if let Some(info) = store.get(&name) {
                    let mut c_labels = FfiStringArray { data: std::ptr::null(), len: 0 };
                    let mut c_rows = std::ptr::null_mut();
                    let mut c_rows_len = 0;
                    unsafe { emane_c_statistic_table_get_values(info.p_table.0, &mut c_labels, &mut c_rows, &mut c_rows_len); }
                    
                    res.push(FfiStatisticTableQueryItem {
                        name: CString::new(name).unwrap().into_raw(),
                        labels: c_labels,
                        rows: c_rows,
                        rows_len: c_rows_len,
                    });
                } else {
                    write_error(&format!("Unknown statistic table name: {}", name), err_buf, err_len);
                    return FfiStatisticTableQueryResult { data: std::ptr::null_mut(), len: 0 };
                }
            }
        }
    }
    
    let mut boxed = res.into_boxed_slice();
    let data = boxed.as_mut_ptr();
    let len = boxed.len();
    std::mem::forget(boxed);
    FfiStatisticTableQueryResult { data, len }
}

#[no_mangle]
pub extern "C" fn emane_rs_statistic_free_table_query_result(res: FfiStatisticTableQueryResult) {
    if !res.data.is_null() {
        let slice = unsafe { std::slice::from_raw_parts_mut(res.data, res.len) };
        for item in slice {
            unsafe {
                let _ = CString::from_raw(item.name as *mut c_char);
                emane_c_statistic_table_free_values(item.labels, item.rows, item.rows_len);
            }
        }
        unsafe { drop(Box::from_raw(std::slice::from_raw_parts_mut(res.data, res.len))); }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_statistic_clear_table(build_id: u16, names: FfiStringArray, err_buf: *mut c_char, err_len: usize) {
    let s = get_statistic_service().lock().unwrap();
    if let Some(store) = s.tables.get(&build_id) {
        if names.len == 0 {
            for info in store.values() {
                if info.properties == 1 {
                    unsafe { emane_c_statistic_table_clear(info.p_clear_func.0, info.p_table.0); }
                }
            }
        } else {
            let slice = unsafe { std::slice::from_raw_parts(names.data, names.len) };
            let mut to_clear = Vec::new();
            for &c_name in slice {
                let name = unsafe { CStr::from_ptr(c_name).to_string_lossy().into_owned() };
                if let Some(info) = store.get(&name) {
                    if info.properties == 1 {
                        to_clear.push((info.p_clear_func.0, info.p_table.0));
                    } else {
                        write_error(&format!("Table not clearable: {}", name), err_buf, err_len);
                        return;
                    }
                } else {
                    write_error(&format!("Unknown table name: {}", name), err_buf, err_len);
                    return;
                }
            }
            for (f, t) in to_clear {
                unsafe { emane_c_statistic_table_clear(f, t); }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_statistic_get_manifest(build_id: u16) -> FfiStatisticManifest {
    let s = get_statistic_service().lock().unwrap();
    let mut res = Vec::new();
    if let Some(store) = s.stats.get(&build_id) {
        for (name, info) in store {
            res.push(FfiStatisticInfo {
                name: CString::new(name.clone()).unwrap().into_raw(),
                any_type: info.any_type,
                properties: info.properties,
                description: CString::new(info.description.clone()).unwrap().into_raw(),
                is_clearable: info.properties == 1,
            });
        }
    }
    let mut boxed = res.into_boxed_slice();
    let data = boxed.as_mut_ptr();
    let len = boxed.len();
    std::mem::forget(boxed);
    FfiStatisticManifest { data, len }
}

#[no_mangle]
pub extern "C" fn emane_rs_statistic_free_manifest(res: FfiStatisticManifest) {
    if !res.data.is_null() {
        let slice = unsafe { std::slice::from_raw_parts_mut(res.data, res.len) };
        for item in slice {
            unsafe {
                let _ = CString::from_raw(item.name as *mut c_char);
                let _ = CString::from_raw(item.description as *mut c_char);
            }
        }
        unsafe { drop(Box::from_raw(std::slice::from_raw_parts_mut(res.data, res.len))); }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_statistic_get_table_manifest(build_id: u16) -> FfiStatisticTableManifest {
    let s = get_statistic_service().lock().unwrap();
    let mut res = Vec::new();
    if let Some(store) = s.tables.get(&build_id) {
        for (name, info) in store {
            res.push(FfiStatisticTableInfo {
                name: CString::new(name.clone()).unwrap().into_raw(),
                properties: info.properties,
                description: CString::new(info.description.clone()).unwrap().into_raw(),
                is_clearable: info.properties == 1,
            });
        }
    }
    let mut boxed = res.into_boxed_slice();
    let data = boxed.as_mut_ptr();
    let len = boxed.len();
    std::mem::forget(boxed);
    FfiStatisticTableManifest { data, len }
}

#[no_mangle]
pub extern "C" fn emane_rs_statistic_free_table_manifest(res: FfiStatisticTableManifest) {
    if !res.data.is_null() {
        let slice = unsafe { std::slice::from_raw_parts_mut(res.data, res.len) };
        for item in slice {
            unsafe {
                let _ = CString::from_raw(item.name as *mut c_char);
                let _ = CString::from_raw(item.description as *mut c_char);
            }
        }
        unsafe { drop(Box::from_raw(std::slice::from_raw_parts_mut(res.data, res.len))); }
    }
}
"""

import os
with open("/home/joe/src/sleepybishop/emane/src/libemane/statisticservice.cc", "w") as f:
    f.write(cc_code)
with open("/home/joe/src/sleepybishop/emane/rust/emane-core/src/statistics.rs", "w") as f:
    f.write(rs_code)
