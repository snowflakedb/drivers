use super::path_resolver::{ConfigPaths, get_config_paths};
use super::settings::Setting;
use super::toml_loader::load_toml_file;
use super::{ConfigError, ConnectionNotFoundSnafu};
use std::collections::HashMap;

/// Load configuration for a specific connection from TOML files
pub fn load_connection_config(
    connection_name: &str,
) -> Result<HashMap<String, Setting>, ConfigError> {
    let paths = get_config_paths()?;
    load_connection_config_with_paths(connection_name, &paths)
}

/// Load configuration for a specific connection using explicit config paths
pub fn load_connection_config_with_paths(
    connection_name: &str,
    paths: &ConfigPaths,
) -> Result<HashMap<String, Setting>, ConfigError> {
    let mut settings = HashMap::new();
    let empty_toml = toml::Value::Table(toml::map::Map::new());

    let config_toml = match &paths.config_file {
        Some(p) => load_toml_file(p)?,
        None => empty_toml.clone(),
    };

    if let Some(connections_section) = config_toml.get("connections").and_then(|v| v.as_table())
        && let Some(conn_config) = connections_section
            .get(connection_name)
            .and_then(|v| v.as_table())
    {
        for (key, value) in conn_config {
            if let Some(setting) = toml_value_to_setting(value) {
                settings.insert(key.clone(), setting);
            }
        }
    }

    let connections_toml = match &paths.connections_file {
        Some(p) => load_toml_file(p)?,
        None => empty_toml,
    };

    if let Some(conn_config) = connections_toml
        .get(connection_name)
        .and_then(|v| v.as_table())
    {
        for (key, value) in conn_config {
            if let Some(setting) = toml_value_to_setting(value) {
                settings.insert(key.clone(), setting);
            }
        }
    }

    if settings.is_empty() {
        return ConnectionNotFoundSnafu {
            name: connection_name,
        }
        .fail();
    }

    Ok(settings)
}

/// Load all connections from config files
pub fn load_all_connections() -> Result<HashMap<String, HashMap<String, Setting>>, ConfigError> {
    let paths = get_config_paths()?;
    load_all_connections_with_paths(&paths)
}

/// Load all connections using explicit config paths
pub fn load_all_connections_with_paths(
    paths: &ConfigPaths,
) -> Result<HashMap<String, HashMap<String, Setting>>, ConfigError> {
    let mut all_connections = HashMap::new();
    let empty_toml = toml::Value::Table(toml::map::Map::new());

    let config_toml = match &paths.config_file {
        Some(p) => load_toml_file(p)?,
        None => empty_toml.clone(),
    };
    if let Some(connections_section) = config_toml.get("connections").and_then(|v| v.as_table()) {
        for (conn_name, conn_config) in connections_section {
            if let Some(table) = conn_config.as_table() {
                let mut settings = HashMap::new();
                for (key, value) in table {
                    if let Some(setting) = toml_value_to_setting(value) {
                        settings.insert(key.clone(), setting);
                    }
                }
                all_connections.insert(conn_name.clone(), settings);
            }
        }
    }

    let connections_toml = match &paths.connections_file {
        Some(p) => load_toml_file(p)?,
        None => empty_toml,
    };
    if let Some(table) = connections_toml.as_table() {
        for (conn_name, conn_config) in table {
            if let Some(config_table) = conn_config.as_table() {
                let settings = all_connections
                    .entry(conn_name.clone())
                    .or_insert_with(HashMap::new);
                for (key, value) in config_table {
                    if let Some(setting) = toml_value_to_setting(value) {
                        settings.insert(key.clone(), setting);
                    }
                }
            }
        }
    }

    Ok(all_connections)
}

/// Convert a TOML value to a Setting
fn toml_value_to_setting(value: &toml::Value) -> Option<Setting> {
    match value {
        toml::Value::String(s) => Some(Setting::String(s.clone())),
        toml::Value::Integer(i) => Some(Setting::Int(*i)),
        toml::Value::Float(f) => Some(Setting::Double(*f)),
        toml::Value::Boolean(b) => Some(Setting::Bool(*b)),
        _ => None,
    }
}

/// Load a specific section from config.toml (not affected by connections.toml)
///
/// Supports both simple and nested sections:
/// - `load_config_section("log")` loads `[log]`
/// - `load_config_section("database.pool")` loads `[database.pool]`
///
/// Returns None if the section doesn't exist or if it's a connections section
pub fn load_config_section(
    section_name: &str,
) -> Result<Option<HashMap<String, Setting>>, ConfigError> {
    let paths = get_config_paths()?;
    load_config_section_with_paths(section_name, &paths)
}

