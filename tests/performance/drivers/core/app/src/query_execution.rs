//! Query execution and performance measurement helpers

type Result<T> = std::result::Result<T, String>;
use sf_core::protobuf::generated::database_driver_v1::*;
use std::time::Instant;

use crate::arrow::fetch_result_rows;
use crate::connection::{DriverRuntime, reset_statement_query};
use crate::results::{
    current_unix_timestamp, print_statistics, write_csv_results, write_metadata_if_not_replay,
};
use crate::types::IterationResult;

pub fn execute_fetch_test(
    rt: &DriverRuntime,
    conn_handle: ConnectionHandle,
    stmt_handle: StatementHandle,
    sql_command: &str,
    warmup_iterations: usize,
    iterations: usize,
    test_name: &str,
) -> Result<()> {
    println!("\n=== Executing SELECT Test ===");
    println!("Query: {}", sql_command);

    // Warmup
    run_warmup(rt, stmt_handle, sql_command, warmup_iterations)
        .map_err(|e| format!("Warmup phase failed: {:?}", e))?;

    if warmup_iterations > 0 {
        reset_statement_query(rt, stmt_handle, sql_command)
            .map_err(|e| format!("Failed to reset statement after warmup: {:?}", e))?;
    }

    // Execute
    let results = run_test_iterations(rt, stmt_handle, sql_command, iterations)
        .map_err(|e| format!("Test phase failed: {:?}", e))?;

    // Write & print
    let results_file = write_csv_results(&results, test_name)
        .map_err(|e| format!("Failed to write results: {:?}", e))?;

    write_metadata_if_not_replay(rt, conn_handle)?;

    print_statistics(&results);

    println!("\n✓ Complete → {}", results_file);

    Ok(())
}

pub fn run_warmup(
    rt: &DriverRuntime,
    stmt_handle: StatementHandle,
    sql: &str,
    warmup_iterations: usize,
) -> Result<()> {
    if warmup_iterations == 0 {
        return Ok(());
    }

    for i in 0..warmup_iterations {
        let (_query_time, _fetch_time, _row_count) = execute_iteration(rt, stmt_handle)?;

        if i < warmup_iterations - 1 {
            reset_statement_query(rt, stmt_handle, sql)?;
        }
    }
    Ok(())
}

pub fn run_test_iterations(
    rt: &DriverRuntime,
    stmt_handle: StatementHandle,
    sql: &str,
    iterations: usize,
) -> Result<Vec<IterationResult>> {
    let mut results = Vec::with_capacity(iterations);
    let mut expected_row_count = get_expected_row_count();

    for i in 0..iterations {
        let (query_time, fetch_time, row_count) = execute_iteration(rt, stmt_handle)?;

        expected_row_count = validate_row_count(row_count, expected_row_count, i)?;

        results.push(IterationResult {
            timestamp: current_unix_timestamp(),
            query_time_s: query_time,
            fetch_time_s: fetch_time,
            row_count,
        });

        if i < iterations - 1 {
            reset_statement_query(rt, stmt_handle, sql)?;
        }
    }

    Ok(results)
}

fn get_expected_row_count() -> Option<usize> {
    let expected_from_recording = std::env::var("EXPECTED_ROW_COUNT")
        .ok()
        .and_then(|s| s.parse::<usize>().ok());

    if let Some(expected) = expected_from_recording {
        println!(
            "Row count baseline: {} rows (from recording phase)",
            expected
        );
        Some(expected)
    } else {
        None
    }
}

fn validate_row_count(
    row_count: usize,
    expected: Option<usize>,
    iteration: usize,
) -> Result<Option<usize>> {
    if let Some(expected) = expected {
        if row_count != expected {
            return Err(format!(
                "Row count mismatch: iteration {} returned {} rows, expected {} rows",
                iteration, row_count, expected
            ));
        }
        Ok(Some(expected))
    } else {
        println!(
            "Row count baseline: {} rows (from first iteration)",
            row_count
        );
        Ok(Some(row_count))
    }
}

fn execute_iteration(
    rt: &DriverRuntime,
    stmt_handle: StatementHandle,
) -> Result<(f64, f64, usize)> {
    let start_query = Instant::now();
    let response = rt
        .block_on(async |c| {
            c.statement_execute_query(StatementExecuteQueryRequest {
                stmt_handle: Some(stmt_handle),
                bindings: None,
            })
            .await
        })
        .map_err(|e| format!("Query execution failed: {:?}", e))?;
    let query_time = start_query.elapsed().as_secs_f64();

    // Fetch results outside the runtime so ChunkReader can use its own block_on
    let start_fetch = Instant::now();
    let row_count = if let Some(result) = response.result {
        fetch_result_rows(result).map_err(|e| format!("Failed to fetch results: {:?}", e))?
    } else {
        0
    };
    let fetch_time = start_fetch.elapsed().as_secs_f64();

    Ok((query_time, fetch_time, row_count))
}
