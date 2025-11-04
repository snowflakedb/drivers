//! Results output and CSV formatting

use crate::types::IterationResult;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub fn write_csv_results(results: &[IterationResult], test_name: &str) -> anyhow::Result<String> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Use RESULTS_DIR env var if set (for local execution), otherwise use /results (Docker)
    let results_dir = std::env::var("RESULTS_DIR").unwrap_or_else(|_| "/results".to_string());
    let results_path = PathBuf::from(&results_dir);
    let filename = results_path.join(format!("{}_core_{}.csv", test_name, timestamp));

    fs::create_dir_all(&results_path)?;
    let mut file = fs::File::create(&filename)?;
    writeln!(file, "query_time_s,fetch_time_s")?;
    for result in results {
        writeln!(
            file,
            "{:.6},{:.6}",
            result.query_time_s, result.fetch_time_s
        )?;
    }

    Ok(filename.display().to_string())
}

pub fn print_statistics(results: &[IterationResult]) {
    if results.is_empty() {
        return;
    }

    let mut query_times: Vec<f64> = results.iter().map(|r| r.query_time_s).collect();
    let mut fetch_times: Vec<f64> = results.iter().map(|r| r.fetch_time_s).collect();

    // Sort for median calculation
    query_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    fetch_times.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let query_median = if query_times.len() % 2 == 0 {
        (query_times[query_times.len() / 2 - 1] + query_times[query_times.len() / 2]) / 2.0
    } else {
        query_times[query_times.len() / 2]
    };
    let query_min = query_times.iter().cloned().fold(f64::INFINITY, f64::min);
    let query_max = query_times
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);

    let fetch_median = if fetch_times.len() % 2 == 0 {
        (fetch_times[fetch_times.len() / 2 - 1] + fetch_times[fetch_times.len() / 2]) / 2.0
    } else {
        fetch_times[fetch_times.len() / 2]
    };
    let fetch_min = fetch_times.iter().cloned().fold(f64::INFINITY, f64::min);
    let fetch_max = fetch_times
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);

    println!("\nSummary:");
    println!(
        "  Query: median={:.3}s  min={:.3}s  max={:.3}s",
        query_median, query_min, query_max
    );
    println!(
        "  Fetch: median={:.3}s  min={:.3}s  max={:.3}s",
        fetch_median, fetch_min, fetch_max
    );
}

fn get_architecture() -> String {
    let arch = std::env::consts::ARCH;

    match arch {
        "x86_64" | "amd64" => "x86_64".to_string(),
        "aarch64" | "arm64" => "arm64".to_string(),
        _ => arch.to_string(), // Return as-is if unknown
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

pub fn write_run_metadata_json(server_version: &str) -> anyhow::Result<String> {
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

    // Detect architecture and OS inside container
    let architecture = get_architecture();
    let os = get_os_version();

    let metadata = serde_json::json!({
        "driver": "core",
        "driver_type": "universal",
        "driver_version": driver_version,
        "server_version": server_version,
        "architecture": architecture,
        "os": os,
        "run_timestamp": timestamp,
    });

    let mut file = fs::File::create(&metadata_filename)?;
    writeln!(file, "{}", serde_json::to_string_pretty(&metadata)?)?;

    println!("✓ Run metadata saved to: {}", metadata_filename.display());

    Ok(metadata_filename.display().to_string())
}
