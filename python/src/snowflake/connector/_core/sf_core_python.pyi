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

    Uses the same level encoding as the inbound wrapper log callback:
    0=ERROR, 1=WARN, 2=INFO, 3 or higher=DEBUG.

    Returns `0` on success, `1` when the pipeline is uninitialised, and `2` if the body panics.
    """
    ...

def call_proto(api: str, method: str, request: bytes) -> tuple[int, bytes]:
    """Synchronous proto API call. Releases the GIL and blocks until complete.

    Returns `(status_code, response_bytes)` where status is:
    - `0` — success
    - `1` — application error (response holds the error payload)
    - `2` — transport error (including panics / missing init caught at this boundary)
    """
    ...

async def call_proto_async(api: str, method: str, request: bytes) -> object:
    """Async proto API call. Returns a Python awaitable → `(status_code, response)`.

    Same contract as [`call_proto`]: always returns `(u32, bytes)`, never raises
    for protocol-level failures.

    Python cancel drops the awaitable which fires the CancellationToken.
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

def native_arrow_enabled() -> bool:
    """Returns True if the ``native-arrow`` Cargo feature is compiled in.
    """
    ...

