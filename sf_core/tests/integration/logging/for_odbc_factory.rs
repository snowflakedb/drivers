use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use sf_core::config::load_ini_files;
use sf_core::logging::LogManager;

/// End-to-end: write a temp INI, seed the process-wide snapshot via
/// `sf_core::config::load_ini_files`, call `LogManager::for_odbc()`, emit
/// an event, and verify it lands in the log file. This is the canonical
/// happy path the ODBC wrapper takes (minus the platform-specific path
/// list, which the wrapper builds in `odbc::api::ini_paths::default_paths`).
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
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&ini_path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }

    load_ini_files(&[PathBuf::from(&ini_path)])
        .expect("first load_ini_files in this test binary must succeed");

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
}
