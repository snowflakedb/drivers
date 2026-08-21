//! Stub implementation of the sf_core C API for integration testing.
//!
//! # Method-string protocol
//!
//! Both `sf_core_api_call_proto` and `sf_core_api_call_proto_async` interpret the
//! `method` C string to decide what to do:
//!
//! | method           | behavior                                                                  |
//! |------------------|------------------------------------------------------------------------   |
//! | `echo_increment` | Return request bytes with each byte incremented by 1 (wrapping), status 0 |
//! | `delay_ms:N`     | Sleep N milliseconds, then return request bytes unchanged, status 0       |
//! | `block`          | Park on a condvar until `sf_core_api_cancel` signals it                   |
//! | `error:N`        | Return request bytes unchanged, status N                                  |
//! | `null_ptr`       | Fire callback with ptr=null, len=0, status 0 (malformed response)         |
//! | `huge_len`       | Fire callback with valid ptr but len=usize::MAX, status 0 (overflow)      |
//! | *(anything else)*| Same as `echo_increment`                                                  |
//!
//! # Leak counter
//!
//! Every response buffer allocated by this stub increments `ALLOC_COUNT`.
//! Every call to `sf_core_free_buffer` decrements it.
//! Tests read the counter via `sf_stub_leaked_alloc_count()` after a workload
//! completes to verify every allocation was freed exactly once.

use std::collections::HashMap;
use std::ffi::{CStr, c_char, c_void};
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::Duration;

static LAST_GENERATED_HANDLE_ID: AtomicU64 = AtomicU64::new(1);

/// Counts live allocated response buffers. Decremented by `sf_core_free_buffer`.
static CURRENTLY_ALLOCATED_RESPONSES_COUNT: AtomicI64 = AtomicI64::new(0);

/// The gate held for each in-flight async call.
/// Inner bool = cancelled flag; condvar wakes the thread.
type AsyncCallExecutionBlockingGate = (Mutex<bool>, Condvar);

static ASYNC_CALL_EXECUTION_BLOCKING_GATES_HANDLE_MAP: OnceLock<Mutex<HashMap<u64, Arc<AsyncCallExecutionBlockingGate>>>> = OnceLock::new();

fn async_call_execution_blocking_gates_handle_map() -> &'static Mutex<HashMap<u64, Arc<AsyncCallExecutionBlockingGate>>> {
    ASYNC_CALL_EXECUTION_BLOCKING_GATES_HANDLE_MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Clone, PartialEq)]
enum Action {
    EchoIncrement,
    DelayMs(u64),
    Block,
    Error(usize),
    NullPtr,
    HugeLen,
}

/// Parse the `method` argument at the FFI boundary into an owned `Action`.
/// All raw-pointer work happens here; callers on background threads use owned
/// values only.
fn parse_action(method: *const c_char) -> Action {
    let s = if method.is_null() {
        return Action::EchoIncrement;
    } else {
        unsafe { CStr::from_ptr(method) }
            .to_str()
            .unwrap_or("echo")
    };

    match s {
        "block" => return Action::Block,
        "null_ptr" => return Action::NullPtr,
        "huge_len" => return Action::HugeLen,
        _ => {
            if let Some(rest) = s.strip_prefix("delay_ms:") {
                if let Ok(ms) = rest.parse::<u64>() {
                    return Action::DelayMs(ms);
                }
            }
            if let Some(rest) = s.strip_prefix("error:") {
                if let Ok(code) = rest.parse::<usize>() {
                    return Action::Error(code);
                }
            }
            Action::EchoIncrement
        }
    }
}

/// Boxes `data` into a tracked heap allocation.
/// The pointer is later freed by `sf_core_free_buffer`, which decrements the counter.
fn alloc_response(data: &[u8]) -> (*const u8, usize) {
    let boxed: Box<[u8]> = data.into();
    let len = data.len();
    let ptr = Box::into_raw(boxed) as *const u8;
    CURRENTLY_ALLOCATED_RESPONSES_COUNT.fetch_add(1, Ordering::Relaxed);
    (ptr, len)
}

