use flate2::read::GzDecoder;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::process::Command;

/// Decompresses a gzipped file and returns its content as a string
pub fn decompress_gzipped_file<P: AsRef<std::path::Path>>(file_path: P) -> std::io::Result<String> {
    let gz_file = fs::File::open(file_path)?;
    let mut decoder = GzDecoder::new(gz_file);
    let mut decompressed_content = String::new();
    decoder.read_to_string(&mut decompressed_content)?;
    Ok(decompressed_content)
}

pub fn create_test_file(
    temp_dir: &std::path::Path,
    filename: &str,
    content: &str,
) -> std::path::PathBuf {
    let file_path = temp_dir.join(filename);
    fs::write(&file_path, content).unwrap();
    file_path
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
    repo_root()
        .join("tests")
        .join("test_data")
        .join("generated_test_data")
}
