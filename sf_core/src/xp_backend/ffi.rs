//! C ABI for registering a host backend.
//!
//! # Why plain C FFI reaches every wrapper
//!
//! `sf_core` is a static rlib inside each wrapper's own shared object
//! (`sf_core_python.*.so`, `libjdbc_bridge.so`, `libsfodbc.so`,
//! `libnodejs_bridge.so`), so each wrapper holds its **own** copy of the
//! [`super::registry`] statics — a host must register into the shared object it
//! loaded, not into "sf_core" in the abstract. Rust re-exports
//! `#[unsafe(no_mangle)]` symbols from every dependent cdylib, so these entry
//! points are reachable in each one without a per-wrapper shim. Locating the
//! library belongs to the host: Python reads `module.__file__` and `dlopen`s
//! it with `RTLD_NOLOAD | RTLD_GLOBAL`, JNI hosts already know their path.
//!
//! # Memory ownership
//!
//! * **Host → driver.** Strings written to a `*_out` parameter, and
//!   [`SfCoreFfiError::message`], are host-allocated and freed by the driver with
//!   `libc::free` once copied. Returning non-zero without setting `message` is
//!   fine; the driver reports a generic failure.
//! * **Driver → host.** Strings the driver returns (only
//!   [`sf_core_get_version`] today) are `CString::into_raw` and **must** be freed
//!   with [`sf_core_free_string`]; the host's own allocator is undefined
//!   behaviour.
//!
//! # Callback contract
//!
//! * Callbacks may run concurrently on several threads, so `context` must be
//!   thread-safe.
//! * Callbacks run on Tokio's blocking thread pool.
//! * A callback must not let a C++ exception escape: Rust cannot unwind through
//!   `extern "C"` and an escaping exception aborts the process. Catch at the
//!   boundary and report through [`SfCoreFfiError`]. Panics inside the driver's
//!   own adapter code are caught here.
//! * Function pointers may be null for an operation the host does not implement,
//!   except the six [`SnowflakeBackend`] methods without a default. Registration
//!   is rejected if one of those is null, rather than crashing on first use.

use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char, c_void};
use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{
    BackendCancelOutcome, BackendCancelTarget, BackendError, BackendQueryOptions, BackendResult,
    SnowflakeBackend, error_codes, registry,
};
use crate::config::rest_parameters::{LoginParameters, QueryParameters};
use crate::config::settings::Setting;
use crate::rest::snowflake::{
    LoginResult, QueryExecutionMode, QueryInput, SessionTokens, query_request, query_response,
};
use crate::sensitive::SensitiveString;

/// Bump on any change to [`XpBackendCallbacks`]'s layout or a callback signature.
/// A mismatched host is rejected at registration, so a layout mismatch is an
/// error message rather than undefined behaviour.
pub const XP_BACKEND_ABI_VERSION: u32 = 1;

#[repr(C)]
struct XpBackendCallbacksHeader {
    abi_version: u32,
    struct_size: usize,
}

/// `message` is host-allocated and freed by the driver. `code` should be a
/// Snowflake server error code where the host has one, so it survives to the user
/// unchanged.
#[repr(C)]
pub struct SfCoreFfiError {
    pub code: i32,
    /// NUL-terminated message, or null when the host has nothing to add.
    pub message: *mut c_char,
}

impl SfCoreFfiError {
    fn empty() -> Self {
        Self {
            code: 0,
            message: std::ptr::null_mut(),
        }
    }
}

/// `params_json` is an [`XpQueryRequest`]; `result_out` receives query-response
/// JSON.
pub type ExecuteQueryFn = unsafe extern "C" fn(
    ctx: *mut c_void,
    sql: *const c_char,
    params_json: *const c_char,
    result_out: *mut *mut c_char,
    error_out: *mut SfCoreFfiError,
) -> i32;

/// `result_out` receives the monitoring endpoint's JSON.
pub type GetStatusFn = unsafe extern "C" fn(
    ctx: *mut c_void,
    query_id: *const c_char,
    result_out: *mut *mut c_char,
    error_out: *mut SfCoreFfiError,
) -> i32;

/// `params_json` is an [`XpCancelRequest`]; `result_out` an [`XpCancelResponse`].
pub type CancelQueryFn = unsafe extern "C" fn(
    ctx: *mut c_void,
    params_json: *const c_char,
    result_out: *mut *mut c_char,
    error_out: *mut SfCoreFfiError,
) -> i32;

/// `result_out` receives query-response JSON.
pub type GetResultFn = unsafe extern "C" fn(
    ctx: *mut c_void,
    query_id: *const c_char,
    result_out: *mut *mut c_char,
    error_out: *mut SfCoreFfiError,
) -> i32;

/// `params_json` is an [`XpAuthRequest`]; `result_out` an [`XpSessionInfo`].
pub type AuthenticateFn = unsafe extern "C" fn(
    ctx: *mut c_void,
    params_json: *const c_char,
    result_out: *mut *mut c_char,
    error_out: *mut SfCoreFfiError,
) -> i32;

/// `result_out` receives a JSON object of session parameter name to value.
pub type GetSessionParamsFn = unsafe extern "C" fn(
    ctx: *mut c_void,
    result_out: *mut *mut c_char,
    error_out: *mut SfCoreFfiError,
) -> i32;

pub type UploadStreamFn = unsafe extern "C" fn(
    ctx: *mut c_void,
    destination: *const c_char,
    data: *const u8,
    data_len: usize,
    metadata_json: *const c_char,
    result_out: *mut *mut c_char,
    error_out: *mut SfCoreFfiError,
) -> i32;

/// On success `data_out` receives a host-allocated buffer of `data_len_out`
/// bytes, freed by the driver with `libc::free`.
pub type DownloadStreamFn = unsafe extern "C" fn(
    ctx: *mut c_void,
    source: *const c_char,
    metadata_json: *const c_char,
    data_out: *mut *mut u8,
    data_len_out: *mut usize,
    error_out: *mut SfCoreFfiError,
) -> i32;

