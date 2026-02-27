use super::{ConfigError, SecureFsSnafu, TomlParseSnafu};
use crate::secure_fs::{
    secure_check_permissions, secure_read_to_string, PermissionCheck, ReadOptions, SecureFsError,
};
use snafu::ResultExt;
use std::env;
use std::io::ErrorKind;
use std::path::Path;

/// Resolve the permission check mode based on the environment variable.
///
/// When `SF_SKIP_WARNING_FOR_READ_PERMISSIONS_ON_CONFIG_FILE` is set, returns
/// `Skip` (suppress all checks). Otherwise returns `Warn` to match the
/// original behavior (writable-by-others is still a hard error inside
/// `secure_fs`, but readable-by-others only emits a warning).
fn config_permission_check() -> PermissionCheck {
    if env::var("SF_SKIP_WARNING_FOR_READ_PERMISSIONS_ON_CONFIG_FILE").is_ok() {
        PermissionCheck::Skip
    } else {
        PermissionCheck::Warn
    }
}

/// Load a TOML file from disk and parse it.
///
/// Returns an empty TOML table when the file does not exist (preserving
/// backward-compatible behaviour for optional config/connections files).
///
/// Permission checks are delegated to `secure_fs`. Set the environment
/// variable `SF_SKIP_WARNING_FOR_READ_PERMISSIONS_ON_CONFIG_FILE` to any
/// value to suppress permission checks entirely.
pub fn load_toml_file(path: &Path) -> Result<toml::Value, ConfigError> {
    let opts = ReadOptions {
        max_size: 10 * 1024 * 1024, // 10 MiB
        check_permissions: config_permission_check(),
    };

    let contents = match secure_read_to_string(path, &opts) {
        Ok(c) => c,
        Err(SecureFsError::Io { ref source, .. }) if source.kind() == ErrorKind::NotFound => {
            return Ok(toml::Value::Table(toml::map::Map::new()));
        }
        Err(e) => {
            return Err(e).context(SecureFsSnafu {
                path: path.display().to_string(),
            });
        }
    };

    let value = toml::from_str(&contents).context(TomlParseSnafu {
        path: path.display().to_string(),
    })?;

    Ok(value)
}

/// Check file permissions for security (Unix only).
///
/// Opens the file and validates permission bits without reading content.
/// Retained for backward compatibility with call sites outside this module.
pub fn check_file_permissions(path: &Path) -> Result<(), ConfigError> {
    secure_check_permissions(path, config_permission_check()).context(SecureFsSnafu {
        path: path.display().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_load_toml_file_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("nonexistent.toml");

        let result = load_toml_file(&file_path);
        assert!(result.is_ok());

        // Should return empty table for non-existent file
        let value = result.unwrap();
        assert!(value.as_table().is_some());
        assert!(value.as_table().unwrap().is_empty());
    }

    #[test]
    fn test_load_toml_file_valid() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.toml");
        let content = r#"
[section]
key = "value"
number = 42
"#;
        fs::write(&file_path, content).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&file_path, fs::Permissions::from_mode(0o600)).unwrap();
        }

        let result = load_toml_file(&file_path);
        assert!(result.is_ok());

        let value = result.unwrap();
        let table = value.as_table().unwrap();
        assert!(table.contains_key("section"));

        let section = table.get("section").unwrap().as_table().unwrap();
        assert_eq!(section.get("key").unwrap().as_str().unwrap(), "value");
        assert_eq!(section.get("number").unwrap().as_integer().unwrap(), 42);
    }

    #[test]
    fn test_load_toml_file_invalid() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("invalid.toml");
        let content = "This is not valid TOML {][@#";
        fs::write(&file_path, content).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&file_path, fs::Permissions::from_mode(0o600)).unwrap();
        }

        let result = load_toml_file(&file_path);
        assert!(result.is_err());
        // Should be a parse error
        assert!(result.unwrap_err().to_string().contains("parse TOML"));
    }

    #[cfg(unix)]
    #[test]
    fn test_check_file_permissions_writable_by_others() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("insecure.toml");
        fs::write(&file_path, "").unwrap();

        // Set writable by others
        fs::set_permissions(&file_path, fs::Permissions::from_mode(0o666)).unwrap();

        let result = check_file_permissions(&file_path);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Insecure permissions")
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_skip_warning_env_var() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("readable.toml");
        fs::write(&file_path, "").unwrap();

        // Set readable by others
        fs::set_permissions(&file_path, fs::Permissions::from_mode(0o644)).unwrap();

        // Set env var to skip warning
        // SAFETY: Test-only, not run in parallel.
        unsafe { env::set_var("SF_SKIP_WARNING_FOR_READ_PERMISSIONS_ON_CONFIG_FILE", "1") };

        // Should not error because permission check is skipped
        let result = check_file_permissions(&file_path);
        assert!(result.is_ok());

        // SAFETY: Test-only, not run in parallel.
        unsafe { env::remove_var("SF_SKIP_WARNING_FOR_READ_PERMISSIONS_ON_CONFIG_FILE") };
    }
}
