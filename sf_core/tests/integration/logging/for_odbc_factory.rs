use std::thread;
use std::time::Duration;

use sf_core::logging::LogManager;

/// End-to-end: write a temp INI, point `SF_ODBC_INI` at it, call
/// `LogManager::for_odbc()`, emit an event, and verify it lands in the log
/// file.
#[test]
fn for_odbc_factory_end_to_end() {
    let dir = tempfile::tempdir().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();

    let ini_path = dir.path().join("sf.odbc.ini");
    std::fs::write(
        &ini_path,
        format!(
            "LogLevel=INFO\nLogPath={}\nLogFile=odbc_factory.log\n",
            log_dir.display()
        ),
    )
    .unwrap();

    temp_env::with_var("SF_ODBC_INI", Some(ini_path.to_str().unwrap()), || {
        LogManager::for_odbc();

        tracing::info!("for_odbc_factory_test_message");

        thread::sleep(Duration::from_millis(200));

        let mut combined = String::new();
        for entry in std::fs::read_dir(&log_dir).unwrap().filter_map(|e| e.ok()) {
            combined.push_str(&std::fs::read_to_string(entry.path()).unwrap_or_default());
        }

        assert!(
            combined.contains("for_odbc_factory_test_message"),
            "expected test message in log file, got: {combined}"
        );
    });
}
