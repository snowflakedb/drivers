//! WASI reactor for the Snowflake driver.
//!
//! This crate builds as a plain WASI core module for use with
//! runtimes like wazero that support WASI Preview 1.

use proto_utils::ProtoError;
use sf_core::protobuf_apis::call_proto;
use std::alloc::{alloc, dealloc, Layout};
use std::slice;
use std::sync::{Mutex, OnceLock};

/// Allocate memory in WASM linear memory.
#[no_mangle]
pub extern "C" fn alloc_bytes(len: u32) -> *mut u8 {
    if len == 0 {
        return std::ptr::null_mut();
    }
    let layout = Layout::from_size_align(len as usize, 1).expect("Invalid layout");
    unsafe { alloc(layout) }
}

/// Deallocate memory in WASM linear memory.
///
/// # Safety
///
/// - `ptr` must be a pointer returned by `alloc_bytes` (or equivalent allocator).
/// - `len` must match the size used during allocation.
/// - The memory must not be used after this call.
#[no_mangle]
pub unsafe extern "C" fn dealloc_bytes(ptr: *mut u8, len: u32) {
    if !ptr.is_null() && len > 0 {
        let layout = Layout::from_size_align(len as usize, 1).expect("Invalid layout");
        dealloc(ptr, layout);
    }
}

/// Result codes
pub const RESULT_OK: u32 = 0;
pub const RESULT_APP_ERROR: u32 = 1;
pub const RESULT_TRANSPORT_ERROR: u32 = 2;

/// Global result storage
static LAST_RESULT: OnceLock<Mutex<Option<Vec<u8>>>> = OnceLock::new();

fn last_result() -> &'static Mutex<Option<Vec<u8>>> {
    LAST_RESULT.get_or_init(|| Mutex::new(None))
}

/// Call a protobuf API method.
///
/// Returns result code. Use `get_result_len` and `get_result` to retrieve response.
///
/// # Safety
///
/// - `api_ptr`/`api_len` must point to a valid UTF-8 string.
/// - `method_ptr`/`method_len` must point to a valid UTF-8 string.
/// - `request_ptr`/`request_len` must point to a valid byte buffer.
/// - The memory ranges must be valid for reads for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn api_call(
    api_ptr: *const u8,
    api_len: u32,
    method_ptr: *const u8,
    method_len: u32,
    request_ptr: *const u8,
    request_len: u32,
) -> u32 {
    let api = match std::str::from_utf8(slice::from_raw_parts(api_ptr, api_len as usize)) {
        Ok(s) => s,
        Err(_) => {
            *last_result().lock().unwrap() = Some(b"Invalid API name encoding".to_vec());
            return RESULT_TRANSPORT_ERROR;
        }
    };

    let method = match std::str::from_utf8(slice::from_raw_parts(method_ptr, method_len as usize)) {
        Ok(s) => s,
        Err(_) => {
            *last_result().lock().unwrap() = Some(b"Invalid method name encoding".to_vec());
            return RESULT_TRANSPORT_ERROR;
        }
    };

    let request = slice::from_raw_parts(request_ptr, request_len as usize);

    // Call the actual protobuf API
    match call_proto(api, method, request) {
        Ok(response) => {
            *last_result().lock().unwrap() = Some(response);
            RESULT_OK
        }
        Err(ProtoError::Application(error_bytes)) => {
            *last_result().lock().unwrap() = Some(error_bytes);
            RESULT_APP_ERROR
        }
        Err(ProtoError::Transport(message)) => {
            *last_result().lock().unwrap() = Some(message.into_bytes());
            RESULT_TRANSPORT_ERROR
        }
    }
}

/// Get the length of the last result.
#[no_mangle]
pub extern "C" fn get_result_len() -> u32 {
    last_result()
        .lock()
        .unwrap()
        .as_ref()
        .map(|v| v.len() as u32)
        .unwrap_or(0)
}

/// Copy the last result to the provided buffer.
///
/// # Safety
///
/// - `dest` must point to a buffer of at least `get_result_len()` bytes.
/// - The buffer must be valid for writes for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn get_result(dest: *mut u8) {
    if let Some(result) = last_result().lock().unwrap().as_ref() {
        std::ptr::copy_nonoverlapping(result.as_ptr(), dest, result.len());
    }
}

/// Clear the last result to free memory.
#[no_mangle]
pub extern "C" fn clear_result() {
    *last_result().lock().unwrap() = None;
}

/// Get the driver version string length.
#[no_mangle]
pub extern "C" fn get_version_len() -> u32 {
    env!("CARGO_PKG_VERSION").len() as u32
}

/// Copy the version string to the provided buffer.
///
/// # Safety
///
/// - `dest` must point to a buffer of at least `get_version_len()` bytes.
/// - The buffer must be valid for writes for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn get_version(dest: *mut u8) {
    let version = env!("CARGO_PKG_VERSION");
    std::ptr::copy_nonoverlapping(version.as_ptr(), dest, version.len());
}

/// Release Arrow data that was exported to WASM.
/// The handle comes from WasmArrowResult.release_handle.
#[no_mangle]
pub extern "C" fn release_arrow_result(handle: u64) {
    sf_core::arrow_wasm::release_arrow_result(handle);
}
