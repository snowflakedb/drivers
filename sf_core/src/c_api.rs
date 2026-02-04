use crate::config::{config_manager, settings::Setting};
use crate::logging;
use crate::protobuf_apis::call_proto;
use base64::Engine;
use proto_utils::ProtoError;
use serde_json;
use std::collections::HashMap;
use std::ffi::CStr;
use std::os::raw::c_char;

#[unsafe(no_mangle)]
pub extern "C" fn sf_core_init_logger(callback: logging::CLogCallback) -> u32 {
    let config = logging::LoggingConfig::new(None, false, false);
    let layer = logging::CallbackLayer::new(callback);
    match logging::init_logging(config, Some(layer)) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("Failed to initialize logging: {e:?}");
            1
        }
    }
}

fn write_buffer(vec: Vec<u8>, buffer: *mut *const u8, len: *mut usize) {
    let boxed = vec.into_boxed_slice();
    unsafe {
        *len = boxed.len();
        *buffer = Box::into_raw(boxed) as *const u8;
    }
}

/// Frees a buffer previously returned by `sf_core_api_call_proto` via `write_buffer`.
///
/// # Safety
/// The caller must pass the exact `buffer` pointer and `len` that were written by a prior
/// call to `sf_core_api_call_proto`. Each (buffer, len) pair must be freed at most once.
/// Passing any other pointer or length is undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sf_core_free_buffer(buffer: *const u8, len: usize) {
    if !buffer.is_null() {
        unsafe {
            drop(Box::from_raw(std::slice::from_raw_parts_mut(
                buffer as *mut u8,
                len,
            )));
        }
    }
}

/// # Safety
/// This function dereferences raw pointers `api`, `method`, `request`, `response`, and `response_len`.
/// The caller must ensure that `api`, `method`, `request`, `response`, and `response_len` are valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sf_core_api_call_proto(
    api: *const c_char,
    method: *const c_char,
    request: *mut u8,
    request_len: usize,
    response: *mut *const u8,
    response_len: *mut usize,
) -> usize {
    // Prevent unwinding across the FFI boundary. Any panic will be converted to a transport error.
    let result = std::panic::catch_unwind(|| unsafe {
        let api = std::ffi::CStr::from_ptr(api).to_string_lossy().to_string();
        let method = std::ffi::CStr::from_ptr(method)
            .to_string_lossy()
            .to_string();
        let message = std::slice::from_raw_parts(request, request_len);
        call_proto(&api, &method, message)
    });

    match result {
        Ok(Ok(response_vec)) => {
            write_buffer(response_vec, response, response_len);
            0
        }
        Ok(Err(ProtoError::Application(error_vec))) => {
            write_buffer(error_vec, response, response_len);
            1
        }
        Ok(Err(ProtoError::Transport(e))) => {
            write_buffer(e.as_bytes().to_vec(), response, response_len);
            2
        }
        Err(_) => {
            let msg = b"sf_core panic in sf_core_api_call_proto".to_vec();
            write_buffer(msg, response, response_len);
            2
        }
    }
}

/// Load configuration for a specific connection from TOML files
/// Returns JSON serialized HashMap<String, Setting> on success
/// # Safety
/// This function dereferences raw pointers `connection_name`, `result`, and `result_len`.
/// The caller must ensure that `connection_name`, `result`, and `result_len` are valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sf_core_config_load_connection(
    connection_name: *const c_char,
    result: *mut *const u8,
    result_len: *mut usize,
) -> u32 {
    let conn_name = match unsafe { CStr::from_ptr(connection_name) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            let msg = b"Invalid UTF-8 in connection name".to_vec();
            write_buffer(msg, result, result_len);
            return 1;
        }
    };

    match config_manager::load_connection_config(conn_name) {
        Ok(settings) => {
            // Convert HashMap<String, Setting> to JSON
            let json_map: HashMap<String, serde_json::Value> = settings
                .into_iter()
                .map(|(k, v)| {
                    let json_value = match v {
                        Setting::String(s) => serde_json::json!({"type": "string", "value": s}),
                        Setting::Int(i) => serde_json::json!({"type": "int", "value": i}),
                        Setting::Double(d) => serde_json::json!({"type": "double", "value": d}),
                        Setting::Bytes(b) => {
                            serde_json::json!({"type": "bytes", "value": base64::engine::general_purpose::STANDARD.encode(b)})
                        }
                    };
                    (k, json_value)
                })
                .collect();

            match serde_json::to_vec(&json_map) {
                Ok(json) => {
                    write_buffer(json, result, result_len);
                    0
                }
                Err(e) => {
                    let msg = format!("Failed to serialize settings: {}", e).into_bytes();
                    write_buffer(msg, result, result_len);
                    2
                }
            }
        }
        Err(e) => {
            let msg = format!("Config error: {}", e).into_bytes();
            write_buffer(msg, result, result_len);
            1
        }
    }
}

/// Load all sections from config files (including connections) with env overrides applied
/// Returns JSON serialized HashMap<String, HashMap<String, Setting>> on success
/// # Safety
/// This function dereferences raw pointers `result` and `result_len`.
/// The caller must ensure that `result` and `result_len` are valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sf_core_config_load_all_sections(
    result: *mut *const u8,
    result_len: *mut usize,
) -> u32 {
    unsafe { sf_core_config_load_all_sections_with_options(true, result, result_len) }
}

/// Load all sections from config files with configurable env override behavior
/// Returns JSON serialized HashMap<String, HashMap<String, Setting>> on success
/// # Safety
/// This function dereferences raw pointers `result` and `result_len`.
/// The caller must ensure that `result` and `result_len` are valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sf_core_config_load_all_sections_with_options(
    apply_env_overrides: bool,
    result: *mut *const u8,
    result_len: *mut usize,
) -> u32 {
    match config_manager::load_all_config_sections_with_options(apply_env_overrides) {
        Ok(all_sections) => {
            // Convert HashMap<String, HashMap<String, Setting>> to JSON
            let json_map: HashMap<String, HashMap<String, serde_json::Value>> = all_sections
                .into_iter()
                .map(|(section_name, settings)| {
                    let settings_json: HashMap<String, serde_json::Value> = settings
                        .into_iter()
                        .map(|(k, v)| {
                            let json_value = match v {
                                Setting::String(s) => {
                                    serde_json::json!({"type": "string", "value": s})
                                }
                                Setting::Int(i) => serde_json::json!({"type": "int", "value": i}),
                                Setting::Double(d) => {
                                    serde_json::json!({"type": "double", "value": d})
                                }
                                Setting::Bytes(b) => {
                                    serde_json::json!({"type": "bytes", "value": base64::engine::general_purpose::STANDARD.encode(b)})
                                }
                            };
                            (k, json_value)
                        })
                        .collect();
                    (section_name, settings_json)
                })
                .collect();

            match serde_json::to_vec(&json_map) {
                Ok(json) => {
                    write_buffer(json, result, result_len);
                    0
                }
                Err(e) => {
                    let msg = format!("Failed to serialize settings: {}", e).into_bytes();
                    write_buffer(msg, result, result_len);
                    2
                }
            }
        }
        Err(e) => {
            let msg = format!("Config error: {}", e).into_bytes();
            write_buffer(msg, result, result_len);
            1
        }
    }
}
