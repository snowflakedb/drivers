pub mod environment;
pub mod types;

// These modules are public for integration tests but are not part of the stable API.
#[doc(hidden)]
pub mod serialization;
#[doc(hidden)]
pub mod snowflake_exporter;

use environment::EnvironmentInfo;

/// Build a `session_init` telemetry payload ready to POST to `/telemetry/send`.
pub fn build_session_init_payload(env: &EnvironmentInfo, session_id: i64) -> serde_json::Value {
    let now_millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let mut message = serde_json::json!({
        "type": "session_init",
        "driver_name": env.driver_name,
        "driver_version": env.driver_version,
        "language_runtime": env.language_runtime,
        "language_version": env.language_version,
        "os_name": env.os_name,
        "os_version": env.os_version,
        "os_architecture": env.os_architecture,
        "session_id": session_id,
    });

    if let Some(ref compiler) = env.language_compiler {
        message["language_compiler"] = serde_json::Value::String(compiler.clone());
    }

    serde_json::json!({
        "logs": [{
            "message": message,
            "timestamp": now_millis.to_string()
        }]
    })
}
