// This crate reaches `sf_core`'s RPC dispatch through its own async layers, so
// rustc computes the layout of each operation's future from several frames
// deeper than `sf_core` does. An operation's `await` chain (settings → HTTP →
// TLS → auth/rest) is ~130 levels on its own, which clears the default
// `recursion_limit` of 128 from here and fails with
// `error: queries overflow the depth limit!`.
//
// Raising the ceiling is compile-time only and costs nothing at runtime. It is
// preferred over `Box::pin`ning individual operations in `sf_core`, because
// which operation is deepest varies by target — Linux overflows on
// `connection_get_query_result` where macOS overflows on `connection_init` — so
// boxing one arm fixes only the platform it was measured on.
//
// Two things that make this class of error easy to miss: clippy never reports it
// (it does no codegen), and dev-profile incremental builds cache the layout
// queries, so it can compile fine locally after an earlier successful build.
// Reproduce with `CARGO_INCREMENTAL=0` on a cleaned crate.
#![recursion_limit = "256"]

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, OnceLock};

use jni::JNIEnv;
use jni::objects::{JByteArray, JClass, JObject, JString, JValue};
use jni::sys::{jint, jlong, jobject};
use proto_utils::{ProtoError, Transport};
use sf_core::logging::LogManager;
use sf_core::protobuf::apis::RustTransport;
use sf_core::protobuf::apis::database_driver_v1::{DriverProviders, WrapperPresets};
use sf_core::telemetry::snowflake_exporter::SessionRegistry;
use sf_core::utils::sync::MutexRecoverExt;
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

/// In-flight async RPC tasks, keyed by the operation handle. Each task resolves
/// to the `(code, bytes)` pair `nativeAwaitMessage` returns.
type ResultMap = HashMap<u64, tokio::task::JoinHandle<(i32, Vec<u8>)>>;

struct JdbcBridge {
    runtime: tokio::runtime::Runtime,
    transport: RustTransport,
    dispatch: tracing::dispatcher::Dispatch,
    /// In-flight async RPCs submitted via `nativeSubmitMessage`, keyed by the
    /// operation handle. Holds only the result plumbing — the cancellation token
    /// lives on the transport's registry (`RustTransport::register`).
    results: Mutex<ResultMap>,
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
            results: Mutex::new(HashMap::new()),
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
        ))
    }

    fn results(&self) -> std::sync::MutexGuard<'_, ResultMap> {
        self.results.lock_recover()
    }

    /// Non-blocking submit: register a cancellation handle, spawn the RPC under
    /// it, stash the `JoinHandle`, and return the handle immediately.
    fn submit(&self, service_name: String, method_name: String, request_bytes: Vec<u8>) -> u64 {
        let (handle, _token) = self.transport.register();
        let dispatch = self.dispatch.clone();
        let join = self.runtime.spawn(async move {
            let _guard = tracing::dispatcher::set_default(&dispatch);
            let result = JDBC_BRIDGE
                .transport
                .handle_message_cancellable(&service_name, &method_name, request_bytes, handle)
                .await;
            encode_result(result)
        });
        self.results().insert(handle, join);
        handle
    }

    /// Blocking await for a submitted handle. Blocks the calling (JVM) thread on
    /// the spawned task, then deregisters the token. `None` if the handle is
    /// unknown (already awaited, or never submitted).
    fn await_result(&self, handle: u64) -> Option<(i32, Vec<u8>)> {
        // The spawned task deregisters its own handle on completion (RAII inside
        // handle_message_cancellable), so a concurrent cancel(handle) during
        // block_on still finds the token until the op actually finishes.
        let join = self.results().remove(&handle)?;
        let out = self.runtime.block_on(join).unwrap_or_else(|e| {
            tracing::error!(cause = %e, "async RPC task panicked");
            (2, b"async task join error".to_vec())
        });
        Some(out)
    }
}

/// Map a transport result to the `(code, bytes)` pair the Java side decodes:
/// 0 = success, 1 = application error (proto-encoded), 2 = transport error.
fn encode_result(result: Result<Vec<u8>, ProtoError<Vec<u8>>>) -> (i32, Vec<u8>) {
    match result {
        Ok(response) => (0, response),
        Err(ProtoError::Application(error)) => (1, error),
        Err(ProtoError::Transport(message)) => (2, message.into_bytes()),
    }
}

