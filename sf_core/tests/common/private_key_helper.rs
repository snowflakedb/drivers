use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

use super::config::Parameters;
use super::file_utils::repo_root;

/// A temporary private key file, automatically cleaned up when dropped.
pub struct TempPrivateKeyFile {
    _temp_dir: TempDir,
    file_path: PathBuf,
}

impl TempPrivateKeyFile {
    fn new(private_key_contents: &[String]) -> Result<Self, String> {
        let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp dir: {e}"))?;
        let file_path = temp_dir.path().join("private_key.p8");
        let private_key_str = private_key_contents.join("\n") + "\n";
        fs::write(&file_path, private_key_str)
            .map_err(|e| format!("Failed to write private key file: {e}"))?;

        Ok(Self {
            _temp_dir: temp_dir,
            file_path,
        })
    }

    pub fn path(&self) -> &Path {
        &self.file_path
    }
}

pub fn get_private_key_from_parameters(
    parameters: &Parameters,
) -> Result<TempPrivateKeyFile, String> {
    let private_key_contents = parameters.private_key_contents.as_ref().ok_or_else(|| {
        "SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS not found in parameters.json".to_string()
    })?;
    TempPrivateKeyFile::new(private_key_contents)
}

fn get_test_private_key_contents() -> Vec<String> {
    let key_path = repo_root()
        .join("tests")
        .join("test_data")
        .join("invalid_rsa_key.p8");

    let key_content = fs::read_to_string(&key_path).unwrap_or_else(|_| {
        panic!(
            "Failed to read test private key file: {}",
            key_path.display()
        )
    });

    key_content.lines().map(|s| s.to_string()).collect()
}

pub fn get_test_private_key_file() -> Result<TempPrivateKeyFile, String> {
    let key_contents = get_test_private_key_contents();
    TempPrivateKeyFile::new(&key_contents)
}
