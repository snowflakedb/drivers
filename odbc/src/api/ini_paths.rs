use std::path::PathBuf;

/// Absolute path of the driver-installed default `sf.odbc.ini` on macOS.
#[cfg(target_os = "macos")]
const MACOS_INSTALLER_INI: &str = "/opt/snowflake/snowflakeodbcud/sf.odbc.ini";

/// Ordered candidates for `sf.odbc.ini` discovery:
///
/// 1. `SF_ODBC_INI` environment variable (explicit override; useful in
///    tests and CI),
/// 2. `<config_dir>/snowflake/sf.odbc.ini` (e.g. `~/Library/Application
///    Support/snowflake/sf.odbc.ini` on macOS, `~/.config/snowflake/sf.odbc.ini`
///    on Linux),
/// 3. `~/.snowflake/sf.odbc.ini`,
/// 4. (macOS only) `/opt/snowflake/snowflakeodbcud/sf.odbc.ini`
///
/// Paths that cannot be constructed (no home dir, no platform config dir)
/// are silently omitted.
pub fn default_paths() -> Vec<PathBuf> {
    let mut paths = Vec::with_capacity(4);
    if let Ok(env_path) = std::env::var("SF_ODBC_INI") {
        paths.push(PathBuf::from(env_path));
    }
    if let Some(config_dir) = dirs::config_dir() {
        paths.push(config_dir.join("snowflake").join("sf.odbc.ini"));
    }
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".snowflake").join("sf.odbc.ini"));
    }
    #[cfg(target_os = "macos")]
    paths.push(PathBuf::from(MACOS_INSTALLER_INI));
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_var_takes_priority() {
        temp_env::with_var("SF_ODBC_INI", Some("/tmp/explicit.ini"), || {
            let paths = default_paths();
            assert_eq!(
                paths.first().map(PathBuf::as_path),
                Some("/tmp/explicit.ini".as_ref())
            );
        });
    }

    #[test]
    fn env_var_unset_omits_first_entry() {
        temp_env::with_var_unset("SF_ODBC_INI", || {
            let paths = default_paths();
            for p in &paths {
                assert!(
                    !p.to_string_lossy().ends_with("explicit.ini"),
                    "env path should not appear when SF_ODBC_INI is unset"
                );
            }
        });
    }

    #[test]
    fn platform_and_home_paths_end_with_sf_odbc_ini() {
        temp_env::with_var_unset("SF_ODBC_INI", || {
            let paths = default_paths();
            assert!(
                paths
                    .iter()
                    .all(|p| p.file_name().and_then(|n| n.to_str()) == Some("sf.odbc.ini")),
                "all candidate paths must point at a file called sf.odbc.ini, got {paths:?}"
            );
        });
    }

    /// The macOS installer default must appear LAST so the per-user files
    /// (config_dir + ~/.snowflake) take precedence - anything else and a
    /// user-managed `DriverManagerEncoding=UTF-16` would be shadowed by
    /// the .pkg's `DriverManagerEncoding=UTF-32`.
    #[cfg(target_os = "macos")]
    #[test]
    fn macos_installer_path_is_last_and_pinned() {
        temp_env::with_var_unset("SF_ODBC_INI", || {
            let paths = default_paths();
            assert_eq!(
                paths.last().map(PathBuf::as_path),
                Some(std::path::Path::new(MACOS_INSTALLER_INI)),
                "installer default must be the last candidate; got {paths:?}"
            );
            assert!(
                paths
                    .iter()
                    .filter(|p| p.as_path() == std::path::Path::new(MACOS_INSTALLER_INI))
                    .count()
                    == 1,
                "installer default must appear exactly once; got {paths:?}"
            );
        });
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn non_macos_does_not_include_installer_path() {
        temp_env::with_var_unset("SF_ODBC_INI", || {
            let paths = default_paths();
            assert!(
                paths
                    .iter()
                    .all(|p| !p.to_string_lossy().contains("snowflakeodbcud")),
                "non-macOS builds must not probe the macOS installer ini; got {paths:?}"
            );
        });
    }
}
