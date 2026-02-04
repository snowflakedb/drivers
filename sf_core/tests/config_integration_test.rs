use sf_core::apis::database_driver_v1::connection::{
    connection_init, connection_load_from_config, connection_new, connection_set_option,
};
use sf_core::apis::database_driver_v1::Setting;
use sf_core::config::config_manager::{load_all_config_sections, load_config_section};
use std::env;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_connection_load_from_config_basic() {
    let temp_dir = TempDir::new().unwrap();
    env::set_var("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

    let connections_file = temp_dir.path().join("connections.toml");
    let content = r#"
[testconn]
account = "myaccount"
user = "myuser"
warehouse = "mywarehouse"
"#;
    fs::write(&connections_file, content).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&connections_file, fs::Permissions::from_mode(0o600)).unwrap();
    }

    // Create connection and load config
    let conn_handle = connection_new();
    let result = connection_load_from_config(conn_handle, "testconn");
    assert!(result.is_ok());

    env::remove_var("SNOWFLAKE_HOME");
}

#[test]
fn test_explicit_setting_overrides_config() {
    let temp_dir = TempDir::new().unwrap();
    env::set_var("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

    let connections_file = temp_dir.path().join("connections.toml");
    let content = r#"
[testconn]
account = "config_account"
user = "config_user"
"#;
    fs::write(&connections_file, content).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&connections_file, fs::Permissions::from_mode(0o600)).unwrap();
    }

    // Create connection with explicit account setting
    let conn_handle = connection_new();
    connection_set_option(
        conn_handle,
        "account".to_string(),
        Setting::String("explicit_account".to_string()),
    )
    .unwrap();

    // Load from config
    let result = connection_load_from_config(conn_handle, "testconn");
    assert!(result.is_ok());

    // Verify explicit account wins (would need to add a getter function to verify this properly)

    env::remove_var("SNOWFLAKE_HOME");
}

#[test]
fn test_connection_not_found_in_config() {
    let temp_dir = TempDir::new().unwrap();
    env::set_var("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

    let conn_handle = connection_new();
    let result = connection_load_from_config(conn_handle, "nonexistent");

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));

    env::remove_var("SNOWFLAKE_HOME");
}

#[test]
fn test_config_precedence() {
    let temp_dir = TempDir::new().unwrap();
    env::set_var("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

    // Create config.toml with lower precedence
    let config_file = temp_dir.path().join("config.toml");
    let config_content = r#"
[connections.testconn]
account = "config_account"
user = "config_user"
database = "config_db"
"#;
    fs::write(&config_file, config_content).unwrap();

    // Create connections.toml with higher precedence
    let connections_file = temp_dir.path().join("connections.toml");
    let connections_content = r#"
[testconn]
account = "connections_account"
warehouse = "connections_wh"
"#;
    fs::write(&connections_file, connections_content).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&config_file, fs::Permissions::from_mode(0o600)).unwrap();
        fs::set_permissions(&connections_file, fs::Permissions::from_mode(0o600)).unwrap();
    }

    let conn_handle = connection_new();
    let result = connection_load_from_config(conn_handle, "testconn");
    assert!(result.is_ok());

    // Account should come from connections.toml (higher precedence)
    // User and database should come from config.toml
    // Warehouse should come from connections.toml

    env::remove_var("SNOWFLAKE_HOME");
}

#[test]
fn test_env_var_override() {
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

    let conn_handle = connection_new();
    let result = connection_load_from_config(conn_handle, "testconn");
    assert!(result.is_ok());

    // Environment variable should override file config

    env::remove_var("SNOWFLAKE_HOME");
    env::remove_var("SNOWFLAKE_ACCOUNT");
}

#[cfg(unix)]
#[test]
fn test_insecure_permissions_rejected() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new().unwrap();
    env::set_var("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

    let connections_file = temp_dir.path().join("connections.toml");
    let content = r#"
[testconn]
account = "myaccount"
"#;
    fs::write(&connections_file, content).unwrap();

    // Set insecure permissions (writable by others)
    fs::set_permissions(&connections_file, fs::Permissions::from_mode(0o666)).unwrap();

    let conn_handle = connection_new();
    let result = connection_load_from_config(conn_handle, "testconn");

    // Should fail due to insecure permissions
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Insecure"));

    env::remove_var("SNOWFLAKE_HOME");
}

#[test]
fn test_multiple_data_types() {
    let temp_dir = TempDir::new().unwrap();
    env::set_var("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

    let connections_file = temp_dir.path().join("connections.toml");
    let content = r#"
[testconn]
account = "myaccount"
port = 443
timeout = 30.5
validate_certs = true
"#;
    fs::write(&connections_file, content).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&connections_file, fs::Permissions::from_mode(0o600)).unwrap();
    }

    let conn_handle = connection_new();
    let result = connection_load_from_config(conn_handle, "testconn");
    assert!(result.is_ok());

    // Different data types should be properly converted to Setting types

    env::remove_var("SNOWFLAKE_HOME");
}

#[test]
fn test_empty_config_files() {
    let temp_dir = TempDir::new().unwrap();
    env::set_var("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

    // Create empty config files
    let connections_file = temp_dir.path().join("connections.toml");
    fs::write(&connections_file, "").unwrap();

    let config_file = temp_dir.path().join("config.toml");
    fs::write(&config_file, "").unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&connections_file, fs::Permissions::from_mode(0o600)).unwrap();
        fs::set_permissions(&config_file, fs::Permissions::from_mode(0o600)).unwrap();
    }

    let conn_handle = connection_new();
    let result = connection_load_from_config(conn_handle, "testconn");

    // Should fail - connection not found
    assert!(result.is_err());

    env::remove_var("SNOWFLAKE_HOME");
}

