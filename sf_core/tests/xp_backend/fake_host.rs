//! A fake host implementing the backend C callbacks, shared by the
//! `xp_backend_mode_*` binaries. Returned strings are allocated with `strdup` so
//! the driver's `libc::free` runs against a real C allocation.

#![allow(dead_code)] // each binary uses a different subset

use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char, c_void};
use std::sync::Mutex;

use sf_core::xp_backend::ffi::{SfCoreFfiError, XP_BACKEND_ABI_VERSION, XpBackendCallbacks};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    pub operation: &'static str,
    /// SQL text for `execute_query`, the query id for the per-query calls.
    pub subject: Option<String>,
    pub payload: Option<String>,
}

/// Reached through the callbacks' `context` pointer.
#[derive(Default)]
pub struct HostState {
    calls: Mutex<Vec<Call>>,
    /// Body returned by `execute_query` and `get_query_result`.
    query_response: Mutex<Option<String>>,
    /// Body returned by `authenticate`.
    session_info: Mutex<Option<String>>,
    /// Body returned by `get_session_parameters` and `get_query_status`.
    json_payload: Mutex<Option<String>>,
    /// Body returned by `cancel_query`.
    cancel_response: Mutex<Option<String>>,
    /// Consumed by the next call, which then fails instead of succeeding.
    scripted_failure: Mutex<Option<(i32, Option<String>)>>,
    /// Bytes `download_query_chunk` reports, keyed by chunk URL.
    chunk_bodies: Mutex<HashMap<String, Vec<u8>>>,
    /// `headers_json` observed by the most recent `download_query_chunk` call
    /// for a given URL.
    chunk_headers: Mutex<HashMap<String, String>>,
}

impl HostState {
    /// Leaked to satisfy the ABI contract that `context` outlives the process's
    /// use of the driver: the registration is never withdrawn.
    pub fn leaked() -> &'static HostState {
        Box::leak(Box::new(HostState::default()))
    }

    pub fn set_query_response(&self, body: impl Into<String>) {
        *self.query_response.lock().unwrap() = Some(body.into());
    }

    pub fn set_session_info(&self, body: impl Into<String>) {
        *self.session_info.lock().unwrap() = Some(body.into());
    }

    pub fn set_json_payload(&self, body: impl Into<String>) {
        *self.json_payload.lock().unwrap() = Some(body.into());
    }

    /// `false` is how a host reports "nothing was running".
    pub fn set_cancel_outcome(&self, cancelled: bool) {
        *self.cancel_response.lock().unwrap() = Some(format!(r#"{{"cancelled":{cancelled}}}"#));
    }

    /// Next callback fails. `code` is written to `error.code`. The C return is
    /// non-zero even when `code` is 0, so a zero host code cannot look like
    /// success; the adapter then substitutes its PROTOCOL code.
    pub fn fail_next(&self, code: i32, message: Option<&str>) {
        *self.scripted_failure.lock().unwrap() = Some((code, message.map(str::to_string)));
    }

    /// Body `download_query_chunk` returns for `url`.
    pub fn set_chunk_body(&self, url: impl Into<String>, bytes: impl Into<Vec<u8>>) {
        self.chunk_bodies
            .lock()
            .unwrap()
            .insert(url.into(), bytes.into());
    }

    /// `headers_json` observed by the most recent `download_query_chunk` call
    /// for `url`.
    pub fn chunk_headers(&self, url: &str) -> Option<String> {
        self.chunk_headers.lock().unwrap().get(url).cloned()
    }

    pub fn calls(&self) -> Vec<Call> {
        self.calls.lock().unwrap().clone()
    }

    pub fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }

    pub fn operation_count(&self, operation: &str) -> usize {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|c| c.operation == operation)
            .count()
    }

    pub fn last_call(&self, operation: &str) -> Option<Call> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|c| c.operation == operation)
            .cloned()
    }

    fn record(&self, operation: &'static str, subject: Option<String>, payload: Option<String>) {
        self.calls.lock().unwrap().push(Call {
            operation,
            subject,
            payload,
        });
    }

    fn take_failure(&self) -> Option<(i32, Option<String>)> {
        self.scripted_failure.lock().unwrap().take()
    }
}

