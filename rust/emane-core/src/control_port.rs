use crate::build_id_service::native_layer_manifest;
use crate::log_service::emane_rs_log_set_level;
use crate::native_configuration::{self, ConfigurationValue};
use crate::protobufs::emane_remote_control_port_api::any::AnyType;
use crate::protobufs::emane_remote_control_port_api::{request, response, Any, Request, Response};
use crate::statistics::{
    emane_rs_statistic_clear, emane_rs_statistic_clear_table, emane_rs_statistic_free_query_result,
    emane_rs_statistic_free_table_query_result, emane_rs_statistic_query,
    emane_rs_statistic_query_table, FfiAny, FfiStringArray,
};
use prost::Message;
use std::ffi::{CStr, CString};
use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

const MAX_REQUEST_LENGTH: usize = 64 * 1024 * 1024;

fn statistic_any(value: &FfiAny) -> Any {
    let mut any = Any {
        r#type: value.any_type,
        ..Default::default()
    };
    match value.any_type {
        1 | 3 | 5 => any.i32_value = Some(value.i64_value as i32),
        2 | 4 | 6 => any.u32_value = Some(value.u64_value as u32),
        7 => any.i64_value = Some(value.i64_value),
        8 => any.u64_value = Some(value.u64_value),
        9 => any.f_value = Some(value.d_value as f32),
        10 => any.d_value = Some(value.d_value),
        11 | 13 if !value.s_value.is_null() => {
            any.s_value = Some(
                unsafe { CStr::from_ptr(value.s_value) }
                    .to_string_lossy()
                    .into_owned(),
            );
        }
        12 => any.b_value = Some(value.i64_value != 0),
        _ => {}
    }
    any
}

fn configuration_any(value: &ConfigurationValue) -> Any {
    let mut any = Any {
        r#type: value.any_type(),
        ..Default::default()
    };
    match value {
        ConfigurationValue::Int8(value) => any.i32_value = Some(i32::from(*value)),
        ConfigurationValue::UInt8(value) => any.u32_value = Some(u32::from(*value)),
        ConfigurationValue::Int16(value) => any.i32_value = Some(i32::from(*value)),
        ConfigurationValue::UInt16(value) => any.u32_value = Some(u32::from(*value)),
        ConfigurationValue::Int32(value) => any.i32_value = Some(*value),
        ConfigurationValue::UInt32(value) => any.u32_value = Some(*value),
        ConfigurationValue::Int64(value) => any.i64_value = Some(*value),
        ConfigurationValue::UInt64(value) => any.u64_value = Some(*value),
        ConfigurationValue::Float(value) => any.f_value = Some(*value),
        ConfigurationValue::Double(value) => any.d_value = Some(*value),
        ConfigurationValue::String(value) | ConfigurationValue::InetAddr(value) => {
            any.s_value = Some(value.clone());
        }
        ConfigurationValue::Boolean(value) => any.b_value = Some(*value),
    }
    any
}

fn request_value(any: &Any) -> Result<ConfigurationValue, String> {
    match AnyType::try_from(any.r#type).map_err(|_| "unknown Any value type".to_string())? {
        AnyType::TypeAnyInt8 => any
            .i32_value
            .and_then(|value| i8::try_from(value).ok())
            .map(ConfigurationValue::Int8)
            .ok_or_else(|| "int8 configuration value is missing or out of range".to_string()),
        AnyType::TypeAnyUint8 => any
            .u32_value
            .and_then(|value| u8::try_from(value).ok())
            .map(ConfigurationValue::UInt8)
            .ok_or_else(|| "uint8 configuration value is missing or out of range".to_string()),
        AnyType::TypeAnyInt16 => any
            .i32_value
            .and_then(|value| i16::try_from(value).ok())
            .map(ConfigurationValue::Int16)
            .ok_or_else(|| "int16 configuration value is missing or out of range".to_string()),
        AnyType::TypeAnyUint16 => any
            .u32_value
            .and_then(|value| u16::try_from(value).ok())
            .map(ConfigurationValue::UInt16)
            .ok_or_else(|| "uint16 configuration value is missing or out of range".to_string()),
        AnyType::TypeAnyInt32 => any
            .i32_value
            .map(ConfigurationValue::Int32)
            .ok_or_else(|| "int32 configuration value is missing".to_string()),
        AnyType::TypeAnyUint32 => any
            .u32_value
            .map(ConfigurationValue::UInt32)
            .ok_or_else(|| "uint32 configuration value is missing".to_string()),
        AnyType::TypeAnyInt64 => any
            .i64_value
            .map(ConfigurationValue::Int64)
            .ok_or_else(|| "int64 configuration value is missing".to_string()),
        AnyType::TypeAnyUint64 => any
            .u64_value
            .map(ConfigurationValue::UInt64)
            .ok_or_else(|| "uint64 configuration value is missing".to_string()),
        AnyType::TypeAnyFloat => any
            .f_value
            .map(ConfigurationValue::Float)
            .ok_or_else(|| "float configuration value is missing".to_string()),
        AnyType::TypeAnyDouble => any
            .d_value
            .map(ConfigurationValue::Double)
            .ok_or_else(|| "double configuration value is missing".to_string()),
        AnyType::TypeAnyString => any
            .s_value
            .clone()
            .map(ConfigurationValue::String)
            .ok_or_else(|| "string configuration value is missing".to_string()),
        AnyType::TypeAnyBoolean => any
            .b_value
            .map(ConfigurationValue::Boolean)
            .ok_or_else(|| "boolean configuration value is missing".to_string()),
        AnyType::TypeAnyInetaddr => any
            .s_value
            .clone()
            .map(ConfigurationValue::InetAddr)
            .ok_or_else(|| "INET address configuration value is missing".to_string()),
    }
}

