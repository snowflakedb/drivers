use super::{ConfigError, ConfigFileReadSnafu, InsecurePermissionsSnafu, TomlParseSnafu};
use snafu::ResultExt;
use std::env;
use std::fs;
use std::path::Path;

/// Load a TOML file from disk and parse it
pub fn load_toml_file(path: &Path) -> Result<toml::Value, ConfigError> {
    match path.try_exists() {
        Ok(false) => return Ok(toml::Value::Table(toml::map::Map::new())),
        Err(e) => {
            return Err(ConfigError::ConfigFileRead {
                path: path.display().to_string(),
                source: e,
                location: snafu::Location::new(file!(), line!(), 0),
            });
        }
        Ok(true) => {}
    }

    // Check file permissions before reading
    check_file_permissions(path)?;

    // Read file contents
    let contents = fs::read_to_string(path).context(ConfigFileReadSnafu {
        path: path.display().to_string(),
    })?;

    // Parse TOML
    let value = toml::from_str(&contents).context(TomlParseSnafu {
        path: path.display().to_string(),
    })?;

    Ok(value)
}

/// Check file permissions for security (Unix only)
pub fn check_file_permissions(path: &Path) -> Result<(), ConfigError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let metadata = fs::metadata(path).context(ConfigFileReadSnafu {
            path: path.display().to_string(),
        })?;
        let permissions = metadata.permissions();
        let mode = permissions.mode();

        // Error if writable by group or others (0o022)
        if mode & 0o022 != 0 {
            return InsecurePermissionsSnafu {
                path: path.display().to_string(),
                reason: "File is writable by group or others",
            }
            .fail();
        }

        // Warn if readable by group or others (0o044), unless skip env var is set
        if mode & 0o044 != 0
            && env::var("SF_SKIP_WARNING_FOR_READ_PERMISSIONS_ON_CONFIG_FILE").is_err()
        {
            eprintln!(
                "Warning: Config file {} is readable by group or others",
                path.display()
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
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
                .contains("Insecure file permissions")
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

        // Should not print warning (we can't easily test stderr output,
        // but at least verify it doesn't error)
        let result = check_file_permissions(&file_path);
        assert!(result.is_ok());

        // SAFETY: Test-only, not run in parallel.
        unsafe { env::remove_var("SF_SKIP_WARNING_FOR_READ_PERMISSIONS_ON_CONFIG_FILE") };
    }
}
