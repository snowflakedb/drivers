//! One-shot reader for `sf.odbc.ini`.
//!
//! The instance is created lazily by [`SfOdbcIni::global`] on first
//! access. The driver's bootstrap calls it during environment allocation
//! to seed the [`LogManager`]; the ODBC wrappers call it on the first
//! wide-string operation. Both observers see the same snapshot.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::logging::LoggingConfig;
use crate::logging::ini_config;

/// Process-wide snapshot of `sf.odbc.ini`.
///
/// Construction is fallible-by-default: any I/O or parse error in the load
/// path is surfaced via `eprintln!` (the [`LogManager`] is not yet wired up
/// when the snapshot is first built) and the field defaults are used so
/// the driver continues to function. The `path` field records which file,
/// if any, was actually read so diagnostics elsewhere can reference it.
///
/// [`LogManager`]: crate::logging::LogManager
pub struct SfOdbcIni {
    /// Path that was actually read, if any. Kept so future diagnostics can
    /// answer "where did this config come from?"; currently surfaced only
    /// to tests via [`SfOdbcIni::path`].
    #[allow(dead_code)]
    path: Option<PathBuf>,
    logging: LoggingConfig,
    /// Lowercased keys that are not part of the logging namespace. Other
    /// subsystems look these up via [`SfOdbcIni::raw_value`].
    /// Keeping the values untyped lets `sf_core` host the singleton
    /// without taking a dependency on every consumer's types.
    raw_values: HashMap<String, String>,
}

impl SfOdbcIni {
    /// Returns the process-global snapshot, loading and caching it on
    /// first access. Subsequent calls return the same instance.
    pub fn global() -> &'static SfOdbcIni {
        static INSTANCE: OnceLock<SfOdbcIni> = OnceLock::new();
        INSTANCE.get_or_init(Self::load)
    }

    /// Locate, permission-check, and parse `sf.odbc.ini`. Any failure
    /// falls back to defaults and is reported on `stderr` (we run before
    /// `LogManager::init`, so the `tracing` subscriber is not yet
    /// installed).
    fn load() -> Self {
        let Some(path) = ini_config::find_odbc_ini() else {
            return Self::defaults(None);
        };

        if let Err(e) = crate::config::toml_loader::check_file_permissions(&path) {
            eprintln!(
                "sf.odbc.ini at {} has insecure permissions ({e}); using defaults",
                path.display()
            );
            return Self::defaults(Some(path));
        }

        let ini = match ini::Ini::load_from_file_noescape(&path) {
            Ok(ini) => ini,
            Err(e) => {
                eprintln!(
                    "Failed to parse sf.odbc.ini at {}: {e}; using defaults",
                    path.display()
                );
                return Self::defaults(Some(path));
            }
        };

        let (logging, raw_values) = parse_section(ini.general_section(), Some(&path));
        Self {
            path: Some(path),
            logging,
            raw_values,
        }
    }

    fn defaults(path: Option<PathBuf>) -> Self {
        Self {
            path,
            logging: LoggingConfig::default(),
            raw_values: HashMap::new(),
        }
    }

    /// Path to the INI file that was actually read, if any. Currently
    /// surfaced only to tests; kept for future diagnostics.
    #[allow(dead_code)]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Logging configuration parsed from the INI. Defaults when the file
    /// was not found, was unreadable, or contained no logging keys. Used
    /// by the ODBC bootstrap to seed [`crate::logging::LogManager::init`].
    pub fn logging(&self) -> &LoggingConfig {
        &self.logging
    }

    /// Untyped value for a key outside the logging namespace.
    /// Returns `None` when the key was absent.
    /// Lookup is case-insensitive.
    pub fn raw_value(&self, key: &str) -> Option<&str> {
        self.raw_values
            .get(&key.to_ascii_lowercase())
            .map(String::as_str)
    }
}