/// Build a `CoreTransport$TransportResponse` Java object; null on any JNI error.
fn build_transport_response(env: &mut JNIEnv, code: i32, bytes: &[u8]) -> jobject {
    let response_class = match env
        .find_class("net/snowflake/client/internal/unicore/CoreTransport$TransportResponse")
    {
        Ok(c) => c,
        Err(_) => return std::ptr::null_mut(),
    };
    let response_array = match env.byte_array_from_slice(bytes) {
        Ok(arr) => arr,
        Err(_) => return std::ptr::null_mut(),
    };
    match env.new_object(
        response_class,
        "(I[B)V",
        &[
            JValue::Int(code),
            JValue::Object(&JObject::from(response_array)),
        ],
    ) {
        Ok(obj) => obj.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Runs a JNI entry-point body, catching any panic so it never unwinds across
/// the JNI boundary (a panic escaping an `extern "system"` fn aborts the
/// process). On panic, logs and returns `fallback`. Mirrors the `catch_unwind`
/// guarding the C API uses.
fn ffi_guard<T>(fallback: T, body: impl FnOnce() -> T) -> T {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)) {
        Ok(value) => value,
        Err(_) => {
            tracing::error!("panic caught at JNI boundary");
            fallback
        }
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
#[allow(non_snake_case)]
pub unsafe extern "system" fn Java_net_snowflake_client_internal_unicore_JNICoreTransport_nativeHandleMessage(
    mut env: JNIEnv,
    _class: JClass,
    service_name: JString,
    method_name: JString,
    request_bytes: JByteArray,
) -> jobject {
    ffi_guard(std::ptr::null_mut(), move || {
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

        let (code, bytes) = encode_result(result);
        build_transport_response(&mut env, code, &bytes)
    })
}

/// Non-blocking submit for an async-first RPC. Returns a cancellation handle
/// (0 on argument-conversion failure). Pair with `nativeAwaitMessage` /
/// `nativeCancel`.
///
/// # Safety
/// Called from Java; args must be valid for the duration of the call.
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub unsafe extern "system" fn Java_net_snowflake_client_internal_unicore_JNICoreTransport_nativeSubmitMessage(
    mut env: JNIEnv,
    _class: JClass,
    service_name: JString,
    method_name: JString,
    request_bytes: JByteArray,
) -> jlong {
    ffi_guard(0, move || {
        let service = match env.get_string(&service_name) {
            Ok(s) => s.to_string_lossy().into_owned(),
            Err(_) => return 0,
        };
        let method = match env.get_string(&method_name) {
            Ok(s) => s.to_string_lossy().into_owned(),
            Err(_) => return 0,
        };
        let request = match env.convert_byte_array(&request_bytes) {
            Ok(b) => b,
            Err(_) => return 0,
        };
        JDBC_BRIDGE.submit(service, method, request) as jlong
    })
}

/// Blocking await for a handle returned by `nativeSubmitMessage`. Returns a
/// `TransportResponse`, or null if the handle is unknown.
///
/// # Safety
/// Called from Java.
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub unsafe extern "system" fn Java_net_snowflake_client_internal_unicore_JNICoreTransport_nativeAwaitMessage(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jobject {
    ffi_guard(std::ptr::null_mut(), move || {
        match JDBC_BRIDGE.await_result(handle as u64) {
            Some((code, bytes)) => build_transport_response(&mut env, code, &bytes),
            None => std::ptr::null_mut(),
        }
    })
}

/// Cancel an in-flight operation by handle. No-op for unknown/completed handles.
///
/// # Safety
/// Called from Java.
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_net_snowflake_client_internal_unicore_JNICoreTransport_nativeCancel(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    ffi_guard((), || {
        JDBC_BRIDGE.transport.cancel(handle as u64);
    });
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_net_snowflake_client_internal_unicore_CoreLoggingBridge_nativeIsTroubleshooting(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    i32::from(JDBC_BRIDGE.transport.is_troubleshooting())
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

    #[test]
    fn submit_then_await_round_trips_through_encode_result() {
        // Exercises the real submit → spawn → await_result → encode_result path
        // (minus JNI). An unknown service resolves deterministically to a
        // transport error (code 2), so no network/WireMock is needed here;
        // cross-thread cancellation timing is covered by the sf_core tests.
        let handle = JDBC_BRIDGE.submit("UnknownService".into(), "whatever".into(), vec![]);
        let (code, bytes) = JDBC_BRIDGE.await_result(handle).expect("handle is present");
        assert_eq!(code, 2, "unknown service should map to a transport error");
        assert!(String::from_utf8_lossy(&bytes).contains("Unknown API"));
    }

    #[test]
    fn await_unknown_handle_returns_none() {
        assert!(JDBC_BRIDGE.await_result(u64::MAX).is_none());
    }
}
