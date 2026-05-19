use std::collections::BTreeMap;
use std::ffi::{c_char, c_void};
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::apis::database_driver_v1::{DriverProviders, WrapperPresets};
use crate::logging::LogManager;
use crate::protobuf::apis::RustTransport;
use futures::FutureExt;
use proto_utils::{ProtoError, Transport};
use tokio::sync::Notify;
use tokio::task::AbortHandle;

/// C callback type invoked when an async proto API call completes.
///
/// Parameters: `(user_data, status, response_ptr, response_len)`.
/// Status codes: `0`=success, `1`=application error, `2`=transport error.
///
/// # Safety
/// - Must not unwind across the FFI boundary.
/// - The `response_ptr`/`response_len` buffer is owned by Rust and is freed
///   immediately after the callback returns; the callback must copy any data
///   it needs before returning.
type ResponseCallback = unsafe extern "C" fn(*mut c_void, usize, *const u8, usize);

/// Wrapper that promises a raw pointer is safe to send across threads.
/// We never dereference it on the Rust side — it is opaque user data passed
/// straight back to the callback.
#[derive(Copy, Clone)]
struct UserData(*mut c_void);
unsafe impl Send for UserData {}

struct CApiState {
    runtime: tokio::runtime::Runtime,
    transport: RustTransport,
}

static STATE: OnceLock<CApiState> = OnceLock::new();

/// Monotonic source for request IDs returned by [`sf_core_api_call_proto_async`].
/// Wraps after `2^64` requests, which we treat as never.
static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

/// Registry of in-flight async requests, keyed by request ID. Holding the
/// `AbortHandle` lets [`sf_core_cancel_request`] tell tokio to drop the task at
/// its next await point. Entries are removed either when the task self-completes
/// (success path) or when the caller cancels (cancel path) — whichever happens
/// first wins; the loser is a no-op.
static REQUEST_REGISTRY: std::sync::Mutex<BTreeMap<u64, AbortHandle>> =
    std::sync::Mutex::new(BTreeMap::new());

/// Eagerly build the entire core state (tokio runtime + transport) using the
/// given `LogManager`.  Called once from `sf_core_init`.
pub(crate) fn init_core_state(lm: LogManager, wrapper_presets: WrapperPresets) {
    STATE.get_or_init(|| {
        let providers = DriverProviders {
            log_manager: Some(lm),
            wrapper_presets,
            ..Default::default()
        };

        CApiState {
            runtime: tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime"),
            transport: RustTransport::new_with(providers),
        }
    });
}

fn write_buffer(vec: Vec<u8>, buffer: *mut *const u8, len: *mut usize) {
    let boxed = vec.into_boxed_slice();
    unsafe {
        *len = boxed.len();
        *buffer = Box::into_raw(boxed) as *const u8;
    }
}

/// Frees a buffer previously returned by `sf_core_api_call_proto` via `write_buffer`.
///
/// # Safety
/// The caller must pass the exact `buffer` pointer and `len` that were written by a prior
/// call to `sf_core_api_call_proto`. Each (buffer, len) pair must be freed at most once.
/// Passing any other pointer or length is undefined behavior.
#[unsafe(no_mangle)]
#[cfg(feature = "protobuf")]
pub unsafe extern "C" fn sf_core_free_buffer(buffer: *const u8, len: usize) {
    if !buffer.is_null() {
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                buffer as *mut u8,
                len,
            )));
        }
    }
}