/// Apply the section's keys to a fresh [`LoggingConfig`], collecting
/// everything not owned by logging into a `raw_values` map. Used by
/// [`SfOdbcIni::load`] (where `source` is the on-disk path for diagnostic
/// messages) and by tests (where `source` is `None`).
fn parse_section(
    props: &ini::Properties,
    source: Option<&Path>,
) -> (LoggingConfig, HashMap<String, String>) {
    let mut logging = LoggingConfig::default();
    let mut raw_values = HashMap::new();
    for (key, value) in props.iter() {
        match ini_config::apply_logging_key(key, value, &mut logging) {
            Ok(true) => {}
            Ok(false) => {
                raw_values.insert(key.to_ascii_lowercase(), value.to_string());
            }
            Err(e) => match source {
                Some(path) => eprintln!(
                    "Invalid value for `{key}` in sf.odbc.ini at {}: {e}; skipping key",
                    path.display()
                ),
                None => eprintln!("Invalid value for `{key}` in sf.odbc.ini: {e}; skipping key"),
            },
        }
    }
    (logging, raw_values)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build an `SfOdbcIni` directly from in-memory INI content, bypassing
    /// the global singleton. Mirrors the dispatching logic in [`load`] so
    /// the per-key behavior can be unit-tested without touching the
    /// process-global `OnceLock`.
    fn load_from_content(content: &str) -> SfOdbcIni {
        let ini = ini::Ini::load_from_str_noescape(content).expect("valid INI");
        let (logging, raw_values) = parse_section(ini.general_section(), None);
        SfOdbcIni {
            path: None,
            logging,
            raw_values,
        }
    }

    #[test]
    fn empty_ini_returns_defaults() {
        let ini = load_from_content("");
        assert!(ini.raw_value("DriverManagerEncoding").is_none());
        assert!(ini.logging().enabled);
        assert!(ini.logging().log_path.is_none());
    }

    #[test]
    fn dispatches_logging_keys_to_logging_config() {
        let ini = load_from_content("LogLevel=DEBUG\nLogFile=driver.log\n");
        assert_eq!(
            ini.logging().level,
            tracing::level_filters::LevelFilter::DEBUG
        );
        assert_eq!(ini.logging().log_file_name.as_deref(), Some("driver.log"));
        assert!(ini.raw_value("DriverManagerEncoding").is_none());
    }

    #[test]
    fn non_logging_keys_land_in_raw_values() {
        let ini = load_from_content("DriverManagerEncoding=UTF-32\n");
        assert_eq!(ini.raw_value("DriverManagerEncoding"), Some("UTF-32"));
        // Logging defaults are untouched.
        assert!(ini.logging().enabled);
    }

    #[test]
    fn combined_keys_route_to_their_subsystems() {
        let ini =
            load_from_content("LogLevel=WARN\nDriverManagerEncoding=UTF-16\nLogEnabled=false\n");
        assert_eq!(
            ini.logging().level,
            tracing::level_filters::LevelFilter::WARN
        );
        assert!(!ini.logging().enabled);
        assert_eq!(ini.raw_value("DriverManagerEncoding"), Some("UTF-16"));
    }

    #[test]
    fn raw_value_lookup_is_case_insensitive() {
        let ini = load_from_content("DriverManagerEncoding=UTF-32\n");
        for spelling in [
            "drivermanagerencoding",
            "DRIVERMANAGERENCODING",
            "DriverManagerEncoding",
            "driverManagerEncoding",
        ] {
            assert_eq!(
                ini.raw_value(spelling),
                Some("UTF-32"),
                "spelling `{spelling}` should be recognised"
            );
        }
    }

    #[test]
    fn raw_value_preserves_original_case_insensitive_storage() {
        // Keys are stored lowercased so foreign-subsystem lookups don't
        // depend on the INI author's capitalisation choices.
        let ini = load_from_content("DRIVERMANAGERENCODING=UTF-16\n");
        assert_eq!(ini.raw_value("DriverManagerEncoding"), Some("UTF-16"));
    }

    #[test]
    fn unknown_logging_value_is_dropped() {
        // The logging parser rejects the value, so the key is neither
        // applied to LoggingConfig nor stashed in raw_values.
        let ini = load_from_content("LogLevel=NOISY\n");
        assert_eq!(
            ini.logging().level,
            tracing::level_filters::LevelFilter::INFO
        );
        assert!(ini.raw_value("LogLevel").is_none());
    }

    #[test]
    fn load_from_missing_file_returns_defaults() {
        // Point SF_ODBC_INI at a non-existent file. Use the constructor
        // directly so we don't race with `global()`'s OnceLock.
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope.ini");
        temp_env::with_var("SF_ODBC_INI", Some(missing.to_str().unwrap()), || {
            let ini = SfOdbcIni::load();
            assert!(ini.path().is_none(), "find_odbc_ini should skip missing");
            assert!(ini.raw_value("DriverManagerEncoding").is_none());
            assert!(ini.logging().enabled);
        });
    }

    #[test]
    fn load_from_real_file_parses_both_subsystems() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sf.odbc.ini");
        std::fs::write(
            &path,
            "LogLevel=DEBUG\nLogPath=/var/log/sf\nDriverManagerEncoding=UTF-32\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        temp_env::with_var("SF_ODBC_INI", Some(path.to_str().unwrap()), || {
            let ini = SfOdbcIni::load();
            assert_eq!(ini.path(), Some(path.as_path()));
            assert_eq!(
                ini.logging().level,
                tracing::level_filters::LevelFilter::DEBUG
            );
            assert_eq!(
                ini.logging().log_path.as_deref(),
                Some(std::path::Path::new("/var/log/sf"))
            );
            assert_eq!(ini.raw_value("DriverManagerEncoding"), Some("UTF-32"));
        });
    }
}
