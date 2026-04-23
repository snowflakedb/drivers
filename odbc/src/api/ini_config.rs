#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use sf_core::logging::LoggingConfig;
use tracing::level_filters::LevelFilter;

const INI_FILENAME: &str = "sf.snowflake.ini";
const INI_SECTION: &str = "Driver";

const KEY_LOG_LEVEL: &str = "LOGLEVEL";
const KEY_LOG_PATH: &str = "LOGPATH";
const KEY_LOG_FILE_SIZE: &str = "LOGFILESIZE";
const KEY_LOG_FILE_COUNT: &str = "LOGFILECOUNT";

/// Read driver-level logging configuration from `sf.snowflake.ini`.
///
/// Searches for the INI file in the following order:
/// 1. Path specified by the `SF_ODBC_INI` environment variable (full file path)
/// 2. `~/.snowflake/sf.snowflake.ini`
/// 3. `/opt/snowflake/snowflakeodbc/lib/universal/sf.snowflake.ini` (Unix only)
///
/// Parses the `[Driver]` section for logging keys (`LogLevel`, `LogPath`,
/// `LogFileSize`, `LogFileCount`) and returns a [`LoggingConfig`] with the
/// discovered values merged over defaults. If no INI file is found or the
/// `[Driver]` section is absent, returns the default configuration.
pub fn read_driver_logging_config() -> LoggingConfig {
    read_config_from_paths(ini_search_paths())
}

/// Read driver logging config from the first INI file found in `paths`.
fn read_config_from_paths(paths: impl IntoIterator<Item = PathBuf>) -> LoggingConfig {
    for path in paths {
        if let Ok(content) = std::fs::read_to_string(&path)
            && let Some(params) = parse_ini_section(&content, INI_SECTION)
        {
            tracing::debug!("read driver logging config from {}", path.display());
            return apply_ini_params(params);
        }
    }
    tracing::debug!("no {INI_FILENAME} found on search path");
    LoggingConfig::default()
}

/// Build the ordered list of candidate paths for `sf.snowflake.ini`.
fn ini_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(p) = std::env::var("SF_ODBC_INI") {
        paths.push(PathBuf::from(p));
    }

    if let Some(home) = home_dir() {
        paths.push(home.join(".snowflake").join(INI_FILENAME));
    }

    #[cfg(not(windows))]
    paths.push(Path::new("/opt/snowflake/snowflakeodbc/lib/universal").join(INI_FILENAME));

    paths
}

fn home_dir() -> Option<PathBuf> {
    #[cfg(not(windows))]
    {
        std::env::var("HOME").ok().map(PathBuf::from)
    }
    #[cfg(windows)]
    {
        std::env::var("USERPROFILE").ok().map(PathBuf::from)
    }
}

/// Apply parsed INI key-value pairs to a default [`LoggingConfig`].
fn apply_ini_params(params: HashMap<String, String>) -> LoggingConfig {
    let mut config = LoggingConfig::default();

    if let Some(level_str) = params.get(KEY_LOG_LEVEL) {
        match level_str.parse::<u8>() {
            Ok(n) => {
                config.log_level = numeric_to_level_filter(n);
                config.enabled = config.log_level != LevelFilter::OFF;
            }
            Err(_) => tracing::warn!("invalid LogLevel value in {INI_FILENAME}: {level_str}"),
        }
    }

    if let Some(path_str) = params.get(KEY_LOG_PATH)
        && !path_str.is_empty()
    {
        config.log_path = Some(PathBuf::from(path_str));
    }

    if let Some(size_str) = params.get(KEY_LOG_FILE_SIZE) {
        match size_str.parse::<u64>() {
            Ok(size) => config.log_file_size = size,
            Err(_) => tracing::warn!("invalid LogFileSize value in {INI_FILENAME}: {size_str}"),
        }
    }

    if let Some(count_str) = params.get(KEY_LOG_FILE_COUNT) {
        match count_str.parse::<usize>() {
            Ok(count) => config.log_file_count = count,
            Err(_) => {
                tracing::warn!("invalid LogFileCount value in {INI_FILENAME}: {count_str}");
            }
        }
    }

    config
}

/// Map the ODBC numeric log level (0-6) to a tracing [`LevelFilter`].
///
/// | Value | Level     | tracing          |
/// |-------|-----------|------------------|
/// |   0   | OFF       | `OFF`            |
/// |   1   | FATAL     | `ERROR` (mapped) |
/// |   2   | ERROR     | `ERROR`          |
/// |   3   | WARNING   | `WARN`           |
/// |   4   | INFO      | `INFO`           |
/// |   5   | DEBUG     | `DEBUG`          |
/// |   6   | TRACE     | `TRACE`          |
fn numeric_to_level_filter(level: u8) -> LevelFilter {
    match level {
        0 => LevelFilter::OFF,
        1 | 2 => LevelFilter::ERROR,
        3 => LevelFilter::WARN,
        4 => LevelFilter::INFO,
        5 => LevelFilter::DEBUG,
        _ => LevelFilter::TRACE,
    }
}

