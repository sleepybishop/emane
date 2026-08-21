#[cfg(feature = "legacy-ffi")]
use crate::config::{FfiAny, FfiAnyArray, VoidPtr};
use std::collections::HashMap;
#[cfg(not(feature = "legacy-ffi"))]
use std::ffi::c_void;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use crate::rf_signal_table::RFSignalTable;

#[cfg(not(feature = "legacy-ffi"))]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FfiAny {
    pub any_type: i32,
    pub i64_value: i64,
    pub u64_value: u64,
    pub d_value: f64,
    pub s_value: *const c_char,
}

#[cfg(not(feature = "legacy-ffi"))]
#[repr(C)]
pub struct FfiAnyArray {
    pub data: *const FfiAny,
    pub len: usize,
}

#[cfg(not(feature = "legacy-ffi"))]
#[derive(Clone, Copy)]
struct VoidPtr(*mut c_void);

#[cfg(not(feature = "legacy-ffi"))]
unsafe impl Send for VoidPtr {}

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
    pub native: bool,
}

#[repr(C)]
pub struct FfiStatisticTableQueryResult {
    pub data: *mut FfiStatisticTableQueryItem,
    pub len: usize,
}

#[cfg(all(feature = "legacy-ffi", not(test)))]
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

#[cfg(any(test, not(feature = "legacy-ffi")))]
unsafe extern "C" fn emane_c_statistic_as_any(_: *mut std::ffi::c_void) -> FfiAny {
    FfiAny {
        any_type: 1,
        i64_value: 0,
        u64_value: 0,
        d_value: 0.0,
        s_value: std::ptr::null(),
    }
}

#[cfg(any(test, not(feature = "legacy-ffi")))]
unsafe extern "C" fn emane_c_statistic_free_any_string(_: *const c_char) {}

#[cfg(any(test, not(feature = "legacy-ffi")))]
unsafe extern "C" fn emane_c_statistic_clear(_: *mut std::ffi::c_void) {}

#[cfg(any(test, not(feature = "legacy-ffi")))]
unsafe extern "C" fn emane_c_statistic_table_clear(
    _: *mut std::ffi::c_void,
    _: *mut std::ffi::c_void,
) {
}

#[cfg(any(test, not(feature = "legacy-ffi")))]
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

#[cfg(any(test, not(feature = "legacy-ffi")))]
unsafe extern "C" fn emane_c_statistic_table_free_values(
    _: FfiStringArray,
    _: *mut FfiTableRow,
    _: usize,
) {
}

pub struct StatisticInfo {
    any_type: i32,
    properties: u64,
    description: String,
    source: StatisticSource,
}

enum StatisticSource {
    Legacy(VoidPtr),
    NativeU64 { handle: u64, value: Arc<AtomicU64> },
}

pub struct StatisticTableInfo {
    properties: u64,
    description: String,
    source: StatisticTableSource,
}

enum StatisticTableSource {
    Legacy {
        p_table: VoidPtr,
        p_clear_func: VoidPtr,
    },
    NativeRfSignal {
        handle: u64,
        table: Arc<Mutex<RFSignalTable>>,
    },
}

pub struct StatisticService {
    stats: HashMap<u16, HashMap<String, StatisticInfo>>,
    tables: HashMap<u16, HashMap<String, StatisticTableInfo>>,
    native_counters: HashMap<u64, Arc<AtomicU64>>,
    native_rf_signal_tables: HashMap<u64, Arc<Mutex<RFSignalTable>>>,
    next_native_handle: u64,
}

