use crate::api_server::database_driver_v1::DatabaseDriverV1;
use crate::handle_manager::Handle;
use crate::logging;
use crate::transport::{TRANSPORT_HANDLE_MANAGER, ThriftTransport};
use std::fmt::Debug;
use std::sync::Mutex;

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
    let handle = TRANSPORT_HANDLE_MANAGER.add_handle(match api {
        SfCoreApi::DatabaseDriverApiV1 => {
            Mutex::new(ThriftTransport::new(DatabaseDriverV1::processor()))
        }
    });

    CApiHandle::from_handle(handle)
}

#[unsafe(no_mangle)]
pub extern "C" fn sf_core_api_destroy(api: CApiHandle) {
    let handle = api.to_handle();
    TRANSPORT_HANDLE_MANAGER.delete_handle(handle);
}

/// # Safety
/// This function dereferences raw pointers `buf` and uses `len` to create a slice.
/// The caller must ensure that `buf` is valid for reads of `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sf_core_api_write(api: CApiHandle, buf: *mut u8, len: usize) -> usize {
    let tt_ptr = TRANSPORT_HANDLE_MANAGER
        .get_obj(api.to_handle())
        .expect("Thrift transport not found");
    let mut tt = tt_ptr
        .lock()
        .expect("Failed to lock Thrift transport mutex");

    tt.write(unsafe { std::slice::from_raw_parts(buf, len) })
        .expect("Failed to write to Thrift transport")
}

/// # Safety
/// This function dereferences raw pointers `buf` and uses `len` to create a mutable slice.
/// The caller must ensure that `buf` is valid for writes of `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sf_core_api_read(api: CApiHandle, buf: *mut u8, len: usize) -> usize {
    let tt_ptr = TRANSPORT_HANDLE_MANAGER
        .get_obj(api.to_handle())
        .expect("Thrift transport not found");
    let mut tt = tt_ptr
        .lock()
        .expect("Failed to lock Thrift transport mutex");

    tt.read(unsafe { std::slice::from_raw_parts_mut(buf, len) })
        .expect("Failed to read from Thrift transport")
}

#[unsafe(no_mangle)]
pub extern "C" fn sf_core_api_flush(api: CApiHandle) {
    let tt_ptr = TRANSPORT_HANDLE_MANAGER
        .get_obj(api.to_handle())
        .expect("Thrift transport not found");
    let mut tt = tt_ptr
        .lock()
        .expect("Failed to lock Thrift transport mutex");
    tt.flush().expect("Failed to flush Thrift transport");
}
