use flate2::read::GzDecoder;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
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

/// Convert a path to a string suitable for embedding in Snowflake SQL (PUT/GET).
///
/// On Windows, `tempfile::TempDir` can return short (8.3) paths containing `~`
/// (e.g. `C:\Users\RUNNER~1\AppData\...`). Snowflake's SQL parser rejects `~`,
/// causing `syntax error ... unexpected '~'`. This function resolves the path to
/// its long form via `fs::canonicalize` and strips the `\\?\` prefix that Windows
/// canonicalization adds. Backslashes are converted to forward slashes for SQL.
///
/// On non-Windows, this is a simple backslash-to-forward-slash conversion.
pub fn path_to_sql_uri(path: &Path) -> String {
    let long_path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let path_str = long_path.to_string_lossy();
    let stripped = path_str.strip_prefix(r"\\?\").unwrap_or(&path_str);
    stripped.replace('\\', "/")
}
