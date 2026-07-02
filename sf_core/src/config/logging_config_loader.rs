//! Translation between the raw [`IniConfig`] snapshot (or a TOML `[log]`
//! section) and [`LoggingConfig`].
//!
//! Keeping the projection here means the logging crate never sees raw INI:
//! it receives an already-typed [`LoggingConfig`] and focuses on installing
//! tracing layers. New non-logging subsystems add their own `*_from_ini`
//! helpers in the config layer without touching the loader.

use std::collections::HashMap;
use std::path::PathBuf;

use tracing::level_filters::LevelFilter;

use super::settings::Setting;
use super::{ConfigError, IniConfig, IniParseSnafu};
use crate::logging::{LogRotation, LoggingConfig};

/// Project the logging-namespace keys of `ini` into a [`LoggingConfig`].
///
/// Recognised keys (case-insensitive): `LogLevel`, `LogPath`, `LogFile`,
/// `LogMaxSize`, `LogMaxCount`, `LogRotation`, `LogEnabled`, `LogQueryText`,
/// `LogQueryParameters`, `ErrorTraceEnabled`. Keys outside this set are
/// silently ignored — they remain available to other subsystems via the
/// shared [`IniConfig`]. Recognised keys with an invalid value raise
/// [`ConfigError::IniParse`].
pub fn logging_config_from_ini(ini: &IniConfig) -> Result<LoggingConfig, ConfigError> {
    let mut config = LoggingConfig::default();
    for (key, value) in ini.iter() {
        match key {
            "loglevel" => config.level = parse_level(value)?,
            "logpath" => config.log_path = Some(PathBuf::from(value)),
            "logfile" => config.log_file_name = Some(value.to_string()),
            "logmaxsize" => config.max_file_size = Some(parse_u64(value)?),
            "logmaxcount" => config.max_file_count = Some(parse_u32(value)?),
            "logrotation" => config.rotation = parse_rotation(value)?,
            "logenabled" => config.enabled = parse_bool(value)?,
            "logquerytext" => config.log_query_text = Some(parse_bool(value)?),
            "logqueryparameters" => config.log_query_parameters = Some(parse_bool(value)?),
            "errortraceenabled" => config.error_trace_enabled = parse_bool(value)?,
            _ => {}
        }
    }
    Ok(config)
}

/// Build a [`LoggingConfig`] from a TOML `[log]` section loaded via
/// [`crate::config::config_manager::load_config_section`]. Unknown or
/// wrong-typed entries are ignored (TOML is already typed, so a parse
/// failure here means a schema mismatch the caller can recover from).
pub fn logging_config_from_toml_section(section: &HashMap<String, Setting>) -> LoggingConfig {
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
    if let Some(Setting::Bool(b)) = section.get("log_query_text") {
        config.log_query_text = Some(*b);
    }
    if let Some(Setting::Bool(b)) = section.get("log_query_parameters") {
        config.log_query_parameters = Some(*b);
    }
    if let Some(Setting::Bool(error_trace)) = section.get("error_trace_enabled") {
        config.error_trace_enabled = *error_trace;
    }

    config
}

fn parse_level(s: &str) -> Result<LevelFilter, ConfigError> {
    match s.to_uppercase().as_str() {
        "OFF" => Ok(LevelFilter::OFF),
        "ERROR" => Ok(LevelFilter::ERROR),
        "WARN" | "WARNING" => Ok(LevelFilter::WARN),
        "INFO" => Ok(LevelFilter::INFO),
        "DEBUG" => Ok(LevelFilter::DEBUG),
        "TRACE" => Ok(LevelFilter::TRACE),
        _ => IniParseSnafu {
            message: format!("Unknown log level: {s}"),
        }
        .fail(),
    }
}

fn parse_rotation(s: &str) -> Result<LogRotation, ConfigError> {
    match s.to_uppercase().as_str() {
        "NEVER" | "NONE" => Ok(LogRotation::Never),
        "DAILY" => Ok(LogRotation::Daily),
        "HOURLY" => Ok(LogRotation::Hourly),
        "MINUTELY" => Ok(LogRotation::Minutely),
        _ => IniParseSnafu {
            message: format!("Unknown rotation: {s}"),
        }
        .fail(),
    }
}