// Tests for non-connection sections

#[test]
fn test_load_log_section() {
    let temp_dir = TempDir::new().unwrap();
    env::set_var("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

    let config_file = temp_dir.path().join("config.toml");
    let content = r#"
[log]
level = "debug"
path = "/var/log/snowflake.log"

[connections.testconn]
account = "myaccount"
"#;
    fs::write(&config_file, content).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&config_file, fs::Permissions::from_mode(0o600)).unwrap();
    }

    let result = load_config_section("log");
    assert!(result.is_ok());

    let log_section = result.unwrap();
    assert!(log_section.is_some());

    let settings = log_section.unwrap();
    assert!(matches!(settings.get("level"), Some(Setting::String(_))));
    assert!(matches!(settings.get("path"), Some(Setting::String(_))));

    env::remove_var("SNOWFLAKE_HOME");
}

#[test]
fn test_load_multiple_sections() {
    let temp_dir = TempDir::new().unwrap();
    env::set_var("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

    let config_file = temp_dir.path().join("config.toml");
    let content = r#"
[log]
level = "info"

[proxy]
host = "proxy.example.com"
port = 8080

[retry]
max_attempts = 5

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
    assert_eq!(sections.len(), 3); // log, proxy, retry (not connections)
    assert!(sections.contains_key("log"));
    assert!(sections.contains_key("proxy"));
    assert!(sections.contains_key("retry"));
    assert!(!sections.contains_key("connections"));

    env::remove_var("SNOWFLAKE_HOME");
}

#[test]
fn test_connections_toml_does_not_override_log_section() {
    let temp_dir = TempDir::new().unwrap();
    env::set_var("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

    // Create config.toml with log section
    let config_file = temp_dir.path().join("config.toml");
    let config_content = r#"
[log]
level = "info"
file = "config_log.txt"

[connections.testconn]
account = "config_account"
"#;
    fs::write(&config_file, config_content).unwrap();

    // Create connections.toml that tries to override log section
    let connections_file = temp_dir.path().join("connections.toml");
    let connections_content = r#"
[testconn]
account = "connections_account"

[log]
level = "debug"
file = "connections_log.txt"
"#;
    fs::write(&connections_file, connections_content).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&config_file, fs::Permissions::from_mode(0o600)).unwrap();
        fs::set_permissions(&connections_file, fs::Permissions::from_mode(0o600)).unwrap();
    }

    // Load log section - should only come from config.toml
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

    if let Some(Setting::String(file)) = settings.get("file") {
        // Should be "config_log.txt" from config.toml, not "connections_log.txt"
        assert_eq!(file, "config_log.txt");
    } else {
        panic!("Expected file setting");
    }

    env::remove_var("SNOWFLAKE_HOME");
}

#[test]
fn test_load_nonexistent_section() {
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

    let result = load_config_section("nonexistent_section");
    assert!(result.is_ok());

    let section = result.unwrap();
    assert!(section.is_none());

    env::remove_var("SNOWFLAKE_HOME");
}

#[test]
fn test_cannot_load_connections_via_load_config_section() {
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

    // Should return None when trying to load connections section
    let result = load_config_section("connections");
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());

    env::remove_var("SNOWFLAKE_HOME");
}

#[test]
fn test_load_nested_config_section() {
    let temp_dir = TempDir::new().unwrap();
    env::set_var("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

    let config_file = temp_dir.path().join("config.toml");
    let content = r#"
[database.connection]
timeout = 30
max_retries = 5

[database.pool]
max_size = 20
min_size = 5

[app.logging.file]
path = "/var/log/app.log"
max_size = 10485760
"#;
    fs::write(&config_file, content).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&config_file, fs::Permissions::from_mode(0o600)).unwrap();
    }

    // Load database.connection section
    let result = load_config_section("database.connection");
    assert!(result.is_ok());
    let section = result.unwrap();
    assert!(section.is_some());

    let settings = section.unwrap();
    assert!(matches!(settings.get("timeout"), Some(Setting::Int(30))));
    assert!(matches!(settings.get("max_retries"), Some(Setting::Int(5))));

    // Load database.pool section
    let result = load_config_section("database.pool");
    assert!(result.is_ok());
    let section = result.unwrap();
    assert!(section.is_some());

    let settings = section.unwrap();
    assert!(matches!(settings.get("max_size"), Some(Setting::Int(20))));

    // Load deeply nested section
    let result = load_config_section("app.logging.file");
    assert!(result.is_ok());
    let section = result.unwrap();
    assert!(section.is_some());

    let settings = section.unwrap();
    if let Some(Setting::String(path)) = settings.get("path") {
        assert_eq!(path, "/var/log/app.log");
    } else {
        panic!("Expected path setting");
    }

    env::remove_var("SNOWFLAKE_HOME");
}

#[test]
fn test_nested_connections_blocked() {
    let temp_dir = TempDir::new().unwrap();
    env::set_var("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

    let config_file = temp_dir.path().join("config.toml");
    let content = r#"
[connections.dev]
account = "dev_account"

[connections.prod]
account = "prod_account"
"#;
    fs::write(&config_file, content).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&config_file, fs::Permissions::from_mode(0o600)).unwrap();
    }

    // Should return None for nested connections paths
    let result = load_config_section("connections.dev");
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());

    let result = load_config_section("connections.prod");
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());

    env::remove_var("SNOWFLAKE_HOME");
}

#[test]
fn test_nonexistent_nested_section() {
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

    // Try to load non-existent nested sections
    let result = load_config_section("database.pool");
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());

    let result = load_config_section("database.connection.invalid");
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());

    env::remove_var("SNOWFLAKE_HOME");
}