/// # Safety
/// This function dereferences raw pointers `api`, `method`, `request`, `response`, and `response_len`.
/// The caller must ensure that `api`, `method`, `request`, `response`, and `response_len` are valid.
#[unsafe(no_mangle)]
#[cfg(feature = "protobuf")]
pub unsafe extern "C" fn sf_core_api_call_proto(
    api: *const c_char,
    method: *const c_char,
    request: *mut u8,
    request_len: usize,
    response: *mut *const u8,
    response_len: *mut usize,
) -> usize {
    // Prevent unwinding across the FFI boundary. Any panic will be converted to a transport error.
    let result = std::panic::catch_unwind(|| unsafe {
        let state = STATE.get().expect("sf_core_init was not called");
        let api = std::ffi::CStr::from_ptr(api).to_string_lossy().to_string();
        let method = std::ffi::CStr::from_ptr(method)
            .to_string_lossy()
            .to_string();
        let message = std::slice::from_raw_parts(request, request_len);
        state.runtime.block_on(
            state
                .transport
                .handle_message(&api, &method, message.to_vec()),
        )
    });

    match result {
        Ok(Ok(response_vec)) => {
            write_buffer(response_vec, response, response_len);
            0
        }
        Ok(Err(ProtoError::Application(error_vec))) => {
            write_buffer(error_vec, response, response_len);
            1
        }
        Ok(Err(ProtoError::Transport(e))) => {
            write_buffer(e.as_bytes().to_vec(), response, response_len);
            2
        }
        Err(_) => {
            let msg = b"sf_core panic in sf_core_api_call_proto".to_vec();
            write_buffer(msg, response, response_len);
            2
        }
    }
}

/// Invoke `callback` with the result of an FFI async dispatch, then free the
/// response buffer. Splitting this out of the FFI function keeps the unsafe
/// pointer arithmetic in one place.
///
/// # Safety
/// `callback` must satisfy the contract documented on [`ResponseCallback`].
unsafe fn fire_callback(
    callback: ResponseCallback,
    user_data: UserData,
    status: usize,
    response_vec: Vec<u8>,
) {
    // `boxed` must outlive the callback invocation: the callback receives a
    // raw pointer into this allocation and reads it before returning.
    // Binding it here keeps it alive for the whole scope; the explicit `drop`
    // after the callback makes the lifetime extension intent unambiguous to
    // future readers (and is a no-op the optimiser will elide anyway).
    let boxed = response_vec.into_boxed_slice();
    let ptr = boxed.as_ptr();
    let len = boxed.len();
    // SAFETY: caller-supplied callback contract; pointer/len pair is valid
    // for the duration of the call (`boxed` is held below) and freed
    // immediately after via `drop(boxed)`.
    unsafe { callback(user_data.0, status, ptr, len) };
    drop(boxed);
}