/// Tests needing an invalid struct start from this and mutate one field, so the
/// thing under test is the single deviation.
pub fn callbacks_for(state: &'static HostState) -> XpBackendCallbacks {
    XpBackendCallbacks {
        abi_version: XP_BACKEND_ABI_VERSION,
        struct_size: std::mem::size_of::<XpBackendCallbacks>(),
        context: (state as *const HostState).cast_mut().cast::<c_void>(),
        execute_query: Some(execute_query),
        get_query_status: Some(get_query_status),
        cancel_query: Some(cancel_query),
        get_query_result: Some(get_query_result),
        authenticate: Some(authenticate),
        get_session_parameters: Some(get_session_parameters),
        // Null on purpose: the trait's "unsupported" defaults must answer.
        upload_stream: None,
        download_stream: None,
        list_stage_files: None,
        download_query_chunk: Some(download_query_chunk),
    }
}

/// Allocated the way a C host would, so the driver's `libc::free` is correct.
unsafe fn host_strdup(value: &str) -> *mut c_char {
    let owned = CString::new(value).expect("fixture strings contain no interior NUL");
    // SAFETY: `owned` is a valid NUL-terminated string for the call's duration.
    unsafe { libc::strdup(owned.as_ptr()) }
}

/// Allocated the way a C host would, so the driver's `libc::free` is correct.
/// Returns `(null, 0)` for an empty buffer, matching the ABI's "no bytes"
/// contract (see `take_host_buffer` in `ffi.rs`).
unsafe fn host_buffer(bytes: &[u8]) -> (*mut u8, usize) {
    if bytes.is_empty() {
        return (std::ptr::null_mut(), 0);
    }
    // SAFETY: `len` is non-zero and the copy stays within the fresh allocation.
    unsafe {
        let ptr = libc::malloc(bytes.len()).cast::<u8>();
        assert!(!ptr.is_null(), "test host allocation failed");
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
        (ptr, bytes.len())
    }
}

/// SAFETY: `ptr` must be null or a valid NUL-terminated string.
unsafe fn borrow_str(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    // SAFETY: non-null and NUL-terminated per the caller's contract.
    Some(
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned(),
    )
}

/// SAFETY: `host_state` must be the `context` pointer from [`callbacks_for`].
unsafe fn state(host_state: *mut c_void) -> &'static HostState {
    // SAFETY: set from a leaked `&'static HostState`, so it stays valid.
    unsafe { &*host_state.cast::<HostState>() }
}

/// SAFETY: `error_out` must be a valid, writable pointer.
unsafe fn maybe_fail(host: &HostState, error_out: *mut SfCoreFfiError) -> Option<i32> {
    let (code, message) = host.take_failure()?;
    if let Some(message) = message {
        // SAFETY: writable per the caller's contract.
        unsafe { (*error_out).message = host_strdup(&message) };
    }
    unsafe { (*error_out).code = code };
    Some(if code == 0 { -1 } else { code })
}

/// Serve a recorded body, or report that the test forgot to script one.
///
/// SAFETY: `result_out` and `error_out` must be valid, writable pointers.
unsafe fn serve(
    body: Option<String>,
    what: &str,
    result_out: *mut *mut c_char,
    error_out: *mut SfCoreFfiError,
) -> i32 {
    match body {
        Some(body) => {
            // SAFETY: writable per the caller's contract.
            unsafe { *result_out = host_strdup(&body) };
            0
        }
        None => {
            unsafe {
                (*error_out).code = 999;
                (*error_out).message = host_strdup(&format!("test did not script a {what}"));
            }
            999
        }
    }
}

unsafe extern "C" fn execute_query(
    host_state: *mut c_void,
    sql: *const c_char,
    params_json: *const c_char,
    result_out: *mut *mut c_char,
    error_out: *mut SfCoreFfiError,
) -> i32 {
    // SAFETY: all pointers come from the driver and are valid for this call.
    // Same for every callback below.
    unsafe {
        let host = state(host_state);
        host.record("execute_query", borrow_str(sql), borrow_str(params_json));
        if let Some(code) = maybe_fail(host, error_out) {
            return code;
        }
        let body = host.query_response.lock().unwrap().clone();
        serve(body, "query response", result_out, error_out)
    }
}

