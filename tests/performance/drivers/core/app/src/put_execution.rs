//! PUT/GET execution and performance measurement

type Result<T> = std::result::Result<T, String>;
use regex::Regex;
use sf_core::protobuf::generated::database_driver_v1::*;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::connection::{DriverRuntime, reset_statement_query};
use crate::resource_monitor::ResourceMonitor;
use crate::results::{
    current_unix_timestamp_ms, print_statistics_put_get, write_csv_results_put_get,
    write_memory_timeline, write_metadata_if_not_replay,
};
use crate::types::PutGetResult;

pub fn execute_put_get_test(
    rt: &DriverRuntime,
    conn_handle: ConnectionHandle,
    stmt_handle: StatementHandle,
    sql_command: &str,
    warmup_iterations: usize,
    iterations: usize,
    test_name: &str,
) -> Result<()> {
    println!("\n=== Executing PUT_GET Test ===");
    println!("Query: {sql_command}");

    // Warmup
    run_warmup_put_get(rt, stmt_handle, sql_command, warmup_iterations)
        .map_err(|e| format!("Warmup phase failed: {e:?}"))?;

    if warmup_iterations > 0 {
        reset_statement_query(rt, stmt_handle, sql_command)
            .map_err(|e| format!("Failed to reset statement after warmup: {e:?}"))?;
    }

    let mut monitor = ResourceMonitor::new(Duration::from_millis(100));
    monitor.start();

    // Execute
    let results = run_test_iterations_put_get(rt, stmt_handle, sql_command, iterations)
        .map_err(|e| format!("Test phase failed: {e:?}"))?;

    let memory_timeline = monitor.stop();

    // Write & print
    let results_file = write_csv_results_put_get(&results, test_name)
        .map_err(|e| format!("Failed to write results: {e:?}"))?;

    write_memory_timeline(&memory_timeline, test_name);

    write_metadata_if_not_replay(rt, conn_handle)?;

    print_statistics_put_get(&results);
    println!(
        "  Memory timeline: {} samples collected",
        memory_timeline.len()
    );

    println!("\n✓ Complete → {results_file}");

    Ok(())
}

pub fn run_warmup_put_get(
    rt: &DriverRuntime,
    stmt_handle: StatementHandle,
    sql: &str,
    warmup_iterations: usize,
) -> Result<()> {
    if warmup_iterations == 0 {
        return Ok(());
    }

    for i in 0..warmup_iterations {
        execute_put_get_iteration(rt, stmt_handle, sql)?;

        if i < warmup_iterations - 1 {
            reset_statement_query(rt, stmt_handle, sql)?;
        }
    }
    Ok(())
}

pub fn run_test_iterations_put_get(
    rt: &DriverRuntime,
    stmt_handle: StatementHandle,
    sql: &str,
    iterations: usize,
) -> Result<Vec<PutGetResult>> {
    let mut results = Vec::with_capacity(iterations);

    for i in 0..iterations {
        let (query_time, cpu_time_s, peak_rss_mb) =
            execute_put_get_iteration(rt, stmt_handle, sql)?;

        results.push(PutGetResult {
            timestamp: current_unix_timestamp_ms(),
            query_time_s: query_time,
            cpu_time_s,
            peak_rss_mb,
        });

        if i < iterations - 1 {
            reset_statement_query(rt, stmt_handle, sql)?;
        }
    }

    Ok(results)
}

fn execute_put_get_iteration(
    rt: &DriverRuntime,
    stmt_handle: StatementHandle,
    sql: &str,
) -> Result<(f64, f64, f64)> {
    use crate::resource_monitor::{get_peak_rss_mb, process_cpu_seconds};

    create_get_target_directory(sql)?;

    let cpu_before = process_cpu_seconds();
    let start_query = Instant::now();
    rt.block_on(async |c| {
        c.statement_execute_query(StatementExecuteQueryRequest {
            stmt_handle: Some(stmt_handle),
            bindings: None,
        })
        .await
    })
    .map_err(|e| format!("PUT/GET execution failed: {e:?}"))?;
    let query_time = start_query.elapsed().as_secs_f64();
    let cpu_time_s = process_cpu_seconds() - cpu_before;
    let peak_rss_mb = get_peak_rss_mb();

    Ok((query_time, cpu_time_s, peak_rss_mb))
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
