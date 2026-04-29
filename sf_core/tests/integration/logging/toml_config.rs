use std::thread;
use std::time::Duration;

use sf_core::config::config_manager::load_config_section_with_paths;
use sf_core::config::path_resolver::ConfigPaths;
use sf_core::logging::LogManager;
use sf_core::logging::ini_config::load_from_toml_section;

/// Load a `[log]` section from a TOML file, init `LogManager` at DEBUG level,
/// and verify that DEBUG events appear but TRACE events do not.
#[test]
fn toml_log_section_with_level_filtering() {
    let dir = tempfile::tempdir().unwrap();
    let log_dir = dir.path().join("logs");
    std::fs::create_dir_all(&log_dir).unwrap();

    let config_path = dir.path().join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            "[log]\nlevel = \"DEBUG\"\npath = \"{}\"\nfile = \"toml_test.log\"\n",
            log_dir.display().to_string().replace('\\', "/")
        ),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&config_path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }

    let paths = ConfigPaths {
        config_file: Some(config_path),
        connections_file: None,
    };
    let section = load_config_section_with_paths("log", &paths)
        .expect("load_config_section_with_paths failed")
        .expect("[log] section should exist");

    let config = load_from_toml_section(&section);
    assert_eq!(config.level, tracing::level_filters::LevelFilter::DEBUG);

    LogManager::init(config).unwrap();

    tracing::debug!("toml_debug_message_should_appear");
    tracing::trace!("toml_trace_message_should_not_appear");

    thread::sleep(Duration::from_millis(200));

    let mut combined = String::new();
    for entry in std::fs::read_dir(&log_dir).unwrap().filter_map(|e| e.ok()) {
        combined.push_str(&std::fs::read_to_string(entry.path()).unwrap_or_default());
    }

    assert!(
        combined.contains("toml_debug_message_should_appear"),
        "DEBUG message should appear in log file at DEBUG level"
    );
    assert!(
        !combined.contains("toml_trace_message_should_not_appear"),
        "TRACE message should not appear when level is DEBUG"
    );
}