/// Load a specific section from config.toml using explicit config paths
pub fn load_config_section_with_paths(
    section_name: &str,
    paths: &ConfigPaths,
) -> Result<Option<HashMap<String, Setting>>, ConfigError> {
    let config_toml = match &paths.config_file {
        Some(p) => load_toml_file(p)?,
        None => return Ok(None),
    };

    if section_name == "connections" || section_name.starts_with("connections.") {
        return Ok(None);
    }

    let path_parts: Vec<&str> = section_name.split('.').collect();
    let mut current_value = &config_toml;

    for part in path_parts {
        match current_value.get(part) {
            Some(value) => current_value = value,
            None => return Ok(None),
        }
    }

    if let Some(section_table) = current_value.as_table() {
        let mut settings = HashMap::new();
        for (key, value) in section_table {
            if let Some(setting) = toml_value_to_setting(value) {
                settings.insert(key.clone(), setting);
            }
        }
        return Ok(Some(settings));
    }

    Ok(None)
}

/// Load all sections from config files (including connections)
///
/// Returns a map of section names to their settings.
/// Connections are included under "connections.<name>" keys.
pub fn load_all_config_sections() -> Result<HashMap<String, HashMap<String, Setting>>, ConfigError>
{
    let paths = get_config_paths()?;
    load_all_config_sections_with_paths(&paths)
}

/// Load all sections from config files using explicit config paths.
///
/// Only reads files for which a path is provided (`Some`). When a path is
/// `None`, that file is skipped entirely — no fallback to platform defaults.
///
/// When a `connections_file` is present and contains at least one connection,
/// it **replaces** all connections from `config_file` rather than merging.
/// Root-level scalar values are returned under the empty-string key `""`.
pub fn load_all_config_sections_with_paths(
    paths: &ConfigPaths,
) -> Result<HashMap<String, HashMap<String, Setting>>, ConfigError> {
    let empty_toml = toml::Value::Table(toml::map::Map::new());
    let config_toml = match &paths.config_file {
        Some(p) => load_toml_file(p)?,
        None => empty_toml.clone(),
    };
    let mut all_sections = HashMap::new();

    if let Some(table) = config_toml.as_table() {
        for (section_name, section_value) in table {
            if section_name == "connections" {
                if let Some(connections_table) = section_value.as_table() {
                    for (conn_name, conn_value) in connections_table {
                        if let Some(conn_table) = conn_value.as_table() {
                            let mut settings = HashMap::new();
                            for (key, value) in conn_table {
                                if let Some(setting) = toml_value_to_setting(value) {
                                    settings.insert(key.clone(), setting);
                                }
                            }
                            all_sections.insert(format!("connections.{conn_name}"), settings);
                        }
                    }
                }
                continue;
            }

            if let Some(section_table) = section_value.as_table() {
                let mut settings = HashMap::new();
                for (key, value) in section_table {
                    if let Some(setting) = toml_value_to_setting(value) {
                        settings.insert(key.clone(), setting);
                    }
                }
                all_sections.insert(section_name.clone(), settings);
            } else if let Some(setting) = toml_value_to_setting(section_value) {
                all_sections
                    .entry(String::new())
                    .or_insert_with(HashMap::new)
                    .insert(section_name.clone(), setting);
            }
        }
    }

    let connections_toml = match &paths.connections_file {
        Some(p) => load_toml_file(p)?,
        None => empty_toml,
    };
    if let Some(table) = connections_toml.as_table() {
        if !table.is_empty() {
            // connections.toml replaces config.toml connections entirely.
            all_sections.retain(|k, _| !k.starts_with("connections."));
        }
        for (conn_name, conn_config) in table {
            if let Some(config_table) = conn_config.as_table() {
                let mut settings = HashMap::new();
                for (k, value) in config_table {
                    if let Some(setting) = toml_value_to_setting(value) {
                        settings.insert(k.clone(), setting);
                    }
                }
                all_sections.insert(format!("connections.{conn_name}"), settings);
            }
        }
    }

    Ok(all_sections)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::path_resolver::ConfigPaths;
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
    fn test_toml_value_to_setting() {
        let string_val = toml::Value::String("test".to_string());
        assert!(matches!(
            toml_value_to_setting(&string_val),
            Some(Setting::String(_))
        ));

        let int_val = toml::Value::Integer(42);
        assert!(matches!(
            toml_value_to_setting(&int_val),
            Some(Setting::Int(42))
        ));

        let float_val = toml::Value::Float(1.23);
        assert!(matches!(
            toml_value_to_setting(&float_val),
            Some(Setting::Double(_))
        ));

        let bool_val = toml::Value::Boolean(true);
        assert!(matches!(
            toml_value_to_setting(&bool_val),
            Some(Setting::Bool(true))
        ));
    }

    #[test]
    fn test_load_connection_config() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "connections.toml",
            r#"
[testconn]
account = "myaccount"
user = "myuser"
password = "mypass"
"#,
        );

        let result = load_connection_config_with_paths("testconn", &paths);
        assert!(result.is_ok());

        let settings = result.unwrap();
        assert!(matches!(settings.get("account"), Some(Setting::String(_))));
        assert!(matches!(settings.get("user"), Some(Setting::String(_))));
    }

    #[test]
    fn test_connection_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);

        let result = load_connection_config_with_paths("nonexistent", &paths);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_connections_toml_overrides_config_toml() {
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

        let result = load_connection_config_with_paths("testconn", &paths);
        assert!(result.is_ok());

        let settings = result.unwrap();
        if let Some(Setting::String(account)) = settings.get("account") {
            assert_eq!(account, "connections_account");
        } else {
            panic!("Expected account setting");
        }

        if let Some(Setting::String(user)) = settings.get("user") {
            assert_eq!(user, "config_user");
        } else {
            panic!("Expected user setting");
        }
    }

    #[test]
    fn test_load_all_connections() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "connections.toml",
            r#"
