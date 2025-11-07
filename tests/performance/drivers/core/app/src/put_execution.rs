//! PUT/GET execution and performance measurement

type Result<T> = std::result::Result<T, String>;
use regex::Regex;
use sf_core::protobuf_gen::database_driver_v1::*;
use std::fs;
use std::path::Path;
use std::time::Instant;

use crate::connection::{DatabaseDriver, reset_statement_query};
use crate::results::{
    print_statistics_put_get, write_csv_results_put_get, write_run_metadata_json,
};
use crate::types::PutGetResult;

pub fn execute_put_get_test(
    stmt_handle: StatementHandle,
    sql_command: &str,
    warmup_iterations: usize,
    iterations: usize,
    test_name: &str,
    server_version: &str,
) -> Result<()> {
    println!("\n=== Executing PUT_GET Test ===");
    println!("Query: {}", sql_command);

    // Warmup
    run_warmup_put_get(stmt_handle, sql_command, warmup_iterations)
        .map_err(|e| format!("Warmup phase failed: {:?}", e))?;

    if warmup_iterations > 0 {
        reset_statement_query(stmt_handle, sql_command)
            .map_err(|e| format!("Failed to reset statement after warmup: {:?}", e))?;
    }

    // Execute
    let results = run_test_iterations_put_get(stmt_handle, sql_command, iterations)
        .map_err(|e| format!("Test phase failed: {:?}", e))?;

    // Write & print
    let results_file = write_csv_results_put_get(&results, test_name)
        .map_err(|e| format!("Failed to write results: {:?}", e))?;
    write_run_metadata_json(server_version)
        .map_err(|e| format!("Failed to write metadata: {:?}", e))?;
    print_statistics_put_get(&results);

    println!("\n✓ Complete → {}", results_file);

    Ok(())
}

pub fn run_warmup_put_get(
    stmt_handle: StatementHandle,
    sql: &str,
    warmup_iterations: usize,
) -> Result<()> {
    if warmup_iterations == 0 {
        return Ok(());
    }

    for i in 0..warmup_iterations {
        execute_put_get_iteration(stmt_handle, sql)?;

        if i < warmup_iterations - 1 {
            reset_statement_query(stmt_handle, sql)?;
        }
    }
    Ok(())
}

pub fn run_test_iterations_put_get(
    stmt_handle: StatementHandle,
    sql: &str,
    iterations: usize,
) -> Result<Vec<PutGetResult>> {
    let mut results = Vec::with_capacity(iterations);

    for i in 0..iterations {
        let query_time = execute_put_get_iteration(stmt_handle, sql)?;

        results.push(PutGetResult {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            query_time_s: query_time,
        });

        if i < iterations - 1 {
            reset_statement_query(stmt_handle, sql)?;
        }
    }

    Ok(results)
}

fn execute_put_get_iteration(stmt_handle: StatementHandle, sql: &str) -> Result<f64> {
    create_get_target_directory(sql)?;

    let start_query = Instant::now();
    DatabaseDriver::statement_execute_query(StatementExecuteQueryRequest {
        stmt_handle: Some(stmt_handle),
    })
    .map_err(|e| format!("PUT/GET execution failed: {:?}", e))?;
    let query_time = start_query.elapsed().as_secs_f64();

    Ok(query_time)
}

/// For GET commands:
/// - Removes existing directory to ensure clean iteration
/// - Creates fresh directory structure
fn create_get_target_directory(sql: &str) -> Result<()> {
    let sql_upper = sql.trim().to_uppercase();
    if sql_upper.starts_with("GET") || sql_upper.contains(" GET ") {
        let re = Regex::new(r"file://([^\s]+)").unwrap();
        if let Some(captures) = re.captures(sql) {
            if let Some(path_match) = captures.get(1) {
                let target_path = path_match.as_str();
                if Path::new(target_path).exists() {
                    fs::remove_dir_all(target_path).map_err(|e| {
                        format!("Failed to remove directory {}: {:?}", target_path, e)
                    })?;
                }
                fs::create_dir_all(target_path)
                    .map_err(|e| format!("Failed to create directory {}: {:?}", target_path, e))?;
            }
        }
    }
    Ok(())
}