pub type ListStageFilesFn = unsafe extern "C" fn(
    ctx: *mut c_void,
    stage_location: *const c_char,
    is_exact_match: i32,
    result_out: *mut *mut c_char,
    error_out: *mut SfCoreFfiError,
) -> i32;

/// One struct so registration is atomic: there is no way to install a query
/// executor without also providing authentication.
///
/// `abi_version` and `struct_size` come first and must be set to
/// [`XP_BACKEND_ABI_VERSION`] and `sizeof(XpBackendCallbacks)`, so the driver can
/// reject a mismatched host instead of reading past the end of a struct laid out
/// differently.
// Not a doc comment, to keep it out of the generated header: the fields spell
// their signatures out rather than using the `*Fn` aliases because cbindgen
// cannot see through an alias inside `Option<..>` and emits each field as an
// opaque struct. They cannot drift from the aliases — `CBackendAdapter::new`
// assigns each field into a binding of the alias type, so divergence is a
// compile error.
#[repr(C)]
pub struct XpBackendCallbacks {
    pub abi_version: u32,
    pub struct_size: usize,
    /// Opaque host state, passed back to every callback. Must be thread-safe
    /// and must outlive the process's use of the driver.
    pub context: *mut c_void,
    pub execute_query: Option<
        unsafe extern "C" fn(
            ctx: *mut c_void,
            sql: *const c_char,
            params_json: *const c_char,
            result_out: *mut *mut c_char,
            error_out: *mut SfCoreFfiError,
        ) -> i32,
    >,
    pub get_query_status: Option<
        unsafe extern "C" fn(
            ctx: *mut c_void,
            query_id: *const c_char,
            result_out: *mut *mut c_char,
            error_out: *mut SfCoreFfiError,
        ) -> i32,
    >,
    pub cancel_query: Option<
        unsafe extern "C" fn(
            ctx: *mut c_void,
            params_json: *const c_char,
            result_out: *mut *mut c_char,
            error_out: *mut SfCoreFfiError,
        ) -> i32,
    >,
    pub get_query_result: Option<
        unsafe extern "C" fn(
            ctx: *mut c_void,
            query_id: *const c_char,
            result_out: *mut *mut c_char,
            error_out: *mut SfCoreFfiError,
        ) -> i32,
    >,
    pub authenticate: Option<
        unsafe extern "C" fn(
            ctx: *mut c_void,
            params_json: *const c_char,
            result_out: *mut *mut c_char,
            error_out: *mut SfCoreFfiError,
        ) -> i32,
    >,
    pub get_session_parameters: Option<
        unsafe extern "C" fn(
            ctx: *mut c_void,
            result_out: *mut *mut c_char,
            error_out: *mut SfCoreFfiError,
        ) -> i32,
    >,
    pub upload_stream: Option<
        unsafe extern "C" fn(
            ctx: *mut c_void,
            destination: *const c_char,
            data: *const u8,
            data_len: usize,
            metadata_json: *const c_char,
            result_out: *mut *mut c_char,
            error_out: *mut SfCoreFfiError,
        ) -> i32,
    >,
    pub download_stream: Option<
        unsafe extern "C" fn(
            ctx: *mut c_void,
            source: *const c_char,
            metadata_json: *const c_char,
            data_out: *mut *mut u8,
            data_len_out: *mut usize,
            error_out: *mut SfCoreFfiError,
        ) -> i32,
    >,
    pub list_stage_files: Option<
        unsafe extern "C" fn(
            ctx: *mut c_void,
            stage_location: *const c_char,
            is_exact_match: i32,
            result_out: *mut *mut c_char,
            error_out: *mut SfCoreFfiError,
        ) -> i32,
    >,
}

// ---------------------------------------------------------------------------
// JSON payloads on the ABI
// ---------------------------------------------------------------------------

/// Field names mirror XP's existing `_snowflake.execute_sql` parameters, so a host
/// already implementing that call has nothing to translate.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct XpQueryRequest {
    #[serde(default)]
    pub is_describe_only: bool,
    #[serde(default)]
    pub is_async: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statement_params: Option<HashMap<String, serde_json::Value>>,
    /// Bind variables in the driver's on-the-wire shape.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bindings: Option<serde_json::Value>,
    /// Stage holding the bindings when they were too large to inline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_stage: Option<String>,
    /// For idempotent retry, and to cancel a query whose id is not yet known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_context: Option<serde_json::Value>,
}

/// Exactly one of `query_id` and `request_id` is set, depending on whether the
/// submission had come back with an id before the cancellation was issued.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct XpCancelRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sql_text: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct XpCancelResponse {
    /// `false` when nothing was running — a normal outcome, so it is reported
    /// here rather than as an error.
    pub cancelled: bool,
}

/// Carries **no credentials**: it describes which session is wanted, not how to
/// obtain one. Nothing from `LoginParameters::login_method` is serialized, so
/// passwords, tokens and private keys never cross the ABI.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct XpAuthRequest {
    pub account_name: String,
    pub server_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warehouse: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_parameters: Option<HashMap<String, String>>,
    pub client_app_id: String,
    pub client_app_version: String,
}

/// [`LoginResult`] holds `Instant` deadlines and is not deserializable, so
/// validity crosses the ABI as seconds and becomes a deadline relative to now.
#[derive(Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct XpSessionInfo {
    pub session_token: String,
    #[serde(default)]
    pub master_token: String,
    pub session_id: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validity_in_seconds: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub master_validity_in_seconds: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_parameters: Option<HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warehouse_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_version: Option<String>,
}

