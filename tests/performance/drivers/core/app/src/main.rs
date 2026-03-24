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
use sf_core::protobuf::generated::database_driver_v1::*;

use config::TestConfig;
use connection::{
    DriverRuntime, create_connection, create_database, create_statement, execute_setup_queries,
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

    let rt = DriverRuntime::new();

    let db_handle =
        create_database(&rt).map_err(|e| format!("Failed to create database: {:?}", e))?;
    let conn_handle = create_connection(&rt, db_handle, &config.params.testconnection)
        .map_err(|e| format!("Failed to connect to Snowflake: {:?}", e))?;

    execute_setup_queries(&rt, conn_handle, &config.setup_queries)
        .map_err(|e| format!("Failed to execute setup queries: {:?}", e))?;

    let stmt_handle = create_statement(
        &rt,
        conn_handle,
        &config.sql_command,
        config.statement_async_override,
    )
    .map_err(|e| format!("Failed to prepare statement: {:?}", e))?;

    match config.test_type {
        TestType::Select => execute_fetch_test(
            &rt,
            conn_handle,
            stmt_handle,
            &config.sql_command,
            config.warmup_iterations,
            config.iterations,
            &config.test_name,
        )?,
        TestType::PutGet => execute_put_get_test(
            &rt,
            conn_handle,
            stmt_handle,
            &config.sql_command,
            config.warmup_iterations,
            config.iterations,
            &config.test_name,
        )?,
    }

    // Cleanup
    rt.block_on(async |c| {
        c.statement_release(StatementReleaseRequest {
            stmt_handle: Some(stmt_handle),
        })
        .await
    })
    .ok();

    rt.block_on(async |c| {
        c.connection_release(ConnectionReleaseRequest {
            conn_handle: Some(conn_handle),
        })
        .await
    })
    .ok();

    Ok(())
}
