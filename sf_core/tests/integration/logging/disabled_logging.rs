use std::thread;
use std::time::Duration;

use sf_core::logging::LogManager;
use sf_core::logging::ini_config::parse_ini_file;

/// When `LogEnabled=false`, the LogManager initialises successfully but no
/// test messages appear in any log file under the configured path.
#[test]
fn disabled_logging_produces_no_output() {
    let dir = tempfile::tempdir().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();

    let ini_path = dir.path().join("sf.odbc.ini");
    std::fs::write(
        &ini_path,
        format!(
            "LogEnabled=false\nLogPath={}\nLogFile=disabled.log\n",
            log_dir.display()
        ),
    )
    .unwrap();

    let config = parse_ini_file(&ini_path).unwrap();
    assert!(!config.enabled);

    LogManager::init(config).unwrap();
    tracing::info!("disabled_logging_test_message");

    thread::sleep(Duration::from_millis(200));

    let has_test_message = std::fs::read_dir(&log_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .any(|e| {
            std::fs::read_to_string(e.path())
                .unwrap_or_default()
                .contains("disabled_logging_test_message")
        });
    assert!(
        !has_test_message,
        "expected no log file containing test message when logging is disabled"
    );
}
