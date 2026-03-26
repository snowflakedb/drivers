//! Results output and CSV formatting

use crate::connection::{DriverRuntime, get_server_version as get_server_version_internal};
use crate::resource_monitor::MemorySample;
use crate::types::{IterationResult, PutGetResult};
use sf_core::protobuf::generated::database_driver_v1::ConnectionHandle;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

type Result<T> = std::result::Result<T, String>;

pub fn write_csv_results(results: &[IterationResult], test_name: &str) -> Result<String> {
    write_csv_file(test_name, |file| {
        writeln!(
            file,
            "timestamp_ms,query_s,fetch_s,row_count,cpu_time_s,peak_rss_mb"
        )
        .map_err(|e| format!("Failed to write: {e:?}"))?;
        for r in results {
            writeln!(
                file,
                "{},{:.6},{:.6},{},{:.6},{:.1}",
                r.timestamp,
                r.query_time_s,
                r.fetch_time_s,
                r.row_count,
                r.cpu_time_s,
                r.peak_rss_mb
            )
            .map_err(|e| format!("Failed to write: {e:?}"))?;
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
        writeln!(file, "timestamp_ms,query_s,cpu_time_s,peak_rss_mb")
            .map_err(|e| format!("Failed to write: {e:?}"))?;
        for r in results {
            writeln!(
                file,
                "{},{:.6},{:.6},{:.1}",
                r.timestamp, r.query_time_s, r.cpu_time_s, r.peak_rss_mb
            )
            .map_err(|e| format!("Failed to write: {e:?}"))?;
        }
        Ok(())
    })
}

pub fn write_memory_timeline(samples: &[MemorySample], test_name: &str) {
    if samples.is_empty() {
        return;
    }

    let timestamp = current_unix_timestamp_ms();
    let results_dir = std::env::var("RESULTS_DIR").unwrap_or_else(|_| "/results".to_string());
    let results_path = PathBuf::from(&results_dir);
    let filename = results_path.join(format!("memory_timeline_{test_name}_core_{timestamp}.csv"));

    let Ok(mut file) = fs::File::create(&filename) else {
        eprintln!("⚠️  Warning: Could not create memory timeline file");
        return;
    };

    let _ = writeln!(file, "timestamp_ms,rss_bytes,vm_bytes");
    for s in samples {
        let _ = writeln!(file, "{},{},{}", s.timestamp_ms, s.rss_bytes, s.vm_bytes);
    }

    println!(
        "✓ Memory timeline → {} ({} samples)",
        filename.display(),
        samples.len()
    );
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

    let timestamp = current_unix_timestamp_ms();

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

pub fn current_unix_timestamp_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// Common CSV file creation logic.
/// Handles timestamp generation, directory creation, and file setup.
/// The caller provides a closure to write the specific CSV content.
fn write_csv_file<F>(test_name: &str, write_content: F) -> Result<String>
where
    F: FnOnce(&mut fs::File) -> Result<()>,
{
    let timestamp = current_unix_timestamp_ms();

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

pub fn write_metadata_if_not_replay(
    rt: &DriverRuntime,
    conn_handle: ConnectionHandle,
) -> Result<()> {
    // In replay mode, skip server version query and use N/A
    let actual_server_version = match std::env::var("WIREMOCK_REPLAY") {
        Ok(val) if val == "true" => "N/A".to_string(),
        _ => get_server_version_internal(rt, conn_handle).unwrap_or_else(|e| {
            eprintln!("⚠️  Warning: Could not retrieve server version: {}", e);
            "UNKNOWN".to_string()
        }),
    };
    write_run_metadata_json(&actual_server_version)
        .map_err(|e| format!("Failed to write metadata: {:?}", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_stats_empty_returns_zeros() {
        let stats = calculate_stats(vec![]);
        assert_eq!(stats.median, 0.0);
        assert_eq!(stats.min, 0.0);
        assert_eq!(stats.max, 0.0);
    }

    #[test]
    fn calculate_stats_single_element() {
        let stats = calculate_stats(vec![5.0]);
        assert_eq!(stats.median, 5.0);
        assert_eq!(stats.min, 5.0);
        assert_eq!(stats.max, 5.0);
    }

    #[test]
    fn calculate_stats_odd_count_median() {
        // [1, 2, 3, 4, 5] -> median is 3 (middle element)
        let stats = calculate_stats(vec![3.0, 1.0, 5.0, 2.0, 4.0]);
        assert_eq!(stats.median, 3.0);
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 5.0);
    }

    #[test]
    fn calculate_stats_even_count_median() {
        // [1, 2, 3, 4] -> median is (2 + 3) / 2 = 2.5
        let stats = calculate_stats(vec![4.0, 1.0, 3.0, 2.0]);
        assert_eq!(stats.median, 2.5);
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 4.0);
    }
}
