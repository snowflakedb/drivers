//! Unified database driver API.
//!
//! This module provides a platform-agnostic database driver API that works
//! with both native (reqwest) and WASM (portable HTTP) clients.
//!
//! The key abstraction is the `SnowflakeRestClient` trait, which is implemented
//! by both the native and WASM REST clients. This allows the same API code
//! to work on both platforms.

#![allow(clippy::result_large_err)]

pub mod chunks;
pub mod connection;
pub mod database;
pub mod error;
pub mod global_state;
pub mod statement;

pub use crate::config::settings::Setting;
pub use crate::handle_manager::Handle;

pub use connection::{connection_init, connection_new, connection_release, connection_set_option};
pub use database::{database_init, database_new, database_release, database_set_option};
pub use error::ApiError;
#[cfg(feature = "native")]
pub use statement::statement_bind_ffi;
pub use statement::{
    ExecuteResult, statement_bind_stream, statement_execute_query, statement_new,
    statement_prepare, statement_release, statement_set_option, statement_set_sql_query,
};
