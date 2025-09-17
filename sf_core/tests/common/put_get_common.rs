pub use super::arrow_deserialize::ArrowDeserialize;
use std::path::{Path, PathBuf};

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

/// Returns repository root path assuming this crate is located at repo_root/sf_core
pub fn repo_root() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir).parent().unwrap().to_path_buf()
}

/// Path to shared test data directory: repo_root/tests/test_data
pub fn shared_test_data_dir() -> PathBuf {
    repo_root().join("tests").join("generated_test_data")
}
