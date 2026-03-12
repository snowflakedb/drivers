#![allow(clippy::result_large_err)]
mod alter_session_parser;
pub mod connection;
mod database;
pub(crate) mod error;
mod global_state;
mod query;
pub(crate) mod statement;

pub use crate::config::settings::Setting;
pub use crate::handle_manager::Handle;
pub use connection::Connection;
pub use connection::ConnectionInfo;
pub use connection::RefreshContext;
pub use connection::with_valid_session;
pub use error::ApiError;
pub use global_state::{DatabaseDriverV1, driver_state};
pub use statement::{BindingType, ColumnMetadata, DataPtr};
