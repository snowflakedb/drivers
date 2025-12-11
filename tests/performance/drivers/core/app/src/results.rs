//! Results output and CSV formatting

use crate::types::{IterationResult, PutGetResult};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

type Result<T> = std::result::Result<T, String>;

pub fn write_csv_results(results: &[IterationResult], test_name: &str) -> Result<String> {
    write_csv_file(test_name, |file| {
        writeln!(file, "timestamp,query_s,fetch_s")
            .map_err(|e| format!("Failed to write: {:?}", e))?;
        for result in results {
            writeln!(
                file,
                "{},{:.6},{:.6}",
                result.timestamp, result.query_time_s, result.fetch_time_s
            )
            .map_err(|e| format!("Failed to write: {:?}", e))?;
        }
        Ok(())
    })
}

pub fn print_statistics(results: &[IterationResult]) {
    if results.is_empty() {
        return;
    }

    let query_times: Vec<f64> = results.iter().map(|r| r.query_time_s).collect();
    let fetch_times: Vec<f64> = results.iter().map(|r| r.fetch_time_s).collect();

    println!("\nSummary:");
    print_timing_stats("Query", query_times);
    print_timing_stats("Fetch", fetch_times);
}

pub fn write_csv_results_put_get(results: &[PutGetResult], test_name: &str) -> Result<String> {
    write_csv_file(test_name, |file| {
        writeln!(file, "timestamp,query_s").map_err(|e| format!("Failed to write: {:?}", e))?;
        for result in results {
            writeln!(file, "{},{:.6}", result.timestamp, result.query_time_s)
                .map_err(|e| format!("Failed to write: {:?}", e))?;
        }
        Ok(())
    })
}

pub fn print_statistics_put_get(results: &[PutGetResult]) {
    if results.is_empty() {
        return;
    }

    let query_times: Vec<f64> = results.iter().map(|r| r.query_time_s).collect();

    println!("\nSummary:");
    print_timing_stats("Operation time", query_times);
}

pub fn write_run_metadata_json(server_version: &str) -> Result<String> {
    let results_dir = std::env::var("RESULTS_DIR").unwrap_or_else(|_| "/results".to_string());
    let results_path = PathBuf::from(&results_dir);
    let metadata_filename = results_path.join("run_metadata_core.json");

    // Check if metadata already exists (only write once per run)
    if metadata_filename.exists() {
        return Ok(metadata_filename.display().to_string());
    }

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Get driver version from env (set at compile time in Cargo.toml)
    let driver_version = env!("CARGO_PKG_VERSION");

    // Get Rust compiler version from environment
    let build_rust_version =
        std::env::var("RUST_VERSION").unwrap_or_else(|_| "unknown".to_string());

    // Detect architecture and OS inside container
    let architecture = get_architecture();
    let os = get_os_version();

    let metadata = serde_json::json!({
        "driver": "core",
        "driver_type": "universal",
        "driver_version": driver_version,
        "build_rust_version": build_rust_version,
        "runtime_language_version": "NA",  // code is compiled
        "server_version": server_version,
        "architecture": architecture,
        "os": os,
        "run_timestamp": timestamp,
    });

    let mut file = fs::File::create(&metadata_filename)
        .map_err(|e| format!("Failed to create file: {:?}", e))?;
    let json_str = serde_json::to_string_pretty(&metadata)
        .map_err(|e| format!("Failed to serialize to JSON: {:?}", e))?;
    writeln!(file, "{}", json_str).map_err(|e| format!("Failed to write: {:?}", e))?;

    println!("✓ Run metadata saved to: {}", metadata_filename.display());

    Ok(metadata_filename.display().to_string())
}

struct TimingStats {
    median: f64,
    min: f64,
    max: f64,
}

fn calculate_stats(mut values: Vec<f64>) -> TimingStats {
    if values.is_empty() {
        return TimingStats {
            median: 0.0,
            min: 0.0,
            max: 0.0,
        };
    }

    values.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let median = if values.len() % 2 == 0 {
        (values[values.len() / 2 - 1] + values[values.len() / 2]) / 2.0
    } else {
        values[values.len() / 2]
    };

    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    TimingStats { median, min, max }
}

fn print_timing_stats(label: &str, values: Vec<f64>) {
    let stats = calculate_stats(values);
    println!(
        "  {}: median={:.3}s  min={:.3}s  max={:.3}s",
        label, stats.median, stats.min, stats.max
    );
}

fn get_architecture() -> String {
    match std::env::consts::ARCH {
        "aarch64" => "arm64".to_string(),
        "amd64" | "x86_64" => "x86_64".to_string(),
        arch => arch.to_string(),
    }
}

fn get_os_version() -> String {
    if let Ok(os_info) = std::env::var("OS_INFO") {
        return os_info;
    }
    match std::env::consts::OS {
        "macos" => "MacOS".to_string(),
        "linux" => "Linux".to_string(),
        other => other.to_string(),
    }
}

/// Common CSV file creation logic.
/// Handles timestamp generation, directory creation, and file setup.
/// The caller provides a closure to write the specific CSV content.
fn write_csv_file<F>(test_name: &str, write_content: F) -> Result<String>
where
    F: FnOnce(&mut fs::File) -> Result<()>,
{
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Use RESULTS_DIR env var if set (for local execution), otherwise use /results (Docker)
    let results_dir = std::env::var("RESULTS_DIR").unwrap_or_else(|_| "/results".to_string());
    let results_path = PathBuf::from(&results_dir);
    let filename = results_path.join(format!("{}_core_{}.csv", test_name, timestamp));

    fs::create_dir_all(&results_path)
        .map_err(|e| format!("Failed to create directory: {:?}", e))?;
    let mut file =
        fs::File::create(&filename).map_err(|e| format!("Failed to create file: {:?}", e))?;

    write_content(&mut file)?;

    Ok(filename.display().to_string())
}