impl From<XpSessionInfo> for LoginResult {
    fn from(info: XpSessionInfo) -> Self {
        let now = std::time::Instant::now();
        let session_validity = info.validity_in_seconds.map(std::time::Duration::from_secs);
        let master_validity = info
            .master_validity_in_seconds
            .map(std::time::Duration::from_secs);

        LoginResult {
            tokens: SessionTokens {
                session_token: SensitiveString::from(info.session_token),
                master_token: SensitiveString::from(info.master_token),
                session_id: info.session_id,
                session_expires_at: session_validity.map(|d| now + d),
                master_expires_at: master_validity.map(|d| now + d),
                master_validity,
            },
            session_parameters: info.session_parameters.map(|params| {
                params
                    .into_iter()
                    .map(|(key, value)| (key, Setting::String(value)))
                    .collect()
            }),
            database_name: info.database_name,
            schema_name: info.schema_name,
            warehouse_name: info.warehouse_name,
            role_name: info.role_name,
            server_version: info.server_version,
        }
    }
}

// ---------------------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------------------

/// Returns `0`, or a negative [`error_codes`] value.
///
/// # Safety
///
/// `callbacks` must point to readable storage containing the `abi_version` and
/// `struct_size` prefix. When those values match this build, it must point to a
/// valid `XpBackendCallbacks`, with `context` and every non-null function
/// pointer remaining valid for as long as the process uses the driver.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sf_core_register_xp_backend(callbacks: *const XpBackendCallbacks) -> i32 {
    let outcome = panic::catch_unwind(|| {
        if callbacks.is_null() {
            return Err(BackendError::invalid_argument(
                "callbacks pointer is null".to_string(),
            ));
        }
        let header = callbacks.cast::<XpBackendCallbacksHeader>();
        // SAFETY: the caller guarantees a readable header.
        let abi_version = unsafe { std::ptr::addr_of!((*header).abi_version).read_unaligned() };

        if abi_version != XP_BACKEND_ABI_VERSION {
            return Err(BackendError::abi_mismatch(
                XP_BACKEND_ABI_VERSION,
                abi_version,
            ));
        }
        // SAFETY: the caller guarantees a readable header.
        let struct_size = unsafe { std::ptr::addr_of!((*header).struct_size).read_unaligned() };
        let expected_size = std::mem::size_of::<XpBackendCallbacks>();
        if struct_size != expected_size {
            return Err(BackendError::new(
                error_codes::ABI_MISMATCH,
                format!(
                    "backend callbacks struct size mismatch: driver expects {expected_size} \
                     bytes, host sent {struct_size}"
                ),
            ));
        }

        // SAFETY: the validated size and the caller's contract guarantee the full layout.
        let cb = unsafe { &*callbacks };
        let adapter = CBackendAdapter::new(cb)?;
        registry::register_backend(Arc::new(adapter))
    });

    match outcome {
        Ok(Ok(())) => 0,
        Ok(Err(err)) => {
            tracing::error!(code = err.code, "failed to register host backend: {err}");
            err.code
        }
        Err(_) => {
            tracing::error!("panic caught in sf_core_register_xp_backend");
            error_codes::PANIC
        }
    }
}

/// Always `1`. Lets a host tell "driver too old to know about backends" (symbol
/// absent, `dlsym` fails) from "backends supported", without parsing a version.
#[unsafe(no_mangle)]
pub extern "C" fn sf_core_xp_backend_is_available() -> i32 {
    1
}

/// Returns `0`. The caller **must** release the string with
/// [`sf_core_free_string`].
///
/// # Safety
///
/// `version_out` must be a valid, writable pointer to a `*mut c_char`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sf_core_get_version(version_out: *mut *mut c_char) -> i32 {
    let outcome = panic::catch_unwind(|| {
        if version_out.is_null() {
            return error_codes::INVALID_ARGUMENT;
        }
        // Cannot fail for a compile-time literal, but a panic must not cross the
        // ABI, so no unwrap.
        match CString::new(env!("CARGO_PKG_VERSION")) {
            Ok(version) => {
                // SAFETY: non-null per the check above; writability is the
                // caller's contract.
                unsafe { *version_out = version.into_raw() };
                0
            }
            Err(_) => error_codes::INVALID_ARGUMENT,
        }
    });

    outcome.unwrap_or_else(|_| {
        tracing::error!("panic caught in sf_core_get_version");
        error_codes::PANIC
    })
}

/// Free a string the driver allocated. No-op on null.
///
/// # Safety
///
/// `ptr` must be null or a not-yet-freed pointer returned by an `sf_core`
/// function. A host-allocated pointer is undefined behaviour: the allocators
/// differ.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sf_core_free_string(ptr: *mut c_char) {
    let _ = panic::catch_unwind(|| {
        if !ptr.is_null() {
            // SAFETY: per the contract, this came from CString::into_raw in this
            // library and has not been freed.
            drop(unsafe { CString::from_raw(ptr) });
        }
    });
}

// ---------------------------------------------------------------------------
// Adapter
// ---------------------------------------------------------------------------

/// Implements [`SnowflakeBackend`] by calling the host's C callbacks.
struct CBackendAdapter {
    context: HostContext,
    execute_query: ExecuteQueryFn,
    get_query_status: GetStatusFn,
    cancel_query: CancelQueryFn,
    get_query_result: GetResultFn,
    authenticate: AuthenticateFn,
    get_session_parameters: GetSessionParamsFn,
    upload_stream: Option<UploadStreamFn>,
    download_stream: Option<DownloadStreamFn>,
    list_stage_files: Option<ListStageFilesFn>,
}

// SAFETY: the host contract requires `context` to be safe for concurrent use, and
// the function pointers are immutable after registration.
unsafe impl Send for CBackendAdapter {}
unsafe impl Sync for CBackendAdapter {}

#[derive(Clone, Copy)]
struct HostContext(*mut c_void);

// SAFETY: the host contract requires the context to be safe for concurrent use.
unsafe impl Send for HostContext {}
unsafe impl Sync for HostContext {}

impl HostContext {
    fn as_ptr(self) -> *mut c_void {
        self.0
    }
}

