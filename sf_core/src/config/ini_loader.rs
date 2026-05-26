//! The wrapper crate supplies an ordered list of candidate paths and calls
//! [`load_ini_files`]; the first existing file wins, its top-level
//! `key = value` pairs are normalised to lowercase keys, and the result is
//! cached in a process-wide [`OnceLock`]. Subsystem extractors (logging today,
//! others later) then read the snapshot via [`get_ini_config`] and project
//! the entries they care about into their own typed configs.
//!
//! See [`crate::config::logging_config_loader`] for the logging projection.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::{ConfigError, IniAlreadyLoadedSnafu, IniParseSnafu};

/// Process-wide snapshot of `sf.odbc.ini` as raw key/value entries.
#[derive(Debug, Default, Clone)]
pub struct IniConfig {
    entries: HashMap<String, String>,
    source: Option<PathBuf>,
}

impl IniConfig {
    /// Path the snapshot was loaded from, if any. `None` means either the
    /// snapshot was built in memory (tests) or no candidate path existed.
    pub fn source(&self) -> Option<&Path> {
        self.source.as_deref()
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries
            .get(&key.to_ascii_lowercase())
            .map(String::as_str)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// Build a snapshot from raw INI text, bypassing the process-wide
    /// [`OnceLock`]. Intended for unit tests that need a fresh `IniConfig`
    /// without poisoning the global.
    pub fn from_ini_content(content: &str) -> Result<Self, ConfigError> {
        let ini = ini::Ini::load_from_str_noescape(content).map_err(|e| {
            IniParseSnafu {
                message: format!("failed to parse INI content: {e}"),
            }
            .build()
        })?;
        Ok(Self::from_section(ini.general_section(), None))
    }

    fn from_section(props: &ini::Properties, source: Option<PathBuf>) -> Self {
        let mut entries = HashMap::with_capacity(props.len());
        for (key, value) in props.iter() {
            entries.insert(key.to_ascii_lowercase(), value.to_string());
        }
        Self { entries, source }
    }
}

static INI_CONFIG: OnceLock<IniConfig> = OnceLock::new();

/// Load `sf.odbc.ini` from the first existing path in `paths` and cache the
/// resulting snapshot in the process-wide [`OnceLock`].
///
/// `paths` is walked in order. The first path that exists on disk is
/// permission-checked, parsed, and used to seed the snapshot; remaining
/// paths are ignored. If no candidate exists, an empty [`IniConfig`] is
/// still cached so subsequent [`get_ini_config`] calls return
/// `Some(&default)`.
///
/// The first successful call wins. Subsequent calls return
/// [`ConfigError::IniAlreadyLoaded`] without modifying the global; callers
/// that re-enter (e.g. the ODBC wrapper across multiple environment
/// allocations) typically treat that error as benign.
///
/// Permission or parse failures surface as `Err` and leave the global
/// uninitialised so the caller may try a different path list.
pub fn load_ini_files(paths: &[PathBuf]) -> Result<(), ConfigError> {
    let snapshot = read_first_existing(paths)?;
    INI_CONFIG
        .set(snapshot)
        .map_err(|_| IniAlreadyLoadedSnafu.build())
}

fn read_first_existing(paths: &[PathBuf]) -> Result<IniConfig, ConfigError> {
    for path in paths {
        if !path.exists() {
            continue;
        }
        super::toml_loader::check_file_permissions(path)?;
        let ini = ini::Ini::load_from_file_noescape(path).map_err(|e| match e {
            ini::Error::Io(io) => ConfigError::ConfigFileRead {
                path: path.display().to_string(),
                source: io,
                location: snafu::Location::new(file!(), line!(), 0),
            },
            ini::Error::Parse(p) => IniParseSnafu {
                message: format!("failed to parse {}: {p}", path.display()),
            }
            .build(),
        })?;
        return Ok(IniConfig::from_section(
            ini.general_section(),
            Some(path.clone()),
        ));
    }
    Ok(IniConfig::default())
}

/// Returns the process-wide INI snapshot, or `None` until [`load_ini_files`]
/// has been called successfully.
pub fn get_ini_config() -> Option<&'static IniConfig> {
    INI_CONFIG.get()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_ini_content_empty_is_empty_snapshot() {
        let ini = IniConfig::from_ini_content("").unwrap();
        assert!(ini.iter().next().is_none());
        assert!(ini.source().is_none());
    }

    #[test]
    fn from_ini_content_lowercases_keys() {
        let ini =
            IniConfig::from_ini_content("LogLevel=DEBUG\nDriverManagerEncoding=UTF-32\n").unwrap();
        assert_eq!(ini.get("loglevel"), Some("DEBUG"));
        assert_eq!(ini.get("LOGLEVEL"), Some("DEBUG"));
        assert_eq!(ini.get("drivermanagerencoding"), Some("UTF-32"));
    }

    #[test]
    fn from_ini_content_preserves_values_verbatim() {
        let ini = IniConfig::from_ini_content("LogPath=/var/log/snowflake\n").unwrap();
        assert_eq!(ini.get("logpath"), Some("/var/log/snowflake"));
    }

    #[test]
    fn from_ini_content_invalid_returns_ini_parse_error() {
        let err = IniConfig::from_ini_content("[unclosed\n").unwrap_err();
        assert!(matches!(err, ConfigError::IniParse { .. }), "got: {err:?}");
    }

    #[test]
    fn read_first_existing_skips_missing_paths_and_reads_first_present() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.ini");
        let present = dir.path().join("present.ini");
        std::fs::write(&present, "LogLevel=DEBUG\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&present, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        let other = dir.path().join("other.ini");
        std::fs::write(&other, "LogLevel=WARN\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&other, std::fs::Permissions::from_mode(0o600)).unwrap();
        }

        let ini = read_first_existing(&[missing.clone(), present.clone(), other.clone()]).unwrap();
        assert_eq!(ini.get("loglevel"), Some("DEBUG"));
        assert_eq!(ini.source(), Some(present.as_path()));
    }

    #[test]
    fn read_first_existing_empty_paths_returns_default_snapshot() {
        let ini = read_first_existing(&[]).unwrap();
        assert!(ini.iter().next().is_none());
        assert!(ini.source().is_none());
    }

    #[test]
    fn read_first_existing_no_existing_file_returns_default_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let ini =
            read_first_existing(&[dir.path().join("a.ini"), dir.path().join("b.ini")]).unwrap();
        assert!(ini.iter().next().is_none());
        assert!(ini.source().is_none());
    }

    #[cfg(unix)]
    #[test]
    fn read_first_existing_rejects_world_writable() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("insecure.ini");
        std::fs::write(&path, "LogLevel=INFO\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o666)).unwrap();

        let err = read_first_existing(&[path]).unwrap_err();
        assert!(
            matches!(err, ConfigError::InsecurePermissions { .. }),
            "got: {err:?}"
        );
    }
}
