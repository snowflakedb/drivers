use std::os::raw::c_char;
use std::sync::LazyLock;

use crate::protobuf::apis::RustTransport;
use proto_utils::{ProtoError, Transport};

struct CApiState {
    runtime: tokio::runtime::Runtime,
    transport: RustTransport,
}

static STATE: LazyLock<CApiState> = LazyLock::new(|| CApiState {
    // Single worker thread is intentional: keeps contention minimal and
    // makes deadlocks easier to detect. Will be increased during
    // performance optimization.
    runtime: tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime"),
    transport: RustTransport::new(),
});

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
#[cfg(feature = "protobuf")]
pub unsafe extern "C" fn sf_core_free_buffer(buffer: *const u8, len: usize) {
    if !buffer.is_null() {
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
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
#[cfg(feature = "protobuf")]
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
        STATE.runtime.block_on(
            STATE
                .transport
                .handle_message(&api, &method, message.to_vec()),
        )
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