fn query_manifest() -> response::query::Manifest {
    use response::query::manifest::{nem, Nem};
    response::query::Manifest {
        nems: native_layer_manifest()
            .into_iter()
            .map(|(nem_id, components)| Nem {
                id: u32::from(nem_id),
                components: components
                    .into_iter()
                    .map(|component| nem::Component {
                        build_id: u32::from(component.build_id),
                        r#type: component.layer_type,
                        plugin: component.plugin_name,
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn query_configuration(
    query: request::query::Configuration,
) -> Result<response::query::Configuration, String> {
    Ok(response::query::Configuration {
        build_id: query.build_id,
        parameters: native_configuration::query(
            u16::try_from(query.build_id).map_err(|_| "build id is out of range".to_string())?,
            &query.names,
        )?
        .into_iter()
        .map(|(name, values)| response::query::configuration::Parameter {
            name,
            values: values.iter().map(configuration_any).collect(),
        })
        .collect(),
    })
}

fn ffi_names(names: &[String]) -> Result<(Vec<CString>, Vec<*const i8>, FfiStringArray), String> {
    let strings = names
        .iter()
        .map(|name| CString::new(name.as_str()).map_err(|_| "name contains a NUL byte".to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    let pointers = strings.iter().map(|name| name.as_ptr()).collect::<Vec<_>>();
    let array = FfiStringArray {
        data: if pointers.is_empty() {
            std::ptr::null()
        } else {
            pointers.as_ptr()
        },
        len: pointers.len(),
    };
    Ok((strings, pointers, array))
}

fn error_buffer(buffer: &[i8]) -> Option<String> {
    if buffer.first().copied().unwrap_or_default() == 0 {
        None
    } else {
        Some(
            unsafe { CStr::from_ptr(buffer.as_ptr()) }
                .to_string_lossy()
                .into_owned(),
        )
    }
}

fn query_statistic(query: request::query::Statistic) -> Result<response::query::Statistic, String> {
    let (_strings, _pointers, names) = ffi_names(&query.names)?;
    let mut error = [0i8; 1024];
    let result = emane_rs_statistic_query(
        u16::try_from(query.build_id).map_err(|_| "build id is out of range".to_string())?,
        names,
        error.as_mut_ptr(),
        error.len(),
    );
    if let Some(error) = error_buffer(&error) {
        emane_rs_statistic_free_query_result(result);
        return Err(error);
    }
    let elements = if result.data.is_null() {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(result.data, result.len) }
            .iter()
            .map(|item| response::query::statistic::Element {
                name: unsafe { CStr::from_ptr(item.name) }
                    .to_string_lossy()
                    .into_owned(),
                value: statistic_any(&item.value),
            })
            .collect()
    };
    emane_rs_statistic_free_query_result(result);
    Ok(response::query::Statistic {
        build_id: query.build_id,
        elements,
    })
}

fn query_statistic_table(
    query: request::query::StatisticTable,
) -> Result<response::query::StatisticTable, String> {
    let (_strings, _pointers, names) = ffi_names(&query.names)?;
    let mut error = [0i8; 1024];
    let result = emane_rs_statistic_query_table(
        u16::try_from(query.build_id).map_err(|_| "build id is out of range".to_string())?,
        names,
        error.as_mut_ptr(),
        error.len(),
    );
    if let Some(error) = error_buffer(&error) {
        emane_rs_statistic_free_table_query_result(result);
        return Err(error);
    }
    let tables = if result.data.is_null() {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(result.data, result.len) }
            .iter()
            .map(|item| {
                let labels = if item.labels.data.is_null() {
                    Vec::new()
                } else {
                    unsafe { std::slice::from_raw_parts(item.labels.data, item.labels.len) }
                        .iter()
                        .map(|label| {
                            unsafe { CStr::from_ptr(*label) }
                                .to_string_lossy()
                                .into_owned()
                        })
                        .collect()
                };
                let rows = if item.rows.is_null() {
                    Vec::new()
                } else {
                    unsafe { std::slice::from_raw_parts(item.rows, item.rows_len) }
                        .iter()
                        .map(|row| response::query::statistic_table::table::Row {
                            values: if row.values.data.is_null() {
                                Vec::new()
                            } else {
                                unsafe {
                                    std::slice::from_raw_parts(row.values.data, row.values.len)
                                }
                                .iter()
                                .map(statistic_any)
                                .collect()
                            },
                        })
                        .collect()
                };
                response::query::statistic_table::Table {
                    name: unsafe { CStr::from_ptr(item.name) }
                        .to_string_lossy()
                        .into_owned(),
                    rows,
                    labels,
                }
            })
            .collect()
    };
    emane_rs_statistic_free_table_query_result(result);
    Ok(response::query::StatisticTable {
        build_id: query.build_id,
        tables,
    })
}

fn clear_statistics(build_id: u32, names: &[String], tables: bool) -> Result<(), String> {
    let (_strings, _pointers, names) = ffi_names(names)?;
    let mut error = [0i8; 1024];
    let build_id = u16::try_from(build_id).map_err(|_| "build id is out of range".to_string())?;
    if tables {
        emane_rs_statistic_clear_table(build_id, names, error.as_mut_ptr(), error.len());
    } else {
        emane_rs_statistic_clear(build_id, names, error.as_mut_ptr(), error.len());
    }
    error_buffer(&error).map_or(Ok(()), Err)
}

fn error_response(sequence: u32, description: String) -> Response {
    Response {
        sequence,
        reference: sequence,
        r#type: response::ResponseMessageType::TypeResponseError as i32,
        error: Some(response::Error {
            r#type: response::error::ErrorType::TypeErrorParameter as i32,
            description,
        }),
        ..Default::default()
    }
}

fn process_request(request: Request) -> Response {
    let sequence = request.sequence;
    let result = match request.r#type {
        value if value == request::RequestMessageType::TypeRequestQuery as i32 => {
            let query = request
                .query
                .ok_or_else(|| "query request has no query".to_string());
            query.and_then(|query| {
                let mut result = response::Query {
                    r#type: query.r#type,
                    ..Default::default()
                };
                match query.r#type {
                    1 => {
                        result.configuration =
                            Some(query_configuration(query.configuration.ok_or_else(
                                || "configuration query has no payload".to_string(),
                            )?)?);
                    }
                    2 => result.manifest = Some(query_manifest()),
                    3 => {
                        result.statistic =
                            Some(query_statistic(query.statistic.ok_or_else(|| {
                                "statistic query has no payload".to_string()
                            })?)?);
                    }
                    4 => {
                        result.statistic_table =
                            Some(query_statistic_table(query.statistic_table.ok_or_else(
                                || "statistic-table query has no payload".to_string(),
                            )?)?);
                    }
                    _ => return Err("unknown query type".to_string()),
                }
                Ok(Response {
                    sequence,
                    reference: sequence,
                    r#type: response::ResponseMessageType::TypeResponseQuery as i32,
                    query: Some(result),
                    ..Default::default()
                })
            })
        }
        value if value == request::RequestMessageType::TypeRequestUpdate as i32 => {
            let update = request
                .update
                .ok_or_else(|| "update request has no update".to_string());
            update.and_then(|update| {
                match update.r#type {
                    1 => {
                        let configuration = update
                            .configuration
                            .ok_or_else(|| "configuration update has no payload".to_string())?;
                        let build_id = u16::try_from(configuration.build_id)
                            .map_err(|_| "build id is out of range".to_string())?;
                        let updates = configuration
                            .parameters
                            .iter()
                            .map(|parameter| {
                                Ok((
                                    parameter.name.clone(),
                                    parameter
                                        .values
                                        .iter()
                                        .map(request_value)
                                        .collect::<Result<Vec<_>, String>>()?,
                                ))
                            })
                            .collect::<Result<Vec<_>, String>>()?;
                        native_configuration::update(build_id, updates)?;
                    }
                    2 => {
                        let clear = update
                            .statistic_clear
                            .ok_or_else(|| "statistic clear update has no payload".to_string())?;
                        clear_statistics(clear.build_id, &clear.names, false)?;
                    }
                    3 => {
                        let clear = update.statistic_table_clear.ok_or_else(|| {
                            "statistic-table clear update has no payload".to_string()
                        })?;
                        clear_statistics(clear.build_id, &clear.names, true)?;
                    }
                    4 => {
                        let level = update
                            .log_level
                            .ok_or_else(|| "log-level update has no payload".to_string())?
                            .level;
                        if level > 4 {
                            return Err("log level must be between 0 and 4".to_string());
                        }
                        emane_rs_log_set_level(level as i32);
                    }
                    _ => return Err("unknown update type".to_string()),
                }
                Ok(Response {
                    sequence,
                    reference: sequence,
                    r#type: response::ResponseMessageType::TypeResponseUpdate as i32,
                    update: Some(response::Update {
                        r#type: update.r#type,
                    }),
                    ..Default::default()
                })
            })
        }
        _ => Err("unknown request type".to_string()),
    };
    result.unwrap_or_else(|error| error_response(sequence, error))
}

fn handle_client(mut stream: TcpStream, cancel: &AtomicBool) {
    let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
    let mut header = [0u8; 4];
    while !cancel.load(Ordering::Acquire) {
        match stream.read_exact(&mut header) {
            Ok(()) => {}
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                continue;
            }
            Err(_) => break,
        }
        let length = u32::from_be_bytes(header) as usize;
        if length == 0 || length > MAX_REQUEST_LENGTH {
            break;
        }
        let mut bytes = vec![0u8; length];
        if stream.read_exact(&mut bytes).is_err() {
            break;
        }
        let response = match Request::decode(bytes.as_slice()) {
            Ok(request) => process_request(request),
            Err(error) => error_response(0, format!("malformed control request: {error}")),
        };
        let bytes = response.encode_to_vec();
        if bytes.len() > u32::MAX as usize
            || stream
                .write_all(&(bytes.len() as u32).to_be_bytes())
                .is_err()
            || stream.write_all(&bytes).is_err()
        {
            break;
        }
    }
    let _ = stream.shutdown(Shutdown::Both);
}

pub struct ControlPort {
    cancel: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl ControlPort {
    pub fn open(endpoint: &str) -> Result<Self, String> {
        let listener = TcpListener::bind(endpoint)
            .map_err(|error| format!("failed to bind control port {endpoint}: {error}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| format!("failed to configure control port: {error}"))?;
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancel);
        let worker = thread::Builder::new()
            .name("emane-control-port".to_string())
            .spawn(move || {
                while !worker_cancel.load(Ordering::Acquire) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            let client_cancel = Arc::clone(&worker_cancel);
                            let _ = thread::Builder::new()
                                .name("emane-control-session".to_string())
                                .spawn(move || handle_client(stream, &client_cancel));
                        }
                        Err(error) if error.kind() == ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(50));
                        }
                        Err(_) => break,
                    }
                }
            })
            .map_err(|error| format!("failed to start control-port worker: {error}"))?;
        Ok(Self {
            cancel,
            worker: Some(worker),
        })
    }

    pub fn close(&mut self) {
        self.cancel.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for ControlPort {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_query_returns_protocol_error() {
        let response = process_request(Request {
            sequence: 7,
            r#type: request::RequestMessageType::TypeRequestQuery as i32,
            ..Default::default()
        });
        assert_eq!(response.reference, 7);
        assert_eq!(
            response.r#type,
            response::ResponseMessageType::TypeResponseError as i32
        );
        assert!(response.error.unwrap().description.contains("no query"));
    }
}
