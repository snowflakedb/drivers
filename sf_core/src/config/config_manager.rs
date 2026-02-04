use super::path_resolver::get_config_paths;
use super::settings::Setting;
use super::toml_loader::load_toml_file;
use super::{ConfigError, ConnectionNotFoundSnafu};
use std::collections::HashMap;
use std::env;

/// Load configuration for a specific connection from TOML files
pub fn load_connection_config(connection_name: &str) -> Result<HashMap<String, Setting>, ConfigError> {
    let paths = get_config_paths();
    let mut settings = HashMap::new();

    // Load config.toml first (lower precedence)
    let config_toml = load_toml_file(&paths.config_file)?;

    // Check for [connections.connection_name] section in config.toml
    if let Some(connections_section) = config_toml.get("connections").and_then(|v| v.as_table()) {
        if let Some(conn_config) = connections_section.get(connection_name).and_then(|v| v.as_table()) {
            for (key, value) in conn_config {
                if let Some(setting) = toml_value_to_setting(value) {
                    settings.insert(key.clone(), setting);
                }
            }
        }
    }

    // Load connections.toml (higher precedence - overrides config.toml)
    let connections_toml = load_toml_file(&paths.connections_file)?;

    if let Some(conn_config) = connections_toml.get(connection_name).and_then(|v| v.as_table()) {
        for (key, value) in conn_config {
            if let Some(setting) = toml_value_to_setting(value) {
                settings.insert(key.clone(), setting);
            }
        }
    }

    // If no configuration was found, return error
    if settings.is_empty() {
        return ConnectionNotFoundSnafu {
            name: connection_name,
        }
        .fail();
    }

    // Apply environment variable overrides
    apply_env_overrides(&mut settings);

    Ok(settings)
}