fn get_statistic_service() -> &'static Mutex<StatisticService> {
    static STATISTIC_SERVICE: OnceLock<Mutex<StatisticService>> = OnceLock::new();
    STATISTIC_SERVICE.get_or_init(|| {
        Mutex::new(StatisticService {
            stats: HashMap::new(),
            tables: HashMap::new(),
            native_counters: HashMap::new(),
            native_rf_signal_tables: HashMap::new(),
            next_native_handle: 1,
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
    properties == 1
}

fn valid_name(name: &str) -> bool {
    !name.is_empty() && !name.chars().any(|c| !c.is_alphanumeric() && c != '.')
}

fn statistic_value(info: &StatisticInfo) -> FfiAny {
    match &info.source {
        StatisticSource::Legacy(pointer) => unsafe { emane_c_statistic_as_any(pointer.0) },
        StatisticSource::NativeU64 { value, .. } => FfiAny {
            any_type: 8,
            i64_value: 0,
            u64_value: value.load(Ordering::Relaxed),
            d_value: 0.0,
            s_value: std::ptr::null(),
        },
    }
}

fn clear_statistic(info: &StatisticInfo) {
    match &info.source {
        StatisticSource::Legacy(pointer) => unsafe { emane_c_statistic_clear(pointer.0) },
        StatisticSource::NativeU64 { value, .. } => value.store(0, Ordering::Relaxed),
    }
}

pub fn register_native_counter(
    build_id: u16,
    name: &str,
    description: &str,
    clearable: bool,
) -> Option<u64> {
    if !valid_name(name) {
        return None;
    }
    let mut service = get_statistic_service().lock().ok()?;
    if service
        .stats
        .get(&build_id)
        .is_some_and(|store| store.contains_key(name))
    {
        return None;
    }
    let handle = service.next_native_handle.max(1);
    service.next_native_handle = handle.wrapping_add(1).max(1);
    let value = Arc::new(AtomicU64::new(0));
    service.native_counters.insert(handle, Arc::clone(&value));
    service.stats.entry(build_id).or_default().insert(
        name.to_string(),
        StatisticInfo {
            any_type: 8,
            properties: u64::from(clearable),
            description: description.to_string(),
            source: StatisticSource::NativeU64 { handle, value },
        },
    );
    Some(handle)
}

pub fn increment_native_counter(handle: u64, amount: u64) -> bool {
    let Ok(service) = get_statistic_service().lock() else {
        return false;
    };
    let Some(value) = service.native_counters.get(&handle) else {
        return false;
    };
    value.fetch_add(amount, Ordering::Relaxed);
    true
}

pub fn register_native_rf_signal_table(build_id: u16, nem_id: u16) -> Option<u64> {
    let mut service = get_statistic_service().lock().ok()?;
    if service
        .tables
        .get(&build_id)
        .is_some_and(|store| store.contains_key("ReceiveMetricTable"))
    {
        return None;
    }
    let handle = service.next_native_handle.max(1);
    service.next_native_handle = handle.wrapping_add(1).max(1);
    let table = Arc::new(Mutex::new(RFSignalTable::new(nem_id)));
    service
        .native_rf_signal_tables
        .insert(handle, Arc::clone(&table));
    service.tables.entry(build_id).or_default().insert(
        "ReceiveMetricTable".to_string(),
        StatisticTableInfo {
            properties: 1,
            description: "Table of RF receive metrics from peering NEMs".to_string(),
            source: StatisticTableSource::NativeRfSignal { handle, table },
        },
    );
    Some(handle)
}

pub fn configure_native_rf_signal_table(
    handle: u64,
    average_all_antennas: bool,
    average_all_frequencies: bool,
) -> bool {
    let Ok(service) = get_statistic_service().lock() else {
        return false;
    };
    let Some(table) = service.native_rf_signal_tables.get(&handle) else {
        return false;
    };
    let Ok(mut table) = table.lock() else {
        return false;
    };
    table.set_average_all_antennas(average_all_antennas);
    table.set_average_all_frequencies(average_all_frequencies);
    true
}

#[allow(clippy::too_many_arguments)]
pub fn update_native_rf_signal_table(
    handle: u64,
    source: u16,
    antenna: u16,
    frequency_hz: u64,
    rx_power_dbm: f64,
    sinr_db: f64,
    noise_floor_dbm: f64,
    receiver_sensitivity_dbm: f64,
) -> bool {
    let Ok(service) = get_statistic_service().lock() else {
        return false;
    };
    let Some(table) = service.native_rf_signal_tables.get(&handle) else {
        return false;
    };
    let Ok(mut table) = table.lock() else {
        return false;
    };
    table.update(
        source,
        antenna,
        frequency_hz,
        rx_power_dbm,
        sinr_db,
        noise_floor_dbm,
        receiver_sensitivity_dbm,
    );
    true
}

pub fn unregister_native_statistics(build_id: u16) {
    let Ok(mut service) = get_statistic_service().lock() else {
        return;
    };
    let handles: Vec<_> = service
        .stats
        .remove(&build_id)
        .into_iter()
        .flat_map(|store| store.into_values())
        .filter_map(|info| match info.source {
            StatisticSource::NativeU64 { handle, .. } => Some(handle),
            StatisticSource::Legacy(_) => None,
        })
        .collect();
    for handle in handles {
        service.native_counters.remove(&handle);
    }
    if let Some(tables) = service.tables.remove(&build_id) {
        for info in tables.into_values() {
            if let StatisticTableSource::NativeRfSignal { handle, .. } = info.source {
                service.native_rf_signal_tables.remove(&handle);
            }
        }
    }
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

    if !valid_name(&name) {
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
            any_type,
            properties,
            description: unsafe { CStr::from_ptr(s_desc).to_string_lossy().into_owned() },
            source: StatisticSource::Legacy(VoidPtr(p_statistic)),
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
            properties,
            description: unsafe { CStr::from_ptr(s_desc).to_string_lossy().into_owned() },
            source: StatisticTableSource::Legacy {
                p_table: VoidPtr(p_table),
                p_clear_func: VoidPtr(p_clear_func),
            },
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
                res.push(FfiStatisticQueryItem {
                    name: CString::new(name.clone()).unwrap().into_raw(),
                    value: statistic_value(info),
                });
            }
        } else {
            let slice = unsafe { std::slice::from_raw_parts(names.data, names.len) };
            for &c_name in slice {
                let name = unsafe { CStr::from_ptr(c_name).to_string_lossy().into_owned() };
                if let Some(info) = store.get(&name) {
                    res.push(FfiStatisticQueryItem {
                        name: CString::new(name).unwrap().into_raw(),
                        value: statistic_value(info),
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
                if item.value.any_type == 11 {
                    // STRING
                    emane_c_statistic_free_any_string(item.value.s_value);
                }
            }
        }
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                res.data, res.len,
            )));
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
                if is_clearable(info.properties) {
                    clear_statistic(info);
                }
            }
        } else {
            let slice = unsafe { std::slice::from_raw_parts(names.data, names.len) };
            for &c_name in slice {
                let name = unsafe { CStr::from_ptr(c_name).to_string_lossy().into_owned() };
                if let Some(info) = store.get(&name) {
                    if is_clearable(info.properties) {
                        clear_statistic(info);
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
        }
    }
}

fn native_rf_signal_table_values(
    table: &Arc<Mutex<RFSignalTable>>,
) -> (FfiStringArray, *mut FfiTableRow, usize) {
    let labels = [
        "NEM",
        "Antenna",
        "Frequency",
        "Samples",
        "Avg Rx Power",
        "Avg Noise",
        "Avg SINR",
        "Avg INR",
    ];
    let mut label_ptrs = labels
        .into_iter()
        .map(|label| CString::new(label).unwrap().into_raw() as *const c_char)
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let ffi_labels = FfiStringArray {
        data: label_ptrs.as_mut_ptr(),
        len: label_ptrs.len(),
    };
    std::mem::forget(label_ptrs);

    let rows = table.lock().map(|table| table.rows()).unwrap_or_default();
    let mut ffi_rows = rows
        .into_iter()
        .map(|row| {
            let antenna = row
                .antenna
                .map(|value| (8, value as u64, None))
                .unwrap_or_else(|| (11, 0, Some("NA")));
            let frequency = row
                .frequency_hz
                .map(|value| (8, value, None))
                .unwrap_or_else(|| (11, 0, Some("NA")));
            let mut values = vec![
                FfiAny {
                    any_type: 8,
                    i64_value: 0,
                    u64_value: u64::from(row.source),
                    d_value: 0.0,
                    s_value: std::ptr::null(),
                },
                FfiAny {
                    any_type: antenna.0,
                    i64_value: 0,
                    u64_value: antenna.1,
                    d_value: 0.0,
                    s_value: antenna
                        .2
                        .map(|value| CString::new(value).unwrap().into_raw() as *const c_char)
                        .unwrap_or(std::ptr::null()),
                },
                FfiAny {
                    any_type: frequency.0,
                    i64_value: 0,
                    u64_value: frequency.1,
                    d_value: 0.0,
                    s_value: frequency
                        .2
                        .map(|value| CString::new(value).unwrap().into_raw() as *const c_char)
                        .unwrap_or(std::ptr::null()),
                },
                FfiAny {
                    any_type: 8,
                    i64_value: 0,
                    u64_value: row.samples,
                    d_value: 0.0,
                    s_value: std::ptr::null(),
                },
                FfiAny {
                    any_type: 10,
                    i64_value: 0,
                    u64_value: 0,
                    d_value: row.average_rx_power_dbm,
                    s_value: std::ptr::null(),
                },
                FfiAny {
                    any_type: 10,
                    i64_value: 0,
                    u64_value: 0,
                    d_value: row.average_noise_floor_dbm,
                    s_value: std::ptr::null(),
                },
                FfiAny {
                    any_type: 10,
                    i64_value: 0,
                    u64_value: 0,
                    d_value: row.average_sinr_db,
                    s_value: std::ptr::null(),
                },
                FfiAny {
                    any_type: 10,
                    i64_value: 0,
                    u64_value: 0,
                    d_value: row.average_inr_db,
                    s_value: std::ptr::null(),
                },
            ]
            .into_boxed_slice();
            let result = FfiTableRow {
                values: FfiAnyArray {
                    data: values.as_mut_ptr(),
                    len: values.len(),
                },
            };
            std::mem::forget(values);
            result
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let rows_len = ffi_rows.len();
    let rows_ptr = ffi_rows.as_mut_ptr();
    std::mem::forget(ffi_rows);
    (ffi_labels, rows_ptr, rows_len)
}

unsafe fn free_native_table_values(
    labels: FfiStringArray,
    rows: *mut FfiTableRow,
    rows_len: usize,
) {
    if !labels.data.is_null() {
        let pointers =
            Vec::from_raw_parts(labels.data as *mut *const c_char, labels.len, labels.len);
        for pointer in pointers {
            if !pointer.is_null() {
                drop(CString::from_raw(pointer as *mut c_char));
            }
        }
    }
    if !rows.is_null() {
        let rows = Vec::from_raw_parts(rows, rows_len, rows_len);
        for row in rows {
            if !row.values.data.is_null() {
                let values = Vec::from_raw_parts(
                    row.values.data as *mut FfiAny,
                    row.values.len,
                    row.values.len,
                );
                for value in values {
                    if value.any_type == 11 && !value.s_value.is_null() {
                        drop(CString::from_raw(value.s_value as *mut c_char));
                    }
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
                let (c_labels, c_rows, c_rows_len, native) = match &info.source {
                    StatisticTableSource::Legacy { p_table, .. } => {
                        let mut labels = FfiStringArray {
                            data: std::ptr::null(),
                            len: 0,
                        };
                        let mut rows = std::ptr::null_mut();
                        let mut rows_len = 0;
                        unsafe {
                            emane_c_statistic_table_get_values(
                                p_table.0,
                                &mut labels,
                                &mut rows,
                                &mut rows_len,
                            )
                        };
                        (labels, rows, rows_len, false)
                    }
                    StatisticTableSource::NativeRfSignal { table, .. } => {
                        let (labels, rows, rows_len) = native_rf_signal_table_values(table);
                        (labels, rows, rows_len, true)
                    }
                };

                res.push(FfiStatisticTableQueryItem {
                    name: CString::new(name.clone()).unwrap().into_raw(),
                    labels: c_labels,
                    rows: c_rows,
                    rows_len: c_rows_len,
                    native,
                });
            }
        } else {
            let slice = unsafe { std::slice::from_raw_parts(names.data, names.len) };
            for &c_name in slice {
                let name = unsafe { CStr::from_ptr(c_name).to_string_lossy().into_owned() };
                if let Some(info) = store.get(&name) {
                    let (c_labels, c_rows, c_rows_len, native) = match &info.source {
                        StatisticTableSource::Legacy { p_table, .. } => {
                            let mut labels = FfiStringArray {
                                data: std::ptr::null(),
                                len: 0,
                            };
                            let mut rows = std::ptr::null_mut();
                            let mut rows_len = 0;
                            unsafe {
                                emane_c_statistic_table_get_values(
                                    p_table.0,
                                    &mut labels,
                                    &mut rows,
                                    &mut rows_len,
                                )
                            };
                            (labels, rows, rows_len, false)
                        }
                        StatisticTableSource::NativeRfSignal { table, .. } => {
                            let (labels, rows, rows_len) = native_rf_signal_table_values(table);
                            (labels, rows, rows_len, true)
                        }
                    };

                    res.push(FfiStatisticTableQueryItem {
                        name: CString::new(name).unwrap().into_raw(),
                        labels: c_labels,
                        rows: c_rows,
                        rows_len: c_rows_len,
                        native,
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
                if item.native {
                    free_native_table_values(item.labels, item.rows, item.rows_len);
                } else {
                    emane_c_statistic_table_free_values(item.labels, item.rows, item.rows_len);
                }
            }
        }
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                res.data, res.len,
            )));
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
                    match &info.source {
                        StatisticTableSource::Legacy {
                            p_table,
                            p_clear_func,
                        } => unsafe {
                            emane_c_statistic_table_clear(p_clear_func.0, p_table.0);
                        },
                        StatisticTableSource::NativeRfSignal { table, .. } => {
                            if let Ok(mut table) = table.lock() {
                                table.reset_all();
                            }
                        }
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
                        to_clear.push(&info.source);
                    } else {
                        write_error(&format!("Table not clearable: {}", name), err_buf, err_len);
                        return;
                    }
                } else {
                    write_error(&format!("Unknown table name: {}", name), err_buf, err_len);
                    return;
                }
            }
            for source in to_clear {
                match source {
                    StatisticTableSource::Legacy {
                        p_table,
                        p_clear_func,
                    } => unsafe {
                        emane_c_statistic_table_clear(p_clear_func.0, p_table.0);
                    },
                    StatisticTableSource::NativeRfSignal { table, .. } => {
                        if let Ok(mut table) = table.lock() {
                            table.reset_all();
                        }
                    }
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
                is_clearable: is_clearable(info.properties),
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
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                res.data, res.len,
            )));
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
                is_clearable: is_clearable(info.properties),
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
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                res.data, res.len,
            )));
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
    fn native_rf_signal_table_is_manifested_queried_and_clearable() {
        let build_id = 60_001;
        let handle = register_native_rf_signal_table(build_id, 1).unwrap();
        assert!(update_native_rf_signal_table(
            handle,
            2,
            3,
            2_400_000_000,
            -50.0,
            10.0,
            -60.0,
            -90.0,
        ));
        let manifest = emane_rs_statistic_get_table_manifest(build_id);
        assert_eq!(manifest.len, 1);
        emane_rs_statistic_free_table_manifest(manifest);

        let empty = FfiStringArray {
            data: ptr::null(),
            len: 0,
        };
        let mut error = [0i8; 128];
        let result =
            emane_rs_statistic_query_table(build_id, empty, error.as_mut_ptr(), error.len());
        assert_eq!(result.len, 1);
        let table = unsafe { &*result.data };
        assert!(table.native);
        assert_eq!(table.labels.len, 8);
        assert_eq!(table.rows_len, 1);
        emane_rs_statistic_free_table_query_result(result);

        emane_rs_statistic_clear_table(build_id, empty, error.as_mut_ptr(), error.len());
        let result =
            emane_rs_statistic_query_table(build_id, empty, error.as_mut_ptr(), error.len());
        assert_eq!(unsafe { &*result.data }.rows_len, 0);
        emane_rs_statistic_free_table_query_result(result);
        unregister_native_statistics(build_id);
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

        let names = [s_name.as_ptr()];
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

        let names = [s_name.as_ptr()];
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

        let names = [s_name.as_ptr()];
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
        let slice = unsafe { std::slice::from_raw_parts(manifest.data, manifest.len) };
        let entry = slice
            .iter()
            .find(|entry| unsafe { CStr::from_ptr(entry.name) }.to_bytes() == b"manifest.stat")
            .expect("manifest statistic");
        assert!(entry.is_clearable);

        emane_rs_statistic_free_manifest(manifest);
    }

    #[test]
    fn native_counter_registers_queries_clears_and_unregisters() {
        let build_id = 90;
        let handle = register_native_counter(build_id, "numPackets", "Packet count", true)
            .expect("counter registration");
        assert!(increment_native_counter(handle, 3));

        let mut error = [0i8; 64];
        let query = emane_rs_statistic_query(
            build_id,
            FfiStringArray {
                data: std::ptr::null(),
                len: 0,
            },
            error.as_mut_ptr(),
            error.len(),
        );
        assert_eq!(query.len, 1);
        let item = unsafe { &*query.data };
        assert_eq!(item.value.any_type, 8);
        assert_eq!(item.value.u64_value, 3);
        emane_rs_statistic_free_query_result(query);

        emane_rs_statistic_clear(
            build_id,
            FfiStringArray {
                data: std::ptr::null(),
                len: 0,
            },
            error.as_mut_ptr(),
            error.len(),
        );
        assert_eq!(
            get_statistic_service().lock().unwrap().native_counters[&handle]
                .load(Ordering::Relaxed),
            0
        );
        unregister_native_statistics(build_id);
        assert!(!increment_native_counter(handle, 1));
    }
}