unsafe extern "C" fn get_query_status(
    host_state: *mut c_void,
    query_id: *const c_char,
    result_out: *mut *mut c_char,
    error_out: *mut SfCoreFfiError,
) -> i32 {
    unsafe {
        let host = state(host_state);
        host.record("get_query_status", borrow_str(query_id), None);
        if let Some(code) = maybe_fail(host, error_out) {
            return code;
        }
        let body = host.json_payload.lock().unwrap().clone();
        serve(body, "status payload", result_out, error_out)
    }
}

unsafe extern "C" fn cancel_query(
    host_state: *mut c_void,
    params_json: *const c_char,
    result_out: *mut *mut c_char,
    error_out: *mut SfCoreFfiError,
) -> i32 {
    unsafe {
        let host = state(host_state);
        host.record("cancel_query", None, borrow_str(params_json));
        if let Some(code) = maybe_fail(host, error_out) {
            return code;
        }
        let body = host.cancel_response.lock().unwrap().clone();
        serve(body, "cancel response", result_out, error_out)
    }
}

unsafe extern "C" fn get_query_result(
    host_state: *mut c_void,
    query_id: *const c_char,
    result_out: *mut *mut c_char,
    error_out: *mut SfCoreFfiError,
) -> i32 {
    unsafe {
        let host = state(host_state);
        host.record("get_query_result", borrow_str(query_id), None);
        if let Some(code) = maybe_fail(host, error_out) {
            return code;
        }
        let body = host.query_response.lock().unwrap().clone();
        serve(body, "query response", result_out, error_out)
    }
}

unsafe extern "C" fn authenticate(
    host_state: *mut c_void,
    params_json: *const c_char,
    result_out: *mut *mut c_char,
    error_out: *mut SfCoreFfiError,
) -> i32 {
    unsafe {
        let host = state(host_state);
        host.record("authenticate", None, borrow_str(params_json));
        if let Some(code) = maybe_fail(host, error_out) {
            return code;
        }
        let body = host.session_info.lock().unwrap().clone();
        serve(body, "session info", result_out, error_out)
    }
}

unsafe extern "C" fn get_session_parameters(
    host_state: *mut c_void,
    result_out: *mut *mut c_char,
    error_out: *mut SfCoreFfiError,
) -> i32 {
    unsafe {
        let host = state(host_state);
        host.record("get_session_parameters", None, None);
        if let Some(code) = maybe_fail(host, error_out) {
            return code;
        }
        let body = host.json_payload.lock().unwrap().clone();
        serve(body, "session parameters", result_out, error_out)
    }
}

unsafe extern "C" fn download_query_chunk(
    host_state: *mut c_void,
    url: *const c_char,
    headers_json: *const c_char,
    data_out: *mut *mut u8,
    data_len_out: *mut usize,
    error_out: *mut SfCoreFfiError,
) -> i32 {
    unsafe {
        let host = state(host_state);
        let url_str = borrow_str(url).unwrap_or_default();
        let headers = borrow_str(headers_json);
        host.record(
            "download_query_chunk",
            Some(url_str.clone()),
            headers.clone(),
        );
        if let Some(headers) = headers {
            host.chunk_headers
                .lock()
                .unwrap()
                .insert(url_str.clone(), headers);
        }
        if let Some(code) = maybe_fail(host, error_out) {
            return code;
        }
        match host.chunk_bodies.lock().unwrap().get(&url_str).cloned() {
            Some(bytes) => {
                let (ptr, len) = host_buffer(&bytes);
                *data_out = ptr;
                *data_len_out = len;
                0
            }
            None => {
                (*error_out).code = 999;
                (*error_out).message =
                    host_strdup(&format!("test did not script a chunk body for {url_str}"));
                999
            }
        }
    }
}
