use super::transport::ThriftTransport;
use crate::handle_manager::Handle;
use crate::handle_manager::HandleManager;
use crate::thrift_apis::ThriftApi;
use snafu::ResultExt;
use std::sync::Mutex;

use snafu::{Location, Snafu};

#[derive(Debug, Snafu)]
pub enum ThriftApiError {
    #[snafu(display("Failed to lock Thrift transport mutex"))]
    LockMutex {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to delete Thrift transport handle"))]
    DeleteHandle {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to get Thrift transport handle"))]
    GetHandle {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Failed to write to Thrift transport: {source}"))]
    Write {
        #[snafu(implicit)]
        location: Location,
        source: thrift::Error,
    },
    #[snafu(display("Failed to read from Thrift transport: {source}"))]
    Read {
        #[snafu(implicit)]
        location: Location,
        source: thrift::Error,
    },

    #[snafu(display("Failed to flush Thrift transport: {source}"))]
    Flush {
        #[snafu(implicit)]
        location: Location,
        source: thrift::Error,
    },
}

static API_TRANSPORT_HANDLE_MANAGER: HandleManager<Mutex<ThriftTransport>> = HandleManager::new();

#[allow(private_bounds)]
pub fn create_new_api<T: ThriftApi>() -> Result<Handle, ThriftApiError> {
    let handle =
        API_TRANSPORT_HANDLE_MANAGER.add_handle(Mutex::new(ThriftTransport::new(T::server())));
    Ok(handle)
}

pub fn destroy_api(handle: Handle) -> Result<(), ThriftApiError> {
    if !API_TRANSPORT_HANDLE_MANAGER.delete_handle(handle) {
        DeleteHandleSnafu {}.fail()
    } else {
        Ok(())
    }
}

pub fn write_to_api(handle: Handle, buf: &[u8]) -> Result<usize, ThriftApiError> {
    let arc = API_TRANSPORT_HANDLE_MANAGER
        .get_obj(handle)
        .ok_or(GetHandleSnafu {}.build())?;
    let mut transport = arc.lock().map_err(|_| LockMutexSnafu {}.build())?;
    let bytes_written = transport.write(buf).context(WriteSnafu {})?;
    Ok(bytes_written)
}

pub fn read_from_api(handle: Handle, buf: &mut [u8]) -> Result<usize, ThriftApiError> {
    let arc = API_TRANSPORT_HANDLE_MANAGER
        .get_obj(handle)
        .ok_or(GetHandleSnafu {}.build())?;
    let mut transport = arc.lock().map_err(|_| LockMutexSnafu {}.build())?;
    let bytes_read = transport.read(buf).context(ReadSnafu {})?;
    Ok(bytes_read)
}

pub fn flush_api(handle: Handle) -> Result<(), ThriftApiError> {
    let arc = API_TRANSPORT_HANDLE_MANAGER
        .get_obj(handle)
        .ok_or(GetHandleSnafu {}.build())?;
    let mut transport = arc.lock().map_err(|_| LockMutexSnafu {}.build())?;
    transport.flush().context(FlushSnafu {})?;
    Ok(())
}
