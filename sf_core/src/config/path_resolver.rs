use super::{ConfigDirNotFoundSnafu, ConfigError};
use std::env;
use std::path::PathBuf;

/// Holds the paths to configuration files.
///
/// Fields are `Option<PathBuf>` so callers can specify only the paths they
/// care about. When a path is `None`, the corresponding file is not read
/// (no fallback to platform defaults). Use [`get_config_paths`] to obtain
/// a `ConfigPaths` with both paths resolved to their platform defaults.
#[derive(Debug, Clone)]
pub struct ConfigPaths {
    pub connections_file: Option<PathBuf>,
    pub config_file: Option<PathBuf>,
}

/// Default Snowflake home directory relative to the user's home, matching the
/// old Python driver's `sf_dirs.py` which defaults to `~/.snowflake/`.
const DEFAULT_SNOWFLAKE_HOME_SUFFIX: &str = ".snowflake";

/// Resolve Snowflake home directory.
///
/// Matches the old Python driver (`sf_dirs._resolve_platform_dirs`):
///   1. Read `SNOWFLAKE_HOME` env var, defaulting to `~/.snowflake/`.
///   2. Expand `~` to the user's home directory.
///   3. Return `Some(path)` only if the directory exists on disk.
pub fn get_snowflake_home() -> Option<PathBuf> {
    let raw = env::var("SNOWFLAKE_HOME").ok();
    resolve_snowflake_home(raw, dirs::home_dir())
}

fn resolve_snowflake_home(env_value: Option<String>, home_dir: Option<PathBuf>) -> Option<PathBuf> {
    let path = match env_value {
        Some(val) => expand_tilde(&val, home_dir.as_deref()),
        None => home_dir?.join(DEFAULT_SNOWFLAKE_HOME_SUFFIX),
    };
    path.exists().then_some(path)
}

/// Expand a leading `~` or `~/` to the user's home directory, mirroring
/// Python's `Path.expanduser()`.
fn expand_tilde(raw: &str, home_dir: Option<&std::path::Path>) -> PathBuf {
    if raw == "~" {
        return home_dir
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(raw));
    }
    if let Some(rest) = raw.strip_prefix("~/")
        && let Some(home) = home_dir
    {
        return home.join(rest);
    }
    PathBuf::from(raw)
}

/// Get the configuration file paths based on platform and environment.
///
/// Matches the old Python driver's resolution order:
///   1. If `SNOWFLAKE_HOME` (default `~/.snowflake/`) exists → use it.
///   2. Otherwise fall back to the platform config directory + `"snowflake"`,
///      equivalent to `platformdirs.PlatformDirs(appname="snowflake", appauthor=False).user_config_path`.
pub fn get_config_paths() -> Result<ConfigPaths, ConfigError> {
    resolve_config_paths(get_snowflake_home())
}

fn resolve_config_paths(snowflake_home: Option<PathBuf>) -> Result<ConfigPaths, ConfigError> {
    if let Some(snowflake_home) = snowflake_home {
        return Ok(ConfigPaths {
            connections_file: Some(snowflake_home.join("connections.toml")),
            config_file: Some(snowflake_home.join("config.toml")),
        });
    }

    let config_dir = platform_config_dir()?;

    Ok(ConfigPaths {
        connections_file: Some(config_dir.join("connections.toml")),
        config_file: Some(config_dir.join("config.toml")),
    })
}

