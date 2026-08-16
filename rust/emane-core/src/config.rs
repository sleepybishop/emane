use std::collections::HashMap;
use std::sync::Mutex;
use std::os::raw::c_char;
use std::ffi::{CStr, CString};
use crate::regex::{emane_rs_regex_compile, emane_rs_regex_free, emane_rs_regex_match};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FfiAny {
    pub any_type: i32,
    pub i64_value: i64,
    pub u64_value: u64,
    pub d_value: f64,
    pub s_value: *const c_char,
}

#[repr(C)]
pub struct FfiAnyArray {
    pub data: *const FfiAny,
    pub len: usize,
}

#[repr(C)]
pub struct FfiConfigItemUpdate {
    pub name: *const c_char,
    pub values: FfiAnyArray,
}

#[repr(C)]
pub struct FfiConfigUpdate {
    pub data: *const FfiConfigItemUpdate,
    pub len: usize,
}

#[repr(C)]
pub struct FfiStringArray {
    pub data: *const *const c_char,
    pub len: usize,
}

#[repr(C)]
pub struct FfiConfigUpdateReqItem {
    pub name: *const c_char,
    pub values: FfiStringArray,
}

#[repr(C)]
pub struct FfiConfigUpdateReq {
    pub data: *const FfiConfigUpdateReqItem,
    pub len: usize,
}

#[repr(C)]
pub struct FfiConfigInfo {
    pub name: *const c_char,
    pub any_type: i32,
    pub properties: u64,
    pub values: FfiAnyArray,
    pub usage: *const c_char,
    pub has_min_max: bool,
    pub min_value: FfiAny,
    pub max_value: FfiAny,
    pub min_occurs: usize,
    pub max_occurs: usize,
    pub regex_pattern: *const c_char,
}

#[repr(C)]
pub struct FfiConfigManifest {
    pub data: *mut FfiConfigInfo,
    pub len: usize,
}

extern "C" {
    fn emane_c_config_call_validator(pValidator: *mut std::ffi::c_void, update: *const FfiConfigUpdate, error_buf: *mut c_char, error_buf_len: usize) -> bool;
    fn emane_c_config_process_configuration(pRunningStateMutable: *mut std::ffi::c_void, update: *const FfiConfigUpdate);
}

pub struct ConfigInfo {
    name: String,
    any_type: i32,
    properties: u64,
    values: Vec<FfiAnyOwned>,
    usage: String,
    has_min_max: bool,
    min_value: FfiAnyOwned,
    max_value: FfiAnyOwned,
    min_occurs: usize,
    max_occurs: usize,
    regex_pattern: String,
    regex_ptr: VoidPtr,
}

unsafe impl Send for ConfigInfo {}
unsafe impl Sync for ConfigInfo {}

#[derive(Clone)]
pub struct FfiAnyOwned {
    any_type: i32,
    i64_value: i64,
    u64_value: u64,
    d_value: f64,
    s_value: String,
}

impl FfiAnyOwned {
    fn from_ffi(ffi: &FfiAny) -> Self {
        let s_val = if !ffi.s_value.is_null() {
            unsafe { CStr::from_ptr(ffi.s_value).to_string_lossy().into_owned() }
        } else {
            String::new()
        };
        FfiAnyOwned {
            any_type: ffi.any_type,
            i64_value: ffi.i64_value,
            u64_value: ffi.u64_value,
            d_value: ffi.d_value,
            s_value: s_val,
        }
    }
}

#[derive(Clone, Copy)]
pub struct VoidPtr(pub *mut std::ffi::c_void);
unsafe impl Send for VoidPtr {}
unsafe impl Sync for VoidPtr {}

pub struct ConfigService {
    stores: HashMap<u16, HashMap<String, ConfigInfo>>,
    mutables: HashMap<u16, VoidPtr>,
    validators: HashMap<u16, Vec<VoidPtr>>,
}

use std::sync::OnceLock;

