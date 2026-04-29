use std::thread;
use std::time::Duration;

use sf_core::logging::LogManager;
use sf_core::logging::ini_config::{find_odbc_ini, parse_ini_file};
use tracing::level_filters::LevelFilter;

/// Single-init happy path: parse a temp INI, init `LogManager`, emit events,
/// verify the log file.
#[test]
fn ini_to_log_manager_happy_path() {
    let dir = tempfile::tempdir().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();

    let ini_path = dir.path().join("sf.odbc.ini");
    std::fs::write(
        &ini_path,
        format!(
            "LogLevel=INFO\nLogPath={}\nLogFile=test_driver.log\n",
            log_dir.display()
        ),
    )
    .unwrap();

    let config = parse_ini_file(&ini_path).unwrap();
    assert_eq!(config.level, LevelFilter::INFO);
    assert_eq!(config.log_path.as_deref(), Some(log_dir.as_path()));
    assert_eq!(config.log_file_name.as_deref(), Some("test_driver.log"));

    LogManager::init(config).unwrap();

    tracing::info!("happy_path_info_message");
    tracing::warn!("happy_path_warn_message");
    tracing::debug!("happy_path_debug_should_not_appear");

    thread::sleep(Duration::from_millis(200));

    let entries: Vec<_> = std::fs::read_dir(&log_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(
        !entries.is_empty(),
        "expected at least one log file in {}, found none",
        log_dir.display()
    );

    let mut combined = String::new();
    for entry in &entries {
        combined.push_str(&std::fs::read_to_string(entry.path()).unwrap_or_default());
    }
    assert!(
        combined.contains("happy_path_info_message"),
        "INFO message should appear in log file"
    );
    assert!(
        combined.contains("happy_path_warn_message"),
        "WARN message should appear in log file"
    );
    assert!(
        !combined.contains("happy_path_debug_should_not_appear"),
        "DEBUG message should not appear when level is INFO"
    );
}

/// Verify `find_odbc_ini` picks up the `SF_ODBC_INI` env var.
#[test]
fn find_odbc_ini_resolves_env_var() {
    let dir = tempfile::tempdir().unwrap();
    let ini_path = dir.path().join("sf.odbc.ini");
    std::fs::write(&ini_path, "LogLevel=WARN\n").unwrap();

    temp_env::with_var("SF_ODBC_INI", Some(ini_path.to_str().unwrap()), || {
        let found = find_odbc_ini();
        assert_eq!(found.as_deref(), Some(ini_path.as_path()));
    });
}

/// Verify that a non-existent INI path returns an IO error.
#[test]
fn parse_nonexistent_ini_returns_io_error() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("sf.odbc.ini");
    let result = parse_ini_file(&missing);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, sf_core::logging::LogError::Io { .. }),
        "expected Io error variant, got: {err:?}"
    );
}
