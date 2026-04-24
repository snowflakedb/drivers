use std::thread;
use std::time::Duration;

use sf_core::logging::LogManager;
use sf_core::logging::ini_config::parse_ini_file;

/// Configure `LogFile=my_custom_driver.log`, emit an event, and verify a file
/// matching that prefix exists in the log directory.
#[test]
fn custom_log_file_name_is_used() {
    let dir = tempfile::tempdir().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();

    let ini_path = dir.path().join("sf.odbc.ini");
    std::fs::write(
        &ini_path,
        format!(
            "LogLevel=INFO\nLogPath={}\nLogFile=my_custom_driver.log\n",
            log_dir.display()
        ),
    )
    .unwrap();

    let config = parse_ini_file(&ini_path).unwrap();
    assert_eq!(
        config.log_file_name.as_deref(),
        Some("my_custom_driver.log")
    );

    LogManager::init(config).unwrap();

    tracing::info!("custom_file_name_test_message");

    thread::sleep(Duration::from_millis(200));

    let entries: Vec<_> = std::fs::read_dir(&log_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();

    let has_custom_file = entries
        .iter()
        .any(|e| e.file_name().to_string_lossy().contains("my_custom_driver"));
    assert!(
        has_custom_file,
        "expected a log file with prefix 'my_custom_driver' in {}, found: {:?}",
        log_dir.display(),
        entries.iter().map(|e| e.file_name()).collect::<Vec<_>>()
    );

    let mut combined = String::new();
    for entry in &entries {
        combined.push_str(&std::fs::read_to_string(entry.path()).unwrap_or_default());
    }
    assert!(
        combined.contains("custom_file_name_test_message"),
        "test message should appear in the custom-named log file"
    );
}
