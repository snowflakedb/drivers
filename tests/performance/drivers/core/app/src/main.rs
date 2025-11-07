//! Core Performance Test Driver

mod arrow;
mod config;
mod connection;
mod put_execution;
mod query_execution;
mod results;
mod test_types;
mod types;

use test_types::TestType;

type Result<T> = std::result::Result<T, String>;
use sf_core::protobuf_gen::database_driver_v1::*;

use config::TestConfig;
use connection::{
    DatabaseDriver, create_connection, create_database, create_statement, execute_setup_queries,
    get_server_version,
};
use put_execution::execute_put_get_test;
use query_execution::execute_fetch_test;

fn main() {
    if let Err(e) = run() {
        eprintln!("\n❌ ERROR: {:#}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let config = TestConfig::from_env()?;

    let db_handle = create_database().map_err(|e| format!("Failed to create database: {:?}", e))?;
    let conn_handle = create_connection(db_handle, &config.params.testconnection)
        .map_err(|e| format!("Failed to connect to Snowflake: {:?}", e))?;

    let server_version = get_server_version(conn_handle).unwrap_or_else(|e| {
        eprintln!("⚠️  Warning: Could not retrieve server version: {}", e);
        "UNKNOWN".to_string()
    });

    execute_setup_queries(conn_handle, &config.setup_queries)
        .map_err(|e| format!("Failed to execute setup queries: {:?}", e))?;

    let stmt_handle = create_statement(conn_handle, &config.sql_command)
        .map_err(|e| format!("Failed to prepare statement: {:?}", e))?;

    match config.test_type {
        TestType::Select => execute_fetch_test(
            stmt_handle,
            &config.sql_command,
            config.warmup_iterations,
            config.iterations,
            &config.test_name,
            &server_version,
        )?,
        TestType::PutGet => execute_put_get_test(
            stmt_handle,
            &config.sql_command,
            config.warmup_iterations,
            config.iterations,
            &config.test_name,
            &server_version,
        )?,
    }

    // Cleanup
    DatabaseDriver::statement_release(StatementReleaseRequest {
        stmt_handle: Some(stmt_handle),
    })
    .ok();

    DatabaseDriver::connection_release(ConnectionReleaseRequest {
        conn_handle: Some(conn_handle),
    })
    .ok();

    Ok(())
}
