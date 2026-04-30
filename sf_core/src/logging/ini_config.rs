use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ini::Ini;
use snafu::IntoError;
use tracing::level_filters::LevelFilter;

use super::error::{ConfigParseSnafu, InsecurePermissionsSnafu, IoSnafu, LogError};
use super::{LogRotation, LoggingConfig};
use crate::config::settings::Setting;

/// Parse an `sf.odbc.ini`-style INI file into a [`LoggingConfig`].
///
/// Supported keys (case-insensitive):
/// `LogLevel`, `LogPath`, `LogFile`, `LogMaxSize`, `LogMaxCount`, `LogEnabled`.
///
/// Checks file permissions before reading (rejects group/world-writable files
/// on Unix).
pub fn parse_ini_file(path: &Path) -> Result<LoggingConfig, LogError> {
    crate::config::toml_loader::check_file_permissions(path).map_err(|e| match e {
        crate::config::ConfigError::InsecurePermissions { path, reason, .. } => {
            InsecurePermissionsSnafu { path, reason }.build()
        }
        crate::config::ConfigError::ConfigFileRead { source, .. } => IoSnafu.into_error(source),
        other => IoSnafu.into_error(std::io::Error::other(other.to_string())),
    })?;

    let ini = Ini::load_from_file_noescape(path).map_err(|e| match e {
        ini::Error::Io(io_err) => IoSnafu.into_error(io_err),
        ini::Error::Parse(parse_err) => ConfigParseSnafu {
            message: format!("failed to parse {}: {parse_err}", path.display()),
        }
        .build(),
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
        match key.to_ascii_lowercase().as_str() {
            "loglevel" => config.level = parse_level(value)?,
            "logpath" => config.log_path = Some(PathBuf::from(value)),
            "logfile" => config.log_file_name = Some(value.to_string()),
            "logmaxsize" => config.max_file_size = Some(parse_u64(value)?),
            "logmaxcount" => config.max_file_count = Some(parse_u32(value)?),
            "logrotation" => config.rotation = parse_rotation(value)?,
            "logenabled" => config.enabled = parse_bool(value)?,
            other => eprintln!("ignoring unknown INI key: {other}"),
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

/// Overlay a [`LoggingConfig`] with values from the process environment.
///
/// Honored variables:
///   - `SF_ODBC_LOG_PATH` — path the appender writes to. May be either a
///     directory (existing behavior) or a full file path; if a file path is
///     given and its parent exists, the parent becomes `log_path` and the
///     basename becomes `log_file_name`. Setting this also forces
///     `enabled = true` so a caller who has an otherwise disabled INI can
///     still turn logging on from the environment.
///   - `SF_ODBC_LOG_LEVEL` — one of `OFF|ERROR|WARN|INFO|DEBUG|TRACE` (any
///     casing). Overrides `LogLevel` from the INI.
///   - `RUST_LOG` — convenience fallback for users already setting the
///     standard `tracing_subscriber` env var. We accept either a bare level
///     (`trace`) or a comma-separated directive list (`sf_core=trace,odbc=trace`)
///     and apply the first directive's level. Full per-crate filtering is not
///     wired up; INI / `SF_ODBC_LOG_LEVEL` remain the authoritative controls.
///
/// The env vars are opt-in: when a variable is unset or empty, the existing
/// config value is preserved.
pub fn apply_env_overrides(mut config: LoggingConfig) -> LoggingConfig {
    if let Ok(raw) = std::env::var("SF_ODBC_LOG_PATH")
        && !raw.is_empty()
    {
        let path = PathBuf::from(&raw);
        if path.is_dir() {
            config.log_path = Some(path);
        } else if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            config.log_path = Some(parent.to_path_buf());
            if let Some(name) = path.file_name() {
                config.log_file_name = Some(name.to_string_lossy().into_owned());
            }
        } else {
            // Bare filename with no directory component: treat as a file in
            // the current working directory.
            config.log_path = Some(PathBuf::from("."));
            config.log_file_name = Some(raw);
        }
        config.enabled = true;
    }

    if let Ok(level) = std::env::var("SF_ODBC_LOG_LEVEL")
        && let Ok(parsed) = parse_level(&level)
    {
        config.level = parsed;
    } else if let Ok(rust_log) = std::env::var("RUST_LOG") {
        // Walk the directive list and pick the first level we can parse.
        // Directive shapes accepted: "trace", "sf_core=trace", "warn,odbc=debug".
        for part in rust_log.split(',') {
            let candidate = part.rsplit('=').next().unwrap_or(part).trim();
            if let Ok(parsed) = parse_level(candidate) {
                config.level = parsed;
                break;
            }
        }
    }

    config
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

    // ---- INI content parsing ----

    #[test]
    fn parse_ini_content_all_keys() {
        let ini = "\
LogLevel=DEBUG
LogPath=/var/log/snowflake
LogFile=driver.log
LogMaxSize=1048576
LogMaxCount=5
LogEnabled=true
";
        let config = parse_ini_content(ini).unwrap();
        assert_eq!(config.level, LevelFilter::DEBUG);
        assert_eq!(
            config.log_path.unwrap(),
            PathBuf::from("/var/log/snowflake")
        );
        assert_eq!(config.log_file_name.unwrap(), "driver.log");
        assert_eq!(config.max_file_size.unwrap(), 1_048_576);
        assert_eq!(config.max_file_count.unwrap(), 5);
        assert!(config.enabled);
        assert!(!config.open_telemetry);
    }

    #[test]
    fn parse_ini_content_defaults_for_missing_keys() {
        let config = parse_ini_content("").unwrap();
        assert_eq!(config.level, LevelFilter::INFO);
        assert!(config.log_path.is_none());
        assert!(config.log_file_name.is_none());
        assert!(config.max_file_size.is_none());
        assert!(config.max_file_count.is_none());
        assert!(config.enabled);
        assert!(!config.open_telemetry);
    }

    #[test]
    fn parse_ini_content_skips_comments_and_blank_lines() {
        let ini = "\
# This is a comment
; Another comment

LogLevel=WARN
; non-indented comment
LogPath=/tmp
";
        let config = parse_ini_content(ini).unwrap();
        assert_eq!(config.level, LevelFilter::WARN);
        assert_eq!(config.log_path.unwrap(), PathBuf::from("/tmp"));
    }

    #[test]
    fn parse_ini_content_trims_whitespace() {
        let ini = "  LogLevel  =  TRACE  \n  LogFile = my_log.log  ";
        let config = parse_ini_content(ini).unwrap();
        assert_eq!(config.level, LevelFilter::TRACE);
        assert_eq!(config.log_file_name.unwrap(), "my_log.log");
    }

    #[test]
    fn parse_ini_content_ignores_unknown_keys() {
        let ini = "UnknownKey=value\nLogLevel=ERROR\nFoo=bar";
        let config = parse_ini_content(ini).unwrap();
        assert_eq!(config.level, LevelFilter::ERROR);
    }

    #[test]
    fn parse_ini_content_disabled() {
        let ini = "LogEnabled=false";
        let config = parse_ini_content(ini).unwrap();
        assert!(!config.enabled);
    }

    #[test]
    fn parse_ini_content_bool_variants() {
        for truthy in &["true", "1", "yes", "on", "True", "YES", "ON"] {
            let ini = format!("LogEnabled={truthy}");
            assert!(
                parse_ini_content(&ini).unwrap().enabled,
                "expected true for {truthy}"
            );
        }
        for falsy in &["false", "0", "no", "off", "False", "NO", "OFF"] {
            let ini = format!("LogEnabled={falsy}");
            assert!(
                !parse_ini_content(&ini).unwrap().enabled,
                "expected false for {falsy}"
            );
        }
    }

    #[test]
    fn parse_ini_content_invalid_bool() {
        let ini = "LogEnabled=maybe";
        let err = parse_ini_content(ini).unwrap_err();
        assert!(format!("{err:?}").contains("Invalid boolean"));
    }

    #[test]
    fn parse_ini_content_invalid_number() {
        let ini = "LogMaxSize=not_a_number";
        let err = parse_ini_content(ini).unwrap_err();
        assert!(format!("{err:?}").contains("Invalid number"));
    }

    #[test]
    fn parse_ini_content_invalid_level() {
        let ini = "LogLevel=VERBOSE";
        let err = parse_ini_content(ini).unwrap_err();
        assert!(format!("{err:?}").contains("Unknown log level"));
    }

    #[test]
    fn parse_ini_content_level_case_insensitive() {
        for (input, expected) in [
            ("off", LevelFilter::OFF),
            ("error", LevelFilter::ERROR),
            ("warn", LevelFilter::WARN),
            ("warning", LevelFilter::WARN),
            ("info", LevelFilter::INFO),
            ("debug", LevelFilter::DEBUG),
            ("trace", LevelFilter::TRACE),
            ("Info", LevelFilter::INFO),
            ("DEBUG", LevelFilter::DEBUG),
        ] {
            let ini = format!("LogLevel={input}");
            let config = parse_ini_content(&ini).unwrap();
            assert_eq!(config.level, expected, "level mismatch for input '{input}'");
        }
    }

    // ---- Case-insensitive INI keys ----

    #[test]
    fn parse_ini_content_lowercase_keys() {
        let ini = "\
loglevel=DEBUG
logpath=/var/log/snowflake
logfile=driver.log
logmaxsize=1048576
logmaxcount=5
logenabled=true
";
        let config = parse_ini_content(ini).unwrap();
        assert_eq!(config.level, LevelFilter::DEBUG);
        assert_eq!(
            config.log_path.unwrap(),
            PathBuf::from("/var/log/snowflake")
        );
        assert_eq!(config.log_file_name.unwrap(), "driver.log");
        assert_eq!(config.max_file_size.unwrap(), 1_048_576);
        assert_eq!(config.max_file_count.unwrap(), 5);
        assert!(config.enabled);
    }

    #[test]
    fn parse_ini_content_uppercase_keys() {
        let ini = "\
LOGLEVEL=TRACE
LOGPATH=/tmp/logs
LOGFILE=upper.log
LOGMAXSIZE=2097152
LOGMAXCOUNT=3
LOGENABLED=false
";
        let config = parse_ini_content(ini).unwrap();
        assert_eq!(config.level, LevelFilter::TRACE);
        assert_eq!(config.log_path.unwrap(), PathBuf::from("/tmp/logs"));
        assert_eq!(config.log_file_name.unwrap(), "upper.log");
        assert_eq!(config.max_file_size.unwrap(), 2_097_152);
        assert_eq!(config.max_file_count.unwrap(), 3);
        assert!(!config.enabled);
    }

    #[test]
    fn parse_ini_content_mixed_case_keys() {
        let ini = "logLevel=ERROR\nLogPATH=/tmp\nlogFILE=mixed.log\nLogMaxSIZE=512\n";
        let config = parse_ini_content(ini).unwrap();
        assert_eq!(config.level, LevelFilter::ERROR);
        assert_eq!(config.log_path.unwrap(), PathBuf::from("/tmp"));
        assert_eq!(config.log_file_name.unwrap(), "mixed.log");
        assert_eq!(config.max_file_size.unwrap(), 512);
    }

    // ---- INI file parsing ----

    #[test]
    fn parse_ini_file_valid() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sf.odbc.ini");
        std::fs::write(&path, "LogLevel=DEBUG\nLogPath=/tmp/logs\n").unwrap();
        let config = parse_ini_file(&path).unwrap();
        assert_eq!(config.level, LevelFilter::DEBUG);
        assert_eq!(config.log_path.unwrap(), PathBuf::from("/tmp/logs"));
    }

    #[test]
    fn parse_ini_file_missing_file() {
        let err = parse_ini_file(Path::new("/nonexistent/sf.odbc.ini")).unwrap_err();
        assert!(matches!(err, LogError::Io { .. }));
    }

    // ---- TOML section loading ----

    #[test]
    fn load_from_toml_section_all_fields() {
        let mut section = HashMap::new();
        section.insert("level".into(), Setting::String("DEBUG".into()));
        section.insert("path".into(), Setting::String("/var/log".into()));
        section.insert("file".into(), Setting::String("app.log".into()));
        section.insert("max_size".into(), Setting::Int(2_000_000));
        section.insert("max_count".into(), Setting::Int(3));
        section.insert("rotation".into(), Setting::String("DAILY".into()));
        section.insert("enabled".into(), Setting::Bool(false));
        section.insert("opentelemetry".into(), Setting::Bool(true));

        let config = load_from_toml_section(&section);
        assert_eq!(config.level, LevelFilter::DEBUG);
        assert_eq!(config.log_path.unwrap(), PathBuf::from("/var/log"));
        assert_eq!(config.log_file_name.unwrap(), "app.log");
        assert_eq!(config.max_file_size.unwrap(), 2_000_000);
        assert_eq!(config.max_file_count.unwrap(), 3);
        assert_eq!(config.rotation, LogRotation::Daily);
        assert!(!config.enabled);
        assert!(config.open_telemetry);
    }

    #[test]
    fn load_from_toml_section_empty_returns_defaults() {
        let section = HashMap::new();
        let config = load_from_toml_section(&section);
        assert_eq!(config.level, LevelFilter::INFO);
        assert!(config.log_path.is_none());
        assert!(config.log_file_name.is_none());
        assert!(config.max_file_size.is_none());
        assert!(config.max_file_count.is_none());
        assert!(config.enabled);
        assert!(!config.open_telemetry);
    }

    #[test]
    fn load_from_toml_section_invalid_level_keeps_default() {
        let mut section = HashMap::new();
        section.insert("level".into(), Setting::String("VERBOSE".into()));
        let config = load_from_toml_section(&section);
        assert_eq!(config.level, LevelFilter::INFO);
    }

    #[test]
    fn load_from_toml_section_negative_size_ignored() {
        let mut section = HashMap::new();
        section.insert("max_size".into(), Setting::Int(-100));
        section.insert("max_count".into(), Setting::Int(-1));
        let config = load_from_toml_section(&section);
        assert!(config.max_file_size.is_none());
        assert!(config.max_file_count.is_none());
    }

    #[test]
    fn load_from_toml_section_zero_size_ignored() {
        let mut section = HashMap::new();
        section.insert("max_size".into(), Setting::Int(0));
        section.insert("max_count".into(), Setting::Int(0));
        let config = load_from_toml_section(&section);
        assert!(config.max_file_size.is_none());
        assert!(config.max_file_count.is_none());
    }

    #[test]
    fn load_from_toml_section_wrong_type_ignored() {
        let mut section = HashMap::new();
        section.insert("level".into(), Setting::Int(42));
        section.insert("max_size".into(), Setting::String("big".into()));
        let config = load_from_toml_section(&section);
        assert_eq!(config.level, LevelFilter::INFO);
        assert!(config.max_file_size.is_none());
    }

    // ---- find_odbc_ini ----

    #[test]
    fn find_odbc_ini_via_env_var() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sf.odbc.ini");
        std::fs::write(&path, "LogLevel=DEBUG\n").unwrap();
        temp_env::with_var("SF_ODBC_INI", Some(path.to_str().unwrap()), || {
            let found = find_odbc_ini();
            assert_eq!(found.unwrap(), path);
        });
    }

    #[test]
    fn find_odbc_ini_env_var_nonexistent_file() {
        temp_env::with_var("SF_ODBC_INI", Some("/nonexistent/sf.odbc.ini"), || {
            // Should not return the non-existent env var path; may return None
            // or a platform path.  Just ensure it doesn't return the env var path.
            let found = find_odbc_ini();
            assert_ne!(
                found.as_deref(),
                Some(Path::new("/nonexistent/sf.odbc.ini"))
            );
        });
    }

    // ========================================================================
    // apply_env_overrides — env-var overlay on top of INI/TOML config
    // ========================================================================

    /// `SF_ODBC_LOG_PATH` pointing at a directory is taken verbatim as
    /// `log_path` and enables logging.
    #[test]
    fn env_override_path_directory_sets_log_path_and_enables() {
        let dir = tempfile::tempdir().unwrap();
        let dir_path = dir.path().to_path_buf();
        let dir_str = dir_path.to_string_lossy().into_owned();
        let cfg = LoggingConfig {
            enabled: false,
            ..LoggingConfig::default()
        };
        temp_env::with_vars(
            [
                ("SF_ODBC_LOG_PATH", Some(dir_str.as_str())),
                ("SF_ODBC_LOG_LEVEL", None),
                ("RUST_LOG", None),
            ],
            || {
                let out = apply_env_overrides(cfg);
                assert_eq!(out.log_path.as_deref(), Some(dir_path.as_path()));
                assert!(out.enabled);
            },
        );
    }

    /// A full file path is split into `log_path` (parent dir) + `log_file_name`.
    #[test]
    fn env_override_path_file_splits_parent_and_basename() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("odbc.log");
        let file_str = file_path.to_string_lossy().into_owned();
        temp_env::with_vars(
            [
                ("SF_ODBC_LOG_PATH", Some(file_str.as_str())),
                ("SF_ODBC_LOG_LEVEL", None),
                ("RUST_LOG", None),
            ],
            || {
                let out = apply_env_overrides(LoggingConfig::default());
                assert_eq!(out.log_path.as_deref(), Some(dir.path()));
                assert_eq!(out.log_file_name.as_deref(), Some("odbc.log"));
            },
        );
    }

    /// Bare filename (no directory component) falls back to "." so the
    /// appender writes to the process CWD.
    #[test]
    fn env_override_path_bare_filename_uses_cwd() {
        temp_env::with_vars(
            [
                ("SF_ODBC_LOG_PATH", Some("odbc.log")),
                ("SF_ODBC_LOG_LEVEL", None),
                ("RUST_LOG", None),
            ],
            || {
                let out = apply_env_overrides(LoggingConfig::default());
                assert_eq!(out.log_path.as_deref(), Some(Path::new(".")));
                assert_eq!(out.log_file_name.as_deref(), Some("odbc.log"));
            },
        );
    }

    /// An empty env var must be ignored — it should NOT clobber an existing
    /// INI-provided path.
    #[test]
    fn env_override_empty_path_is_ignored() {
        let cfg = LoggingConfig {
            log_path: Some(PathBuf::from("/from/ini")),
            ..LoggingConfig::default()
        };
        temp_env::with_vars(
            [
                ("SF_ODBC_LOG_PATH", Some("")),
                ("SF_ODBC_LOG_LEVEL", None),
                ("RUST_LOG", None),
            ],
            || {
                let out = apply_env_overrides(cfg);
                assert_eq!(out.log_path.as_deref(), Some(Path::new("/from/ini")));
            },
        );
    }

    /// `SF_ODBC_LOG_LEVEL` (any casing) overrides the level.
    #[test]
    fn env_override_level_parses_and_overrides() {
        temp_env::with_vars(
            [
                ("SF_ODBC_LOG_PATH", None),
                ("SF_ODBC_LOG_LEVEL", Some("trace")),
                ("RUST_LOG", None),
            ],
            || {
                let out = apply_env_overrides(LoggingConfig::default());
                assert_eq!(out.level, LevelFilter::TRACE);
            },
        );
    }

    /// `RUST_LOG` is honored only when `SF_ODBC_LOG_LEVEL` is absent.
    #[test]
    fn env_override_rust_log_fallback_parses_first_directive() {
        temp_env::with_vars(
            [
                ("SF_ODBC_LOG_PATH", None),
                ("SF_ODBC_LOG_LEVEL", None),
                ("RUST_LOG", Some("sf_core=debug,odbc=trace")),
            ],
            || {
                let out = apply_env_overrides(LoggingConfig::default());
                // First directive: sf_core=debug.
                assert_eq!(out.level, LevelFilter::DEBUG);
            },
        );
    }

    /// `SF_ODBC_LOG_LEVEL` wins over `RUST_LOG` when both are set.
    #[test]
    fn env_override_sf_level_wins_over_rust_log() {
        temp_env::with_vars(
            [
                ("SF_ODBC_LOG_PATH", None),
                ("SF_ODBC_LOG_LEVEL", Some("warn")),
                ("RUST_LOG", Some("trace")),
            ],
            || {
                let out = apply_env_overrides(LoggingConfig::default());
                assert_eq!(out.level, LevelFilter::WARN);
            },
        );
    }

    /// Unparseable level strings leave the config alone.
    #[test]
    fn env_override_invalid_level_is_ignored() {
        let cfg = LoggingConfig {
            level: LevelFilter::INFO,
            ..LoggingConfig::default()
        };
        temp_env::with_vars(
            [
                ("SF_ODBC_LOG_PATH", None),
                ("SF_ODBC_LOG_LEVEL", Some("bogus")),
                ("RUST_LOG", None),
            ],
            || {
                let out = apply_env_overrides(cfg);
                assert_eq!(out.level, LevelFilter::INFO);
            },
        );
    }
}
