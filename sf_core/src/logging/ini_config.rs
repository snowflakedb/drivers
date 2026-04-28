use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ini::Ini;
use tracing::level_filters::LevelFilter;

use super::LoggingConfig;
use super::error::{ConfigParseSnafu, LogError};
use crate::config::settings::Setting;

/// Parse an `sf.odbc.ini`-style INI file into a [`LoggingConfig`].
///
/// Supported keys (case-sensitive):
/// `LogLevel`, `LogPath`, `LogFile`, `LogMaxSize`, `LogMaxCount`, `LogEnabled`.
pub fn parse_ini_file(path: &Path) -> Result<LoggingConfig, LogError> {
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