/// Load all connections from config files
pub fn load_all_connections() -> Result<HashMap<String, HashMap<String, Setting>>, ConfigError> {
    let paths = get_config_paths();
    let mut all_connections = HashMap::new();

    // Load from config.toml
    let config_toml = load_toml_file(&paths.config_file)?;
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

    // Load from connections.toml (overrides config.toml)
    let connections_toml = load_toml_file(&paths.connections_file)?;
    if let Some(table) = connections_toml.as_table() {
        for (conn_name, conn_config) in table {
            if let Some(config_table) = conn_config.as_table() {
                let mut settings = HashMap::new();
                for (key, value) in config_table {
                    if let Some(setting) = toml_value_to_setting(value) {
                        settings.insert(key.clone(), setting);
                    }
                }
                all_connections.insert(conn_name.clone(), settings);
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
        toml::Value::Boolean(b) => Some(Setting::String(b.to_string())),
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
pub fn load_config_section(section_name: &str) -> Result<Option<HashMap<String, Setting>>, ConfigError> {
    let paths = get_config_paths();
    let config_toml = load_toml_file(&paths.config_file)?;

    // Block access to connections section (and nested connections sections)
    if section_name == "connections" || section_name.starts_with("connections.") {
        // Connections should be loaded via load_connection_config or load_all_connections
        return Ok(None);
    }

    // Navigate to the nested section by splitting on '.'
    let path_parts: Vec<&str> = section_name.split('.').collect();
    let mut current_value = &config_toml;

    for part in path_parts {
        match current_value.get(part) {
            Some(value) => current_value = value,
            None => return Ok(None), // Section doesn't exist
        }
    }

    // Convert the final table to settings
    if let Some(section_table) = current_value.as_table() {
        let mut settings = HashMap::new();
        for (key, value) in section_table {
            if let Some(setting) = toml_value_to_setting(value) {
                settings.insert(key.clone(), setting);
            }
        }
        return Ok(Some(settings));
    }

    // Not a table, can't convert to settings
    Ok(None)
}

/// Load all sections from config files (including connections)
///
/// Returns a map of section names to their settings.
/// Connections are included under "connections.<name>" keys.
/// Environment variable overrides are applied automatically:
/// - For connections: SNOWFLAKE_<KEY> (e.g., SNOWFLAKE_ACCOUNT)
/// - For other sections: SNOWFLAKE_<SECTION>_<KEY> (e.g., SNOWFLAKE_LOG_LEVEL)
///
/// Use `apply_env_overrides` parameter to control whether env vars are applied.
pub fn load_all_config_sections_with_options(
    apply_env_overrides_flag: bool,
) -> Result<HashMap<String, HashMap<String, Setting>>, ConfigError> {
    let paths = get_config_paths();
    let config_toml = load_toml_file(&paths.config_file)?;
    let mut all_sections = HashMap::new();

    if let Some(table) = config_toml.as_table() {
        for (section_name, section_value) in table {
            // Handle connections section specially - flatten to "connections.<name>"
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
                            all_sections.insert(format!("connections.{}", conn_name), settings);
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
            }
        }
    }

    // Also load connections from connections.toml (higher precedence)
    let connections_toml = load_toml_file(&paths.connections_file)?;
    if let Some(table) = connections_toml.as_table() {
        for (conn_name, conn_config) in table {
            if let Some(config_table) = conn_config.as_table() {
                let key = format!("connections.{}", conn_name);
                // Get existing settings or create new
                let settings = all_sections.entry(key).or_insert_with(HashMap::new);
                for (k, value) in config_table {
                    if let Some(setting) = toml_value_to_setting(value) {
                        settings.insert(k.clone(), setting);
                    }
                }
            }
        }
    }

    // Apply environment variable overrides to all sections if requested
    if apply_env_overrides_flag {
        for (section_name, settings) in all_sections.iter_mut() {
            apply_env_overrides_for_section(section_name, settings);
        }
    }

    Ok(all_sections)
}

/// Load all sections from config files with env overrides applied (default behavior)
pub fn load_all_config_sections() -> Result<HashMap<String, HashMap<String, Setting>>, ConfigError> {
    load_all_config_sections_with_options(true)
}

/// Apply environment variable overrides to settings based on section name
///
/// For connections (section starts with "connections."):
///   - Uses SNOWFLAKE_<KEY> format (e.g., SNOWFLAKE_ACCOUNT)
/// For other sections:
///   - Uses SNOWFLAKE_<SECTION>_<KEY> format (e.g., SNOWFLAKE_LOG_LEVEL)
fn apply_env_overrides_for_section(section_name: &str, settings: &mut HashMap<String, Setting>) {
    let is_connection = section_name.starts_with("connections.");

    for key in settings.keys().cloned().collect::<Vec<_>>() {
        let env_key = if is_connection {
            // For connections: SNOWFLAKE_<KEY>
            format!("SNOWFLAKE_{}", key.to_uppercase())
        } else {
            // For other sections: SNOWFLAKE_<SECTION>_<KEY>
            // Convert section.name to SECTION_NAME
            let section_upper = section_name.replace('.', "_").to_uppercase();
            format!("SNOWFLAKE_{}_{}", section_upper, key.to_uppercase())
        };

        if let Ok(env_value) = env::var(&env_key) {
            settings.insert(key, Setting::String(env_value));
        }
    }
}

/// Apply environment variable overrides to settings (legacy, for load_connection_config)
fn apply_env_overrides(settings: &mut HashMap<String, Setting>) {
    for key in settings.keys().cloned().collect::<Vec<_>>() {
        let env_key = format!("SNOWFLAKE_{}", key.to_uppercase());
        if let Ok(env_value) = env::var(&env_key) {
            settings.insert(key, Setting::String(env_value));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_toml_value_to_setting() {
        let string_val = toml::Value::String("test".to_string());
        assert!(matches!(toml_value_to_setting(&string_val), Some(Setting::String(_))));

        let int_val = toml::Value::Integer(42);
        assert!(matches!(toml_value_to_setting(&int_val), Some(Setting::Int(42))));

        let float_val = toml::Value::Float(3.14);
        assert!(matches!(toml_value_to_setting(&float_val), Some(Setting::Double(_))));

        let bool_val = toml::Value::Boolean(true);
        if let Some(Setting::String(s)) = toml_value_to_setting(&bool_val) {
            assert_eq!(s, "true");
        } else {
            panic!("Expected String setting");
        }
    }

    #[test]
    fn test_load_connection_config() {
        let temp_dir = TempDir::new().unwrap();
        env::set_var("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

        let connections_file = temp_dir.path().join("connections.toml");
        let content = r#"
[testconn]
account = "myaccount"
user = "myuser"
password = "mypass"
"#;
        fs::write(&connections_file, content).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&connections_file, fs::Permissions::from_mode(0o600)).unwrap();
        }

        let result = load_connection_config("testconn");
        assert!(result.is_ok());

        let settings = result.unwrap();
        assert!(matches!(settings.get("account"), Some(Setting::String(_))));
        assert!(matches!(settings.get("user"), Some(Setting::String(_))));

        env::remove_var("SNOWFLAKE_HOME");
    }

    #[test]
    fn test_connection_not_found() {
        let temp_dir = TempDir::new().unwrap();
        env::set_var("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

        let result = load_connection_config("nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));

        env::remove_var("SNOWFLAKE_HOME");
    }

    #[test]
    fn test_connections_toml_overrides_config_toml() {
        let temp_dir = TempDir::new().unwrap();
        env::set_var("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

        let config_file = temp_dir.path().join("config.toml");
        let config_content = r#"
[connections.testconn]
account = "config_account"
user = "config_user"
"#;
        fs::write(&config_file, config_content).unwrap();

        let connections_file = temp_dir.path().join("connections.toml");
        let connections_content = r#"
[testconn]
account = "connections_account"
"#;
        fs::write(&connections_file, connections_content).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&config_file, fs::Permissions::from_mode(0o600)).unwrap();
            fs::set_permissions(&connections_file, fs::Permissions::from_mode(0o600)).unwrap();
        }

        let result = load_connection_config("testconn");
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

        env::remove_var("SNOWFLAKE_HOME");
    }

    #[test]
    fn test_env_override() {
        let temp_dir = TempDir::new().unwrap();
        env::set_var("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());
        env::set_var("SNOWFLAKE_ACCOUNT", "env_account");

        let connections_file = temp_dir.path().join("connections.toml");
        let content = r#"
[testconn]
account = "file_account"
user = "testuser"
"#;
        fs::write(&connections_file, content).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&connections_file, fs::Permissions::from_mode(0o600)).unwrap();
        }

        let result = load_connection_config("testconn");
        assert!(result.is_ok());

        let settings = result.unwrap();
        if let Some(Setting::String(account)) = settings.get("account") {
            assert_eq!(account, "env_account");
        } else {
            panic!("Expected account setting");
        }

        env::remove_var("SNOWFLAKE_HOME");
        env::remove_var("SNOWFLAKE_ACCOUNT");
    }

    #[test]
    fn test_load_all_connections() {
        let temp_dir = TempDir::new().unwrap();
        env::set_var("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

        let connections_file = temp_dir.path().join("connections.toml");
        let content = r#"
[conn1]
account = "account1"

[conn2]
account = "account2"
"#;
        fs::write(&connections_file, content).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&connections_file, fs::Permissions::from_mode(0o600)).unwrap();
        }

        let result = load_all_connections();
        assert!(result.is_ok());

        let all_conns = result.unwrap();
        assert_eq!(all_conns.len(), 2);
        assert!(all_conns.contains_key("conn1"));
        assert!(all_conns.contains_key("conn2"));

        env::remove_var("SNOWFLAKE_HOME");
    }

    #[test]
    fn test_load_config_section() {
        let temp_dir = TempDir::new().unwrap();
        env::set_var("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

        let config_file = temp_dir.path().join("config.toml");
        let content = r#"
[log]
level = "debug"
file = "/var/log/snowflake.log"

[connections.testconn]
account = "myaccount"
"#;
        fs::write(&config_file, content).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&config_file, fs::Permissions::from_mode(0o600)).unwrap();
        }

        // Load log section
        let result = load_config_section("log");
        assert!(result.is_ok());
        let log_section = result.unwrap();
        assert!(log_section.is_some());

        let settings = log_section.unwrap();
        assert!(matches!(settings.get("level"), Some(Setting::String(_))));
        assert!(matches!(settings.get("file"), Some(Setting::String(_))));

        env::remove_var("SNOWFLAKE_HOME");
    }

    #[test]
    fn test_load_config_section_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        env::set_var("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

        let config_file = temp_dir.path().join("config.toml");
        let content = r#"
[log]
level = "info"
"#;
        fs::write(&config_file, content).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&config_file, fs::Permissions::from_mode(0o600)).unwrap();
        }

        // Try to load non-existent section
        let result = load_config_section("nonexistent");
        assert!(result.is_ok());
        let section = result.unwrap();
        assert!(section.is_none());

        env::remove_var("SNOWFLAKE_HOME");
    }

    #[test]
    fn test_load_config_section_excludes_connections() {
        let temp_dir = TempDir::new().unwrap();
        env::set_var("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

        let config_file = temp_dir.path().join("config.toml");
        let content = r#"
[connections.testconn]
account = "myaccount"
"#;
        fs::write(&config_file, content).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&config_file, fs::Permissions::from_mode(0o600)).unwrap();
        }

        // Should return None for connections section
        let result = load_config_section("connections");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        env::remove_var("SNOWFLAKE_HOME");
    }

    #[test]
    fn test_load_all_config_sections() {
        let temp_dir = TempDir::new().unwrap();
        env::set_var("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

        let config_file = temp_dir.path().join("config.toml");
        let content = r#"
[log]
level = "debug"
file = "/var/log/snowflake.log"

[proxy]
host = "proxy.example.com"
port = 8080

[connections.testconn]
account = "myaccount"
"#;
        fs::write(&config_file, content).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&config_file, fs::Permissions::from_mode(0o600)).unwrap();
        }

        let result = load_all_config_sections();
        assert!(result.is_ok());

        let sections = result.unwrap();
        // Should have log, proxy, and connections.testconn
        assert_eq!(sections.len(), 3);
        assert!(sections.contains_key("log"));
        assert!(sections.contains_key("proxy"));
        assert!(sections.contains_key("connections.testconn"));

        // Verify log section content
        let log_settings = sections.get("log").unwrap();
        assert!(matches!(log_settings.get("level"), Some(Setting::String(_))));

        // Verify proxy section content
        let proxy_settings = sections.get("proxy").unwrap();
        assert!(matches!(proxy_settings.get("host"), Some(Setting::String(_))));

        // Verify connection section content
        let conn_settings = sections.get("connections.testconn").unwrap();
        assert!(matches!(conn_settings.get("account"), Some(Setting::String(_))));

        env::remove_var("SNOWFLAKE_HOME");
    }

    #[test]
    fn test_load_nested_section() {
        let temp_dir = TempDir::new().unwrap();
        env::set_var("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

        let config_file = temp_dir.path().join("config.toml");
        let content = r#"
[database.connection]
timeout = 30
retry_count = 3

[database.pool]
max_size = 10
min_size = 2
"#;
        fs::write(&config_file, content).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&config_file, fs::Permissions::from_mode(0o600)).unwrap();
        }

        // Load nested section using dotted path
        let result = load_config_section("database.connection");
        assert!(result.is_ok());
        let section = result.unwrap();
        assert!(section.is_some());

        let settings = section.unwrap();
        assert!(matches!(settings.get("timeout"), Some(Setting::Int(30))));
        assert!(matches!(settings.get("retry_count"), Some(Setting::Int(3))));

        // Load another nested section
        let result2 = load_config_section("database.pool");
        assert!(result2.is_ok());
        let section2 = result2.unwrap();
        assert!(section2.is_some());

        let settings2 = section2.unwrap();
        assert!(matches!(settings2.get("max_size"), Some(Setting::Int(10))));
        assert!(matches!(settings2.get("min_size"), Some(Setting::Int(2))));

        env::remove_var("SNOWFLAKE_HOME");
    }

    #[test]
    fn test_load_deeply_nested_section() {
        let temp_dir = TempDir::new().unwrap();
        env::set_var("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

        let config_file = temp_dir.path().join("config.toml");
        let content = r#"
[app.server.tls]
enabled = true
cert_path = "/etc/certs/server.crt"
"#;
        fs::write(&config_file, content).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&config_file, fs::Permissions::from_mode(0o600)).unwrap();
        }

        // Load deeply nested section
        let result = load_config_section("app.server.tls");
        assert!(result.is_ok());
        let section = result.unwrap();
        assert!(section.is_some());

        let settings = section.unwrap();
        if let Some(Setting::String(enabled)) = settings.get("enabled") {
            assert_eq!(enabled, "true");
        } else {
            panic!("Expected enabled setting");
        }

        env::remove_var("SNOWFLAKE_HOME");
    }

    #[test]
    fn test_load_nonexistent_nested_section() {
        let temp_dir = TempDir::new().unwrap();
        env::set_var("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

        let config_file = temp_dir.path().join("config.toml");
        let content = r#"
[database.connection]
timeout = 30
"#;
        fs::write(&config_file, content).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&config_file, fs::Permissions::from_mode(0o600)).unwrap();
        }

        // Try to load non-existent nested section
        let result = load_config_section("database.pool");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        // Try to load with wrong parent
        let result2 = load_config_section("other.connection");
        assert!(result2.is_ok());
        assert!(result2.unwrap().is_none());

        env::remove_var("SNOWFLAKE_HOME");
    }

    #[test]
    fn test_cannot_load_nested_connections_section() {
        let temp_dir = TempDir::new().unwrap();
        env::set_var("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

        let config_file = temp_dir.path().join("config.toml");
        let content = r#"
[connections.testconn]
account = "myaccount"
"#;
        fs::write(&config_file, content).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&config_file, fs::Permissions::from_mode(0o600)).unwrap();
        }

        // Should return None for nested connections path
        let result = load_config_section("connections.testconn");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        env::remove_var("SNOWFLAKE_HOME");
    }

    #[test]
    fn test_connections_toml_does_not_affect_other_sections() {
        let temp_dir = TempDir::new().unwrap();
        env::set_var("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

        // Create config.toml with log section
        let config_file = temp_dir.path().join("config.toml");
        let config_content = r#"
[log]
level = "info"

[connections.testconn]
account = "config_account"
"#;
        fs::write(&config_file, config_content).unwrap();

        // Create connections.toml - should NOT affect log section
        let connections_file = temp_dir.path().join("connections.toml");
        let connections_content = r#"
[testconn]
account = "connections_account"

[log]
level = "debug"
"#;
        fs::write(&connections_file, connections_content).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&config_file, fs::Permissions::from_mode(0o600)).unwrap();
            fs::set_permissions(&connections_file, fs::Permissions::from_mode(0o600)).unwrap();
        }

        // Load log section - should come from config.toml only
        let result = load_config_section("log");
        assert!(result.is_ok());
        let log_section = result.unwrap();
        assert!(log_section.is_some());

        let settings = log_section.unwrap();
        if let Some(Setting::String(level)) = settings.get("level") {
            // Should be "info" from config.toml, not "debug" from connections.toml
            assert_eq!(level, "info");
        } else {
            panic!("Expected level setting");
        }

        env::remove_var("SNOWFLAKE_HOME");
    }

    #[test]
    fn test_env_override_snowflake_section_key_pattern() {
        // Test the generic pattern: SNOWFLAKE_<SECTION>_<KEY> overrides [section].key
        let temp_dir = TempDir::new().unwrap();
        unsafe {
            env::set_var("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());
        }

        let config_file = temp_dir.path().join("config.toml");
        let content = r#"
[bar]
foo = "file_value"
"#;
        fs::write(&config_file, content).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&config_file, fs::Permissions::from_mode(0o600)).unwrap();
        }

        // Set SNOWFLAKE_BAR_FOO env var
        unsafe {
            env::set_var("SNOWFLAKE_BAR_FOO", "env_value");
        }

        // Load with env overrides enabled (default)
        let result = load_all_config_sections();
        assert!(result.is_ok());

        let sections = result.unwrap();
        let bar_section = sections.get("bar").expect("bar section should exist");
        if let Some(Setting::String(value)) = bar_section.get("foo") {
            assert_eq!(value, "env_value", "SNOWFLAKE_BAR_FOO should override [bar].foo");
        } else {
            panic!("Expected foo setting in bar section");
        }

        unsafe {
            env::remove_var("SNOWFLAKE_HOME");
            env::remove_var("SNOWFLAKE_BAR_FOO");
        }
    }

    #[test]
    fn test_env_override_disabled() {
        // Test that env overrides can be skipped
        let temp_dir = TempDir::new().unwrap();
        unsafe {
            env::set_var("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());
        }

        let config_file = temp_dir.path().join("config.toml");
        let content = r#"
[bar]
foo = "file_value"
"#;
        fs::write(&config_file, content).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&config_file, fs::Permissions::from_mode(0o600)).unwrap();
        }

        // Set SNOWFLAKE_BAR_FOO env var
        unsafe {
            env::set_var("SNOWFLAKE_BAR_FOO", "env_value");
        }

        // Load with env overrides DISABLED
        let result = load_all_config_sections_with_options(false);
        assert!(result.is_ok());

        let sections = result.unwrap();
        let bar_section = sections.get("bar").expect("bar section should exist");
        if let Some(Setting::String(value)) = bar_section.get("foo") {
            assert_eq!(value, "file_value", "With env overrides disabled, should get file value");
        } else {
            panic!("Expected foo setting in bar section");
        }

        unsafe {
            env::remove_var("SNOWFLAKE_HOME");
            env::remove_var("SNOWFLAKE_BAR_FOO");
        }
    }
}