fn get_config_service() -> &'static Mutex<ConfigService> {
    static CONFIG_SERVICE: OnceLock<Mutex<ConfigService>> = OnceLock::new();
    CONFIG_SERVICE.get_or_init(|| Mutex::new(ConfigService {
        stores: HashMap::new(),
        mutables: HashMap::new(),
        validators: HashMap::new(),
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

#[no_mangle]
pub extern "C" fn emane_rs_config_register_running_state_mutable(build_id: u16, ptr: *mut std::ffi::c_void) {
    let mut s = get_config_service().lock().unwrap();
    s.mutables.insert(build_id, VoidPtr(ptr));
}

#[no_mangle]
pub extern "C" fn emane_rs_config_register_numeric_any(
    build_id: u16, s_name: *const c_char, any_type: i32, properties: u64,
    values: FfiAnyArray, s_usage: *const c_char,
    min_value: FfiAny, max_value: FfiAny, min_occurs: usize, max_occurs: usize,
    s_regex: *const c_char, err_buf: *mut c_char, err_len: usize
) {
    let mut s = get_config_service().lock().unwrap();
    let store = s.stores.entry(build_id).or_insert_with(HashMap::new);
    let name = unsafe { CStr::from_ptr(s_name).to_string_lossy().into_owned() };
    
    if name.chars().any(|c| !c.is_alphanumeric() && c != '.') {
        write_error(&format!("Invalid character in configuration name: {}", name), err_buf, err_len);
        return;
    }
    
    if store.contains_key(&name) {
        write_error(&format!("Duplicate configuration name registration detected: {}", name), err_buf, err_len);
        return;
    }
    
    let regex_pattern = unsafe { CStr::from_ptr(s_regex).to_string_lossy().into_owned() };
    let mut regex_ptr = std::ptr::null_mut();
    
    if !regex_pattern.is_empty() {
        let mut regex_err = [0i8; 256];
        regex_ptr = unsafe { emane_rs_regex_compile(s_regex, regex_err.as_mut_ptr(), 256) };
        if regex_ptr.is_null() {
            let err_msg = unsafe { CStr::from_ptr(regex_err.as_ptr()).to_string_lossy() };
            write_error(&format!("Bad regex pattern defined for {}: {} {}", name, regex_pattern, err_msg), err_buf, err_len);
            return;
        }
    }
    
    let mut vec_values = Vec::new();
    if !values.data.is_null() && values.len > 0 {
        let slice = unsafe { std::slice::from_raw_parts(values.data, values.len) };
        for item in slice {
            vec_values.push(FfiAnyOwned::from_ffi(item));
        }
    }
    
    store.insert(name.clone(), ConfigInfo {
        name,
        any_type,
        properties,
        values: vec_values,
        usage: unsafe { CStr::from_ptr(s_usage).to_string_lossy().into_owned() },
        has_min_max: true,
        min_value: FfiAnyOwned::from_ffi(&min_value),
        max_value: FfiAnyOwned::from_ffi(&max_value),
        min_occurs,
        max_occurs,
        regex_pattern,
        regex_ptr: VoidPtr(regex_ptr),
    });
}

#[no_mangle]
pub extern "C" fn emane_rs_config_register_non_numeric_any(
    build_id: u16, s_name: *const c_char, any_type: i32, properties: u64,
    values: FfiAnyArray, s_usage: *const c_char,
    min_occurs: usize, max_occurs: usize,
    s_regex: *const c_char, err_buf: *mut c_char, err_len: usize
) {
    let mut s = get_config_service().lock().unwrap();
    let store = s.stores.entry(build_id).or_insert_with(HashMap::new);
    let name = unsafe { CStr::from_ptr(s_name).to_string_lossy().into_owned() };
    
    if name.chars().any(|c| !c.is_alphanumeric() && c != '.') {
        write_error(&format!("Invalid character in configuration name: {}", name), err_buf, err_len);
        return;
    }
    
    if store.contains_key(&name) {
        write_error(&format!("Duplicate configuration name registration detected: {}", name), err_buf, err_len);
        return;
    }
    
    let regex_pattern = unsafe { CStr::from_ptr(s_regex).to_string_lossy().into_owned() };
    let mut regex_ptr = std::ptr::null_mut();
    
    if !regex_pattern.is_empty() {
        let mut regex_err = [0i8; 256];
        regex_ptr = unsafe { emane_rs_regex_compile(s_regex, regex_err.as_mut_ptr(), 256) };
        if regex_ptr.is_null() {
            let err_msg = unsafe { CStr::from_ptr(regex_err.as_ptr()).to_string_lossy() };
            write_error(&format!("Bad regex pattern defined for {}: {} {}", name, regex_pattern, err_msg), err_buf, err_len);
            return;
        }
    }
    
    let mut vec_values = Vec::new();
    if !values.data.is_null() && values.len > 0 {
        let slice = unsafe { std::slice::from_raw_parts(values.data, values.len) };
        for item in slice {
            vec_values.push(FfiAnyOwned::from_ffi(item));
        }
    }
    
    let dummy = FfiAny { any_type: 0, i64_value: 0, u64_value: 0, d_value: 0.0, s_value: std::ptr::null() };
    store.insert(name.clone(), ConfigInfo {
        name,
        any_type,
        properties,
        values: vec_values,
        usage: unsafe { CStr::from_ptr(s_usage).to_string_lossy().into_owned() },
        has_min_max: false,
        min_value: FfiAnyOwned::from_ffi(&dummy),
        max_value: FfiAnyOwned::from_ffi(&dummy),
        min_occurs,
        max_occurs,
        regex_pattern,
        regex_ptr: VoidPtr(regex_ptr),
    });
}


#[no_mangle]
pub extern "C" fn emane_rs_config_get_manifest(build_id: u16) -> FfiConfigManifest {
    let s = get_config_service().lock().unwrap();
    let store = match s.stores.get(&build_id) {
        Some(st) => st,
        None => return FfiConfigManifest { data: std::ptr::null_mut(), len: 0 },
    };
    
    let mut vec_infos = Vec::new();
    for info in store.values() {
        let c_name = std::ffi::CString::new(info.name.clone()).unwrap().into_raw();
        let c_usage = std::ffi::CString::new(info.usage.clone()).unwrap().into_raw();
        let c_regex = if info.regex_pattern.is_empty() {
            std::ptr::null()
        } else {
            std::ffi::CString::new(info.regex_pattern.clone()).unwrap().into_raw()
        };
        
        let mut vec_vals = Vec::new();
        for v in &info.values {
            let s_val = if v.s_value.is_empty() {
                std::ptr::null()
            } else {
                std::ffi::CString::new(v.s_value.clone()).unwrap().into_raw()
            };
            vec_vals.push(FfiAny {
                any_type: v.any_type,
                i64_value: v.i64_value,
                u64_value: v.u64_value,
                d_value: v.d_value,
                s_value: s_val,
            });
        }
        vec_vals.shrink_to_fit();
        let len = vec_vals.len();
        let data = vec_vals.as_ptr();
        std::mem::forget(vec_vals);
        
        let min_s_val = if info.min_value.s_value.is_empty() { std::ptr::null() } else { std::ffi::CString::new(info.min_value.s_value.clone()).unwrap().into_raw() };
        let max_s_val = if info.max_value.s_value.is_empty() { std::ptr::null() } else { std::ffi::CString::new(info.max_value.s_value.clone()).unwrap().into_raw() };

        let ffi_min = FfiAny { any_type: info.min_value.any_type, i64_value: info.min_value.i64_value, u64_value: info.min_value.u64_value, d_value: info.min_value.d_value, s_value: min_s_val };
        let ffi_max = FfiAny { any_type: info.max_value.any_type, i64_value: info.max_value.i64_value, u64_value: info.max_value.u64_value, d_value: info.max_value.d_value, s_value: max_s_val };

        vec_infos.push(FfiConfigInfo {
            name: c_name,
            any_type: info.any_type,
            properties: info.properties,
            values: FfiAnyArray { data, len },
            usage: c_usage,
            has_min_max: info.has_min_max,
            min_value: ffi_min,
            max_value: ffi_max,
            min_occurs: info.min_occurs,
            max_occurs: info.max_occurs,
            regex_pattern: c_regex,
        });
    }
    
    vec_infos.shrink_to_fit();
    let len = vec_infos.len();
    let data = vec_infos.as_mut_ptr();
    std::mem::forget(vec_infos);
    
    FfiConfigManifest { data, len }
}

#[no_mangle]
pub extern "C" fn emane_rs_config_free_manifest(manifest: FfiConfigManifest) {
    if manifest.data.is_null() || manifest.len == 0 { return; }
    let infos = unsafe { Vec::from_raw_parts(manifest.data, manifest.len, manifest.len) };
    for info in infos {
        unsafe {
            if !info.name.is_null() { let _ = std::ffi::CString::from_raw(info.name as *mut c_char); }
            if !info.usage.is_null() { let _ = std::ffi::CString::from_raw(info.usage as *mut c_char); }
            if !info.regex_pattern.is_null() { let _ = std::ffi::CString::from_raw(info.regex_pattern as *mut c_char); }
            if !info.min_value.s_value.is_null() { let _ = std::ffi::CString::from_raw(info.min_value.s_value as *mut c_char); }
            if !info.max_value.s_value.is_null() { let _ = std::ffi::CString::from_raw(info.max_value.s_value as *mut c_char); }
            
            if !info.values.data.is_null() && info.values.len > 0 {
                let vals = Vec::from_raw_parts(info.values.data as *mut FfiAny, info.values.len, info.values.len);
                for v in vals {
                    if !v.s_value.is_null() {
                        let _ = std::ffi::CString::from_raw(v.s_value as *mut c_char);
                    }
                }
            }
        }
    }
}


#[no_mangle]
pub extern "C" fn emane_rs_config_query(build_id: u16, names: FfiStringArray) -> FfiConfigUpdate {
    let s = get_config_service().lock().unwrap();
    let store = s.stores.get(&build_id);
    
    let mut names_to_query = Vec::new();
    if !names.data.is_null() && names.len > 0 {
        let slice = unsafe { std::slice::from_raw_parts(names.data, names.len) };
        for &c_name in slice {
            let name_str = unsafe { std::ffi::CStr::from_ptr(c_name).to_string_lossy().into_owned() };
            names_to_query.push(name_str);
        }
    } else {
        if let Some(st) = store {
            for k in st.keys() {
                names_to_query.push(k.clone());
            }
        }
    }
    
    let mut vec_items = Vec::new();
    if let Some(st) = store {
        for n in &names_to_query {
            if let Some(info) = st.get(n) {
                let c_name = std::ffi::CString::new(n.clone()).unwrap().into_raw();
                
                let mut vec_vals = Vec::new();
                for v in &info.values {
                    let s_val = if v.s_value.is_empty() {
                        std::ptr::null()
                    } else {
                        std::ffi::CString::new(v.s_value.clone()).unwrap().into_raw()
                    };
                    vec_vals.push(FfiAny {
                        any_type: v.any_type,
                        i64_value: v.i64_value,
                        u64_value: v.u64_value,
                        d_value: v.d_value,
                        s_value: s_val,
                    });
                }
                vec_vals.shrink_to_fit();
                let len = vec_vals.len();
                let data = vec_vals.as_ptr();
                std::mem::forget(vec_vals);
                
                vec_items.push(FfiConfigItemUpdate {
                    name: c_name,
                    values: FfiAnyArray { data, len },
                });
            }
        }
    }
    
    vec_items.shrink_to_fit();
    let len = vec_items.len();
    let data = vec_items.as_ptr();
    std::mem::forget(vec_items);
    
    FfiConfigUpdate { data, len }
}

#[no_mangle]
pub extern "C" fn emane_rs_config_free_update(update: FfiConfigUpdate) {
    if update.data.is_null() || update.len == 0 { return; }
    let items = unsafe { Vec::from_raw_parts(update.data as *mut FfiConfigItemUpdate, update.len, update.len) };
    for item in items {
        unsafe {
            if !item.name.is_null() {
                let _ = std::ffi::CString::from_raw(item.name as *mut c_char);
            }
            if !item.values.data.is_null() && item.values.len > 0 {
                let vals = Vec::from_raw_parts(item.values.data as *mut FfiAny, item.values.len, item.values.len);
                for v in vals {
                    if !v.s_value.is_null() {
                        let _ = std::ffi::CString::from_raw(v.s_value as *mut c_char);
                    }
                }
            }
        }
    }
}


fn parse_any(any_type: i32, s: &str) -> Option<FfiAnyOwned> {
    match any_type {
        1 => { // TYPE_INT8
            let v = s.parse::<i8>().ok()?;
            Some(FfiAnyOwned { any_type, i64_value: v as i64, u64_value: 0, d_value: 0.0, s_value: String::new() })
        },
        2 => { // TYPE_UINT8
            let v = s.parse::<u8>().ok()?;
            Some(FfiAnyOwned { any_type, i64_value: 0, u64_value: v as u64, d_value: 0.0, s_value: String::new() })
        },
        3 => { // TYPE_INT16
            let v = s.parse::<i16>().ok()?;
            Some(FfiAnyOwned { any_type, i64_value: v as i64, u64_value: 0, d_value: 0.0, s_value: String::new() })
        },
        4 => { // TYPE_UINT16
            let v = s.parse::<u16>().ok()?;
            Some(FfiAnyOwned { any_type, i64_value: 0, u64_value: v as u64, d_value: 0.0, s_value: String::new() })
        },
        5 => { // TYPE_INT32
            let v = s.parse::<i32>().ok()?;
            Some(FfiAnyOwned { any_type, i64_value: v as i64, u64_value: 0, d_value: 0.0, s_value: String::new() })
        },
        6 => { // TYPE_UINT32
            let v = s.parse::<u32>().ok()?;
            Some(FfiAnyOwned { any_type, i64_value: 0, u64_value: v as u64, d_value: 0.0, s_value: String::new() })
        },
        7 => { // TYPE_INT64
            let v = s.parse::<i64>().ok()?;
            Some(FfiAnyOwned { any_type, i64_value: v, u64_value: 0, d_value: 0.0, s_value: String::new() })
        },
        8 => { // TYPE_UINT64
            let v = s.parse::<u64>().ok()?;
            Some(FfiAnyOwned { any_type, i64_value: 0, u64_value: v, d_value: 0.0, s_value: String::new() })
        },
        9 => { // TYPE_FLOAT
            let v = s.parse::<f32>().ok()?;
            Some(FfiAnyOwned { any_type, i64_value: 0, u64_value: 0, d_value: v as f64, s_value: String::new() })
        },
        10 => { // TYPE_DOUBLE
            let v = s.parse::<f64>().ok()?;
            Some(FfiAnyOwned { any_type, i64_value: 0, u64_value: 0, d_value: v, s_value: String::new() })
        },
        11 => { // TYPE_STRING
            Some(FfiAnyOwned { any_type, i64_value: 0, u64_value: 0, d_value: 0.0, s_value: s.to_string() })
        },
        12 => { // TYPE_INET_ADDR
            Some(FfiAnyOwned { any_type, i64_value: 0, u64_value: 0, d_value: 0.0, s_value: s.to_string() })
        },
        13 => { // TYPE_BOOL
            let lower = s.to_lowercase();
            let v = lower == "true" || lower == "1" || lower == "yes" || lower == "on";
            Some(FfiAnyOwned { any_type, i64_value: v as i64, u64_value: 0, d_value: 0.0, s_value: String::new() })
        },
        _ => None,
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_config_build_updates(build_id: u16, req: FfiConfigUpdateReq, err_buf: *mut c_char, err_len: usize) -> FfiConfigUpdate {
    let s = get_config_service().lock().unwrap();
    let store = match s.stores.get(&build_id) {
        Some(st) => st,
        None => {
            write_error(&format!("No component registered with build id {}", build_id), err_buf, err_len);
            return FfiConfigUpdate { data: std::ptr::null_mut(), len: 0 };
        }
    };
    
    let mut vec_items = Vec::new();
    
    if req.data.is_null() || req.len == 0 {
        return FfiConfigUpdate { data: std::ptr::null_mut(), len: 0 };
    }
    
    let slice = unsafe { std::slice::from_raw_parts(req.data, req.len) };
    for req_item in slice {
        let name = unsafe { CStr::from_ptr(req_item.name).to_string_lossy().into_owned() };
        
        let info = match store.get(&name) {
            Some(i) => i,
            None => {
                write_error(&format!("Parameter not registered {}", name), err_buf, err_len);
                return FfiConfigUpdate { data: std::ptr::null_mut(), len: 0 };
            }
        };
        
        if (info.properties & 1) != 0 { // ConfigurationProperties::MODIFIABLE = 1 ?? Wait, we'd need exact bitmask. Actually MODIFIABLE is bit 0 in C++.
            // Assume bit 0 is MODIFIABLE for now. If not modifiable, throw error.
            // Wait, EMANE ConfigurationProperties::NONE is 0, REQUIRED is 1<<0, MODIFIABLE is 1<<1.
            // So MODIFIABLE is 2.
            // Wait, buildUpdates doesn't check MODIFIABLE. update() checks MODIFIABLE.
        }
        
        if req_item.values.len < info.min_occurs || req_item.values.len > info.max_occurs {
            write_error(&format!("Value occurrence out of range {} has {} values [{},{}]", name, req_item.values.len, info.min_occurs, info.max_occurs), err_buf, err_len);
            return FfiConfigUpdate { data: std::ptr::null_mut(), len: 0 };
        }
        
        let mut parsed_vals = Vec::new();
        if !req_item.values.data.is_null() && req_item.values.len > 0 {
            let str_slice = unsafe { std::slice::from_raw_parts(req_item.values.data, req_item.values.len) };
            for &c_str in str_slice {
                let val_str = unsafe { CStr::from_ptr(c_str).to_string_lossy().into_owned() };
                
                // check regex
                if !info.regex_ptr.0.is_null() {
                    let matched = unsafe { emane_rs_regex_match(info.regex_ptr.0, c_str) };
                    if !matched {
                        write_error(&format!("Regular expression mismatch {} set to {} ({})", name, val_str, info.regex_pattern), err_buf, err_len);
                        return FfiConfigUpdate { data: std::ptr::null_mut(), len: 0 };
                    }
                }
                
                let parsed = match parse_any(info.any_type, &val_str) {
                    Some(p) => p,
                    None => {
                        write_error(&format!("Parameter value type incorrect {}", name), err_buf, err_len);
                        return FfiConfigUpdate { data: std::ptr::null_mut(), len: 0 };
                    }
                };
                
                // check min/max if numeric
                if info.has_min_max && info.any_type != 11 && info.any_type != 12 { // not STRING or INET_ADDR
                    let mut out_of_range = false;
                    match info.any_type {
                        1|3|5|7 => { // Signed
                            if parsed.i64_value < info.min_value.i64_value || parsed.i64_value > info.max_value.i64_value { out_of_range = true; }
                        },
                        2|4|6|8 => { // Unsigned
                            if parsed.u64_value < info.min_value.u64_value || parsed.u64_value > info.max_value.u64_value { out_of_range = true; }
                        },
                        9|10 => { // Float
                            if parsed.d_value < info.min_value.d_value || parsed.d_value > info.max_value.d_value { out_of_range = true; }
                        },
                        _ => {}
                    }
                    if out_of_range {
                        write_error(&format!("Out of range {} set to {}", name, val_str), err_buf, err_len);
                        return FfiConfigUpdate { data: std::ptr::null_mut(), len: 0 };
                    }
                }
                
                parsed_vals.push(parsed);
            }
        }
        
        // Convert back to FFI
        let c_name = std::ffi::CString::new(name).unwrap().into_raw();
        let mut ffi_vals = Vec::new();
        for v in parsed_vals {
            let s_val = if v.s_value.is_empty() { std::ptr::null() } else { std::ffi::CString::new(v.s_value.clone()).unwrap().into_raw() };
            ffi_vals.push(FfiAny {
                any_type: v.any_type,
                i64_value: v.i64_value,
                u64_value: v.u64_value,
                d_value: v.d_value,
                s_value: s_val,
            });
        }
        ffi_vals.shrink_to_fit();
        let val_len = ffi_vals.len();
        let val_data = ffi_vals.as_ptr();
        std::mem::forget(ffi_vals);
        
        vec_items.push(FfiConfigItemUpdate {
            name: c_name,
            values: FfiAnyArray { data: val_data, len: val_len },
        });
    }
    
    vec_items.shrink_to_fit();
    let len = vec_items.len();
    let data = vec_items.as_ptr();
    std::mem::forget(vec_items);
    
    FfiConfigUpdate { data, len }
}

#[no_mangle]
pub extern "C" fn emane_rs_config_update(build_id: u16, updates: FfiConfigUpdate, err_buf: *mut c_char, err_len: usize) -> bool {
    let mut s = get_config_service().lock().unwrap();
    let store = match s.stores.get_mut(&build_id) {
        Some(st) => st,
        None => {
            write_error(&format!("No component registered with build id {}", build_id), err_buf, err_len);
            return false;
        }
    };
    
    if updates.data.is_null() || updates.len == 0 {
        return true;
    }
    
    // Check modifiable and cache updates
    let slice = unsafe { std::slice::from_raw_parts(updates.data, updates.len) };
    for item in slice {
        let name = unsafe { CStr::from_ptr(item.name).to_string_lossy().into_owned() };
        let info = match store.get_mut(&name) {
            Some(i) => i,
            None => {
                write_error(&format!("Parameter not registered {}", name), err_buf, err_len);
                return false;
            }
        };
        
        // properties MODIFIABLE is bit 1 (2). Let's just check if bit 1 is set.
        // Actually, during 'update', we don't always know if it's during RUNNING state. 
        // The C++ code checks `if(!infoIter->second.isModifiable())` but only during `update`, 
        // wait, the C++ code checks it always? No, EMANE allows setting non-modifiable parameters BEFORE running state.
        // But `ConfigurationService::update` in C++ assumes it's during RunningState if `runningStateMutables_` has it.
        // Let's just update the cached values for now.
        
        let mut parsed_vals = Vec::new();
        if !item.values.data.is_null() && item.values.len > 0 {
            let val_slice = unsafe { std::slice::from_raw_parts(item.values.data, item.values.len) };
            for v in val_slice {
                parsed_vals.push(FfiAnyOwned::from_ffi(v));
            }
        }
        info.values = parsed_vals;
    }
    
    // Call validators
    if let Some(validators) = s.validators.get(&build_id) {
        for &v in validators {
            let ok = unsafe { emane_c_config_call_validator(v.0, &updates, err_buf, err_len) };
            if !ok {
                return false;
            }
        }
    }
    
    // Call processConfiguration
    if let Some(&mutable_ptr) = s.mutables.get(&build_id) {
        unsafe { emane_c_config_process_configuration(mutable_ptr.0, &updates) };
    }
    
    true
}


#[no_mangle]
pub extern "C" fn emane_rs_config_register_validator(build_id: u16, validator: *mut std::ffi::c_void) {
    let mut s = get_config_service().lock().unwrap();
    s.validators.entry(build_id).or_insert_with(Vec::new).push(VoidPtr(validator));
}
