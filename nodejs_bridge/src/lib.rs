mod connection;
mod error;
mod sql_value;
mod statement;

pub use connection::Connection;
pub use statement::Statement;

use sf_core::apis::database_driver_v1::DatabaseDriverV1;
use std::sync::LazyLock;

pub(crate) static DRIVER: LazyLock<DatabaseDriverV1> = LazyLock::new(|| {
    // TODO:
    // Implement proper bidirectional logger with configurable level,  as is done by other driver wrappers.
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(tracing::Level::DEBUG)
        .try_init();
    DatabaseDriverV1::new()
});

pub(crate) static RUNTIME: LazyLock<tokio::runtime::Runtime> =
    LazyLock::new(|| tokio::runtime::Runtime::new().expect("failed to build tokio runtime"));
