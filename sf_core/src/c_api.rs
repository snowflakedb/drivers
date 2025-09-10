use crate::handle_manager::Handle;
use crate::logging;
use crate::thrift_apis::{
    DatabaseDriverV1,
    server::{create_new_api, destroy_api, flush_api, read_from_api, write_to_api},
};
use std::fmt::Debug;

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
