import re

def update_config_rs():
    path = "/home/joe/src/sleepybishop/emane/rust/emane-core/src/config.rs"
    with open(path, "r") as f:
        content = f.read()
    
    query_impl = """
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
"""
    
    content = re.sub(r'#\[no_mangle\]\npub extern "C" fn emane_rs_config_query.*?\n}\n\n#\[no_mangle\]\npub extern "C" fn emane_rs_config_free_update.*?\n}', query_impl.strip(), content, flags=re.DOTALL)
    
    with open(path, "w") as f:
        f.write(content)

def update_statistics_rs():
    path = "/home/joe/src/sleepybishop/emane/rust/emane-core/src/statistics.rs"
    with open(path, "r") as f:
        content = f.read()
    
    query_impl = """
#[no_mangle]
pub extern "C" fn emane_rs_statistic_query(build_id: u16, names: FfiStringArray) -> FfiStatisticUpdate {
    let s = get_statistic_service().lock().unwrap();
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
                
                // For statistics, we read the current value from p_statistic!
                let current_val = unsafe {
                    if !info.p_statistic.0.is_null() {
                        emane_c_statistic_get_value(info.p_statistic.0)
                    } else {
                        FfiAny { any_type: 0, i64_value: 0, u64_value: 0, d_value: 0.0, s_value: std::ptr::null() }
                    }
                };
                
                // Deep copy the C-string if it has one (the C++ get_value will return a pointer we need to duplicate to own)
                let s_val = if !current_val.s_value.is_null() {
                    let str_val = unsafe { std::ffi::CStr::from_ptr(current_val.s_value).to_string_lossy().into_owned() };
                    unsafe { emane_c_statistic_free_value(current_val) };
                    std::ffi::CString::new(str_val).unwrap().into_raw()
                } else {
                    std::ptr::null()
                };
                
                let final_val = FfiAny {
                    any_type: current_val.any_type,
                    i64_value: current_val.i64_value,
                    u64_value: current_val.u64_value,
                    d_value: current_val.d_value,
                    s_value: s_val,
                };
                
                vec_items.push(FfiStatisticItemUpdate {
                    name: c_name,
                    value: final_val,
                });
            }
        }
    }
    
    vec_items.shrink_to_fit();
    let len = vec_items.len();
    let data = vec_items.as_ptr();
    std::mem::forget(vec_items);
    
    FfiStatisticUpdate { data, len }
}

#[no_mangle]
pub extern "C" fn emane_rs_statistic_free_update(update: FfiStatisticUpdate) {
    if update.data.is_null() || update.len == 0 { return; }
    let items = unsafe { Vec::from_raw_parts(update.data as *mut FfiStatisticItemUpdate, update.len, update.len) };
    for item in items {
        unsafe {
            if !item.name.is_null() {
                let _ = std::ffi::CString::from_raw(item.name as *mut c_char);
            }
            if !item.value.s_value.is_null() {
                let _ = std::ffi::CString::from_raw(item.value.s_value as *mut c_char);
            }
        }
    }
}
"""
    
    content = re.sub(r'#\[no_mangle\]\npub extern "C" fn emane_rs_statistic_query.*?\n}\n\n#\[no_mangle\]\npub extern "C" fn emane_rs_statistic_free_update.*?\n}', query_impl.strip(), content, flags=re.DOTALL)
    
    with open(path, "w") as f:
        f.write(content)

update_config_rs()
update_statistics_rs()
print("Updated config.rs and statistics.rs")
