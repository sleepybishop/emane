use prost::Message;
use std::ffi::{CStr, CString};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

use crate::protobufs::emane_remote_control_port_api::any::AnyType;
use crate::protobufs::emane_remote_control_port_api::{request, response, Any, Request, Response};

use crate::config::{
    emane_rs_config_free_update, emane_rs_config_query, emane_rs_config_update,
    FfiConfigItemUpdate, FfiConfigUpdate, FfiStringArray as ConfigFfiStringArray,
};
use crate::config::{FfiAny, FfiAnyArray};
use crate::statistics::{
    emane_rs_statistic_clear, emane_rs_statistic_clear_table, emane_rs_statistic_free_query_result,
    emane_rs_statistic_free_table_query_result, emane_rs_statistic_query,
    emane_rs_statistic_query_table, FfiStringArray as StatFfiStringArray,
};

fn parse_ffi_any(val: &FfiAny) -> Any {
    let mut any = Any::default();
    any.r#type = match val.any_type {
        1 => AnyType::TypeAnyInt8 as i32,
        2 => AnyType::TypeAnyUint8 as i32,
        3 => AnyType::TypeAnyInt16 as i32,
        4 => AnyType::TypeAnyUint16 as i32,
        5 => AnyType::TypeAnyInt32 as i32,
        6 => AnyType::TypeAnyUint32 as i32,
        7 => AnyType::TypeAnyInt64 as i32,
        8 => AnyType::TypeAnyUint64 as i32,
        9 => AnyType::TypeAnyFloat as i32,
        10 => AnyType::TypeAnyDouble as i32,
        11 => AnyType::TypeAnyString as i32,
        12 => AnyType::TypeAnyBoolean as i32,
        13 => AnyType::TypeAnyInetaddr as i32,
        _ => AnyType::TypeAnyString as i32,
    };

    match val.any_type {
        1 | 3 | 5 => any.i32_value = Some(val.i64_value as i32),
        2 | 4 | 6 => any.u32_value = Some(val.u64_value as u32),
        7 => any.i64_value = Some(val.i64_value),
        8 => any.u64_value = Some(val.u64_value),
        9 => any.f_value = Some(val.d_value as f32),
        10 => any.d_value = Some(val.d_value),
        12 => any.b_value = Some(val.i64_value != 0),
        11 | 13 => {
            if !val.s_value.is_null() {
                any.s_value = Some(
                    unsafe { CStr::from_ptr(val.s_value) }
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
        _ => {}
    }
    any
}

fn any_to_ffi_any(any: &Any) -> (FfiAny, Option<CString>) {
    let mut ffi = FfiAny {
        any_type: any.r#type,
        i64_value: 0,
        u64_value: 0,
        d_value: 0.0,
        s_value: std::ptr::null(),
    };
    let mut cstr = None;

    match any.r#type {
        1 | 3 | 5 => {
            if let Some(v) = any.i32_value {
                ffi.i64_value = v as i64;
            }
        }
        2 | 4 | 6 => {
            if let Some(v) = any.u32_value {
                ffi.u64_value = v as u64;
            }
        }
        7 => {
            if let Some(v) = any.i64_value {
                ffi.i64_value = v;
            }
        }
        8 => {
            if let Some(v) = any.u64_value {
                ffi.u64_value = v;
            }
        }
        9 => {
            if let Some(v) = any.f_value {
                ffi.d_value = v as f64;
            }
        }
        10 => {
            if let Some(v) = any.d_value {
                ffi.d_value = v;
            }
        }
        12 => {
            if let Some(v) = any.b_value {
                ffi.i64_value = if v { 1 } else { 0 };
            }
        }
        11 | 13 => {
            if let Some(ref s) = any.s_value {
                let c = CString::new(s.clone()).unwrap();
                ffi.s_value = c.as_ptr();
                cstr = Some(c);
            }
        }
        _ => {}
    }
    (ffi, cstr)
}

fn handle_query_manifest(_build_id: u16) -> response::query::Manifest {
    let manifest = response::query::Manifest::default();
    manifest
}

fn handle_query_configuration(
    query: &request::query::Configuration,
) -> response::query::Configuration {
    let mut resp = response::query::Configuration::default();
    resp.build_id = query.build_id;

    let mut c_names = Vec::new();
    let mut c_ptrs = Vec::new();
    for name in &query.names {
        let c_str = CString::new(name.clone()).unwrap();
        c_ptrs.push(c_str.as_ptr());
        c_names.push(c_str);
    }

    let arr = ConfigFfiStringArray {
        data: if c_ptrs.is_empty() {
            std::ptr::null()
        } else {
            c_ptrs.as_ptr()
        },
        len: c_ptrs.len(),
    };

    let res = emane_rs_config_query(query.build_id as u16, arr);

    if !res.data.is_null() {
        let slice = unsafe { std::slice::from_raw_parts(res.data, res.len) };
        for item in slice {
            let name = unsafe { CStr::from_ptr(item.name) }
                .to_string_lossy()
                .into_owned();
            let mut param = response::query::configuration::Parameter::default();
            param.name = name;

            let val_slice =
                unsafe { std::slice::from_raw_parts(item.values.data, item.values.len) };
            for v in val_slice {
                param.values.push(parse_ffi_any(v));
            }
            resp.parameters.push(param);
        }
    }

    emane_rs_config_free_update(res);
    resp
}

fn handle_query_statistic(query: &request::query::Statistic) -> response::query::Statistic {
    let mut resp = response::query::Statistic::default();
    resp.build_id = query.build_id;

    let mut c_names = Vec::new();
    let mut c_ptrs = Vec::new();
    for name in &query.names {
        let c_str = CString::new(name.clone()).unwrap();
        c_ptrs.push(c_str.as_ptr());
        c_names.push(c_str);
    }

    let arr = StatFfiStringArray {
        data: if c_ptrs.is_empty() {
            std::ptr::null()
        } else {
            c_ptrs.as_ptr()
        },
        len: c_ptrs.len(),
    };

    let mut err_buf = vec![0i8; 1024];
    let res = emane_rs_statistic_query(
        query.build_id as u16,
        arr,
        err_buf.as_mut_ptr(),
        err_buf.len(),
    );

    if !res.data.is_null() {
        let slice = unsafe { std::slice::from_raw_parts(res.data, res.len) };
        for item in slice {
            let name = unsafe { CStr::from_ptr(item.name) }
                .to_string_lossy()
                .into_owned();
            let mut element = response::query::statistic::Element::default();
            element.name = name;
            element.value = parse_ffi_any(&item.value);
            resp.elements.push(element);
        }
    }

    emane_rs_statistic_free_query_result(res);
    resp
}

fn handle_query_statistic_table(
    query: &request::query::StatisticTable,
) -> response::query::StatisticTable {
    let mut resp = response::query::StatisticTable::default();
    resp.build_id = query.build_id;

    let mut c_names = Vec::new();
    let mut c_ptrs = Vec::new();
    for name in &query.names {
        let c_str = CString::new(name.clone()).unwrap();
        c_ptrs.push(c_str.as_ptr());
        c_names.push(c_str);
    }

    let arr = StatFfiStringArray {
        data: if c_ptrs.is_empty() {
            std::ptr::null()
        } else {
            c_ptrs.as_ptr()
        },
        len: c_ptrs.len(),
    };

    let mut err_buf = vec![0i8; 1024];
    let res = emane_rs_statistic_query_table(
        query.build_id as u16,
        arr,
        err_buf.as_mut_ptr(),
        err_buf.len(),
    );

    if !res.data.is_null() {
        let slice = unsafe { std::slice::from_raw_parts(res.data, res.len) };
        for item in slice {
            let name = unsafe { CStr::from_ptr(item.name) }
                .to_string_lossy()
                .into_owned();
            let mut table = response::query::statistic_table::Table::default();
            table.name = name;

            let labels = unsafe { std::slice::from_raw_parts(item.labels.data, item.labels.len) };
            for &lbl in labels {
                table.labels.push(
                    unsafe { CStr::from_ptr(lbl) }
                        .to_string_lossy()
                        .into_owned(),
                );
            }

            let rows = unsafe { std::slice::from_raw_parts(item.rows, item.rows_len) };
            for row in rows {
                let mut r = response::query::statistic_table::table::Row::default();
                let vals = unsafe { std::slice::from_raw_parts(row.values.data, row.values.len) };
                for v in vals {
                    r.values.push(parse_ffi_any(v));
                }
                table.rows.push(r);
            }
            resp.tables.push(table);
        }
    }

    emane_rs_statistic_free_table_query_result(res);
    resp
}

fn process_request(req: Request) -> Response {
    let mut resp = Response::default();
    resp.sequence = req.sequence;
    resp.reference = req.sequence; // or whatever reference semantics are

    match req.r#type {
        1 => {
            // TYPE_REQUEST_QUERY
            resp.r#type = response::ResponseMessageType::TypeResponseQuery as i32;
            let mut query_resp = response::Query::default();
            if let Some(query) = req.query {
                query_resp.r#type = query.r#type;
                match query.r#type {
                    1 => {
                        // Configuration
                        if let Some(c) = query.configuration {
                            query_resp.configuration = Some(handle_query_configuration(&c));
                        }
                    }
                    2 => {
                        // Manifest
                        query_resp.manifest = Some(handle_query_manifest(0));
                    }
                    3 => {
                        // Statistic
                        if let Some(s) = query.statistic {
                            query_resp.statistic = Some(handle_query_statistic(&s));
                        }
                    }
                    4 => {
                        // StatisticTable
                        if let Some(s) = query.statistic_table {
                            query_resp.statistic_table = Some(handle_query_statistic_table(&s));
                        }
                    }
                    _ => {}
                }
            }
            resp.query = Some(query_resp);
        }
        2 => {
            // TYPE_REQUEST_UPDATE
            resp.r#type = response::ResponseMessageType::TypeResponseUpdate as i32;
            let mut upd_resp = response::Update::default();
            if let Some(update) = req.update {
                upd_resp.r#type = update.r#type;
                match update.r#type {
                    1 => {
                        // Configuration
                        if let Some(c) = update.configuration {
                            let mut req_items = Vec::new();
                            let mut c_names = Vec::new();
                            let mut string_arrays = Vec::new();
                            let mut ffi_anys_arrays = Vec::new();

                            for param in &c.parameters {
                                let c_name = CString::new(param.name.clone()).unwrap();
                                let mut ffi_anys = Vec::new();
                                for val in &param.values {
                                    let (ffi_any, cstr_opt) = any_to_ffi_any(val);
                                    if let Some(c) = cstr_opt {
                                        string_arrays.push(c);
                                    }
                                    ffi_anys.push(ffi_any);
                                }
                                ffi_anys_arrays.push(ffi_anys);
                                c_names.push(c_name);
                            }

                            for i in 0..c.parameters.len() {
                                req_items.push(FfiConfigItemUpdate {
                                    name: c_names[i].as_ptr(),
                                    values: FfiAnyArray {
                                        data: if ffi_anys_arrays[i].is_empty() {
                                            std::ptr::null()
                                        } else {
                                            ffi_anys_arrays[i].as_ptr()
                                        },
                                        len: ffi_anys_arrays[i].len(),
                                    },
                                });
                            }

                            let ffi_req = FfiConfigUpdate {
                                data: if req_items.is_empty() {
                                    std::ptr::null()
                                } else {
                                    req_items.as_ptr()
                                },
                                len: req_items.len(),
                            };

                            let mut err_buf = vec![0i8; 1024];
                            unsafe {
                                emane_rs_config_update(
                                    c.build_id as u16,
                                    ffi_req,
                                    err_buf.as_mut_ptr(),
                                    err_buf.len(),
                                );
                            }
                        }
                    }
                    2 => {
                        // StatisticClear
                        if let Some(s) = update.statistic_clear {
                            let mut c_names = Vec::new();
                            let mut c_ptrs = Vec::new();
                            for name in &s.names {
                                let c_str = CString::new(name.clone()).unwrap();
                                c_ptrs.push(c_str.as_ptr());
                                c_names.push(c_str);
                            }
                            let arr = StatFfiStringArray {
                                data: if c_ptrs.is_empty() {
                                    std::ptr::null()
                                } else {
                                    c_ptrs.as_ptr()
                                },
                                len: c_ptrs.len(),
                            };
                            let mut err_buf = vec![0i8; 1024];
                            emane_rs_statistic_clear(
                                s.build_id as u16,
                                arr,
                                err_buf.as_mut_ptr(),
                                err_buf.len(),
                            );
                        }
                    }
                    3 => {
                        // StatisticTableClear
                        if let Some(s) = update.statistic_table_clear {
                            let mut c_names = Vec::new();
                            let mut c_ptrs = Vec::new();
                            for name in &s.names {
                                let c_str = CString::new(name.clone()).unwrap();
                                c_ptrs.push(c_str.as_ptr());
                                c_names.push(c_str);
                            }
                            let arr = StatFfiStringArray {
                                data: if c_ptrs.is_empty() {
                                    std::ptr::null()
                                } else {
                                    c_ptrs.as_ptr()
                                },
                                len: c_ptrs.len(),
                            };
                            let mut err_buf = vec![0i8; 1024];
                            emane_rs_statistic_clear_table(
                                s.build_id as u16,
                                arr,
                                err_buf.as_mut_ptr(),
                                err_buf.len(),
                            );
                        }
                    }
                    _ => {}
                }
            }
            resp.update = Some(upd_resp);
        }
        _ => {}
    }

    resp
}

fn handle_client(mut stream: TcpStream) {
    let mut header = [0u8; 4];
    loop {
        if stream.read_exact(&mut header).is_err() {
            break;
        }
        let len = u32::from_be_bytes(header) as usize;
        let mut buf = vec![0u8; len];
        if stream.read_exact(&mut buf).is_err() {
            break;
        }

        if let Ok(req) = Request::decode(&buf[..]) {
            let resp = process_request(req);
            let mut out_buf = Vec::new();
            if resp.encode(&mut out_buf).is_ok() {
                let out_len = out_buf.len() as u32;
                if stream.write_all(&out_len.to_be_bytes()).is_err() {
                    break;
                }
                if stream.write_all(&out_buf).is_err() {
                    break;
                }
            }
        } else {
            break;
        }
    }
}

pub fn start_control_port(endpoint: &str) {
    let listener = match TcpListener::bind(endpoint) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind Control Port to {}: {}", endpoint, e);
            return;
        }
    };

    println!("Control Port listening on {}", endpoint);

    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    thread::spawn(move || {
                        handle_client(stream);
                    });
                }
                Err(e) => {
                    eprintln!("Control Port connection error: {}", e);
                }
            }
        }
    });
}
