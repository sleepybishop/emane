use crate::config::{FfiAny, FfiAnyArray, VoidPtr};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Mutex;
use std::sync::OnceLock;

#[repr(C)]
#[derive(Clone, Copy)]
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

#[cfg(not(test))]
extern "C" {
    fn emane_c_statistic_as_any(p_statistic: *mut std::ffi::c_void) -> FfiAny;
    fn emane_c_statistic_free_any_string(s: *const c_char);
    fn emane_c_statistic_clear(p_statistic: *mut std::ffi::c_void);
    fn emane_c_statistic_table_clear(
        p_clear_func: *mut std::ffi::c_void,
        p_table: *mut std::ffi::c_void,
    );
    fn emane_c_statistic_table_get_values(
        p_table: *mut std::ffi::c_void,
        out_labels: *mut FfiStringArray,
        out_rows: *mut *mut FfiTableRow,
        out_rows_len: *mut usize,
    );
    fn emane_c_statistic_table_free_values(
        labels: FfiStringArray,
        rows: *mut FfiTableRow,
        rows_len: usize,
    );
}

#[cfg(test)]
unsafe extern "C" fn emane_c_statistic_as_any(_: *mut std::ffi::c_void) -> FfiAny {
    FfiAny {
        any_type: 1,
        i64_value: 0,
        u64_value: 0,
        d_value: 0.0,
        s_value: std::ptr::null(),
    }
}

#[cfg(test)]
unsafe extern "C" fn emane_c_statistic_free_any_string(_: *const c_char) {}

#[cfg(test)]
unsafe extern "C" fn emane_c_statistic_clear(_: *mut std::ffi::c_void) {}

#[cfg(test)]
unsafe extern "C" fn emane_c_statistic_table_clear(
    _: *mut std::ffi::c_void,
    _: *mut std::ffi::c_void,
) {
}

#[cfg(test)]
unsafe extern "C" fn emane_c_statistic_table_get_values(
    _: *mut std::ffi::c_void,
    out_labels: *mut FfiStringArray,
    out_rows: *mut *mut FfiTableRow,
    out_rows_len: *mut usize,
) {
    if let Some(labels) = out_labels.as_mut() {
        labels.data = std::ptr::null();
        labels.len = 0;
    }
    if let Some(rows) = out_rows.as_mut() {
        *rows = std::ptr::null_mut();
    }
    if let Some(length) = out_rows_len.as_mut() {
        *length = 0;
    }
}

#[cfg(test)]
unsafe extern "C" fn emane_c_statistic_table_free_values(
    _: FfiStringArray,
    _: *mut FfiTableRow,
    _: usize,
) {
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
    STATISTIC_SERVICE.get_or_init(|| {
        Mutex::new(StatisticService {
            stats: HashMap::new(),
            tables: HashMap::new(),
        })
    })
}

fn write_error(msg: &str, err_buf: *mut c_char, err_len: usize) {
    if err_buf.is_null() || err_len == 0 {
        return;
    }
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
    build_id: u16,
    s_name: *const c_char,
    any_type: i32,
    properties: u64,
    s_desc: *const c_char,
    p_statistic: *mut std::ffi::c_void,
    err_buf: *mut c_char,
    err_len: usize,
) {
    let mut s = get_statistic_service().lock().unwrap();
    let store = s.stats.entry(build_id).or_default();
    let name = unsafe { CStr::from_ptr(s_name).to_string_lossy().into_owned() };

    if name.chars().any(|c| !c.is_alphanumeric() && c != '.') {
        write_error(
            &format!("Invalid character in the statistic name: {}", name),
            err_buf,
            err_len,
        );
        return;
    }

    if store.contains_key(&name) {
        write_error(
            &format!("Statistic already registered: {}", name),
            err_buf,
            err_len,
        );
        return;
    }

    store.insert(
        name.clone(),
        StatisticInfo {
            name,
            any_type,
            properties,
            description: unsafe { CStr::from_ptr(s_desc).to_string_lossy().into_owned() },
            p_statistic: VoidPtr(p_statistic),
        },
    );
}

