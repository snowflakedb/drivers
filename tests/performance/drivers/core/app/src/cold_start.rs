//! Fresh-process cold start: connect + SELECT 1. No import/load phase.

use crate::config::TestConfig;
use crate::connection::{
    DriverRuntime, create_connection, create_database, create_statement,
};
use crate::query_execution::execute_iteration;
use crate::resource_monitor::{get_peak_rss_mb, process_cpu_seconds};
use crate::results::{
    current_unix_timestamp_ms, write_csv_results_cold_start, write_run_metadata_json,
};
use sf_core::protobuf::apis::database_driver_v1::DatabaseDriverClientBlockingExt;
use sf_core::protobuf::generated::database_driver_v1::{
    ConnectionReleaseRequest, StatementReleaseRequest,
};
use std::io::{self, Write};
use std::process::{Command, Stdio};
use std::time::Instant;

type Result<T> = std::result::Result<T, String>;

pub fn run_cold_start(config: &TestConfig) -> Result<()> {
    if std::env::var("COLD_START_CHILD").ok().as_deref() == Some("1") {
        return run_child(config);
    }
    run_parent(config)
}

fn run_parent(config: &TestConfig) -> Result<()> {
    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    println!("\n=== Cold-Start Test ({} iterations) ===", config.iterations);
    let mut rows = Vec::new();
    for i in 0..config.iterations {
        let label = format!("iter {}/{}", i + 1, config.iterations);
        let output = Command::new(&exe)
            .env("COLD_START_CHILD", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| format!("spawn child: {e}"))?;
        if !output.status.success() {
            return Err(format!("[{label}] child failed (status {})", output.status));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let line = stdout
            .lines()
            .last()
            .ok_or_else(|| format!("[{label}] no child output"))?
            .to_string();
        println!("  [{label}] {line}");
        rows.push(line);
    }
    let filename = write_csv_results_cold_start(&rows, &config.test_name)?;
    write_run_metadata_json("N/A")?;
    println!("\n✓ Complete → {filename}");
    Ok(())
}

fn run_child(config: &TestConfig) -> Result<()> {
    let t0 = Instant::now();
    let rt = DriverRuntime::new();
    let db_handle = create_database(&rt)?;
    let conn_handle = create_connection(&rt, db_handle, &config.params.testconnection)?;
    let connect_s = t0.elapsed().as_secs_f64();

    let t1 = Instant::now();
    let stmt_handle = create_statement(
        &rt,
        conn_handle,
        &config.sql_command,
        config.statement_async_override,
    )?;
    let iteration = execute_iteration(&rt, stmt_handle)?;
    if iteration.row_count != 1 {
        return Err(format!("Expected 1 row, got {}", iteration.row_count));
    }
    let select1_s = t1.elapsed().as_secs_f64();
    let e2e_s = t0.elapsed().as_secs_f64();

    let _ = rt.client().statement_release_blocking(StatementReleaseRequest {
        stmt_handle: Some(stmt_handle),
    });
    let _ = rt
        .client()
        .connection_release_blocking(ConnectionReleaseRequest {
            conn_handle: Some(conn_handle),
        });

    let line = format!(
        "{},{:.6},{:.6},{:.6},{:.6},{:.1}",
        current_unix_timestamp_ms(),
        e2e_s,
        connect_s,
        select1_s,
        process_cpu_seconds(),
        get_peak_rss_mb()
    );
    println!("{line}");
    io::stdout().flush().ok();
    Ok(())
}
