"""Type stubs for the sf_core_python native extension (auto-generated)."""

def init(logger_callback: object) -> tuple[int, bool]:
    """Initialize the core state: logging, tokio runtime, and transport.

    Called by the Python connector at import time, before any API call.
    Must be called before any other function.

    Returns `(status, troubleshooting_enabled)` where:
    - `status`: `0` = success, non-zero = failure
    - `troubleshooting_enabled`: whether troubleshooting mode is active at init time

    If already initialised, returns success with the current troubleshooting flag
    without creating another [`Bridge`].
    """
    ...

def log_event(level: int, message: str, file: str, line: int, function: str, logger_name: str) -> int:
    """Emit a wrapper-originated log event through the tracing pipeline.

    Uses the same level encoding as the inbound [`sf_core::logging::CLogCallback`]:
    0=ERROR, 1=WARN, 2=INFO, 3 or higher=DEBUG.

    Returns `0` on success, `1` when the pipeline is uninitialised, and `2` if the body panics.
    """
    ...

def call_proto(api: str, method: str, request: object) -> tuple[int, bytes]:
    """Synchronous proto API call. Releases the GIL and blocks until complete.

    Returns `(status_code, response_bytes)` where status is:
    - `0` — success
    - `1` — application error (response holds the error payload)
    - `2` — transport error (including panics / missing init caught at this boundary)
    """
    ...

def call_proto_async(api: str, method: str, request: object, callback: object) -> int:
    """Async proto API call. Returns immediately and invokes
    `callback(status, response_bytes)` from a tokio worker thread when complete.

    Returns a non-zero **async handle** for cancellation via [`cancel`], or `0`
    if [`init`] has not been called (no task is spawned).

    Unlike the sync variant, this does **not** block the caller, so multiple
    requests run concurrently on the shared tokio runtime.

    The callback fires exactly once (unless cancelled). It is called from a
    tokio worker thread — the Python side must use `loop.call_soon_threadsafe`
    to resolve a Future from within the callback.

    Python equivalent of `sf_core_api_call_proto_async` from the C API.
    """
    ...

def cancel(async_handle: int) -> None:
    """Cancel the wait for an in-flight async call started by [`call_proto_async`].

    Signals the call's [`CancellationToken`] so the waiter skips the Python
    callback. Until SNOW-3675196, in-flight `handle_message` work is not aborted.

    Unknown async handles (and calls before [`init`]) are silently ignored.
    """
    ...

def perf_enabled() -> bool:
    """Returns True if perf_timing feature is compiled in.
    """
    ...

def get_perf_data() -> tuple[int, int, int]:
    """Read-and-reset perf counters. Returns (batch_wait_ns, chunk_download_ns, arrow_decode_ns).
    """
    ...

def reset_perf_metrics() -> None:
    """Reset perf counters without reading.
    """
    ...

