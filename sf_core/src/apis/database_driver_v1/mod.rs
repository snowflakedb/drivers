#![allow(clippy::result_large_err)]
mod alter_session_parser;
pub mod async_query_registry;
pub mod connection;
mod database;
pub(crate) mod error;
mod error_kind;
pub mod final_session_names;
pub(crate) mod get_objects;
mod global_state;
// Gated public visibility so integration tests (via the `test-utils` feature) can reach
// `spawn_heartbeat_task` / `HeartbeatHandle` without widening the production surface of `sf_core`.
// Runtime callers in `connection.rs` only need crate-level visibility.
#[cfg(any(test, feature = "test-utils"))]
pub mod heartbeat;
#[cfg(not(any(test, feature = "test-utils")))]
pub(crate) mod heartbeat;
pub(crate) mod like_pattern;
mod logout;
pub(crate) mod multistatement;
mod query;
// `query` stays private; test builds re-export just the one factory
// `file_manager::azure_transfer` tests need to drive the REAL single-flight
// coordinator (block-level coalescing proof), rather than widening the module.
#[cfg(test)]
pub(crate) use self::query::test_counting_coordinator;
pub(crate) mod result_set;
pub mod spcs_token;
pub(crate) mod statement;
mod stream_transfer;
pub(crate) mod validation;

pub use crate::chunks::FetchChunkInput;
pub use crate::config::settings::Setting;
pub use crate::handle_manager::Handle;
pub use async_query_registry::AsyncQueryRegistry;
pub use connection::{Connection, ConnectionInfo, RefreshContext, with_valid_session};
pub use error::{ApiError, CancellationAbortResult};
pub use error_kind::ErrorKind;
pub use get_objects::{
    ColumnDescriptor, DEPTH_CATALOGS, DEPTH_COLUMNS, DEPTH_DB_SCHEMAS, DEPTH_TABLES,
    FIELD_CATALOG_DB_SCHEMAS, FIELD_CATALOG_NAME, FIELD_COLUMN_BYTE_LENGTH,
    FIELD_COLUMN_CHAR_LENGTH, FIELD_COLUMN_DEF, FIELD_COLUMN_LOGICAL_TYPE, FIELD_COLUMN_NAME,
    FIELD_COLUMN_NULLABLE, FIELD_COLUMN_ORDINAL_POSITION, FIELD_COLUMN_PRECISION,
    FIELD_COLUMN_REMARKS, FIELD_COLUMN_SCALE, FIELD_DB_SCHEMA_NAME, FIELD_DB_SCHEMA_TABLES,
    FIELD_TABLE_COLUMNS, FIELD_TABLE_CONSTRAINTS, FIELD_TABLE_NAME, FIELD_TABLE_TYPE,
    GetObjectsRequest, nested_get_objects_schema,
};
pub use global_state::{DatabaseDriverV1, DriverProviders, PutGetResultsetFlavor, WrapperPresets};
pub use like_pattern::ESCAPE_CHAR;
pub use result_set::{
    ChunkData, ChunkDataWithDescriptor, ColumnMetadata, ExecuteQueryResult, InlineData,
    ResultSetDescriptor, ResultSetInfo,
};
pub use statement::{BindingType, DataPtr};
pub use validation::{ValidationCode, ValidationIssue, ValidationSeverity};