[conn1]
account = "account1"

[conn2]
account = "account2"
"#,
        );

        let result = load_all_connections_with_paths(&paths);
        assert!(result.is_ok());

        let all_conns = result.unwrap();
        assert_eq!(all_conns.len(), 2);
        assert!(all_conns.contains_key("conn1"));
        assert!(all_conns.contains_key("conn2"));
    }

    #[test]
    fn test_load_config_section() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "config.toml",
            r#"
[log]
level = "debug"
file = "/var/log/snowflake.log"

[connections.testconn]
account = "myaccount"
"#,
        );

        let result = load_config_section_with_paths("log", &paths);
        assert!(result.is_ok());
        let log_section = result.unwrap();
        assert!(log_section.is_some());

        let settings = log_section.unwrap();
        assert!(matches!(settings.get("level"), Some(Setting::String(_))));
        assert!(matches!(settings.get("file"), Some(Setting::String(_))));
    }

    #[test]
    fn test_load_config_section_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "config.toml",
            r#"
[log]
level = "info"
"#,
        );

        let result = load_config_section_with_paths("nonexistent", &paths);
        assert!(result.is_ok());
        let section = result.unwrap();
        assert!(section.is_none());
    }

    #[test]
    fn test_load_config_section_excludes_connections() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "config.toml",
            r#"
[connections.testconn]
account = "myaccount"
"#,
        );

        let result = load_config_section_with_paths("connections", &paths);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_load_all_config_sections() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "config.toml",
            r#"
[log]
level = "debug"
file = "/var/log/snowflake.log"

[proxy]
host = "proxy.example.com"
port = 8080

[connections.testconn]
account = "myaccount"
"#,
        );

        let result = load_all_config_sections_with_paths(&paths);
        assert!(result.is_ok());

        let sections = result.unwrap();
        assert_eq!(sections.len(), 3);
        assert!(sections.contains_key("log"));
        assert!(sections.contains_key("proxy"));
        assert!(sections.contains_key("connections.testconn"));

        let log_settings = sections.get("log").unwrap();
        assert!(matches!(
            log_settings.get("level"),
            Some(Setting::String(_))
        ));

        let proxy_settings = sections.get("proxy").unwrap();
        assert!(matches!(
            proxy_settings.get("host"),
            Some(Setting::String(_))
        ));

        let conn_settings = sections.get("connections.testconn").unwrap();
        assert!(matches!(
            conn_settings.get("account"),
            Some(Setting::String(_))
        ));
    }

    #[test]
    fn test_load_nested_section() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "config.toml",
            r#"
[database.connection]
timeout = 30
retry_count = 3

[database.pool]
max_size = 10
min_size = 2
"#,
        );

        let result = load_config_section_with_paths("database.connection", &paths);
        assert!(result.is_ok());
        let section = result.unwrap();
        assert!(section.is_some());

        let settings = section.unwrap();
        assert!(matches!(settings.get("timeout"), Some(Setting::Int(30))));
        assert!(matches!(settings.get("retry_count"), Some(Setting::Int(3))));

        let result2 = load_config_section_with_paths("database.pool", &paths);
        assert!(result2.is_ok());
        let section2 = result2.unwrap();
        assert!(section2.is_some());

        let settings2 = section2.unwrap();
        assert!(matches!(settings2.get("max_size"), Some(Setting::Int(10))));
        assert!(matches!(settings2.get("min_size"), Some(Setting::Int(2))));
    }

    #[test]
    fn test_load_deeply_nested_section() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "config.toml",
            r#"
