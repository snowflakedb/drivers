use sf_core::apis::database_driver_v1::Setting;
use sf_core::apis::database_driver_v1::connection::{Connection, connection_load_from_config};
use sf_core::config::config_manager::{load_all_config_sections, load_config_section};
use std::env;
use std::fs;
use tempfile::TempDir;

fn set_env(key: &str, value: &str) {
    unsafe { env::set_var(key, value) }
}

fn remove_env(key: &str) {
    unsafe { env::remove_var(key) }
}

#[test]
fn connection_load_from_config_basic() {
    // Given A connections.toml file with test_connection defined
    let temp_dir = TempDir::new().unwrap();
    set_env("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

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

    // When sf_core loads the connection config
    let mut conn = Connection::new();
    let result = connection_load_from_config(&mut conn, "testconn");

    // Then The connection settings should be loaded
    assert!(result.is_ok());

    remove_env("SNOWFLAKE_HOME");
}

#[test]
fn explicit_setting_overrides_config() {
    // Given A connections.toml with connection having account setting
    let temp_dir = TempDir::new().unwrap();
    set_env("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

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

    // And An explicit account setting on the connection
    let mut conn = Connection::new();
    conn.settings.insert(
        "account".to_string(),
        Setting::String("explicit_account".to_string()),
    );

    // When sf_core loads the connection config
    let result = connection_load_from_config(&mut conn, "testconn");

    // Then The explicit setting should take precedence
    assert!(result.is_ok());

    remove_env("SNOWFLAKE_HOME");
}

#[test]
fn connection_not_found_in_config() {
    // Given No configuration files exist
    let temp_dir = TempDir::new().unwrap();
    set_env("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

    // When sf_core loads connection named nonexistent
    let mut conn = Connection::new();
    let result = connection_load_from_config(&mut conn, "nonexistent");

    // Then ConnectionNotFound error should be returned
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));

    remove_env("SNOWFLAKE_HOME");
}

#[test]
fn config_precedence() {
    // Given A config.toml with connection account set to config_account
    let temp_dir = TempDir::new().unwrap();
    set_env("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

    let config_file = temp_dir.path().join("config.toml");
    let config_content = r#"
[connections.testconn]
account = "config_account"
user = "config_user"
database = "config_db"
"#;
    fs::write(&config_file, config_content).unwrap();

    // And A connections.toml with same connection account set to connections_account
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

    // When sf_core loads the connection config
    let mut conn = Connection::new();
    let result = connection_load_from_config(&mut conn, "testconn");

    // Then connections.toml values should override config.toml
    assert!(result.is_ok());

    remove_env("SNOWFLAKE_HOME");
}

#[cfg(unix)]
#[test]
fn insecure_permissions_rejected() {
    use std::os::unix::fs::PermissionsExt;

    // Given A connections.toml file with insecure permissions
    let temp_dir = TempDir::new().unwrap();
    set_env("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

    let connections_file = temp_dir.path().join("connections.toml");
    let content = r#"
[testconn]
account = "myaccount"
"#;
    fs::write(&connections_file, content).unwrap();

    // Set insecure permissions (writable by others)
    fs::set_permissions(&connections_file, fs::Permissions::from_mode(0o666)).unwrap();

    // When sf_core loads the connection config
    let mut conn = Connection::new();
    let result = connection_load_from_config(&mut conn, "testconn");

    // Then An insecure permissions error should be returned
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Insecure"));

    remove_env("SNOWFLAKE_HOME");
}

#[test]
fn multiple_data_types() {
    // Given A connections.toml with string, integer, float, and boolean values
    let temp_dir = TempDir::new().unwrap();
    set_env("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

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

    // When sf_core loads the connection config
    let mut conn = Connection::new();
    let result = connection_load_from_config(&mut conn, "testconn");

    // Then Each value should be parsed to the correct Setting type
    assert!(result.is_ok());

    remove_env("SNOWFLAKE_HOME");
}

#[test]
fn empty_config_files() {
    // Given Empty config.toml and connections.toml files
    let temp_dir = TempDir::new().unwrap();
    set_env("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

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

    // When sf_core loads connection named testconn
    let mut conn = Connection::new();
    let result = connection_load_from_config(&mut conn, "testconn");

    // Then ConnectionNotFound error should be returned
    assert!(result.is_err());

    remove_env("SNOWFLAKE_HOME");
}

// Tests for non-connection sections

#[test]
fn load_log_section() {
    // Given A config.toml with a log section
    let temp_dir = TempDir::new().unwrap();
    set_env("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

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

    // When sf_core loads the log section
    let result = load_config_section("log");
    assert!(result.is_ok());

    // Then The log settings should be returned
    let log_section = result.unwrap();
    assert!(log_section.is_some());

    let settings = log_section.unwrap();
    assert!(matches!(settings.get("level"), Some(Setting::String(_))));
    assert!(matches!(settings.get("path"), Some(Setting::String(_))));

    remove_env("SNOWFLAKE_HOME");
}

#[test]
fn load_multiple_sections() {
    // Given A config.toml with log, proxy, and retry sections
    let temp_dir = TempDir::new().unwrap();
    set_env("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

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

    // When sf_core loads all config sections
    let result = load_all_config_sections();
    assert!(result.is_ok());

    // Then All sections should be returned including connections
    let sections = result.unwrap();
    assert_eq!(sections.len(), 4); // log, proxy, retry, connections.testconn
    assert!(sections.contains_key("log"));
    assert!(sections.contains_key("proxy"));
    assert!(sections.contains_key("retry"));
    assert!(sections.contains_key("connections.testconn"));
    assert!(!sections.contains_key("connections"));

    remove_env("SNOWFLAKE_HOME");
}

#[test]
fn connections_toml_does_not_override_log_section() {
    // Given A config.toml with log section and a connections.toml with log section
    let temp_dir = TempDir::new().unwrap();
    set_env("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

    let config_file = temp_dir.path().join("config.toml");
    let config_content = r#"
[log]
level = "info"
file = "config_log.txt"

[connections.testconn]
account = "config_account"
"#;
    fs::write(&config_file, config_content).unwrap();

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

    // When sf_core loads the log config section
    let result = load_config_section("log");
    assert!(result.is_ok());

    // Then The config.toml log values should be used
    let log_section = result.unwrap();
    assert!(log_section.is_some());

    let settings = log_section.unwrap();
    if let Some(Setting::String(level)) = settings.get("level") {
        assert_eq!(level, "info");
    } else {
        panic!("Expected level setting");
    }

    if let Some(Setting::String(file)) = settings.get("file") {
        assert_eq!(file, "config_log.txt");
    } else {
        panic!("Expected file setting");
    }

    remove_env("SNOWFLAKE_HOME");
}

#[test]
fn load_nonexistent_section() {
    // Given A config.toml with a log section
    let temp_dir = TempDir::new().unwrap();
    set_env("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

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

    // When sf_core loads a nonexistent section
    let result = load_config_section("nonexistent_section");
    assert!(result.is_ok());

    // Then None should be returned
    let section = result.unwrap();
    assert!(section.is_none());

    remove_env("SNOWFLAKE_HOME");
}

#[test]
fn cannot_load_connections_via_load_config_section() {
    // Given A config.toml with connections section
    let temp_dir = TempDir::new().unwrap();
    set_env("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

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

    // When sf_core loads section connections
    let result = load_config_section("connections");

    // Then None should be returned
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());

    remove_env("SNOWFLAKE_HOME");
}

#[test]
fn load_nested_config_section() {
    // Given A config.toml with nested sections like database.connection
    let temp_dir = TempDir::new().unwrap();
    set_env("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

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

    // When sf_core loads a nested section by dotted path
    let result = load_config_section("database.connection");
    assert!(result.is_ok());
    let section = result.unwrap();
    assert!(section.is_some());

    // Then The nested section settings should be returned
    let settings = section.unwrap();
    assert!(matches!(settings.get("timeout"), Some(Setting::Int(30))));
    assert!(matches!(settings.get("max_retries"), Some(Setting::Int(5))));

    // Also verify other nested sections
    let result = load_config_section("database.pool");
    assert!(result.is_ok());
    let section = result.unwrap();
    assert!(section.is_some());

    let settings = section.unwrap();
    assert!(matches!(settings.get("max_size"), Some(Setting::Int(20))));

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

    remove_env("SNOWFLAKE_HOME");
}

#[test]
fn nested_connections_blocked() {
    // Given A config.toml with connections.dev and connections.prod
    let temp_dir = TempDir::new().unwrap();
    set_env("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

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

    // When sf_core loads section connections.dev
    let result = load_config_section("connections.dev");

    // Then None should be returned
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());

    let result = load_config_section("connections.prod");
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());

    remove_env("SNOWFLAKE_HOME");
}

#[test]
fn nonexistent_nested_section() {
    // Given A config.toml with database.connection section
    let temp_dir = TempDir::new().unwrap();
    set_env("SNOWFLAKE_HOME", temp_dir.path().to_str().unwrap());

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

    // When sf_core loads section database.pool
    let result = load_config_section("database.pool");

    // Then None should be returned
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());

    let result = load_config_section("database.connection.invalid");
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());

    remove_env("SNOWFLAKE_HOME");
}