impl CBackendAdapter {
    /// Checking the required callbacks here makes a null pointer a rejected
    /// registration rather than a segfault on the first query.
    fn new(cb: &XpBackendCallbacks) -> BackendResult<Self> {
        fn required<T>(slot: Option<T>, name: &str) -> BackendResult<T> {
            slot.ok_or_else(|| {
                BackendError::invalid_argument(format!("required callback `{name}` is null"))
            })
        }

        Ok(Self {
            context: HostContext(cb.context),
            execute_query: required(cb.execute_query, "execute_query")?,
            get_query_status: required(cb.get_query_status, "get_query_status")?,
            cancel_query: required(cb.cancel_query, "cancel_query")?,
            get_query_result: required(cb.get_query_result, "get_query_result")?,
            authenticate: required(cb.authenticate, "authenticate")?,
            get_session_parameters: required(cb.get_session_parameters, "get_session_parameters")?,
            upload_stream: cb.upload_stream,
            download_stream: cb.download_stream,
            list_stage_files: cb.list_stage_files,
        })
    }
}

/// Not an `unwrap`: SQL arrives from user code and can contain anything, and a
/// stray NUL byte would panic across the ABI.
fn to_c_string(value: &str, what: &str) -> BackendResult<CString> {
    CString::new(value).map_err(|_| {
        BackendError::invalid_argument(format!("{what} contains an interior NUL byte"))
    })
}

/// Consume a callback's return code, output, and error, taking ownership of all
/// host allocations.
///
/// # Safety
///
/// `error.message` and allocations consumed by the closures must satisfy their
/// respective host-allocation contracts.
unsafe fn take_result<T>(
    ret: i32,
    error: SfCoreFfiError,
    operation: &str,
    take_output: impl FnOnce() -> BackendResult<T>,
    discard_output: impl FnOnce(),
) -> BackendResult<T> {
    let host_message = unsafe { take_c_string(error.message) };

    if ret != 0 || host_message.is_some() {
        discard_output();
        let message = host_message
            .unwrap_or_else(|| format!("backend operation `{operation}` failed without a message"));
        let code = if error.code != 0 {
            error.code
        } else {
            error_codes::PROTOCOL
        };
        return Err(BackendError::new(code, message));
    }

    take_output()
}

unsafe fn take_string_result(
    ret: i32,
    result: *mut c_char,
    error: SfCoreFfiError,
    operation: &str,
) -> BackendResult<Option<String>> {
    unsafe {
        take_result(
            ret,
            error,
            operation,
            || Ok(take_c_string(result)),
            || {
                take_c_string(result);
            },
        )
    }
}

/// Copy a host-allocated C string and free it. Non-UTF-8 bytes are replaced
/// rather than rejected: the string is usually a diagnostic, and losing it
/// entirely is worse than decoding it lossily.
///
/// # Safety
///
/// `ptr` must be null or a host-allocated NUL-terminated string whose ownership
/// has passed to the driver.
unsafe fn take_c_string(ptr: *mut c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    // SAFETY: non-null and NUL-terminated per the contract.
    let owned = unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned();
    // SAFETY: host malloc/strdup allocation, ownership transferred.
    unsafe { libc::free(ptr.cast::<c_void>()) };
    Some(owned)
}

fn require_payload(payload: Option<String>, operation: &str) -> BackendResult<String> {
    payload.ok_or_else(|| {
        BackendError::protocol(format!(
            "backend operation `{operation}` reported success but returned no payload"
        ))
    })
}

fn parse_json<T: serde::de::DeserializeOwned>(json: &str, operation: &str) -> BackendResult<T> {
    serde_json::from_str(json).map_err(|e| {
        BackendError::protocol(format!(
            "backend operation `{operation}` returned a payload the driver could not parse: {e}"
        ))
    })
}

fn to_json<T: Serialize>(value: &T, operation: &str) -> BackendResult<String> {
    serde_json::to_string(value).map_err(|e| {
        BackendError::protocol(format!(
            "could not serialize the request for backend operation `{operation}`: {e}"
        ))
    })
}

fn query_context_json(
    query_context: &query_request::QueryContext,
) -> BackendResult<Option<serde_json::Value>> {
    let value = serde_json::to_value(query_context).map_err(|e| {
        BackendError::protocol(format!(
            "could not serialize the request for backend operation `execute_query`: {e}"
        ))
    })?;
    match value {
        serde_json::Value::Object(map) if map.is_empty() => Ok(None),
        value => Ok(Some(value)),
    }
}

/// Guards the driver's own work either side of a callback — serialization, string
/// conversion, JSON parsing — so a bug there surfaces as a failed query rather
/// than unwinding into a host that cannot handle it. A C++ exception escaping the
/// callback itself is already undefined behaviour; see the module docs.
fn guard<T>(operation: &str, f: impl FnOnce() -> BackendResult<T>) -> BackendResult<T> {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(result) => result,
        Err(_) => {
            tracing::error!(operation, "panic caught in backend adapter");
            Err(BackendError::panicked(operation))
        }
    }
}

async fn run_callback<T: Send + 'static>(
    operation: &'static str,
    callback: impl FnOnce() -> BackendResult<T> + Send + 'static,
) -> BackendResult<T> {
    match tokio::task::spawn_blocking(move || guard(operation, callback)).await {
        Ok(result) => result,
        Err(error) => {
            tracing::error!(
                operation,
                cause = std::any::type_name_of_val(&error),
                "backend callback task failed"
            );
            tracing::debug!(operation, %error, "backend callback task failure detail");
            Err(BackendError::panicked(operation))
        }
    }
}