// ---------------------------------------------------------------------------
// C API exports
// ---------------------------------------------------------------------------

/// Initialize the stub.
///
/// Returns a value that matches the `SfCoreInitResult` layout used by the real
/// sf_core: `{ status: u32, troubleshooting_enabled: u32 }` packed as `u64`.
/// The .NET side reads only the low 32 bits (status = 0 = success).
#[unsafe(no_mangle)]
pub extern "C" fn sf_core_init(
    _callback: unsafe extern "C" fn(u32, *const c_char, *const c_char, u32, *const c_char) -> u32,
) -> u64 {
    0u64
}

/// Synchronous proto call.  Behavior driven by the `method` string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sf_core_api_call_proto(
    _api: *const c_char,
    method: *const c_char,
    request: *const u8,
    request_len: usize,
    response: *mut *const u8,
    response_len: *mut usize,
) -> usize {
    let request_bytes: &[u8] = if request.is_null() || request_len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(request, request_len) }
    };
    let action = parse_action(method);

    let status: usize = match action {
        Action::DelayMs(ms) => {
            std::thread::sleep(Duration::from_millis(ms));
            0
        }
        Action::Block => {
            panic!("Sync callers don't have a cancel handle");
        }
        Action::Error(code) => code,
        Action::EchoIncrement => 0,
        Action::NullPtr => {
            unsafe {
                *response = std::ptr::null();
                *response_len = 0;
            }
            0
        }
        Action::HugeLen => {
            // Allocate a tiny buffer but report an absurdly large length.
            let (ptr, _) = alloc_response(&[0u8; 1]);
            unsafe {
                *response = ptr;
                *response_len = usize::MAX;
            }
            0
        }
    };

    // echo_increment returns bytes with each value incremented by 1 (wrapping).
    // All other actions return the original request bytes.
    let incremented: Vec<u8>;
    let response_payload: &[u8] = if action == Action::EchoIncrement {
        incremented = request_bytes.iter().map(|b| b + 1).collect();
        &incremented
    } else {
        request_bytes
    };

    let (ptr, len) = alloc_response(response_payload);
    unsafe {
        *response = ptr;
        *response_len = len;
    }
    status
}

