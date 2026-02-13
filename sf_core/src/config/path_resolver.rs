use super::{ConfigDirNotFoundSnafu, ConfigError};
use std::env;
use std::path::PathBuf;

/// Holds the paths to configuration files
#[derive(Debug, Clone)]
pub struct ConfigPaths {
    pub connections_file: PathBuf,
    pub config_file: PathBuf,
}

/// Get the Snowflake home directory from SNOWFLAKE_HOME environment variable
pub fn get_snowflake_home() -> Option<PathBuf> {
    env::var("SNOWFLAKE_HOME")
        .ok()
        .map(PathBuf::from)
        .filter(|path| path.exists())
}

/// Get the configuration file paths based on platform and environment
pub fn get_config_paths() -> Result<ConfigPaths, ConfigError> {
    // First, check if SNOWFLAKE_HOME is set
    if let Some(snowflake_home) = get_snowflake_home() {
        return Ok(ConfigPaths {
            connections_file: snowflake_home.join("connections.toml"),
            config_file: snowflake_home.join("config.toml"),
        });
    }

    // Otherwise, use platform-specific defaults
    let config_dir = dirs::config_dir()
        .ok_or_else(|| ConfigDirNotFoundSnafu.build())?
        .join("snowflake");

    Ok(ConfigPaths {
        connections_file: config_dir.join("connections.toml"),
        config_file: config_dir.join("config.toml"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    #[test]
    fn test_get_config_paths_default() {
        // Remove SNOWFLAKE_HOME if set
        let _guard = env::var("SNOWFLAKE_HOME").ok().map(|_| {
            env::remove_var("SNOWFLAKE_HOME");
            ()
        });

        let paths = get_config_paths().unwrap();

        // Should contain 'snowflake' in the path
        assert!(
            paths
                .connections_file
                .to_string_lossy()
                .contains("snowflake")
        );
        assert!(paths.config_file.to_string_lossy().contains("snowflake"));

        // Should end with the correct file names
        assert!(
            paths
                .connections_file
                .to_string_lossy()
                .ends_with("connections.toml")
        );
        assert!(paths.config_file.to_string_lossy().ends_with("config.toml"));
    }

    #[test]
    fn test_snowflake_home_override() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();

        env::set_var("SNOWFLAKE_HOME", temp_path);

        let paths = get_config_paths().unwrap();

        assert!(paths.connections_file.starts_with(temp_path));
        assert!(paths.config_file.starts_with(temp_path));
        assert!(
            paths
                .connections_file
                .to_string_lossy()
                .ends_with("connections.toml")
        );
        assert!(paths.config_file.to_string_lossy().ends_with("config.toml"));

        // Clean up
        env::remove_var("SNOWFLAKE_HOME");
    }

    #[test]
    fn test_snowflake_home_nonexistent() {
        env::set_var("SNOWFLAKE_HOME", "/nonexistent/path/that/does/not/exist");

        let snowflake_home = get_snowflake_home();

        // Should return None since path doesn't exist
        assert!(snowflake_home.is_none());

        // get_config_paths should fall back to default
        let paths = get_config_paths().unwrap();
        assert!(
            paths
                .connections_file
                .to_string_lossy()
                .contains("snowflake")
        );

        // Clean up
        env::remove_var("SNOWFLAKE_HOME");
    }

    #[test]
    fn test_get_snowflake_home_set() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();

        env::set_var("SNOWFLAKE_HOME", temp_path);

        let snowflake_home = get_snowflake_home();
        assert!(snowflake_home.is_some());
        assert_eq!(snowflake_home.unwrap().to_str().unwrap(), temp_path);

        // Clean up
        env::remove_var("SNOWFLAKE_HOME");
    }

    #[test]
    fn test_get_snowflake_home_not_set() {
        // Ensure SNOWFLAKE_HOME is not set
        let _guard = env::var("SNOWFLAKE_HOME").ok().map(|_| {
            env::remove_var("SNOWFLAKE_HOME");
            ()
        });

        let snowflake_home = get_snowflake_home();
        assert!(snowflake_home.is_none());
    }
}
