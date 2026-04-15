#![allow(clippy::result_large_err)]
mod alter_session_parser;
pub mod async_query_registry;
pub mod connection;
mod database;
pub(crate) mod error;
mod global_state;
mod logout;
mod query;
pub(crate) mod statement;
pub(crate) mod validation;

pub use crate::config::settings::Setting;
pub use crate::handle_manager::Handle;
pub use async_query_registry::AsyncQueryRegistry;
pub use connection::{Connection, ConnectionInfo, RefreshContext, with_valid_session};
pub use database::FetchChunkInput;
pub use error::ApiError;
pub use global_state::DatabaseDriverV1;
pub use statement::{BindingType, ColumnMetadata, DataPtr, ExecuteResult, StoredChunkInfo};
pub use validation::{ValidationCode, ValidationIssue, ValidationSeverity};

// Free-function wrappers over the blocking protobuf client.
// Used by integration tests that call the low-level API directly.
pub fn database_new() -> Handle {
    use crate::protobuf::apis::database_driver_v1::{
        DatabaseDriverClientBlockingExt, database_driver_client,
    };
    use crate::protobuf::generated::database_driver_v1::DatabaseNewRequest;
    database_driver_client()
        .database_new_blocking(DatabaseNewRequest {})
        .expect("database_new failed")
        .db_handle
        .expect("no db_handle in response")
        .into()
}

pub fn database_init(db_handle: Handle) -> Result<(), ApiError> {
    use crate::protobuf::apis::database_driver_v1::{
        DatabaseDriverClientBlockingExt, database_driver_client,
    };
    use crate::protobuf::generated::database_driver_v1::{DatabaseHandle, DatabaseInitRequest};
    database_driver_client()
        .database_init_blocking(DatabaseInitRequest {
            db_handle: Some(DatabaseHandle::from(db_handle)),
        })
        .map(|_| ())
        .map_err(|e| {
            error::InvalidArgumentSnafu {
                argument: format!("database_init failed: {e:?}"),
            }
            .build()
        })
}

pub fn database_release(db_handle: Handle) -> Result<(), ApiError> {
    use crate::protobuf::apis::database_driver_v1::{
        DatabaseDriverClientBlockingExt, database_driver_client,
    };
    use crate::protobuf::generated::database_driver_v1::{DatabaseHandle, DatabaseReleaseRequest};
    database_driver_client()
        .database_release_blocking(DatabaseReleaseRequest {
            db_handle: Some(DatabaseHandle::from(db_handle)),
        })
        .map(|_| ())
        .map_err(|e| {
            error::InvalidArgumentSnafu {
                argument: format!("database_release failed: {e:?}"),
            }
            .build()
        })
}

pub fn connection_new() -> Handle {
    use crate::protobuf::apis::database_driver_v1::{
        DatabaseDriverClientBlockingExt, database_driver_client,
    };
    use crate::protobuf::generated::database_driver_v1::ConnectionNewRequest;
    database_driver_client()
        .connection_new_blocking(ConnectionNewRequest {})
        .expect("connection_new failed")
        .conn_handle
        .expect("no conn_handle in response")
        .into()
}

pub fn connection_release(conn_handle: Handle) -> Result<(), ApiError> {
    use crate::protobuf::apis::database_driver_v1::{
        DatabaseDriverClientBlockingExt, database_driver_client,
    };
    use crate::protobuf::generated::database_driver_v1::{
        ConnectionHandle, ConnectionReleaseRequest,
    };
    database_driver_client()
        .connection_release_blocking(ConnectionReleaseRequest {
            conn_handle: Some(ConnectionHandle::from(conn_handle)),
        })
        .map(|_| ())
        .map_err(|e| {
            error::InvalidArgumentSnafu {
                argument: format!("connection_release failed: {e:?}"),
            }
            .build()
        })
}

pub fn connection_close(conn_handle: impl Into<Handle>) -> Result<(), ApiError> {
    use crate::protobuf::apis::database_driver_v1::{
        DatabaseDriverClientBlockingExt, database_driver_client,
    };
    use crate::protobuf::generated::database_driver_v1::{
        ConnectionCloseRequest, ConnectionHandle,
    };
    database_driver_client()
        .connection_close_blocking(ConnectionCloseRequest {
            conn_handle: Some(ConnectionHandle::from(conn_handle.into())),
            ..Default::default()
        })
        .map(|_| ())
        .map_err(|e| {
            error::LogoutFailedSnafu {
                message: format!("connection_close failed: {e:?}"),
            }
            .build()
        })
}

pub fn connection_is_closed(conn_handle: impl Into<Handle>) -> Result<bool, ApiError> {
    use crate::protobuf::apis::database_driver_v1::{
        DatabaseDriverClientBlockingExt, database_driver_client,
    };
    use crate::protobuf::generated::database_driver_v1::{
        ConnectionHandle, ConnectionIsClosedRequest,
    };
    database_driver_client()
        .connection_is_closed_blocking(ConnectionIsClosedRequest {
            conn_handle: Some(ConnectionHandle::from(conn_handle.into())),
        })
        .map(|r| r.is_closed)
        .map_err(|e| {
            error::InvalidArgumentSnafu {
                argument: format!("connection_is_closed failed: {e:?}"),
            }
            .build()
        })
}
