use crate::handle_manager::Handle;
use crate::logging;
use crate::protobuf_apis::call_proto;
use crate::thrift_apis::{
    DatabaseDriverV1,
    server::{create_new_api, destroy_api, flush_api, read_from_api, write_to_api},
};
use proto_utils::ProtoError;
use std::fmt::Debug;
use std::os::raw::c_char;

#[repr(C)]
pub enum SfCoreApi {
    DatabaseDriverApiV1 = 1,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CApiHandle {
    pub id: u64,
    pub magic: u64,
}

impl CApiHandle {
    pub fn from_handle(handle: Handle) -> Self {
        CApiHandle {
            id: handle.id,
            magic: handle.magic,
        }
    }

    pub fn to_handle(&self) -> Handle {
        Handle {
            id: self.id,
            magic: self.magic,
        }
    }
}

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

#[unsafe(no_mangle)]
pub extern "C" fn sf_core_api_init(api: SfCoreApi) -> CApiHandle {
    // TODO pass the error to the caller
    let handle = match api {
        SfCoreApi::DatabaseDriverApiV1 => create_new_api::<DatabaseDriverV1>(),
    };
    if let Err(e) = &handle {
        eprintln!("Failed to create API: {e:?}");
    }
    CApiHandle::from_handle(handle.unwrap())
}

#[unsafe(no_mangle)]
pub extern "C" fn sf_core_api_destroy(api: CApiHandle) {
    let handle = api.to_handle();
    let result = destroy_api(handle);
    if let Err(e) = result {
        eprintln!("Failed to destroy API: {e:?}");
    }
}

/// # Safety
/// This function dereferences raw pointers `buf` and uses `len` to create a slice.
/// The caller must ensure that `buf` is valid for reads of `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sf_core_api_write(api: CApiHandle, buf: *mut u8, len: usize) -> usize {
    // TODO pass the error to the caller
    let result = write_to_api(api.to_handle(), unsafe {
        std::slice::from_raw_parts(buf, len)
    });
    if let Err(e) = &result {
        eprintln!("Failed to write to API: {e:?}");
    }
    result.unwrap_or(0)
}

/// # Safety
/// This function dereferences raw pointers `buf` and uses `len` to create a mutable slice.
/// The caller must ensure that `buf` is valid for writes of `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sf_core_api_read(api: CApiHandle, buf: *mut u8, len: usize) -> usize {
    // TODO pass the error to the caller
    let result = read_from_api(api.to_handle(), unsafe {
        std::slice::from_raw_parts_mut(buf, len)
    });
    if let Err(e) = &result {
        eprintln!("Failed to read from API: {e:?}");
    }
    result.unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn sf_core_api_flush(api: CApiHandle) {
    // TODO pass the error to the caller
    let result = flush_api(api.to_handle());
    if let Err(e) = &result {
        eprintln!("Failed to flush API: {e:?}");
    }
}

fn write_buffer(vec: Vec<u8>, buffer: *mut *const u8, len: *mut usize) {
    unsafe {
        *buffer = vec.as_ptr();
        *len = vec.len();
        std::mem::forget(vec);
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