fn parse_u64(s: &str) -> Result<u64, ConfigError> {
    s.parse().map_err(|_| {
        IniParseSnafu {
            message: format!("Invalid number: {s}"),
        }
        .build()
    })
}

fn parse_u32(s: &str) -> Result<u32, ConfigError> {
    s.parse().map_err(|_| {
        IniParseSnafu {
            message: format!("Invalid number: {s}"),
        }
        .build()
    })
}

fn parse_bool(s: &str) -> Result<bool, ConfigError> {
    match s.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => IniParseSnafu {
            message: format!("Invalid boolean: {s}"),
        }
        .fail(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ini_from(content: &str) -> IniConfig {
        IniConfig::from_ini_content(content).unwrap()
    }

    // ---- INI projection ----

    #[test]
    fn empty_ini_yields_default_logging_config() {
        let config = logging_config_from_ini(&ini_from("")).unwrap();
        assert_eq!(config.level, LevelFilter::INFO);
        assert!(config.log_path.is_none());
        assert!(config.log_file_name.is_none());
        assert!(config.max_file_size.is_none());
        assert!(config.max_file_count.is_none());
        assert!(config.enabled);
        assert!(!config.open_telemetry);
        assert!(config.error_trace_enabled);
    }

    #[test]
    fn all_logging_keys_recognised() {
        let ini = ini_from(
            "LogLevel=DEBUG\n\
             LogPath=/var/log/snowflake\n\
             LogFile=driver.log\n\
             LogMaxSize=1048576\n\
             LogMaxCount=5\n\
             LogRotation=DAILY\n\
             LogEnabled=true\n\
             LogQueryText=true\n\
             LogQueryParameters=false\n\
             ErrorTraceEnabled=false\n",
        );
        let config = logging_config_from_ini(&ini).unwrap();
        assert_eq!(config.level, LevelFilter::DEBUG);
        assert_eq!(
            config.log_path.unwrap(),
            PathBuf::from("/var/log/snowflake")
        );
        assert_eq!(config.log_file_name.unwrap(), "driver.log");
        assert_eq!(config.max_file_size.unwrap(), 1_048_576);
        assert_eq!(config.max_file_count.unwrap(), 5);
        assert_eq!(config.rotation, LogRotation::Daily);
        assert!(config.enabled);
        assert_eq!(config.log_query_text, Some(true));
        assert_eq!(config.log_query_parameters, Some(false));
        assert!(!config.error_trace_enabled);
    }

    #[test]
    fn non_logging_keys_are_ignored() {
        let ini = ini_from("LogLevel=WARN\nDriverManagerEncoding=UTF-32\n");
        let config = logging_config_from_ini(&ini).unwrap();
        assert_eq!(config.level, LevelFilter::WARN);
        assert!(config.log_path.is_none());
    }

    #[test]
    fn case_insensitive_keys() {
        let ini = ini_from("loglevel=trace\nLOGENABLED=false\n");
        let config = logging_config_from_ini(&ini).unwrap();
        assert_eq!(config.level, LevelFilter::TRACE);
        assert!(!config.enabled);
    }

    #[test]
    fn unknown_log_level_surfaces_parse_error() {
        let ini = ini_from("LogLevel=VERBOSE\n");
        let err = logging_config_from_ini(&ini).unwrap_err();
        assert!(matches!(err, ConfigError::IniParse { .. }), "got: {err:?}");
        assert!(err.to_string().contains("Unknown log level"));
    }

    #[test]
    fn invalid_bool_surfaces_parse_error() {
        let ini = ini_from("LogEnabled=maybe\n");
        let err = logging_config_from_ini(&ini).unwrap_err();
        assert!(matches!(err, ConfigError::IniParse { .. }), "got: {err:?}");
        assert!(err.to_string().contains("Invalid boolean"));
    }

    #[test]
    fn invalid_number_surfaces_parse_error() {
        let ini = ini_from("LogMaxSize=not_a_number\n");
        let err = logging_config_from_ini(&ini).unwrap_err();
        assert!(matches!(err, ConfigError::IniParse { .. }), "got: {err:?}");
        assert!(err.to_string().contains("Invalid number"));
    }

    #[test]
    fn bool_truthy_variants() {
        for truthy in ["true", "1", "yes", "on", "True", "YES", "ON"] {
            let ini = ini_from(&format!("LogEnabled={truthy}\n"));
            assert!(
                logging_config_from_ini(&ini).unwrap().enabled,
                "expected true for {truthy}"
            );
        }
    }

    #[test]
    fn bool_falsy_variants() {
        for falsy in ["false", "0", "no", "off", "False", "NO", "OFF"] {
            let ini = ini_from(&format!("LogEnabled={falsy}\n"));
            assert!(
                !logging_config_from_ini(&ini).unwrap().enabled,
                "expected false for {falsy}"
            );
        }
    }

    #[test]
    fn log_query_keys_default_to_none() {
        let config = logging_config_from_ini(&ini_from("LogLevel=INFO\n")).unwrap();
        assert_eq!(config.log_query_text, None);
        assert_eq!(config.log_query_parameters, None);
    }

    // ---- TOML projection ----

    #[test]
    fn toml_section_all_fields() {
        let mut section = HashMap::new();
        section.insert("level".into(), Setting::String("DEBUG".into()));
        section.insert("path".into(), Setting::String("/var/log".into()));
        section.insert("file".into(), Setting::String("app.log".into()));
        section.insert("max_size".into(), Setting::Int(2_000_000));
        section.insert("max_count".into(), Setting::Int(3));
        section.insert("rotation".into(), Setting::String("DAILY".into()));
        section.insert("enabled".into(), Setting::Bool(false));
        section.insert("opentelemetry".into(), Setting::Bool(true));
        section.insert("log_query_text".into(), Setting::Bool(true));
        section.insert("log_query_parameters".into(), Setting::Bool(false));
        section.insert("error_trace_enabled".into(), Setting::Bool(false));

        let config = logging_config_from_toml_section(&section);
        assert_eq!(config.level, LevelFilter::DEBUG);
        assert_eq!(config.log_path.unwrap(), PathBuf::from("/var/log"));
        assert_eq!(config.log_file_name.unwrap(), "app.log");
        assert_eq!(config.max_file_size.unwrap(), 2_000_000);
        assert_eq!(config.max_file_count.unwrap(), 3);
        assert_eq!(config.rotation, LogRotation::Daily);
        assert!(!config.enabled);
        assert!(config.open_telemetry);
        assert_eq!(config.log_query_text, Some(true));
        assert_eq!(config.log_query_parameters, Some(false));
        assert!(!config.error_trace_enabled);
    }

    #[test]
    fn toml_section_empty_returns_defaults() {
        let config = logging_config_from_toml_section(&HashMap::new());
        assert_eq!(config.level, LevelFilter::INFO);
        assert!(config.log_path.is_none());
        assert!(config.enabled);
        assert!(config.error_trace_enabled);
    }

    #[test]
    fn toml_section_wrong_type_for_level_keeps_default() {
        let mut section = HashMap::new();
        section.insert("level".into(), Setting::Int(42));
        let config = logging_config_from_toml_section(&section);
        assert_eq!(config.level, LevelFilter::INFO);
    }

    #[test]
    fn toml_section_negative_and_zero_sizes_ignored() {
        for size in [-100, 0] {
            let mut section = HashMap::new();
            section.insert("max_size".into(), Setting::Int(size));
            section.insert("max_count".into(), Setting::Int(size));
            let config = logging_config_from_toml_section(&section);
            assert!(config.max_file_size.is_none(), "size={size}");
            assert!(config.max_file_count.is_none(), "size={size}");
        }
    }

    #[test]
    fn toml_section_invalid_level_string_keeps_default() {
        let mut section = HashMap::new();
        section.insert("level".into(), Setting::String("VERBOSE".into()));
        let config = logging_config_from_toml_section(&section);
        assert_eq!(config.level, LevelFilter::INFO);
    }
}