/// Async proto call.
///
/// Spawns a background thread that:
/// 1. Waits according to the `method` gate protocol.
/// 2. Fires `callback(user_data, status, ptr, len)` unless cancelled.
///
/// The response buffer passed to the callback is stack/thread-local and freed
/// immediately after the callback returns — the callback must copy before returning,
/// matching the real sf_core contract.  `sf_core_free_buffer` is **not** called for
/// async responses; only sync responses need explicit freeing.
///
/// Returns a non-zero async handle for use with `sf_core_api_cancel`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sf_core_api_call_proto_async(
    _api: *const c_char,
    method: *const c_char,
    request: *const u8,
    request_len: usize,
    callback: unsafe extern "C" fn(*mut c_void, usize, *const u8, usize),
    user_data: *mut c_void,
) -> u64 {
    let action = parse_action(method);
    let request_vec: Vec<u8> = if request.is_null() || request_len == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(request, request_len).to_vec() }
    };
    // Transfer pointers as `usize` which is `Send`; reconstruct inside the thread.
    let callback_addr = callback as usize;
    let user_data_addr = user_data as usize;

    let handle = LAST_GENERATED_HANDLE_ID.fetch_add(1, Ordering::Relaxed);

    let gate: Arc<AsyncCallExecutionBlockingGate> = Arc::new((Mutex::new(false), Condvar::new()));
    async_call_execution_blocking_gates_handle_map()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .insert(handle, gate.clone());

    std::thread::spawn(move || {
        // SAFETY: these values are reconstructed from pointers the .NET side passed
        // in; .NET guarantees liveness until the callback fires.
        let callback: unsafe extern "C" fn(*mut c_void, usize, *const u8, usize) =
            unsafe { std::mem::transmute(callback_addr) };
        let user_data = user_data_addr as *mut c_void;
        let cancelled = match &action {
            Action::Block => {
                let (lock, cvar) = gate.as_ref();
                let guard = lock.lock().unwrap_or_else(|p| p.into_inner());
                *cvar
                    .wait_while(guard, |cancelled| !*cancelled)
                    .unwrap_or_else(|p| p.into_inner())
            }
            Action::DelayMs(ms) => {
                let (lock, cvar) = gate.as_ref();
                let guard = lock.lock().unwrap_or_else(|p| p.into_inner());
                let (guard, _) = cvar
                    .wait_timeout_while(
                        guard,
                        Duration::from_millis(*ms),
                        |cancelled| !*cancelled,
                    )
                    .unwrap_or_else(|p| p.into_inner());
                *guard
            }
            Action::EchoIncrement | Action::Error(_) | Action::NullPtr | Action::HugeLen => {
                // Check the canceled flag even for instant responses.
                *gate.0.lock().unwrap_or_else(|p| p.into_inner())
            }
        };

        // Clean up gate registration regardless of outcome.
        async_call_execution_blocking_gates_handle_map()
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(&handle);

        if cancelled {
            // Fire callback with status=2 (cancellation) — matching real sf_core contract.
            // The response is empty; the managed side detects status=2 and transitions
            // the TCS to Canceled.
            let empty: &[u8] = &[];
            unsafe { callback(user_data, 2, empty.as_ptr(), 0) };
            return;
        }

        // Handle malformed response actions (NullPtr / HugeLen).
        match action {
            Action::NullPtr => {
                unsafe { callback(user_data, 0, std::ptr::null(), 0) };
                return;
            }
            Action::HugeLen => {
                let one_byte: [u8; 1] = [0];
                unsafe { callback(user_data, 0, one_byte.as_ptr(), usize::MAX) };
                return;
            }
            _ => {}
        }

        let status: usize = match action {
            Action::Error(code) => code,
            _ => 0,
        };

        // echo_increment returns bytes with each value incremented by 1 (wrapping).
        // All other actions return the original request bytes.
        let response_vec: Vec<u8> = if matches!(action, Action::EchoIncrement) {
            request_vec.iter().map(|&b| b.wrapping_add(1)).collect()
        } else {
            request_vec
        };

        // The response buffer lives only for the callback invocation.
        let boxed: Box<[u8]> = response_vec.into_boxed_slice();
        let ptr = boxed.as_ptr();
        let len = boxed.len();
        // SAFETY: ptr/len are valid for the duration of the call; callback must copy.
        unsafe { callback(user_data, status, ptr, len) };
        // `boxed` drops here, freeing the response buffer.
        // No ALLOC_COUNT change: the callback owns any copy it makes via ArrayPool.
    });

    handle
}

/// Free a buffer previously returned by `sf_core_api_call_proto`.
/// Decrements the leak counter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sf_core_free_buffer(buffer: *const u8, len: usize) {
    unsafe {
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            buffer as *mut u8,
            len,
        )));
    }
    CURRENTLY_ALLOCATED_RESPONSES_COUNT.fetch_sub(1, Ordering::Relaxed);
}

/// Signal cancellation for an in-flight async call.
///
/// Sets the gate's canceled flag and notifies the condvar so the parked thread
/// wakes, sees the flag, and skips the response callback — matching real sf_core.
/// Unknown handles are silently ignored.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sf_core_api_cancel(async_handle: u64) {
    let gate = async_call_execution_blocking_gates_handle_map()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .remove(&async_handle);

    if let Some(gate) = gate {
        let (lock, cvar) = gate.as_ref();
        *lock.lock().unwrap_or_else(|p| p.into_inner()) = true;
        cvar.notify_all();
    }
}

// ---------------------------------------------------------------------------
// Test-control exports
// ---------------------------------------------------------------------------

/// Returns the current live allocation count.
///
/// Zero after a complete workload = every sync response buffer was freed exactly
/// once.  Positive = leak.  Negative = double-free.
#[unsafe(no_mangle)]
pub extern "C" fn sf_stub_leaked_alloc_count() -> i64 {
    CURRENTLY_ALLOCATED_RESPONSES_COUNT.load(Ordering::Relaxed)
}
