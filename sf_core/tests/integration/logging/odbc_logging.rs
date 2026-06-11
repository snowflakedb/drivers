use std::thread;
use std::time::Duration;

use sf_core::config::{ConfigError, IniConfig, load_ini_files, logging_config_from_ini};
use sf_core::logging::LogManager;
use tracing::level_filters::LevelFilter;

/// Single-init happy path: build a snapshot from an in-memory INI, init
/// `LogManager`, emit events, verify the log file.
#[test]
fn ini_to_log_manager_happy_path() {
    let dir = tempfile::tempdir().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();

    let ini = IniConfig::from_ini_content(&format!(
        "LogLevel=INFO\nLogPath={}\nLogFile=test_driver.log\n",
        log_dir.display()
    ))
    .unwrap();

    let config = logging_config_from_ini(&ini).unwrap();
    assert_eq!(config.level, LevelFilter::INFO);
    assert_eq!(config.log_path.as_deref(), Some(log_dir.as_path()));
    assert_eq!(config.log_file_name.as_deref(), Some("test_driver.log"));

    let lm = LogManager::init(config).unwrap();
    let _guard = tracing::dispatcher::set_default(lm.dispatch());

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

/// `load_ini_files` walks the ordered path list and stops at the first
/// existing file, ignoring earlier missing candidates. A repeat call hits
/// the `OnceLock` and returns `IniAlreadyLoaded` without modifying the
/// snapshot.
#[test]
fn load_ini_files_walks_paths_then_locks() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing.ini");
    let present = dir.path().join("sf.odbc.ini");
    std::fs::write(&present, "LogLevel=WARN\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&present, std::fs::Permissions::from_mode(0o600)).unwrap();
    }

    load_ini_files(&[missing.clone(), present.clone()]).expect("first load should succeed");

    let snapshot = sf_core::config::get_ini_config().expect("snapshot installed");
    assert_eq!(snapshot.get("loglevel"), Some("WARN"));
    assert_eq!(snapshot.source(), Some(present.as_path()));

    let other = dir.path().join("other.ini");
    std::fs::write(&other, "LogLevel=DEBUG\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&other, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let err = load_ini_files(&[other]).expect_err("second load must be rejected");
    assert!(
        matches!(err, ConfigError::IniAlreadyLoaded { .. }),
        "expected IniAlreadyLoaded, got: {err:?}"
    );
    // Snapshot remains the original WARN-level config.
    assert_eq!(
        sf_core::config::get_ini_config().and_then(|i| i.get("loglevel")),
        Some("WARN")
    );
}
