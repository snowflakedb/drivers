use std::thread;
use std::time::Duration;

use sf_core::logging::LogManager;
use sf_core::logging::ini_config::parse_ini_file;

/// Configure LogLevel=WARN, emit ERROR/WARN/INFO/DEBUG events, and verify
/// that only ERROR and WARN appear in the log file.
#[test]
fn warn_level_filters_info_and_debug() {
    let dir = tempfile::tempdir().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();

    let ini_path = dir.path().join("sf.odbc.ini");
    std::fs::write(
        &ini_path,
        format!(
            "LogLevel=WARN\nLogPath={}\nLogFile=level_filter.log\n",
            log_dir.display()
        ),
    )
    .unwrap();

    let config = parse_ini_file(&ini_path).unwrap();
    assert_eq!(config.level, tracing::level_filters::LevelFilter::WARN);

    LogManager::init(config).unwrap();

    tracing::error!("level_filter_error_msg");
    tracing::warn!("level_filter_warn_msg");
    tracing::info!("level_filter_info_msg");
    tracing::debug!("level_filter_debug_msg");

    thread::sleep(Duration::from_millis(200));

    let mut combined = String::new();
    for entry in std::fs::read_dir(&log_dir).unwrap().filter_map(|e| e.ok()) {
        combined.push_str(&std::fs::read_to_string(entry.path()).unwrap_or_default());
    }

    assert!(
        combined.contains("level_filter_error_msg"),
        "ERROR message should appear at WARN level"
    );
    assert!(
        combined.contains("level_filter_warn_msg"),
        "WARN message should appear at WARN level"
    );
    assert!(
        !combined.contains("level_filter_info_msg"),
        "INFO message should NOT appear at WARN level"
    );
    assert!(
        !combined.contains("level_filter_debug_msg"),
        "DEBUG message should NOT appear at WARN level"
    );
}
