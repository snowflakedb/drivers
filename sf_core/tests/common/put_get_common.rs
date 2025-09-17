pub use super::arrow_deserialize::ArrowDeserialize;
use std::path::PathBuf;
use std::process::Command;

// Structured types for Snowflake command results using our arrow_deserialize macro
#[derive(ArrowDeserialize, Debug, PartialEq)]
pub struct PutResult {
    pub source: String,
    pub target: String,
    pub source_size: i64,
    pub target_size: i64,
    pub source_compression: String,
    pub target_compression: String,
    pub status: String,
    pub message: String,
}

#[derive(ArrowDeserialize, Debug, PartialEq)]
pub struct GetResult {
    pub file: String,
    pub size: i64,
    pub status: String,
    pub message: String,
}

/// Returns repository root path
pub fn repo_root() -> PathBuf {
    if let Ok(output) = Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output()
        && output.status.success()
        && let Ok(stdout) = String::from_utf8(output.stdout)
    {
        let root = stdout.trim();
        if !root.is_empty() {
            return PathBuf::from(root);
        }
    }
    panic!("Failed to determine repository root");
}

/// Path to shared test data directory: repo_root/tests/test_data
pub fn shared_test_data_dir() -> PathBuf {
    repo_root().join("tests").join("generated_test_data")
}
