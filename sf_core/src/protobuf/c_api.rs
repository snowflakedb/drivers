use std::ffi::{CStr, c_char, c_void};
use std::sync::OnceLock;

use crate::apis::database_driver_v1::{DriverProviders, WrapperPresets};
use crate::logging::LogManager;
use crate::protobuf::apis::RustTransport;
use futures::FutureExt;
use proto_utils::{CancellableTransport, ProtoError, Transport};
use tracing::instrument::WithSubscriber;

/// C callback type - fn ptr invoked when an async proto API call completes.
///
/// `user_data` is an opaque pointer passed through unchanged from the caller.
/// This C API is the universal FFI surface consumed by various language wrappers.
/// Languages whose fn ptrs cannot capture state need it to associate a callback with its context.
/// Higher-level languages may ignore it, as their closures capture state directly.
///
/// # Safety
/// - Must not unwind across the FFI boundary.
/// - The response buffer is owned by Rust and is freed immediately after the callback returns.
///   The callback must copy any data it needs before returning.
type ResponseCallback = unsafe extern "C" fn(*mut c_void, usize, *const u8, usize);

/// Opaque user-data pointer forwarded to the callback unchanged. Rust never dereferences it!
#[derive(Copy, Clone)]
struct UserData(*mut c_void);
unsafe impl Send for UserData {}

struct CApiState {
    runtime: tokio::runtime::Runtime,
    transport: RustTransport,
    dispatch: tracing::dispatcher::Dispatch,
}

static STATE: OnceLock<CApiState> = OnceLock::new();

