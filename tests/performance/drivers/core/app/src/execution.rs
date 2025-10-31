//! Query execution and performance measurement helpers

use anyhow::Result;
use sf_core::protobuf_gen::database_driver_v1::*;
use std::time::Instant;

use crate::arrow::fetch_result_rows;
use crate::connection::{DatabaseDriver, reset_statement_query};
use crate::types::IterationResult;

pub fn execute_iteration(stmt_handle: StatementHandle) -> Result<(f64, f64, usize)> {
    // Execute query (measure query execution time)
    let start_query = Instant::now();
    let response = DatabaseDriver::statement_execute_query(StatementExecuteQueryRequest {
        stmt_handle: Some(stmt_handle),
    })
    .map_err(|e| anyhow::anyhow!("Query execution failed: {:?}", e))?;
    let query_time = start_query.elapsed().as_secs_f64();

    // Fetch results (measure fetch time)
    let start_fetch = Instant::now();
    let row_count = if let Some(result) = response.result {
        fetch_result_rows(result)
            .map_err(|e| anyhow::anyhow!("Failed to fetch results: {:?}", e))?
    } else {
        0
    };
    let fetch_time = start_fetch.elapsed().as_secs_f64();

    Ok((query_time, fetch_time, row_count))
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
            query_time_s: query_time,
            fetch_time_s: fetch_time,
        });

        if i < iterations - 1 {
            reset_statement_query(stmt_handle, sql)?;
        }
    }

    Ok(results)
}
