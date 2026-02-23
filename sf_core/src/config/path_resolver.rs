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

/// Get the Snowflake home directory from SNOWFLAKE_HOME environment variable
pub fn get_snowflake_home() -> Option<PathBuf> {
    resolve_snowflake_home(env::var("SNOWFLAKE_HOME").ok())
}

fn resolve_snowflake_home(env_value: Option<String>) -> Option<PathBuf> {
    env_value.map(PathBuf::from).filter(|path| path.exists())
}

/// Get the configuration file paths based on platform and environment
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

    let config_dir = dirs::config_dir()
        .ok_or_else(|| ConfigDirNotFoundSnafu.build())?
        .join("snowflake");

    Ok(ConfigPaths {
        connections_file: Some(config_dir.join("connections.toml")),
        config_file: Some(config_dir.join("config.toml")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_resolve_config_paths_default() {
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

    #[test]
    fn test_resolve_snowflake_home_nonexistent() {
        let result =
            resolve_snowflake_home(Some("/nonexistent/path/that/does/not/exist".to_string()));
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_snowflake_home_existing() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();

        let result = resolve_snowflake_home(Some(temp_path.clone()));
        assert!(result.is_some());
        assert_eq!(result.unwrap().to_str().unwrap(), temp_path);
    }

    #[test]
    fn test_resolve_snowflake_home_not_set() {
        let result = resolve_snowflake_home(None);
        assert!(result.is_none());
    }
}
