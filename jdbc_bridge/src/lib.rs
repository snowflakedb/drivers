use std::sync::{LazyLock, Mutex, OnceLock};

use jni::JNIEnv;
use jni::objects::{JByteArray, JClass, JObject, JString, JValue};
use jni::sys::{jint, jobject};
use proto_utils::{ProtoError, Transport};
use sf_core::logging::LogManager;
use sf_core::protobuf::apis::RustTransport;
use sf_core::protobuf::apis::database_driver_v1::{DriverProviders, WrapperPresets};
use sf_core::telemetry::snowflake_exporter::SessionRegistry;
use sf_core::wrapper_event;

static JDBC_LOG_MANAGER: Mutex<Option<LogManager>> = Mutex::new(None);

fn jstring_to_string(env: &mut JNIEnv, s: &JString) -> String {
    if s.is_null() {
        return String::new();
    }
    env.get_string(s)
        .map(|js| js.to_string_lossy().into_owned())
        .unwrap_or_default()
}

struct JdbcBridge {
    runtime: tokio::runtime::Runtime,
    transport: RustTransport,
    dispatch: tracing::dispatcher::Dispatch,
}

impl JdbcBridge {
    pub fn new() -> Self {
        let lm = JDBC_LOG_MANAGER
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        let dispatch = lm
            .as_ref()
            .map(|m| m.dispatch().clone())
            .unwrap_or_else(tracing::dispatcher::Dispatch::none);
        let providers = DriverProviders {
            log_manager: lm,
            wrapper_presets: WrapperPresets::jdbc(),
            ..Default::default()
        };
        Self {
            runtime: tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime"),
            transport: RustTransport::new_with(providers),
            dispatch,
        }
    }

    pub fn handle_message_sync(
        &self,
        service_name: &str,
        method_name: &str,
        request_bytes: Vec<u8>,
    ) -> Result<Vec<u8>, ProtoError<Vec<u8>>> {
        let _guard = tracing::dispatcher::set_default(&self.dispatch);
        self.runtime.block_on(self.transport.handle_message(
            service_name,
            method_name,
            request_bytes,
            tokio_util::sync::CancellationToken::new(),
        ))
    }
}

static JDBC_BRIDGE: LazyLock<JdbcBridge> = LazyLock::new(JdbcBridge::new);

mod sflogger_layer;

static LOG_DISPATCH: OnceLock<tracing::dispatcher::Dispatch> = OnceLock::new();

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn JNI_OnLoad(jvm: *mut jni::sys::JavaVM, _: *mut u8) -> jint {
    let layer = sflogger_layer::SFLoggerLayer::new(jvm);
    let sessions = SessionRegistry::default();
    match LogManager::with_app_sink(sf_core::logging::LoggingConfig::default(), layer, sessions) {
        Ok(lm) => {
            let _ = LOG_DISPATCH.set(lm.dispatch().clone());
            *JDBC_LOG_MANAGER.lock().unwrap_or_else(|e| e.into_inner()) = Some(lm);
            jni::sys::JNI_VERSION_1_2
        }
        Err(e) => {
            eprintln!("Failed to initialize logging: {e:?}");
            -1
        }
    }
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn JNI_OnUnload(_jvm: *mut jni::sys::JavaVM, _: *mut u8) -> jint {
    0
}

/// Handle a protobuf message
///
/// # Arguments
/// * `env` - JNI environment
/// * `_class` - The calling Java class
/// * `service_name` - The service name
/// * `method_name` - The method name
/// * `request_bytes` - The request bytes
///
/// # Returns
/// A TransportResponse object containing the status code and response bytes
///
/// # Safety
/// Called from Java, so we need to be careful with the pointer.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_net_snowflake_client_internal_unicore_JNICoreTransport_nativeHandleMessage(
    mut env: JNIEnv,
    _class: JClass,
    service_name: JString,
    method_name: JString,
    request_bytes: JByteArray,
) -> jobject {
    // Convert Java strings and byte array to Rust types
    let service_name_str = match env.get_string(&service_name) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    let method_name_str = match env.get_string(&method_name) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    let request_bytes_vec = match env.convert_byte_array(&request_bytes) {
        Ok(b) => b,
        Err(_) => return std::ptr::null_mut(),
    };

    let result = JDBC_BRIDGE.handle_message_sync(
        &service_name_str.to_string_lossy(),
        &method_name_str.to_string_lossy(),
        request_bytes_vec,
    );

    // Find the TransportResponse class
    let response_class = match env
        .find_class("net/snowflake/client/internal/unicore/CoreTransport$TransportResponse")
    {
        Ok(c) => c,
        Err(_) => return std::ptr::null_mut(),
    };

    // Create the appropriate response based on the result
    let response_obj = match result {
        Ok(response) => {
            // Success case - code 0
            let response_array = match env.byte_array_from_slice(&response) {
                Ok(arr) => arr,
                Err(_) => return std::ptr::null_mut(),
            };
            match env.new_object(
                response_class,
                "(I[B)V",
                &[
                    JValue::Int(0),
                    JValue::Object(&JObject::from(response_array)),
                ],
            ) {
                Ok(obj) => obj,
                Err(_) => return std::ptr::null_mut(),
            }
        }
        Err(ProtoError::Application(error)) => {
            // Application error - code 1
            let error_array = match env.byte_array_from_slice(&error) {
                Ok(arr) => arr,
                Err(_) => return std::ptr::null_mut(),
            };
            match env.new_object(
                response_class,
                "(I[B)V",
                &[JValue::Int(1), JValue::Object(&JObject::from(error_array))],
            ) {
                Ok(obj) => obj,
                Err(_) => return std::ptr::null_mut(),
            }
        }
        Err(ProtoError::Transport(error_msg)) => {
            // Transport error - code 2
            let error_array = match env.byte_array_from_slice(error_msg.as_bytes()) {
                Ok(arr) => arr,
                Err(_) => return std::ptr::null_mut(),
            };
            match env.new_object(
                response_class,
                "(I[B)V",
                &[JValue::Int(2), JValue::Object(&JObject::from(error_array))],
            ) {
                Ok(obj) => obj,
                Err(_) => return std::ptr::null_mut(),
            }
        }
    };

    response_obj.into_raw()
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_net_snowflake_client_internal_unicore_CoreLoggingBridge_nativeLogEvent(
    mut env: JNIEnv,
    _class: JClass,
    level: jint,
    message: JString,
    file: JString,
    line: jint,
    function: JString,
    logger_name: JString,
) -> jint {
    // Prevent unwinding across the JNI boundary; any panic becomes status 2.
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Some(dispatch) = LOG_DISPATCH.get() else {
            return 1;
        };
        let _guard = tracing::dispatcher::set_default(dispatch);

        let message = jstring_to_string(&mut env, &message);
        let file = jstring_to_string(&mut env, &file);
        let function = jstring_to_string(&mut env, &function);
        let logger_name = jstring_to_string(&mut env, &logger_name);

        wrapper_event!(
            level,
            message = message,
            file = file,
            function = function,
            line = line,
            logger_name = logger_name,
        );
        0
    }))
    .unwrap_or(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_new_succeeds_without_log_manager() {
        // LogManager::get() returns None when not initialised; construction must not panic.
        let _bridge = JdbcBridge::new();
    }
}