[app.server.tls]
enabled = true
cert_path = "/etc/certs/server.crt"
"#,
        );

        let result = load_config_section_with_paths("app.server.tls", &paths);
        assert!(result.is_ok());
        let section = result.unwrap();
        assert!(section.is_some());

        let settings = section.unwrap();
        assert!(
            matches!(settings.get("enabled"), Some(Setting::Bool(true))),
            "Expected Setting::Bool(true) for TOML boolean, got {:?}",
            settings.get("enabled")
        );
    }

    #[test]
    fn test_load_nonexistent_nested_section() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "config.toml",
            r#"
[database.connection]
timeout = 30
"#,
        );

        let result = load_config_section_with_paths("database.pool", &paths);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        let result2 = load_config_section_with_paths("other.connection", &paths);
        assert!(result2.is_ok());
        assert!(result2.unwrap().is_none());
    }

    #[test]
    fn test_cannot_load_nested_connections_section() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "config.toml",
            r#"
[connections.testconn]
account = "myaccount"
"#,
        );

        let result = load_config_section_with_paths("connections.testconn", &paths);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_connections_toml_does_not_affect_other_sections() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "config.toml",
            r#"
[log]
level = "info"

[connections.testconn]
account = "config_account"
"#,
        );
        write_config(
            &temp_dir,
            "connections.toml",
            r#"
[testconn]
account = "connections_account"

[log]
level = "debug"
"#,
        );

        let result = load_config_section_with_paths("log", &paths);
        assert!(result.is_ok());
        let log_section = result.unwrap();
        assert!(log_section.is_some());

        let settings = log_section.unwrap();
        if let Some(Setting::String(level)) = settings.get("level") {
            assert_eq!(level, "info");
        } else {
            panic!("Expected level setting");
        }
    }

    #[test]
    fn test_connections_toml_replaces_config_toml_connections() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "config.toml",
            r#"
default_connection_name = "default"

[connections.default]
database = "db_for_test"
schema = "test_public"

[connections.full]
account = "dev_account"

[log]
level = "debug"
"#,
        );
        write_config(
            &temp_dir,
            "connections.toml",
            r#"
[default]
database = "overridden_database"
"#,
        );

        let result = load_all_config_sections_with_paths(&paths).unwrap();

        // Only the connections.toml connection should survive
        assert!(
            result.contains_key("connections.default"),
            "connections.toml connection must be present"
        );
        assert!(
            !result.contains_key("connections.full"),
            "config.toml-only connection must be removed"
        );

        let default_conn = result.get("connections.default").unwrap();
        assert!(matches!(
            default_conn.get("database"),
            Some(Setting::String(s)) if s == "overridden_database"
        ));
        // config.toml settings for the same connection are NOT merged
        assert!(
            !default_conn.contains_key("schema"),
            "config.toml settings must not leak into connections.toml connection"
        );

        // Non-connection sections from config.toml are preserved
        assert!(result.contains_key("log"));
    }

    #[test]
    fn test_root_level_values_use_empty_key() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "config.toml",
            r#"
default_connection_name = "default"

[connections.myconn]
account = "acct"
"#,
        );

        let result = load_all_config_sections_with_paths(&paths).unwrap();

        // Root-level scalars should be under the empty-string key
        let root = result
            .get("")
            .expect("root values must use empty-string key");
        assert!(matches!(
            root.get("default_connection_name"),
            Some(Setting::String(s)) if s == "default"
        ));
        assert!(!result.contains_key("_root"), "_root key must not appear");
    }

    #[test]
    fn test_empty_connections_toml_does_not_remove_config_connections() {
        let temp_dir = TempDir::new().unwrap();
        let paths = make_paths(&temp_dir);
        write_config(
            &temp_dir,
            "config.toml",
            r#"
[connections.myconn]
account = "acct"
"#,
        );
        write_config(&temp_dir, "connections.toml", "");

        let result = load_all_config_sections_with_paths(&paths).unwrap();
        assert!(
            result.contains_key("connections.myconn"),
            "config.toml connections should remain when connections.toml is empty"
        );
    }

    #[test]
    fn test_nonexistent_connections_toml_keeps_config_connections() {
        let temp_dir = TempDir::new().unwrap();
        let paths = ConfigPaths {
            config_file: Some(temp_dir.path().join("config.toml")),
            connections_file: Some(temp_dir.path().join("connections.toml")),
        };
        write_config(
            &temp_dir,
            "config.toml",
            r#"
[connections.myconn]
account = "acct"
"#,
        );
        // connections.toml does not exist on disk

        let result = load_all_config_sections_with_paths(&paths).unwrap();
        assert!(
            result.contains_key("connections.myconn"),
            "config.toml connections should remain when connections.toml does not exist"
        );
    }
}