#[async_trait]
impl SnowflakeBackend for CBackendAdapter {
    async fn authenticate(
        &self,
        params: &LoginParameters,
        session_parameters: Option<&HashMap<String, String>>,
    ) -> BackendResult<LoginResult> {
        const OP: &str = "authenticate";
        let request = guard(OP, || {
            Ok(XpAuthRequest {
                account_name: params.account_name.clone(),
                server_url: params.server_url.clone(),
                database: params.database.clone(),
                schema: params.schema.clone(),
                warehouse: params.warehouse.clone(),
                role: params.role.clone(),
                session_parameters: session_parameters
                    .cloned()
                    .or_else(|| params.session_parameters.clone()),
                client_app_id: params.client_info.client_app_id.clone(),
                client_app_version: params.client_info.version.clone(),
            })
        })?;
        let context = self.context;
        let callback = self.authenticate;
        let payload = run_callback(OP, move || {
            let params_c = to_c_string(&to_json(&request, OP)?, "authenticate request")?;

            let mut result = std::ptr::null_mut();
            let mut error = SfCoreFfiError::empty();
            let ret =
                unsafe { callback(context.as_ptr(), params_c.as_ptr(), &mut result, &mut error) };
            unsafe { take_string_result(ret, result, error, OP) }
        })
        .await?;
        guard(OP, || {
            let info: XpSessionInfo = parse_json(&require_payload(payload, OP)?, OP)?;
            Ok(info.into())
        })
    }

    async fn execute_query(
        &self,
        input: &QueryInput<'_>,
        _params: &QueryParameters,
        options: BackendQueryOptions,
    ) -> BackendResult<query_response::Response> {
        const OP: &str = "execute_query";
        let (request, sql) = guard(OP, || {
            Ok((
                XpQueryRequest {
                    is_describe_only: input.describe_only.unwrap_or(false),
                    is_async: matches!(options.execution_mode, QueryExecutionMode::Async),
                    statement_params: input.query_parameters.clone(),
                    bindings: input
                        .bindings
                        .map(|raw| parse_json::<serde_json::Value>(raw.get(), OP))
                        .transpose()?,
                    bind_stage: input.bind_stage.clone(),
                    request_id: Some(options.request_id.to_string()),
                    query_context: query_context_json(&input.query_context)?,
                },
                input.sql.to_string(),
            ))
        })?;
        let context = self.context;
        let callback = self.execute_query;
        let payload = run_callback(OP, move || {
            let sql_c = to_c_string(&sql, "SQL text")?;
            let params_c = to_c_string(&to_json(&request, OP)?, "query request")?;

            let mut result = std::ptr::null_mut();
            let mut error = SfCoreFfiError::empty();
            let ret = unsafe {
                callback(
                    context.as_ptr(),
                    sql_c.as_ptr(),
                    params_c.as_ptr(),
                    &mut result,
                    &mut error,
                )
            };
            unsafe { take_string_result(ret, result, error, OP) }
        })
        .await?;
        guard(OP, || parse_json(&require_payload(payload, OP)?, OP))
    }

    async fn get_query_status(&self, query_id: &str) -> BackendResult<String> {
        const OP: &str = "get_query_status";
        let query_id = query_id.to_string();
        let context = self.context;
        let callback = self.get_query_status;
        let payload = run_callback(OP, move || {
            let query_id_c = to_c_string(&query_id, "query id")?;
            let mut result = std::ptr::null_mut();
            let mut error = SfCoreFfiError::empty();
            let ret = unsafe {
                callback(
                    context.as_ptr(),
                    query_id_c.as_ptr(),
                    &mut result,
                    &mut error,
                )
            };
            unsafe { take_string_result(ret, result, error, OP) }
        })
        .await?;
        guard(OP, || require_payload(payload, OP))
    }

    async fn get_query_result(&self, query_id: &str) -> BackendResult<query_response::Response> {
        const OP: &str = "get_query_result";
        let query_id = query_id.to_string();
        let context = self.context;
        let callback = self.get_query_result;
        let payload = run_callback(OP, move || {
            let query_id_c = to_c_string(&query_id, "query id")?;
            let mut result = std::ptr::null_mut();
            let mut error = SfCoreFfiError::empty();
            let ret = unsafe {
                callback(
                    context.as_ptr(),
                    query_id_c.as_ptr(),
                    &mut result,
                    &mut error,
                )
            };
            unsafe { take_string_result(ret, result, error, OP) }
        })
        .await?;
        guard(OP, || parse_json(&require_payload(payload, OP)?, OP))
    }

    async fn cancel_query(
        &self,
        target: BackendCancelTarget<'_>,
    ) -> BackendResult<BackendCancelOutcome> {
        const OP: &str = "cancel_query";
        let request = guard(OP, || {
            Ok(match target {
                BackendCancelTarget::QueryId(query_id) => XpCancelRequest {
                    query_id: Some(query_id.to_string()),
                    request_id: None,
                    sql_text: None,
                },
                BackendCancelTarget::RequestId {
                    request_id,
                    sql_text,
                } => XpCancelRequest {
                    query_id: None,
                    request_id: Some(request_id.to_string()),
                    sql_text: Some(sql_text.to_string()),
                },
            })
        })?;
        let context = self.context;
        let callback = self.cancel_query;
        let payload = run_callback(OP, move || {
            let params_c = to_c_string(&to_json(&request, OP)?, "cancel request")?;

            let mut result = std::ptr::null_mut();
            let mut error = SfCoreFfiError::empty();
            let ret =
                unsafe { callback(context.as_ptr(), params_c.as_ptr(), &mut result, &mut error) };
            unsafe { take_string_result(ret, result, error, OP) }
        })
        .await?;
        guard(OP, || {
            let response: XpCancelResponse = parse_json(&require_payload(payload, OP)?, OP)?;
            Ok(if response.cancelled {
                BackendCancelOutcome::Cancelled
            } else {
                BackendCancelOutcome::NotRunning
            })
        })
    }

    async fn get_session_parameters(&self) -> BackendResult<HashMap<String, String>> {
        const OP: &str = "get_session_parameters";
        let context = self.context;
        let callback = self.get_session_parameters;
        let payload = run_callback(OP, move || {
            let mut result = std::ptr::null_mut();
            let mut error = SfCoreFfiError::empty();
            let ret = unsafe { callback(context.as_ptr(), &mut result, &mut error) };
            unsafe { take_string_result(ret, result, error, OP) }
        })
        .await?;
        guard(OP, || parse_json(&require_payload(payload, OP)?, OP))
    }

