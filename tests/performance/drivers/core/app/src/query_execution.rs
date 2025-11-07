//! Query execution and performance measurement helpers

type Result<T> = std::result::Result<T, String>;
use sf_core::protobuf_gen::database_driver_v1::*;
use std::time::Instant;

use crate::arrow::fetch_result_rows;
use crate::connection::{DatabaseDriver, reset_statement_query};
use crate::results::{print_statistics, write_csv_results, write_run_metadata_json};
use crate::types::IterationResult;

pub fn execute_fetch_test(
    stmt_handle: StatementHandle,
    sql_command: &str,
    warmup_iterations: usize,
    iterations: usize,
    test_name: &str,
    server_version: &str,
) -> Result<()> {
    println!("\n=== Executing SELECT Test ===");
    println!("Query: {}", sql_command);

    // Warmup
    run_warmup(stmt_handle, sql_command, warmup_iterations)
        .map_err(|e| format!("Warmup phase failed: {:?}", e))?;

    if warmup_iterations > 0 {
        reset_statement_query(stmt_handle, sql_command)
            .map_err(|e| format!("Failed to reset statement after warmup: {:?}", e))?;
    }

    // Execute
    let results = run_test_iterations(stmt_handle, sql_command, iterations)
        .map_err(|e| format!("Test phase failed: {:?}", e))?;

    // Write & print
    let results_file = write_csv_results(&results, test_name)
        .map_err(|e| format!("Failed to write results: {:?}", e))?;
    write_run_metadata_json(server_version)
        .map_err(|e| format!("Failed to write metadata: {:?}", e))?;
    print_statistics(&results);

    println!("\n✓ Complete → {}", results_file);

    Ok(())
}

pub fn run_warmup(stmt_handle: StatementHandle, sql: &str, warmup_iterations: usize) -> Result<()> {
    if warmup_iterations == 0 {
        return Ok(());
    }

    for i in 0..warmup_iterations {
        let (_query_time, _fetch_time, _row_count) = execute_iteration(stmt_handle)?;

        if i < warmup_iterations - 1 {
            reset_statement_query(stmt_handle, sql)?;
        }
    }
    Ok(())
}

pub fn run_test_iterations(
    stmt_handle: StatementHandle,
    sql: &str,
    iterations: usize,
) -> Result<Vec<IterationResult>> {
    let mut results = Vec::with_capacity(iterations);

    for i in 0..iterations {
        let (query_time, fetch_time, _row_count) = execute_iteration(stmt_handle)?;

        results.push(IterationResult {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            query_time_s: query_time,
            fetch_time_s: fetch_time,
        });

        if i < iterations - 1 {
            reset_statement_query(stmt_handle, sql)?;
        }
    }

    Ok(results)
}

fn execute_iteration(stmt_handle: StatementHandle) -> Result<(f64, f64, usize)> {
    // Execute query (measure query execution time)
    let start_query = Instant::now();
    let response = DatabaseDriver::statement_execute_query(StatementExecuteQueryRequest {
        stmt_handle: Some(stmt_handle),
    })
    .map_err(|e| format!("Query execution failed: {:?}", e))?;
    let query_time = start_query.elapsed().as_secs_f64();

    // Fetch results (measure fetch time)
    let start_fetch = Instant::now();
    let row_count = if let Some(result) = response.result {
        fetch_result_rows(result).map_err(|e| format!("Failed to fetch results: {:?}", e))?
    } else {
        0
    };
    let fetch_time = start_fetch.elapsed().as_secs_f64();

    Ok((query_time, fetch_time, row_count))
}