/// Eagerly build the entire core state (tokio runtime + transport) using the
/// given `LogManager`.  Called once from `sf_core_init`.
pub(crate) fn init_core_state(lm: LogManager, wrapper_presets: WrapperPresets) {
    STATE.get_or_init(|| {
        let dispatch = lm.dispatch().clone();
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
            dispatch,
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
        let _guard = tracing::dispatcher::set_default(&state.dispatch);
        let api = CStr::from_ptr(api).to_string_lossy().to_string();
        let method = CStr::from_ptr(method).to_string_lossy().to_string();
        let message = if request_len == 0 || request.is_null() {
            &[]
        } else {
            std::slice::from_raw_parts(request, request_len)
        };
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

unsafe fn fire_callback(
    callback: ResponseCallback,
    user_data: UserData,
    status: usize,
    response_vec: Vec<u8>,
) {
    // `boxed` is dropped at the end of this scope, immediately after the callback returns
    let boxed = response_vec.into_boxed_slice();
    let ptr = boxed.as_ptr();
    let len = boxed.len();
    // SAFETY: ptr/len point into `boxed` which is alive for the call.
    unsafe { callback(user_data.0, status, ptr, len) };
}

/// Async variant of [`sf_core_api_call_proto`] that returns immediately and
/// invokes `callback` from a tokio worker thread when the request completes.
///
/// Returns a non-zero **async handle** that uniquely identifies this in-flight call.
///
/// Unlike the sync variant, this does **not** block the caller, so multiple requests run concurrently on the shared tokio runtime.
///
/// # Safety
/// - `api`, `method`, `request` must point to valid data of the specified lengths for the duration of this call.
///   *They are copied immediately. The caller may free them after this returns.*
/// - `callback` is invoked exactly once and must be `Send`-safe (it will fire from a tokio worker thread, not the calling thread).
/// - `user_data` is opaque and is forwarded to the callback unchanged.
///   *The caller is responsible for keeping any referent alive until the callback fires.*
/// - The response buffer passed to the callback is owned by Rust and freed immediately.
///   *The callback must copy before returning!*
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
    let api_str = unsafe { CStr::from_ptr(api).to_string_lossy().into_owned() };
    let method_str = unsafe { CStr::from_ptr(method).to_string_lossy().into_owned() };
    let request_vec = if request_len == 0 || request.is_null() {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(request, request_len).to_vec() }
    };
    let user_data = UserData(user_data);

    let state = STATE.get().expect("sf_core_init was not called");
    // The client passes this handle to `sf_core_api_cancel`. Registering it on
    // the transport before we return means it can never be cancelled before it
    // exists. The token stays in the registry — cancel by handle.
    let (async_handle, _token) = state.transport.register();

    let dispatch = state.dispatch.clone();
    state.runtime.spawn(
        async move {
            // `handle_message_cancellable` dispatches the operation under the token
            // registered for `async_handle`; the operation observes that token and
            // unwinds with a Cancelled `DriverException`. The completion callback
            // fires on every outcome, including cancellation; Python guards
            // `on_response` with `future.done()`.
            let result = std::panic::AssertUnwindSafe(state.transport.handle_message_cancellable(
                &api_str,
                &method_str,
                request_vec,
                async_handle,
            ))
            .catch_unwind()
            .await;

            let (status, response_vec): (usize, Vec<u8>) = match result {
                Ok(Ok(r)) => (0, r),
                Ok(Err(ProtoError::Application(e))) => (1, e),
                Ok(Err(ProtoError::Transport(e))) => (2, e.as_bytes().to_vec()),
                Err(_) => (2, b"sf_core panic in async task".to_vec()),
            };

            // handle_message_cancellable deregistered async_handle on completion.
            // SAFETY: callback contract documented on this function.
            unsafe { fire_callback(callback, user_data, status, response_vec) };
        }
        .with_subscriber(dispatch),
    );

    async_handle
}

/// Cooperatively cancel an in-flight async call started by [`sf_core_api_call_proto_async`].
///
/// # Safety
/// No preconditions. Unknown async handles are silently ignored.
#[unsafe(no_mangle)]
#[cfg(feature = "protobuf")]
pub unsafe extern "C" fn sf_core_api_cancel(async_handle: u64) {
    if let Some(state) = STATE.get() {
        state.transport.cancel(async_handle);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use tracing::Subscriber;
    use tracing::instrument::WithSubscriber;
    use tracing_subscriber::Layer;
    use tracing_subscriber::layer::{Context, SubscriberExt};

    struct CaptureLayer {
        messages: Arc<Mutex<Vec<String>>>,
    }

    impl<S: Subscriber> Layer<S> for CaptureLayer {
        fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
            let normalized = crate::logging::normalize_event(event);
            self.messages
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(normalized.message);
        }
    }

    fn hop_and_capture(use_with_subscriber: bool) -> Option<Vec<String>> {
        let messages = Arc::new(Mutex::new(Vec::new()));
        let dispatch =
            tracing::dispatcher::Dispatch::new(tracing_subscriber::registry().with(CaptureLayer {
                messages: Arc::clone(&messages),
            }));
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .expect("test runtime");
        let (tx, rx) = tokio::sync::oneshot::channel();
        let task = async move {
            let before = std::thread::current().id();
            let _ = rx.await;
            tracing::info!("after_await_on_possibly_other_worker");
            (before, std::thread::current().id())
        };
        let join = if use_with_subscriber {
            runtime.spawn(task.with_subscriber(dispatch))
        } else {
            runtime.spawn(async move {
                let _guard = tracing::dispatcher::set_default(&dispatch);
                task.await
            })
        };
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            let _ = tx.send(());
        });
        let (before, after) = runtime.block_on(join).expect("spawned task");
        if before == after {
            return None;
        }
        Some(messages.lock().unwrap_or_else(|e| e.into_inner()).clone())
    }

    fn capture_after_hop(use_with_subscriber: bool) -> Vec<String> {
        for _ in 0..40 {
            if let Some(captured) = hop_and_capture(use_with_subscriber) {
                return captured;
            }
        }
        panic!("could not force a tokio worker hop in 40 attempts");
    }

    #[test]
    fn set_default_across_await_drops_events_after_worker_hop() {
        let captured = capture_after_hop(false);
        assert!(
            !captured
                .iter()
                .any(|m| m.contains("after_await_on_possibly_other_worker")),
            "set_default is thread-local; a worker hop must drop the event; captured = {captured:?}"
        );
    }

    #[test]
    fn with_subscriber_keeps_events_after_worker_hop() {
        let captured = capture_after_hop(true);
        assert!(
            captured
                .iter()
                .any(|m| m.contains("after_await_on_possibly_other_worker")),
            "with_subscriber must keep the event after a worker hop; captured = {captured:?}"
        );
    }
}
