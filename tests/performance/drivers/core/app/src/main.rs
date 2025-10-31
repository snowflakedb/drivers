//! Core Performance Test Driver

mod arrow;
mod config;
mod connection;
mod execution;
mod results;
mod types;

use sf_core::protobuf_gen::database_driver_v1::*;

use config::TestConfig;
use connection::{
    DatabaseDriver, create_connection, create_database, create_statement, get_server_version,
    reset_statement_query,
};
use execution::{run_test_iterations, run_warmup};
use results::{print_statistics, write_csv_results, write_run_metadata_json};

fn main() {
    if let Err(e) = run() {
        eprintln!("\n❌ ERROR: {:#}", e);
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let config = TestConfig::from_env()?;

    let db_handle =
        create_database().map_err(|e| anyhow::anyhow!("Failed to create database: {}", e))?;
    let conn_handle = create_connection(db_handle, &config.params.testconnection)
        .map_err(|e| anyhow::anyhow!("Failed to connect to Snowflake: {}", e))?;

    let server_version = get_server_version(conn_handle).unwrap_or_else(|_| "UNKNOWN".to_string());

    let stmt_handle = create_statement(conn_handle, &config.sql_command)
        .map_err(|e| anyhow::anyhow!("Failed to prepare statement: {}", e))?;

    println!("\n=== Executing Test Query ===");
    run_warmup(stmt_handle, &config.sql_command, config.warmup_iterations)
        .map_err(|e| anyhow::anyhow!("Warmup phase failed: {}", e))?;

    if config.warmup_iterations > 0 {
        reset_statement_query(stmt_handle, &config.sql_command)
            .map_err(|e| anyhow::anyhow!("Failed to reset statement after warmup: {}", e))?;
    }

    let results = run_test_iterations(stmt_handle, &config.sql_command, config.iterations)
        .map_err(|e| anyhow::anyhow!("Test phase failed: {}", e))?;

    let results_file = write_csv_results(&results, &config.test_name)
        .map_err(|e| anyhow::anyhow!("Failed to write results: {}", e))?;

    // Write run metadata (only once per run)
    let _metadata_file = write_run_metadata_json(&server_version)
        .map_err(|e| anyhow::anyhow!("Failed to write metadata: {}", e))?;

    print_statistics(&results);
    println!("\n✓ Complete → {}", results_file);

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
