use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ini::Ini;
use tracing::level_filters::LevelFilter;

use super::error::{ConfigParseSnafu, InsecurePermissionsSnafu, LogError};
use super::{LogRotation, LoggingConfig};
use crate::config::settings::Setting;

/// Parse an `sf.odbc.ini`-style INI file into a [`LoggingConfig`].
///
/// Supported keys (case-sensitive):
/// `LogLevel`, `LogPath`, `LogFile`, `LogMaxSize`, `LogMaxCount`, `LogEnabled`.
///
/// Checks file permissions before reading (rejects group/world-writable files
/// on Unix).
pub fn parse_ini_file(path: &Path) -> Result<LoggingConfig, LogError> {
    crate::config::toml_loader::check_file_permissions(path).map_err(|e| {
        InsecurePermissionsSnafu {
            path: path.display().to_string(),
            reason: e.to_string(),
        }
        .build()
    })?;

    let ini = Ini::load_from_file_noescape(path).map_err(|e| {
        ConfigParseSnafu {
            message: format!("failed to load {}: {e}", path.display()),
        }
        .build()
    })?;
    apply_ini_section(ini.general_section())
}

/// Parse INI content (key=value lines) into a [`LoggingConfig`].
pub fn parse_ini_content(content: &str) -> Result<LoggingConfig, LogError> {
    let ini = Ini::load_from_str_noescape(content).map_err(|e| {
        ConfigParseSnafu {
            message: format!("failed to parse INI content: {e}"),
        }
        .build()
    })?;
    apply_ini_section(ini.general_section())
}

/// Map INI properties to a [`LoggingConfig`].
fn apply_ini_section(props: &ini::Properties) -> Result<LoggingConfig, LogError> {
    let mut config = LoggingConfig::default();
    for (key, value) in props.iter() {
        match key {
            "LogLevel" => config.level = parse_level(value)?,
            "LogPath" => config.log_path = Some(PathBuf::from(value)),
            "LogFile" => config.log_file_name = Some(value.to_string()),
            "LogMaxSize" => config.max_file_size = Some(parse_u64(value)?),
            "LogMaxCount" => config.max_file_count = Some(parse_u32(value)?),
            "LogRotation" => config.rotation = parse_rotation(value)?,
            "LogEnabled" => config.enabled = parse_bool(value)?,
            _ => {}
        }
    }
    Ok(config)
}

/// Build a [`LoggingConfig`] from a TOML `[log]` section loaded via
/// [`crate::config::config_manager::load_config_section`].
pub fn load_from_toml_section(section: &HashMap<String, Setting>) -> LoggingConfig {
    let mut config = LoggingConfig::default();

    if let Some(Setting::String(level)) = section.get("level")
        && let Ok(l) = parse_level(level)
    {
        config.level = l;
    }
    if let Some(Setting::String(path)) = section.get("path") {
        config.log_path = Some(PathBuf::from(path));
    }
    if let Some(Setting::String(file)) = section.get("file") {
        config.log_file_name = Some(file.clone());
    }
    if let Some(Setting::Int(size)) = section.get("max_size")
        && *size > 0
    {
        config.max_file_size = Some(*size as u64);
    }
    if let Some(Setting::Int(count)) = section.get("max_count")
        && *count > 0
    {
        config.max_file_count = Some(*count as u32);
    }
    if let Some(Setting::String(rotation)) = section.get("rotation")
        && let Ok(r) = parse_rotation(rotation)
    {
        config.rotation = r;
    }
    if let Some(Setting::Bool(enabled)) = section.get("enabled") {
        config.enabled = *enabled;
    }
    if let Some(Setting::Bool(otel)) = section.get("opentelemetry") {
        config.open_telemetry = *otel;
    }

    config
}

/// Locate the `sf.odbc.ini` file on the current platform.
///
/// Search order:
/// 1. `SF_ODBC_INI` environment variable
/// 2. `<config_dir>/snowflake/sf.odbc.ini` (platform config directory)
/// 3. `~/.snowflake/sf.odbc.ini`
pub fn find_odbc_ini() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("SF_ODBC_INI") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }

    if let Some(config_dir) = dirs::config_dir() {
        let path = config_dir.join("snowflake").join("sf.odbc.ini");
        if path.exists() {
            return Some(path);
        }
    }

    if let Some(home) = dirs::home_dir() {
        let path = home.join(".snowflake").join("sf.odbc.ini");
        if path.exists() {
            return Some(path);
        }
    }

    None
}

fn parse_level(s: &str) -> Result<LevelFilter, LogError> {
    match s.to_uppercase().as_str() {
        "OFF" => Ok(LevelFilter::OFF),
        "ERROR" => Ok(LevelFilter::ERROR),
        "WARN" | "WARNING" => Ok(LevelFilter::WARN),
        "INFO" => Ok(LevelFilter::INFO),
        "DEBUG" => Ok(LevelFilter::DEBUG),
        "TRACE" => Ok(LevelFilter::TRACE),
        _ => ConfigParseSnafu {
            message: format!("Unknown log level: {s}"),
        }
        .fail(),
    }
}

fn parse_u64(s: &str) -> Result<u64, LogError> {
    s.parse().map_err(|_| {
        ConfigParseSnafu {
            message: format!("Invalid number: {s}"),
        }
        .build()
    })
}

fn parse_u32(s: &str) -> Result<u32, LogError> {
    s.parse().map_err(|_| {
        ConfigParseSnafu {
            message: format!("Invalid number: {s}"),
        }
        .build()
    })
}

fn parse_rotation(s: &str) -> Result<LogRotation, LogError> {
    match s.to_uppercase().as_str() {
        "NEVER" | "NONE" => Ok(LogRotation::Never),
        "DAILY" => Ok(LogRotation::Daily),
        "HOURLY" => Ok(LogRotation::Hourly),
        "MINUTELY" => Ok(LogRotation::Minutely),
        _ => ConfigParseSnafu {
            message: format!("Unknown rotation: {s}"),
        }
        .fail(),
    }
}

fn parse_bool(s: &str) -> Result<bool, LogError> {
    match s.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => ConfigParseSnafu {
            message: format!("Invalid boolean: {s}"),
        }
        .fail(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_ini_content_basic() {
        let content = "LogLevel=DEBUG\nLogEnabled=true\n";
        let config = parse_ini_content(content).unwrap();
        assert_eq!(config.level, LevelFilter::DEBUG);
        assert!(config.enabled);
    }

    #[test]
    fn test_parse_ini_file_valid() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("sf.odbc.ini");
        fs::write(&file_path, "LogLevel=WARN\nLogEnabled=true\n").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&file_path, fs::Permissions::from_mode(0o600)).unwrap();
        }

        let config = parse_ini_file(&file_path).unwrap();
        assert_eq!(config.level, LevelFilter::WARN);
        assert!(config.enabled);
    }

    #[cfg(unix)]
    #[test]
    fn test_parse_ini_file_rejects_world_writable() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("insecure.ini");
        fs::write(&file_path, "LogLevel=INFO\n").unwrap();
        fs::set_permissions(&file_path, fs::Permissions::from_mode(0o666)).unwrap();

        let result = parse_ini_file(&file_path);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Insecure file permissions"),
            "expected InsecurePermissions error, got: {err_msg}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_parse_ini_file_rejects_group_writable() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("group_writable.ini");
        fs::write(&file_path, "LogLevel=INFO\n").unwrap();
        fs::set_permissions(&file_path, fs::Permissions::from_mode(0o620)).unwrap();

        let result = parse_ini_file(&file_path);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Insecure file permissions"),
            "expected InsecurePermissions error, got: {err_msg}"
        );
    }
}