/// Async variant of [`sf_core_api_call_proto`] that returns immediately and
/// invokes `callback` from a tokio worker thread when the request completes.
///
/// Returns a non-zero **request ID**. The caller may pass this ID to
/// [`sf_core_cancel_request`] to ask Rust to abort the in-flight task. The ID
/// is valid until either (a) the callback fires or (b) the caller cancels —
/// whichever comes first; after that it is reused for a future request.
///
/// Unlike the sync variant, this does **not** block the caller — multiple
/// outstanding requests run concurrently on the shared tokio runtime.
///
/// # Safety
/// - `api`, `method`, `request` must point to valid data of the specified
///   lengths for the duration of this call. They are copied immediately;
///   the caller may free them after this returns.
/// - `callback` is invoked exactly once and must be `Send`-safe (it will
///   fire from a tokio worker thread, not the calling thread). It may still
///   be invoked after [`sf_core_cancel_request`] if the task happened to be
///   past its last await point — callers must guard against that on their side.
/// - `user_data` is opaque and is forwarded to the callback unchanged.
///   The caller is responsible for keeping any referent alive until the
///   callback fires.
/// - The response buffer passed to the callback is owned by Rust and freed
///   immediately after the callback returns; copy before returning.
#[unsafe(no_mangle)]
#[cfg(feature = "protobuf")]
pub unsafe extern "C" fn sf_core_api_call_proto_async(
    api: *const c_char,
    method: *const c_char,
    request: *const u8,
    request_len: usize,
    callback: ResponseCallback,
    user_data: *mut c_void,
) -> u64 {
    // Copy inputs eagerly — the caller may free these after we return.
    let api_str = unsafe { std::ffi::CStr::from_ptr(api).to_string_lossy().into_owned() };
    let method_str = unsafe {
        std::ffi::CStr::from_ptr(method)
            .to_string_lossy()
            .into_owned()
    };
    let request_vec = if request_len == 0 {
        // Avoid `slice::from_raw_parts` with len=0 — that requires a non-null
        // aligned pointer, but ctypes may pass null for a zero-length buffer.
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(request, request_len).to_vec() }
    };
    let user_data = UserData(user_data);

    let state = STATE.get().expect("sf_core_init was not called");
    let request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed);

    // Gate the spawned task on registration completing. Without this, the task
    // can run to completion (fast path / no real I/O) BEFORE the calling thread
    // inserts the AbortHandle, which would leak the handle in the registry.
    // `Notify` has permit semantics: `notify_one` before `notified().await` is
    // delivered when the task first awaits, so there is no missed-wakeup race.
    let registered = Arc::new(Notify::new());
    let registered_in_task = registered.clone();

    let join = state.runtime.spawn(async move {
        // Wait until the calling thread has inserted our AbortHandle into
        // REQUEST_REGISTRY. After this await, cancellation can find us.
        registered_in_task.notified().await;

        // Re-fetch state from the static — avoids capturing a borrow across
        // the await point (which would bound the future's lifetime to 'static).
        let state = STATE.get().expect("sf_core_init was not called");

        // Drive the transport future directly — no `block_on`. This yields
        // properly to the tokio scheduler so concurrent calls actually run
        // concurrently. `catch_unwind` ensures a panic in transport code is
        // surfaced as a transport error rather than aborting the runtime.
        let result = std::panic::AssertUnwindSafe(state.transport.handle_message(
            &api_str,
            &method_str,
            request_vec,
        ))
        .catch_unwind()
        .await;

        let (status, response_vec): (usize, Vec<u8>) = match result {
            Ok(Ok(r)) => (0, r),
            Ok(Err(ProtoError::Application(e))) => (1, e),
            Ok(Err(ProtoError::Transport(e))) => (2, e.as_bytes().to_vec()),
            Err(_) => (2, b"sf_core panic in async task".to_vec()),
        };

        // Self-deregister BEFORE firing the callback. Whichever side
        // (completion vs cancel) wins the lock first owns the cleanup; the
        // loser's `remove` is a no-op. Doing this before the callback also
        // means a concurrent cancel can't race with the callback firing.
        let _ = REQUEST_REGISTRY.lock().unwrap().remove(&request_id);

        // SAFETY: callback contract documented on this function.
        unsafe { fire_callback(callback, user_data, status, response_vec) };
    });

    // Register the abort handle, then release the task. The task is gated on
    // `notified().await` above, so it cannot reach `REQUEST_REGISTRY.remove`
    // before this insert lands.
    REQUEST_REGISTRY
        .lock()
        .unwrap()
        .insert(request_id, join.abort_handle());
    registered.notify_one();

    request_id
}

/// Request that an in-flight async call submitted via
/// [`sf_core_api_call_proto_async`] be cancelled.
///
/// This is **best-effort**: tokio aborts the task at its next await point.
/// If the task is past its last await (e.g. about to fire the callback) the
/// callback may still be invoked. Python is responsible for guarding the
/// downstream Future against late callbacks (e.g. checking `done()` before
/// `set_result`). After this call the `request_id` is invalid; passing the
/// same ID twice is safe but a no-op.
///
/// # Safety
/// `request_id` must come from a prior [`sf_core_api_call_proto_async`]; any
/// other value is a no-op.
#[unsafe(no_mangle)]
#[cfg(feature = "protobuf")]
pub unsafe extern "C" fn sf_core_cancel_request(request_id: u64) {
    if let Some(handle) = REQUEST_REGISTRY.lock().unwrap().remove(&request_id) {
        handle.abort();
    }
}