    async fn upload_stream(
        &self,
        destination: &str,
        data: &[u8],
        metadata_json: &str,
    ) -> BackendResult<String> {
        const OP: &str = "upload_stream";
        let Some(callback) = self.upload_stream else {
            return Err(BackendError::unsupported(OP));
        };
        let destination = destination.to_string();
        let data = data.to_vec();
        let metadata_json = metadata_json.to_string();
        let context = self.context;
        let payload = run_callback(OP, move || {
            let destination_c = to_c_string(&destination, "upload destination")?;
            let metadata_c = to_c_string(&metadata_json, "upload metadata")?;
            let mut result = std::ptr::null_mut();
            let mut error = SfCoreFfiError::empty();
            let ret = unsafe {
                callback(
                    context.as_ptr(),
                    destination_c.as_ptr(),
                    data.as_ptr(),
                    data.len(),
                    metadata_c.as_ptr(),
                    &mut result,
                    &mut error,
                )
            };
            unsafe { take_string_result(ret, result, error, OP) }
        })
        .await?;
        guard(OP, || require_payload(payload, OP))
    }

    async fn download_stream(&self, source: &str, metadata_json: &str) -> BackendResult<Vec<u8>> {
        const OP: &str = "download_stream";
        let Some(callback) = self.download_stream else {
            return Err(BackendError::unsupported(OP));
        };
        let source = source.to_string();
        let metadata_json = metadata_json.to_string();
        let context = self.context;
        run_callback(OP, move || {
            let source_c = to_c_string(&source, "download source")?;
            let metadata_c = to_c_string(&metadata_json, "download metadata")?;
            let mut data: *mut u8 = std::ptr::null_mut();
            let mut data_len: usize = 0;
            let mut error = SfCoreFfiError::empty();
            let ret = unsafe {
                callback(
                    context.as_ptr(),
                    source_c.as_ptr(),
                    metadata_c.as_ptr(),
                    &mut data,
                    &mut data_len,
                    &mut error,
                )
            };

            unsafe {
                take_result(
                    ret,
                    error,
                    OP,
                    || take_host_buffer(data, data_len, OP),
                    || free_host_buffer(data),
                )
            }
        })
        .await
    }

    async fn list_stage_files(
        &self,
        stage_location: &str,
        is_exact_match: bool,
    ) -> BackendResult<String> {
        const OP: &str = "list_stage_files";
        let Some(callback) = self.list_stage_files else {
            return Err(BackendError::unsupported(OP));
        };
        let stage_location = stage_location.to_string();
        let context = self.context;
        let payload = run_callback(OP, move || {
            let stage_c = to_c_string(&stage_location, "stage location")?;
            let mut result = std::ptr::null_mut();
            let mut error = SfCoreFfiError::empty();
            let ret = unsafe {
                callback(
                    context.as_ptr(),
                    stage_c.as_ptr(),
                    i32::from(is_exact_match),
                    &mut result,
                    &mut error,
                )
            };
            unsafe { take_string_result(ret, result, error, OP) }
        })
        .await?;
        guard(OP, || require_payload(payload, OP))
    }
}

unsafe fn take_host_buffer(ptr: *mut u8, len: usize, operation: &str) -> BackendResult<Vec<u8>> {
    if ptr.is_null() {
        if len != 0 {
            return Err(BackendError::protocol(format!(
                "backend operation `{operation}` reported {len} bytes but no buffer"
            )));
        }
        return Ok(Vec::new());
    }

    // SAFETY: the host reported `len` readable bytes at `ptr`.
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) }.to_vec();
    unsafe { free_host_buffer(ptr) };
    Ok(bytes)
}