/// Platform-specific config directory matching Python `platformdirs`
/// `PlatformDirs(appname="snowflake", appauthor=False).user_config_path`.
///
///   * Linux : `$XDG_CONFIG_HOME/snowflake`  or `~/.config/snowflake`
///   * macOS : `~/Library/Application Support/snowflake`
///   * Windows: `{FOLDERID_LocalAppData}\snowflake`  (Local, **not** Roaming)
fn platform_config_dir() -> Result<PathBuf, ConfigError> {
    // On Windows the old driver uses *Local* AppData (`platformdirs`
    // `user_config_path` with `appauthor=False`). `dirs::config_local_dir()`
    // returns exactly that.  On non-Windows platforms `config_local_dir()`
    // behaves identically to `config_dir()`.
    #[cfg(windows)]
    let base = dirs::config_local_dir();
    #[cfg(not(windows))]
    let base = dirs::config_dir();

    base.map(|d| d.join("snowflake"))
        .ok_or_else(|| ConfigDirNotFoundSnafu.build())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_resolve_config_paths_platform_fallback() {
        let paths = resolve_config_paths(None).unwrap();
        let connections_file = paths.connections_file.unwrap();
        let config_file = paths.config_file.unwrap();

        assert!(connections_file.to_string_lossy().contains("snowflake"));
        assert!(config_file.to_string_lossy().contains("snowflake"));
        assert!(
            connections_file
                .to_string_lossy()
                .ends_with("connections.toml")
        );
        assert!(config_file.to_string_lossy().ends_with("config.toml"));
    }

    #[test]
    fn test_resolve_config_paths_with_snowflake_home() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        let paths = resolve_config_paths(Some(temp_path.clone())).unwrap();
        let connections_file = paths.connections_file.unwrap();
        let config_file = paths.config_file.unwrap();

        assert!(connections_file.starts_with(&temp_path));
        assert!(config_file.starts_with(&temp_path));
        assert!(
            connections_file
                .to_string_lossy()
                .ends_with("connections.toml")
        );
        assert!(config_file.to_string_lossy().ends_with("config.toml"));
    }

    // --- resolve_snowflake_home tests ---

    #[test]
    fn test_explicit_env_existing_dir() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let home = dirs::home_dir();

        let result = resolve_snowflake_home(Some(temp_path.clone()), home);
        assert_eq!(result.unwrap().to_str().unwrap(), temp_path);
    }

    #[test]
    fn test_explicit_env_nonexistent_dir() {
        let home = dirs::home_dir();
        let result = resolve_snowflake_home(
            Some("/nonexistent/path/that/does/not/exist".to_string()),
            home,
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_no_env_uses_default_snowflake_home() {
        let temp_dir = TempDir::new().unwrap();
        let fake_home = temp_dir.path().to_path_buf();
        // Create ~/.snowflake inside the fake home
        let dot_snowflake = fake_home.join(DEFAULT_SNOWFLAKE_HOME_SUFFIX);
        std::fs::create_dir_all(&dot_snowflake).unwrap();

        let result = resolve_snowflake_home(None, Some(fake_home));
        assert_eq!(result.unwrap(), dot_snowflake);
    }

    #[test]
    fn test_no_env_default_dir_missing_returns_none() {
        let temp_dir = TempDir::new().unwrap();
        // fake home exists but has no .snowflake subdirectory
        let result = resolve_snowflake_home(None, Some(temp_dir.path().to_path_buf()));
        assert!(result.is_none());
    }

    #[test]
    fn test_no_env_no_home_returns_none() {
        let result = resolve_snowflake_home(None, None);
        assert!(result.is_none());
    }

    // --- expand_tilde tests ---

    #[test]
    fn test_expand_tilde_with_suffix() {
        let home = PathBuf::from("/home/testuser");
        let result = expand_tilde("~/my_config", Some(&home));
        assert_eq!(result, PathBuf::from("/home/testuser/my_config"));
    }

    #[test]
    fn test_expand_tilde_bare() {
        let home = PathBuf::from("/home/testuser");
        let result = expand_tilde("~", Some(&home));
        assert_eq!(result, PathBuf::from("/home/testuser"));
    }

    #[test]
    fn test_expand_tilde_no_home() {
        let result = expand_tilde("~/my_config", None);
        assert_eq!(result, PathBuf::from("~/my_config"));
    }

    #[test]
    fn test_expand_absolute_path_unchanged() {
        let home = PathBuf::from("/home/testuser");
        let result = expand_tilde("/opt/snowflake", Some(&home));
        assert_eq!(result, PathBuf::from("/opt/snowflake"));
    }

    #[test]
    fn test_tilde_env_value_expanded_and_checked() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("custom_snow");
        std::fs::create_dir_all(&target).unwrap();

        // Simulate SNOWFLAKE_HOME="~/custom_snow" with fake home = temp_dir
        let fake_home = temp_dir.path().to_path_buf();
        let result = resolve_snowflake_home(Some("~/custom_snow".to_string()), Some(fake_home));
        assert_eq!(result.unwrap(), target);
    }
}
