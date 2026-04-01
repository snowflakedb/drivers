use crate::config::ConfigError;
use crate::config::ParamStore;
use crate::config::config_manager;
use crate::config::param_names;
use crate::config::param_registry;
use crate::config::path_resolver::ConfigPaths;
use crate::config::settings::Setting;

/// Resolve final settings by merging explicit settings with file-based
/// config and registry defaults.
///
/// Precedence (highest to lowest):
/// 1. Explicit programmatic settings (from `SetOptions` / `SetOption*` RPCs)
/// 2. TOML file: `connections.toml` `[connection_name]` section
/// 3. TOML file: `config.toml` `[connections.connection_name]` section
/// 4. Registry defaults (`ParamDef::default`)
///
/// `explicit` contains values set via the programmatic API (already
/// alias-resolved and type-checked by `connection_set_options`).
///
/// If `connection_name` is present in `explicit`, file-based config is
/// loaded and merged underneath.
pub fn resolve(explicit: &ParamStore) -> Result<ParamStore, ConfigError> {
    let paths = crate::config::path_resolver::get_config_paths()?;
    resolve_with_paths(explicit, &paths)
}

/// Same as [`resolve`] but accepts explicit config file paths (for testing).
pub fn resolve_with_paths(
    explicit: &ParamStore,
    paths: &ConfigPaths,
) -> Result<ParamStore, ConfigError> {
    let mut merged = ParamStore::new();

    // Layer 4: Registry defaults (lowest priority)
    for param in param_registry::registry().all_params() {
        if let Some(default_fn) = param.default {
            merged.insert(param.canonical_name.to_owned(), default_fn());
        }
    }

    // Layer 3+2: TOML files (if connection_name is set)
    if let Some(Setting::String(name)) = explicit.get(param_names::CONNECTION_NAME) {
        let file_settings = config_manager::load_connection_config_with_paths(name, paths)?;
        for (k, v) in file_settings {
            merged.insert(k, v);
        }
    }

    // Layer 1: Explicit programmatic settings (highest priority)
    merged.extend_from(explicit);

    Ok(merged)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::param_names;
    use crate::config::path_resolver::ConfigPaths;
    use crate::config::settings::Setting;
    use std::fs;
    use tempfile::TempDir;

    fn make_paths(dir: &TempDir) -> ConfigPaths {
        ConfigPaths {
            config_file: Some(dir.path().join("config.toml")),
            connections_file: Some(dir.path().join("connections.toml")),
        }
    }

    fn write_config(dir: &TempDir, filename: &str, content: &str) {
        let path = dir.path().join(filename);
        fs::write(&path, content).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        }
    }

    #[test]
    fn explicit_settings_override_file_settings() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "connections.toml",
            r#"
[testconn]
account = "file_account"
user = "file_user"
"#,
        );

        let mut explicit = ParamStore::new();
        explicit.insert(
            "connection_name".to_owned(),
            Setting::String("testconn".to_owned()),
        );
        explicit.insert(
            "account".to_owned(),
            Setting::String("explicit_account".to_owned()),
        );

        let resolved = resolve_with_paths(&explicit, &paths).unwrap();

        if let Some(Setting::String(account)) = resolved.get(param_names::ACCOUNT) {
            assert_eq!(account, "explicit_account");
        } else {
            panic!("Expected account setting");
        }

        if let Some(Setting::String(user)) = resolved.get(param_names::USER) {
            assert_eq!(user, "file_user");
        } else {
            panic!("Expected user setting from file");
        }
    }

    #[test]
    fn file_settings_override_registry_defaults() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "connections.toml",
            r#"
[testconn]
account = "file_account"
protocol = "http"
"#,
        );

        let mut explicit = ParamStore::new();
        explicit.insert(
            "connection_name".to_owned(),
            Setting::String("testconn".to_owned()),
        );

        let resolved = resolve_with_paths(&explicit, &paths).unwrap();

        if let Some(Setting::String(protocol)) = resolved.get(param_names::PROTOCOL) {
            assert_eq!(protocol, "http");
        } else {
            panic!("Expected protocol setting");
        }
    }

    #[test]
    fn connections_toml_overrides_config_toml() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "config.toml",
            r#"
[connections.testconn]
account = "config_account"
user = "config_user"
"#,
        );
        write_config(
            &temp_dir,
            "connections.toml",
            r#"
[testconn]
account = "connections_account"
"#,
        );

        let mut explicit = ParamStore::new();
        explicit.insert(
            "connection_name".to_owned(),
            Setting::String("testconn".to_owned()),
        );

        let resolved = resolve_with_paths(&explicit, &paths).unwrap();

        if let Some(Setting::String(account)) = resolved.get(param_names::ACCOUNT) {
            assert_eq!(account, "connections_account");
        } else {
            panic!("Expected account setting");
        }

        if let Some(Setting::String(user)) = resolved.get(param_names::USER) {
            assert_eq!(user, "config_user");
        } else {
            panic!("Expected user setting");
        }
    }

    #[test]
    fn no_connection_name_uses_only_explicit_and_defaults() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "connections.toml",
            r#"
[testconn]
account = "file_account"
"#,
        );

        let mut explicit = ParamStore::new();
        explicit.insert(
            "account".to_owned(),
            Setting::String("explicit_account".to_owned()),
        );

        let resolved = resolve_with_paths(&explicit, &paths).unwrap();

        if let Some(Setting::String(account)) = resolved.get(param_names::ACCOUNT) {
            assert_eq!(account, "explicit_account");
        } else {
            panic!("Expected account setting");
        }

        // Registry default for protocol should be present
        if let Some(Setting::String(protocol)) = resolved.get(param_names::PROTOCOL) {
            assert_eq!(protocol, "https");
        } else {
            panic!("Expected default protocol setting");
        }
    }

    fn get_str(map: &ParamStore, key: crate::config::param_registry::ParamKey) -> Option<String> {
        match map.get(key) {
            Some(Setting::String(v)) => Some(v.clone()),
            _ => None,
        }
    }

    #[test]
    fn integration_full_round_trip() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "config.toml",
            r#"
[connections.myconn]
account = "config_acct"
user = "config_user"
warehouse = "config_wh"
"#,
        );
        write_config(
            &temp_dir,
            "connections.toml",
            r#"
[myconn]
account = "conn_acct"
database = "conn_db"
"#,
        );

        let mut explicit = ParamStore::new();
        explicit.insert(
            "connection_name".to_owned(),
            Setting::String("myconn".to_owned()),
        );
        explicit.insert(
            "account".to_owned(),
            Setting::String("explicit_acct".to_owned()),
        );

        let resolved = resolve_with_paths(&explicit, &paths).unwrap();

        assert_eq!(
            get_str(&resolved, param_names::ACCOUNT),
            Some("explicit_acct".to_owned())
        );
        assert_eq!(
            get_str(&resolved, param_names::DATABASE),
            Some("conn_db".to_owned())
        );
        assert_eq!(
            get_str(&resolved, param_names::USER),
            Some("config_user".to_owned())
        );
        assert_eq!(
            get_str(&resolved, param_names::WAREHOUSE),
            Some("config_wh".to_owned())
        );
        assert_eq!(
            get_str(&resolved, param_names::PROTOCOL),
            Some("https".to_owned())
        );
    }
}
