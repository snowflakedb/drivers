#![allow(clippy::result_large_err)]
mod alter_session_parser;
pub mod async_query_registry;
pub mod connection;
mod database;
pub(crate) mod error;
mod global_state;
// Gated public visibility so integration tests (via the `test-utils` feature) can reach
// `spawn_heartbeat_task` / `HeartbeatHandle` without widening the production surface of `sf_core`.
// Runtime callers in `connection.rs` only need crate-level visibility.
#[cfg(any(test, feature = "test-utils"))]
pub mod heartbeat;
#[cfg(not(any(test, feature = "test-utils")))]
pub(crate) mod heartbeat;
mod logout;
pub(crate) mod multistatement;
mod query;
pub(crate) mod result_set;
pub mod spcs_token;
pub(crate) mod statement;
mod upload_stream;
pub(crate) mod validation;

pub use crate::config::settings::Setting;
pub use crate::handle_manager::Handle;
pub use async_query_registry::AsyncQueryRegistry;
pub use connection::{Connection, ConnectionInfo, RefreshContext, with_valid_session};
pub use database::FetchChunkInput;
pub use error::ApiError;
pub use global_state::{DatabaseDriverV1, DriverProviders, PutGetResultsetFlavor, WrapperPresets};
pub use result_set::{
    ChunkData, ChunkDataWithDescriptor, ColumnMetadata, ExecuteQueryResult, InlineData,
    ResultSetDescriptor, ResultSetInfo,
};
pub use statement::{BindingType, DataPtr};
pub use validation::{ValidationCode, ValidationIssue, ValidationSeverity};
