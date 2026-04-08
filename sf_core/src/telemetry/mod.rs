pub mod environment;
pub mod types;

#[cfg(feature = "otlp_debug")]
pub mod otlp_debug;

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

    let mut message = serde_json::Map::new();
    message.insert(
        "type".to_string(),
        serde_json::Value::String("session_init".to_string()),
    );
    message.insert(
        "driver_name".to_string(),
        serde_json::Value::String(env.driver_name.clone()),
    );
    message.insert(
        "driver_version".to_string(),
        serde_json::Value::String(env.driver_version.clone()),
    );
    message.insert(
        "language_runtime".to_string(),
        serde_json::Value::String(env.language_runtime.clone()),
    );
    message.insert(
        "language_version".to_string(),
        serde_json::Value::String(env.language_version.clone()),
    );
    if let Some(ref compiler) = env.language_compiler {
        message.insert(
            "language_compiler".to_string(),
            serde_json::Value::String(compiler.clone()),
        );
    }
    message.insert(
        "os_name".to_string(),
        serde_json::Value::String(env.os_name.clone()),
    );
    message.insert(
        "os_version".to_string(),
        serde_json::Value::String(env.os_version.clone()),
    );
    message.insert(
        "os_architecture".to_string(),
        serde_json::Value::String(env.os_architecture.clone()),
    );
    message.insert("session_id".to_string(), serde_json::json!(session_id));

    serde_json::json!({
        "logs": [{
            "message": message,
            "timestamp": now_millis.to_string()
        }]
    })
}