/// # Safety
///
/// `ptr` must be null or a host `malloc` allocation transferred to the driver.
unsafe fn free_host_buffer(ptr: *mut u8) {
    if !ptr.is_null() {
        // SAFETY: per the contract above.
        unsafe { libc::free(ptr.cast::<c_void>()) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_request_omits_absent_fields() {
        let json = serde_json::to_string(&XpQueryRequest::default()).unwrap();
        assert_eq!(json, r#"{"is_describe_only":false,"is_async":false}"#);
    }

    #[test]
    fn query_request_round_trips() {
        let request = XpQueryRequest {
            is_describe_only: true,
            is_async: false,
            statement_params: Some(HashMap::from([(
                "TIMEZONE".to_string(),
                serde_json::json!("UTC"),
            )])),
            bindings: Some(serde_json::json!({"1": {"type": "TEXT", "value": "a"}})),
            bind_stage: Some("@stage/binds".to_string()),
            request_id: Some("b0a1".to_string()),
            query_context: Some(serde_json::json!({
                "entries": [{"id": 1, "priority": 0, "timestamp": 1}]
            })),
        };
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(
            serde_json::from_str::<XpQueryRequest>(&json).unwrap(),
            request
        );
    }

    #[test]
    fn auth_request_carries_no_credentials() {
        let json = serde_json::to_string(&XpAuthRequest {
            account_name: "acct".to_string(),
            server_url: "https://acct.snowflakecomputing.com".to_string(),
            client_app_id: "PythonConnector".to_string(),
            client_app_version: "5.0.0".to_string(),
            ..Default::default()
        })
        .unwrap();

        for forbidden in ["password", "token", "private_key", "passcode", "secret"] {
            assert!(
                !json.contains(forbidden),
                "auth request must not carry credentials, found {forbidden:?} in {json}"
            );
        }
    }

    #[test]
    fn session_info_becomes_a_login_result() {
        let info = XpSessionInfo {
            session_token: "session".to_string(),
            master_token: "master".to_string(),
            session_id: 42,
            validity_in_seconds: Some(3600),
            master_validity_in_seconds: Some(14400),
            session_parameters: Some(HashMap::from([(
                "AUTOCOMMIT".to_string(),
                "true".to_string(),
            )])),
            database_name: Some("DB".to_string()),
            schema_name: Some("PUBLIC".to_string()),
            warehouse_name: Some("WH".to_string()),
            role_name: Some("ROLE".to_string()),
            server_version: Some("9.1.0".to_string()),
        };

        let result: LoginResult = info.into();
        assert_eq!(result.tokens.session_token.reveal(), "session");
        assert_eq!(result.tokens.master_token.reveal(), "master");
        assert_eq!(result.tokens.session_id, 42);
        assert_eq!(
            result.tokens.master_validity,
            Some(std::time::Duration::from_secs(14400))
        );
        assert!(!result.tokens.is_session_expired());
        assert!(!result.tokens.is_master_expired());
        assert_eq!(result.database_name.as_deref(), Some("DB"));
        assert_eq!(result.server_version.as_deref(), Some("9.1.0"));
        assert_eq!(
            result
                .session_parameters
                .as_ref()
                .map(|params| params.get("AUTOCOMMIT")),
            Some(Some(&Setting::String("true".to_string())))
        );
    }

    #[test]
    fn session_info_without_validity_never_looks_expired() {
        // XP's session has no client-visible expiry, and "expired at the epoch"
        // would trigger a refresh the host cannot serve.
        let result: LoginResult = XpSessionInfo {
            session_token: "s".to_string(),
            session_id: 1,
            ..Default::default()
        }
        .into();

        assert!(!result.tokens.is_session_expired());
        assert!(!result.tokens.is_master_expired());
        assert_eq!(result.tokens.master_validity, None);
    }

    #[test]
    fn a_real_query_response_body_parses() {
        // The shape GS and `_snowflake.execute_sql` already return, which is why
        // the wire format needed no invention.
        let body = r#"{
            "data": {
                "rowtype": [
                    {"name": "1", "type": "fixed", "length": null, "scale": 0,
                     "precision": 1, "nullable": false}
                ],
                "rowset": [["1"]],
                "total": 1,
                "queryId": "01b2c3d4-0000-0000-0000-000000000001",
                "queryResultFormat": "json"
            },
            "code": null,
            "message": null,
            "success": true
        }"#;

        let response: query_response::Response = parse_json(body, "execute_query").unwrap();
        assert!(response.success);
        assert_eq!(
            response.data.rowset.as_ref().expect("rowset present"),
            &vec![vec![Some("1".to_string())]]
        );
    }

    #[test]
    fn an_error_body_parses_and_keeps_its_code() {
        let body = r#"{"data": null, "code": "1003", "message": "SQL compilation error",
                       "success": false}"#;
        let response: query_response::Response = parse_json(body, "execute_query").unwrap();
        assert!(!response.success);
        assert_eq!(response.code.as_deref(), Some("1003"));
        assert_eq!(response.message.as_deref(), Some("SQL compilation error"));
    }

    #[test]
    fn malformed_payloads_are_protocol_errors_not_panics() {
        // `query_response::Data` is not `Debug`, so discard the Ok side.
        let err = parse_json::<query_response::Response>("not json at all", "execute_query")
            .map(|_| ())
            .expect_err("should not parse");
        assert_eq!(err.code, error_codes::PROTOCOL);
        assert!(err.message.contains("execute_query"));
    }

    #[test]
    fn interior_nul_in_sql_is_an_error_not_a_panic() {
        let err = to_c_string("select 1\0 -- trailing", "SQL text").expect_err("should reject");
        assert_eq!(err.code, error_codes::INVALID_ARGUMENT);
        assert!(err.message.contains("SQL text"));
    }

    #[test]
    fn missing_payload_on_success_is_a_protocol_error() {
        let err = require_payload(None, "execute_query").expect_err("should reject");
        assert_eq!(err.code, error_codes::PROTOCOL);
        assert!(err.message.contains("no payload"));
    }

    #[test]
    fn guard_converts_a_panic_into_an_error() {
        let err = guard("execute_query", || -> BackendResult<()> {
            panic!("simulated adapter bug");
        })
        .expect_err("panic should be caught");
        assert_eq!(err.code, error_codes::PANIC);
        assert!(err.message.contains("execute_query"));
    }

    /// 64-bit only: the expected size includes the padding after `abi_version`,
    /// which a 32-bit target does not need.
    #[test]
    #[cfg(target_pointer_width = "64")]
    fn callbacks_layout_is_pinned_to_the_declared_abi_version() {
        assert_eq!(XP_BACKEND_ABI_VERSION, 1);
        // u32 + 4 padding + struct_size + context + 9 function pointers.
        assert_eq!(
            std::mem::size_of::<XpBackendCallbacks>(),
            4 + 4 + 8 + 8 + 9 * 8,
            "layout changed: bump XP_BACKEND_ABI_VERSION and regenerate \
             include/sf_core_providers.h"
        );
    }

    #[test]
    fn registration_rejects_a_short_struct_before_reading_the_full_layout() {
        let header = XpBackendCallbacksHeader {
            abi_version: XP_BACKEND_ABI_VERSION,
            struct_size: std::mem::size_of::<XpBackendCallbacksHeader>(),
        };

        let code = unsafe {
            sf_core_register_xp_backend((&raw const header).cast::<XpBackendCallbacks>())
        };
        assert_eq!(code, error_codes::ABI_MISMATCH);
    }

    #[test]
    fn callback_failure_without_message_preserves_the_host_code() {
        let mut discarded = false;
        let error = SfCoreFfiError {
            code: 1003,
            message: std::ptr::null_mut(),
        };

        let err = unsafe {
            take_result(
                -1,
                error,
                "download_stream",
                || Ok(Vec::<u8>::new()),
                || discarded = true,
            )
        }
        .expect_err("callback failure should be returned");

        assert_eq!(err.code, 1003);
        assert!(discarded);
    }

    #[test]
    fn nullable_function_pointers_do_not_change_the_layout() {
        // A niche-optimized `Option<fn>` is what makes the cbindgen header
        // describe this struct correctly.
        assert_eq!(
            std::mem::size_of::<Option<ExecuteQueryFn>>(),
            std::mem::size_of::<*const c_void>()
        );
    }

    unsafe extern "C" fn unused_execute(
        _ctx: *mut c_void,
        _sql: *const c_char,
        _params_json: *const c_char,
        _result_out: *mut *mut c_char,
        _error_out: *mut SfCoreFfiError,
    ) -> i32 {
        -1
    }

    unsafe extern "C" fn unused_status(
        _ctx: *mut c_void,
        _query_id: *const c_char,
        _result_out: *mut *mut c_char,
        _error_out: *mut SfCoreFfiError,
    ) -> i32 {
        -1
    }

    unsafe extern "C" fn unused_cancel(
        _ctx: *mut c_void,
        _params_json: *const c_char,
        _result_out: *mut *mut c_char,
        _error_out: *mut SfCoreFfiError,
    ) -> i32 {
        -1
    }

    unsafe extern "C" fn unused_result(
        _ctx: *mut c_void,
        _query_id: *const c_char,
        _result_out: *mut *mut c_char,
        _error_out: *mut SfCoreFfiError,
    ) -> i32 {
        -1
    }

    unsafe extern "C" fn unused_auth(
        _ctx: *mut c_void,
        _params_json: *const c_char,
        _result_out: *mut *mut c_char,
        _error_out: *mut SfCoreFfiError,
    ) -> i32 {
        -1
    }

    unsafe extern "C" fn unused_session_params(
        _ctx: *mut c_void,
        _result_out: *mut *mut c_char,
        _error_out: *mut SfCoreFfiError,
    ) -> i32 {
        -1
    }

    struct BlockingCallbackState {
        release: std::sync::atomic::AtomicBool,
        observed_release: std::sync::atomic::AtomicBool,
    }

    unsafe extern "C" fn waiting_status(
        ctx: *mut c_void,
        _query_id: *const c_char,
        result_out: *mut *mut c_char,
        _error_out: *mut SfCoreFfiError,
    ) -> i32 {
        // SAFETY: the test keeps the state alive until the callback returns.
        let state = unsafe { &*ctx.cast::<BlockingCallbackState>() };
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        while !state.release.load(std::sync::atomic::Ordering::Relaxed)
            && std::time::Instant::now() < deadline
        {
            std::thread::yield_now();
        }
        state.observed_release.store(
            state.release.load(std::sync::atomic::Ordering::Relaxed),
            std::sync::atomic::Ordering::Relaxed,
        );
        // SAFETY: strdup returns a host-compatible allocation and result_out is valid.
        unsafe { *result_out = libc::strdup(c"ready".as_ptr()) };
        0
    }

    fn minimal_callbacks() -> XpBackendCallbacks {
        XpBackendCallbacks {
            abi_version: XP_BACKEND_ABI_VERSION,
            struct_size: std::mem::size_of::<XpBackendCallbacks>(),
            context: std::ptr::null_mut(),
            execute_query: Some(unused_execute),
            get_query_status: Some(unused_status),
            cancel_query: Some(unused_cancel),
            get_query_result: Some(unused_result),
            authenticate: Some(unused_auth),
            get_session_parameters: Some(unused_session_params),
            upload_stream: None,
            download_stream: None,
            list_stage_files: None,
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn blocking_callback_does_not_hold_the_runtime_worker() {
        let state = BlockingCallbackState {
            release: std::sync::atomic::AtomicBool::new(false),
            observed_release: std::sync::atomic::AtomicBool::new(false),
        };
        let mut callbacks = minimal_callbacks();
        callbacks.context = std::ptr::from_ref(&state).cast_mut().cast();
        callbacks.get_query_status = Some(waiting_status);
        let adapter = CBackendAdapter::new(&callbacks).expect("callbacks should be valid");

        let (result, ()) = tokio::join!(adapter.get_query_status("query-id"), async {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            state
                .release
                .store(true, std::sync::atomic::Ordering::Relaxed);
        });

        assert_eq!(result.expect("callback should succeed"), "ready");
        assert!(
            state
                .observed_release
                .load(std::sync::atomic::Ordering::Relaxed),
            "callback blocked the runtime worker"
        );
    }

    #[test]
    fn c_register_attaches_to_an_existing_driver() {
        use crate::apis::database_driver_v1::{DatabaseDriverV1, DriverProviders};

        let driver = DatabaseDriverV1::with_providers(DriverProviders {
            running_inside_xp: Some(true),
            ..Default::default()
        });
        let _guard = registry::TestCRegistration::attach_to(std::sync::Arc::clone(&driver.xp_slot));
        let callbacks = minimal_callbacks();

        let code = unsafe { sf_core_register_xp_backend(&callbacks) };
        assert_eq!(code, 0);
        driver
            .xp_backend()
            .expect("registered backend should be active")
            .expect("XP mode should use the C adapter");

        let code = unsafe { sf_core_register_xp_backend(&callbacks) };
        assert_eq!(code, error_codes::ALREADY_REGISTERED);
    }

    #[test]
    fn c_register_before_driver_construction_is_consumed_once() {
        let _isolation = registry::TestCRegistration::isolate();
        let callbacks = minimal_callbacks();

        let code = unsafe { sf_core_register_xp_backend(&callbacks) };
        assert_eq!(code, 0);

        let attached = super::super::XpSlot::new(true, None);
        attached
            .active()
            .expect("pending C adapter should attach")
            .expect("XP mode should use the pending adapter");

        let leftover = super::super::XpSlot::new(true, None);
        let err = match leftover.active() {
            Err(err) => err,
            Ok(_) => panic!("pending C adapter should be consumed once"),
        };
        assert_eq!(err.code, error_codes::NOT_REGISTERED);
    }
}