/// Parse an INI-format string and return the key/value pairs from `section`.
///
/// Keys are normalized to uppercase. Section name matching is
/// case-insensitive. This mirrors the logic in
/// `connection::parse_ini_section` but is available on all platforms.
fn parse_ini_section(content: &str, section: &str) -> Option<HashMap<String, String>> {
    let mut in_section = false;
    let mut params = HashMap::new();
    let mut found = false;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            let s = &line[1..line.len() - 1];
            in_section = s.eq_ignore_ascii_case(section);
            if in_section {
                found = true;
            }
            continue;
        }
        if !in_section || line.starts_with('#') || line.starts_with(';') || line.is_empty() {
            continue;
        }
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim().to_uppercase();
            let value = line[eq_pos + 1..].trim().to_string();
            params.insert(key, value);
        }
    }

    if found { Some(params) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- numeric_to_level_filter -----------------------------------------------

    #[test]
    fn numeric_to_level_filter_mapping() {
        assert_eq!(numeric_to_level_filter(0), LevelFilter::OFF);
        assert_eq!(numeric_to_level_filter(1), LevelFilter::ERROR);
        assert_eq!(numeric_to_level_filter(2), LevelFilter::ERROR);
        assert_eq!(numeric_to_level_filter(3), LevelFilter::WARN);
        assert_eq!(numeric_to_level_filter(4), LevelFilter::INFO);
        assert_eq!(numeric_to_level_filter(5), LevelFilter::DEBUG);
        assert_eq!(numeric_to_level_filter(6), LevelFilter::TRACE);
    }

    #[test]
    fn numeric_to_level_filter_clamps_above_six() {
        assert_eq!(numeric_to_level_filter(7), LevelFilter::TRACE);
        assert_eq!(numeric_to_level_filter(255), LevelFilter::TRACE);
    }

    // -- parse_ini_section -----------------------------------------------------

    #[test]
    fn parse_driver_section() {
        let ini = "\
[Driver]
LogLevel = 5
LogPath = /tmp/snowflake_logs
LogFileSize = 20971520
LogFileCount = 10
";
        let params = parse_ini_section(ini, "Driver").unwrap();
        assert_eq!(params.get("LOGLEVEL").unwrap(), "5");
        assert_eq!(params.get("LOGPATH").unwrap(), "/tmp/snowflake_logs");
        assert_eq!(params.get("LOGFILESIZE").unwrap(), "20971520");
        assert_eq!(params.get("LOGFILECOUNT").unwrap(), "10");
    }

    #[test]
    fn parse_driver_section_case_insensitive() {
        let ini = "[driver]\nloglevel=3\n";
        let params = parse_ini_section(ini, "Driver").unwrap();
        assert_eq!(params.get("LOGLEVEL").unwrap(), "3");
    }

    #[test]
    fn parse_driver_section_not_found() {
        let ini = "[Other]\nLogLevel = 5\n";
        assert!(parse_ini_section(ini, "Driver").is_none());
    }

    #[test]
    fn parse_driver_section_skips_comments() {
        let ini = "\
[Driver]
# This is a comment
; This is also a comment

LogLevel = 4
";
        let params = parse_ini_section(ini, "Driver").unwrap();
        assert_eq!(params.len(), 1);
        assert_eq!(params.get("LOGLEVEL").unwrap(), "4");
    }

    #[test]
    fn parse_driver_section_stops_at_next_section() {
        let ini = "\
[Driver]
LogLevel = 5

[OtherSection]
LogLevel = 2
";
        let params = parse_ini_section(ini, "Driver").unwrap();
        assert_eq!(params.get("LOGLEVEL").unwrap(), "5");
        assert_eq!(params.len(), 1);
    }

    // -- apply_ini_params ------------------------------------------------------

    #[test]
    fn apply_all_logging_keys() {
        let mut params = HashMap::new();
        params.insert("LOGLEVEL".to_string(), "5".to_string());
        params.insert("LOGPATH".to_string(), "/var/log/snowflake".to_string());
        params.insert("LOGFILESIZE".to_string(), "5242880".to_string());
        params.insert("LOGFILECOUNT".to_string(), "3".to_string());

        let config = apply_ini_params(params);
        assert_eq!(config.log_level, LevelFilter::DEBUG);
        assert_eq!(config.log_path, Some(PathBuf::from("/var/log/snowflake")));
        assert_eq!(config.log_file_size, 5_242_880);
        assert_eq!(config.log_file_count, 3);
        assert!(config.enabled);
    }

    #[test]
    fn apply_level_zero_disables() {
        let mut params = HashMap::new();
        params.insert("LOGLEVEL".to_string(), "0".to_string());

        let config = apply_ini_params(params);
        assert_eq!(config.log_level, LevelFilter::OFF);
        assert!(!config.enabled);
    }

    #[test]
    fn apply_empty_params_returns_defaults() {
        let config = apply_ini_params(HashMap::new());
        let defaults = LoggingConfig::default();
        assert_eq!(config.log_level, defaults.log_level);
        assert_eq!(config.log_path, defaults.log_path);
        assert_eq!(config.log_file_size, defaults.log_file_size);
        assert_eq!(config.log_file_count, defaults.log_file_count);
    }

    #[test]
    fn apply_invalid_values_keeps_defaults() {
        let mut params = HashMap::new();
        params.insert("LOGLEVEL".to_string(), "not_a_number".to_string());
        params.insert("LOGFILESIZE".to_string(), "abc".to_string());
        params.insert("LOGFILECOUNT".to_string(), "-1".to_string());

        let config = apply_ini_params(params);
        let defaults = LoggingConfig::default();
        assert_eq!(config.log_level, defaults.log_level);
        assert_eq!(config.log_file_size, defaults.log_file_size);
        assert_eq!(config.log_file_count, defaults.log_file_count);
    }

    #[test]
    fn apply_empty_log_path_stays_none() {
        let mut params = HashMap::new();
        params.insert("LOGPATH".to_string(), String::new());

        let config = apply_ini_params(params);
        assert_eq!(config.log_path, None);
    }

    #[test]
    fn apply_unrecognized_keys_ignored() {
        let mut params = HashMap::new();
        params.insert("SOMETHING_ELSE".to_string(), "ignored".to_string());
        params.insert("LOGLEVEL".to_string(), "4".to_string());

        let config = apply_ini_params(params);
        assert_eq!(config.log_level, LevelFilter::INFO);
    }

    // -- ini_search_paths ------------------------------------------------------

    #[test]
    fn search_paths_includes_sf_odbc_ini_env() {
        let _guard = EnvGuard::set("SF_ODBC_INI", "/custom/path/sf.snowflake.ini");
        let paths = ini_search_paths();
        assert!(paths.contains(&PathBuf::from("/custom/path/sf.snowflake.ini")));
    }

    #[test]
    fn search_paths_includes_home_dotsnowflake() {
        let _guard = EnvGuard::set("HOME", "/tmp/fakehome");
        let paths = ini_search_paths();
        assert!(paths.contains(&PathBuf::from("/tmp/fakehome/.snowflake/sf.snowflake.ini")));
    }

    // -- read_config_from_paths (via temp file) --------------------------------

    #[test]
    fn read_config_from_ini_file() {
        let dir = tempfile::tempdir().unwrap();
        let ini_path = dir.path().join(INI_FILENAME);
        std::fs::write(
            &ini_path,
            "\
[Driver]
LogLevel = 6
LogPath = /tmp/test_logs
LogFileSize = 1048576
LogFileCount = 2
",
        )
        .unwrap();

        let config = read_config_from_paths(vec![ini_path]);

        assert_eq!(config.log_level, LevelFilter::TRACE);
        assert_eq!(config.log_path, Some(PathBuf::from("/tmp/test_logs")));
        assert_eq!(config.log_file_size, 1_048_576);
        assert_eq!(config.log_file_count, 2);
        assert!(config.enabled);
    }

    #[test]
    fn read_config_missing_ini_returns_defaults() {
        let config =
            read_config_from_paths(vec![PathBuf::from("/nonexistent/path/sf.snowflake.ini")]);
        let defaults = LoggingConfig::default();
        assert_eq!(config.log_level, defaults.log_level);
        assert_eq!(config.log_path, defaults.log_path);
    }

    #[test]
    fn read_config_ini_without_driver_section_returns_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let ini_path = dir.path().join(INI_FILENAME);
        std::fs::write(&ini_path, "[SomeOtherSection]\nKey = Value\n").unwrap();

        let config = read_config_from_paths(vec![ini_path]);
        let defaults = LoggingConfig::default();
        assert_eq!(config.log_level, defaults.log_level);
    }

    // -- test helpers ----------------------------------------------------------

    struct EnvGuard {
        key: String,
        prev: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &str, value: &str) -> Self {
            let prev = std::env::var(key).ok();
            unsafe { std::env::set_var(key, value) };
            Self {
                key: key.to_string(),
                prev,
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => unsafe { std::env::set_var(&self.key, v) },
                None => unsafe { std::env::remove_var(&self.key) },
            }
        }
    }
}
