use std::sync::LazyLock;

use jni::JNIEnv;
use jni::objects::{JByteArray, JClass, JObject, JString, JValue};
use jni::sys::{jint, jobject};
use proto_utils::{ProtoError, Transport};
use sf_core::protobuf::apis::RustTransport;

struct JdbcBridge {
    runtime: tokio::runtime::Runtime,
    transport: RustTransport,
}

impl JdbcBridge {
    pub fn new() -> Self {
        Self {
            // Single worker thread is intentional: keeps contention minimal and
            // makes deadlocks easier to detect. Will be increased during
            // performance optimization.
            runtime: tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime"),
            transport: RustTransport::new(),
        }
    }

    pub fn handle_message_sync(
        &self,
        service_name: &str,
        method_name: &str,
        request_bytes: Vec<u8>,
    ) -> Result<Vec<u8>, ProtoError<Vec<u8>>> {
        self.runtime.block_on(self.transport.handle_message(
            service_name,
            method_name,
            request_bytes,
        ))
    }
}

static JDBC_BRIDGE: LazyLock<JdbcBridge> = LazyLock::new(JdbcBridge::new);

mod slf4j_layer;

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn JNI_OnLoad(jvm: *mut jni::sys::JavaVM, _: *mut u8) -> jint {
    let config = sf_core::logging::LoggingConfig::new(None, false, false);
    let layer = slf4j_layer::SLF4JLayer::new(jvm);
    match sf_core::logging::init_logging(config, Some(layer)) {
        Ok(_) => jni::sys::JNI_VERSION_1_2,
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
