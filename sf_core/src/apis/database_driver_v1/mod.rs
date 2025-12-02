#![allow(clippy::result_large_err)]
mod connection;
mod database;
pub(crate) mod error;
mod global_state;
mod query;
mod statement;

pub use crate::config::settings::Setting;
pub use crate::handle_manager::Handle;
pub use connection::connection_init;
pub use connection::connection_new;
pub use connection::connection_release;
pub use connection::connection_set_option;
pub use database::database_init;
pub use database::database_new;
pub use database::database_release;
pub use database::database_set_option;
pub use error::ApiError;
pub use statement::statement_bind;
pub use statement::statement_execute_query;
pub use statement::statement_new;
pub use statement::statement_prepare;
pub use statement::statement_release;
pub use statement::statement_set_option;
pub use statement::statement_set_sql_query;