#[no_mangle]
pub extern "C" fn emane_rs_statistic_register_table(
    build_id: u16,
    s_name: *const c_char,
    properties: u64,
    s_desc: *const c_char,
    p_table: *mut std::ffi::c_void,
    p_clear_func: *mut std::ffi::c_void,
    err_buf: *mut c_char,
    err_len: usize,
) {
    let mut s = get_statistic_service().lock().unwrap();
    let store = s.tables.entry(build_id).or_default();
    let name = unsafe { CStr::from_ptr(s_name).to_string_lossy().into_owned() };

    if name.chars().any(|c| !c.is_alphanumeric() && c != '.') {
        write_error(
            &format!("Invalid character in the statistic table name: {}", name),
            err_buf,
            err_len,
        );
        return;
    }

    if store.contains_key(&name) {
        write_error(
            &format!("Statistic table already registered: {}", name),
            err_buf,
            err_len,
        );
        return;
    }

    store.insert(
        name.clone(),
        StatisticTableInfo {
            name,
            properties,
            description: unsafe { CStr::from_ptr(s_desc).to_string_lossy().into_owned() },
            p_table: VoidPtr(p_table),
            p_clear_func: VoidPtr(p_clear_func),
        },
    );
}

#[no_mangle]
pub extern "C" fn emane_rs_statistic_query(
    build_id: u16,
    names: FfiStringArray,
    err_buf: *mut c_char,
    err_len: usize,
) -> FfiStatisticQueryResult {
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
                    write_error(
                        &format!("Unknown statistic name: {}", name),
                        err_buf,
                        err_len,
                    );
                    return FfiStatisticQueryResult {
                        data: std::ptr::null_mut(),
                        len: 0,
                    };
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
                if item.value.any_type == 4 {
                    // STRING
                    emane_c_statistic_free_any_string(item.value.s_value);
                }
            }
        }
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(res.data, res.len)));
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_statistic_clear(
    build_id: u16,
    names: FfiStringArray,
    err_buf: *mut c_char,
    err_len: usize,
) {
    let s = get_statistic_service().lock().unwrap();
    if let Some(store) = s.stats.get(&build_id) {
        if names.len == 0 {
            for info in store.values() {
                if info.properties == 1 {
                    // CLEARABLE
                    unsafe {
                        emane_c_statistic_clear(info.p_statistic.0);
                    }
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
                        write_error(
                            &format!("Statistic not clearable: {}", name),
                            err_buf,
                            err_len,
                        );
                        return;
                    }
                } else {
                    write_error(
                        &format!("Unknown statistic name: {}", name),
                        err_buf,
                        err_len,
                    );
                    return;
                }
            }
            for p in to_clear {
                unsafe {
                    emane_c_statistic_clear(p);
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_statistic_query_table(
    build_id: u16,
    names: FfiStringArray,
    err_buf: *mut c_char,
    err_len: usize,
) -> FfiStatisticTableQueryResult {
    let s = get_statistic_service().lock().unwrap();
    let mut res = Vec::new();

    if let Some(store) = s.tables.get(&build_id) {
        if names.len == 0 {
            for (name, info) in store {
                let mut c_labels = FfiStringArray {
                    data: std::ptr::null(),
                    len: 0,
                };
                let mut c_rows = std::ptr::null_mut();
                let mut c_rows_len = 0;
                unsafe {
                    emane_c_statistic_table_get_values(
                        info.p_table.0,
                        &mut c_labels,
                        &mut c_rows,
                        &mut c_rows_len,
                    );
                }

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
                    let mut c_labels = FfiStringArray {
                        data: std::ptr::null(),
                        len: 0,
                    };
                    let mut c_rows = std::ptr::null_mut();
                    let mut c_rows_len = 0;
                    unsafe {
                        emane_c_statistic_table_get_values(
                            info.p_table.0,
                            &mut c_labels,
                            &mut c_rows,
                            &mut c_rows_len,
                        );
                    }

                    res.push(FfiStatisticTableQueryItem {
                        name: CString::new(name).unwrap().into_raw(),
                        labels: c_labels,
                        rows: c_rows,
                        rows_len: c_rows_len,
                    });
                } else {
                    write_error(
                        &format!("Unknown statistic table name: {}", name),
                        err_buf,
                        err_len,
                    );
                    return FfiStatisticTableQueryResult {
                        data: std::ptr::null_mut(),
                        len: 0,
                    };
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
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(res.data, res.len)));
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_statistic_clear_table(
    build_id: u16,
    names: FfiStringArray,
    err_buf: *mut c_char,
    err_len: usize,
) {
    let s = get_statistic_service().lock().unwrap();
    if let Some(store) = s.tables.get(&build_id) {
        if names.len == 0 {
            for info in store.values() {
                if info.properties == 1 {
                    unsafe {
                        emane_c_statistic_table_clear(info.p_clear_func.0, info.p_table.0);
                    }
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
                unsafe {
                    emane_c_statistic_table_clear(f, t);
                }
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
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(res.data, res.len)));
        }
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_statistic_get_table_manifest(
    build_id: u16,
) -> FfiStatisticTableManifest {
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
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(res.data, res.len)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::ptr;

    #[test]
    fn test_register_statistic() {
        let build_id = 1;
        let s_name = CString::new("test.stat1").unwrap();
        let s_desc = CString::new("Test desc 1").unwrap();
        let mut err_buf = [0i8; 256];

        emane_rs_statistic_register(
            build_id,
            s_name.as_ptr(),
            4,
            0,
            s_desc.as_ptr(),
            ptr::null_mut(),
            err_buf.as_mut_ptr(),
            err_buf.len(),
        );

        let s = get_statistic_service().lock().unwrap();
        let store = s.stats.get(&build_id).unwrap();
        assert!(store.contains_key("test.stat1"));
    }

    #[test]
    fn test_register_duplicate_statistic() {
        let build_id = 2;
        let s_name = CString::new("test.stat2").unwrap();
        let s_desc = CString::new("Test desc 2").unwrap();
        let mut err_buf = [0i8; 256];

        emane_rs_statistic_register(
            build_id,
            s_name.as_ptr(),
            4,
            0,
            s_desc.as_ptr(),
            ptr::null_mut(),
            err_buf.as_mut_ptr(),
            err_buf.len(),
        );

        // Register again
        emane_rs_statistic_register(
            build_id,
            s_name.as_ptr(),
            4,
            0,
            s_desc.as_ptr(),
            ptr::null_mut(),
            err_buf.as_mut_ptr(),
            err_buf.len(),
        );

        let err_str = unsafe { CStr::from_ptr(err_buf.as_ptr()) }.to_string_lossy();
        assert!(err_str.contains("Statistic already registered"));
    }

    #[test]
    fn test_register_invalid_name() {
        let build_id = 3;
        let s_name = CString::new("invalid name!").unwrap();
        let s_desc = CString::new("Test desc 3").unwrap();
        let mut err_buf = [0i8; 256];

        emane_rs_statistic_register(
            build_id,
            s_name.as_ptr(),
            4,
            0,
            s_desc.as_ptr(),
            ptr::null_mut(),
            err_buf.as_mut_ptr(),
            err_buf.len(),
        );

        let err_str = unsafe { CStr::from_ptr(err_buf.as_ptr()) }.to_string_lossy();
        assert!(err_str.contains("Invalid character"));
    }

    #[test]
    fn test_register_table() {
        let build_id = 4;
        let s_name = CString::new("test.table1").unwrap();
        let s_desc = CString::new("Test table desc 1").unwrap();
        let mut err_buf = [0i8; 256];

        emane_rs_statistic_register_table(
            build_id,
            s_name.as_ptr(),
            0,
            s_desc.as_ptr(),
            ptr::null_mut(),
            ptr::null_mut(),
            err_buf.as_mut_ptr(),
            err_buf.len(),
        );

        let s = get_statistic_service().lock().unwrap();
        let store = s.tables.get(&build_id).unwrap();
        assert!(store.contains_key("test.table1"));
    }

    #[test]
    fn test_clear_statistic_not_clearable() {
        let build_id = 5;
        let s_name = CString::new("test.statnotclearable").unwrap();
        let s_desc = CString::new("Test desc").unwrap();
        let mut err_buf = [0i8; 256];

        emane_rs_statistic_register(
            build_id,
            s_name.as_ptr(),
            4,
            0, // properties = 0 (not clearable)
            s_desc.as_ptr(),
            ptr::null_mut(),
            err_buf.as_mut_ptr(),
            err_buf.len(),
        );

        let names = vec![s_name.as_ptr()];
        let ffi_names = FfiStringArray {
            data: names.as_ptr(),
            len: names.len(),
        };

        emane_rs_statistic_clear(build_id, ffi_names, err_buf.as_mut_ptr(), err_buf.len());

        let err_str = unsafe { CStr::from_ptr(err_buf.as_ptr()) }.to_string_lossy();
        println!("err_str was: '{}'", err_str);
        assert!(err_str.contains("Statistic not clearable"));
    }

    #[test]
    fn test_clear_statistic_unknown() {
        let build_id = 6;
        let s_name = CString::new("unknown.stat").unwrap();
        let mut err_buf = [0i8; 256];

        let names = vec![s_name.as_ptr()];
        let ffi_names = FfiStringArray {
            data: names.as_ptr(),
            len: names.len(),
        };

        // ensure build_id exists
        let dummy_name = CString::new("dummy").unwrap();
        emane_rs_statistic_register(
            build_id,
            dummy_name.as_ptr(),
            4,
            0,
            CString::new("dummy desc").unwrap().as_ptr(),
            ptr::null_mut(),
            err_buf.as_mut_ptr(),
            err_buf.len(),
        );

        emane_rs_statistic_clear(build_id, ffi_names, err_buf.as_mut_ptr(), err_buf.len());

        let err_str = unsafe { CStr::from_ptr(err_buf.as_ptr()) }.to_string_lossy();
        assert!(err_str.contains("Unknown statistic name"));
    }

    #[test]
    fn test_query_statistic_unknown() {
        let build_id = 7;
        let s_name = CString::new("unknown.stat").unwrap();
        let mut err_buf = [0i8; 256];

        let dummy_name = CString::new("dummy").unwrap();
        emane_rs_statistic_register(
            build_id,
            dummy_name.as_ptr(),
            4,
            0,
            CString::new("dummy desc").unwrap().as_ptr(),
            ptr::null_mut(),
            err_buf.as_mut_ptr(),
            err_buf.len(),
        );

        let names = vec![s_name.as_ptr()];
        let ffi_names = FfiStringArray {
            data: names.as_ptr(),
            len: names.len(),
        };

        let result =
            emane_rs_statistic_query(build_id, ffi_names, err_buf.as_mut_ptr(), err_buf.len());
        assert_eq!(result.len, 0);

        let err_str = unsafe { CStr::from_ptr(err_buf.as_ptr()) }.to_string_lossy();
        assert!(err_str.contains("Unknown statistic name"));
    }

    #[test]
    fn test_manifests() {
        let build_id = 8;
        let s_name = CString::new("manifest.stat").unwrap();
        let s_desc = CString::new("Manifest desc").unwrap();
        let mut err_buf = [0i8; 256];

        emane_rs_statistic_register(
            build_id,
            s_name.as_ptr(),
            1,
            1, // properties = 1 (clearable)
            s_desc.as_ptr(),
            ptr::null_mut(),
            err_buf.as_mut_ptr(),
            err_buf.len(),
        );

        let manifest = emane_rs_statistic_get_manifest(build_id);
        assert_eq!(manifest.len, 1);

        let slice = unsafe { std::slice::from_raw_parts(manifest.data, manifest.len) };
        let name = unsafe { CStr::from_ptr(slice[0].name) }.to_string_lossy();
        assert_eq!(name, "manifest.stat");
        assert!(slice[0].is_clearable);

        emane_rs_statistic_free_manifest(manifest);
    }
}
