from __future__ import annotations

import asyncio
import ctypes
import threading

from typing import Any

from snowflake.connector._internal.status_codes import (
    STATUS_CODE_LABELS,
    STATUS_TO_ERRNO,
    STATUS_TO_EXCEPTION,
    VENDOR_CODE_TO_EXCEPTION,
)
from snowflake.connector.errors import DatabaseError, Error, OperationalError

from ..config_utils import create_config_settings_from_dict
from ..logout_config_mapping import LogoutOptionKeys
from ..protobuf_gen.database_driver_v1_pb2 import (
    AuthenticationError as ProtoAuthenticationError,
)
from ..protobuf_gen.database_driver_v1_pb2 import (
    ColumnMetadata,
    ConfigGetPathsRequest,
    ConfigGetPathsResponse,
    ConfigLoadAllSectionsRequest,
    ConfigLoadAllSectionsResponse,
    ConfigSetting,
    ConnectionAbortQueryRequest,
    ConnectionAbortQueryResponse,
    ConnectionCloseRequest,
    ConnectionCloseResponse,
    ConnectionGetAllParametersRequest,
    ConnectionGetAllParametersResponse,
    ConnectionGetInfoRequest,
    ConnectionGetInfoResponse,
    ConnectionGetParameterRequest,
    ConnectionGetParameterResponse,
    ConnectionGetQueryResultRequest,
    ConnectionGetQueryStatusRequest,
    ConnectionGetQueryStatusResponse,
    ConnectionGetResultSetRequest,
    ConnectionHandle,
    ConnectionHeartbeatRequest,
    ConnectionHeartbeatResponse,
    ConnectionInitRequest,
    ConnectionInitResponse,
    ConnectionIsClosedRequest,
    ConnectionIsClosedResponse,
    ConnectionNewRequest,
    ConnectionNewResponse,
    ConnectionReleaseRequest,
    ConnectionReleaseResponse,
    ConnectionSendHttpRequest,
    ConnectionSendHttpResponse,
    ConnectionSetOptionsRequest,
    ConnectionSetOptionsResponse,
    ConnectionSetSessionParametersRequest,
    ConnectionSetSessionParametersResponse,
    ConnectionTokenRequest,
    ConnectionTokenResponse,
    DatabaseFetchChunkRequest,
    DatabaseFetchChunkResponse,
    DatabaseHandle,
    DatabaseInitRequest,
    DatabaseInitResponse,
    DatabaseNewRequest,
    DatabaseNewResponse,
    DatabaseReleaseRequest,
    DatabaseReleaseResponse,
    ExecuteQueryResponse,
    QueryBindings,
    ResultChunk,
    ResultSetGetChunksRequest,
    ResultSetGetChunksResponse,
    ResultSetGetStreamRequest,
    ResultSetGetStreamResponse,
    ResultSetHandle,
    ResultSetReleaseRequest,
    ResultSetReleaseResponse,
    ResultSetResponse,
    StatementExecuteAsyncRequest,
    StatementExecuteAsyncResponse,
    StatementExecuteQueryRequest,
    StatementHandle,
    StatementNewRequest,
    StatementNewResponse,
    StatementPrepareRequest,
    StatementPrepareResponse,
    StatementReleaseRequest,
    StatementReleaseResponse,
    StatementSetOptionsRequest,
    StatementSetOptionsResponse,
    StatementSetSqlQueryRequest,
    StatementSetSqlQueryResponse,
    TelemetrySendApiUsageRequest,
    TelemetrySendResponse,
    TelemetrySendWrapperErrorRequest,
    TokenRequestType,
    WrapperIdentity,
)
from ..protobuf_gen.database_driver_v1_pb2 import (
    InvalidParameterValue as ProtoInvalidParameterValue,
)
from ..protobuf_gen.database_driver_v1_pb2 import (
    LoginError as ProtoLoginError,
)
from ..protobuf_gen.database_driver_v1_pb2 import (
    MissingParameter as ProtoMissingParameter,
)
from ..protobuf_gen.database_driver_v1_services import (
    DatabaseDriverBlockingClient,
    DatabaseDriverClient,
)
from ..protobuf_gen.proto_exception import (
    ProtoApplicationException,
    ProtoTransportException,
)
from .c_api import RESPONSE_CALLBACK, sf_core_api_call_proto_async, sf_core_cancel_request


# ---------------------------------------------------------------------------
# Proto-to-PEP-249 error conversion (kept here, at the transport boundary)
# ---------------------------------------------------------------------------


def _extract_error_detail(driver_exception: Any) -> str | None:
    error = getattr(driver_exception, "error", None)
    if error is None:
        return None

    error_type = error.WhichOneof("error_type")
    if error_type is None:
        return None

    inner = getattr(error, error_type, None)
    if inner is None:
        return None

    if isinstance(inner, ProtoAuthenticationError):
        return inner.detail or None
    if isinstance(inner, ProtoLoginError):
        if inner.message and inner.code:
            return f"{inner.message} (code={inner.code})"
        return inner.message or None
    if isinstance(inner, ProtoMissingParameter):
        return f"Missing required parameter: {inner.parameter}" if inner.parameter else None
    if isinstance(inner, ProtoInvalidParameterValue):
        parts = [f"Invalid value {inner.value!r} for parameter {inner.parameter!r}"]
        if inner.explanation:
            parts.append(inner.explanation)
        return ". ".join(parts)
    # GenericError, InternalError have no extra fields
    return None


def _append_detail(base: str, detail: str) -> str:
    """Append *detail* to *base* with `. ` separator, avoiding double punctuation."""
    if not base:
        return detail
    base = base.rstrip(".")
    return f"{base}. {detail}"


def _proto_to_public_error(proto_exc: Exception) -> Error:
    """Convert a proto-layer exception into a PEP 249 ``Error`` subclass.

    This function **returns** the converted exception; it does not raise it.
    The caller (``_raise_error`` in the generated client) is responsible for
    raising the returned value.
    """
    if isinstance(proto_exc, ProtoApplicationException):
        return _convert_application_error(proto_exc)
    if isinstance(proto_exc, ProtoTransportException):
        return OperationalError(f"Driver communication error: {proto_exc}")
    return DatabaseError(str(proto_exc))


def _resolve_exception_class(status_code: int, vendor_code: int | None) -> type[Error]:
    """Pick the PEP 249 exception class for a proto error.

    Resolution order:
      1. VENDOR_CODE_TO_EXCEPTION — Snowflake-specific vendor_code overrides (e.g. 100072 → IntegrityError).
      2. STATUS_TO_EXCEPTION — default mapping from the proto StatusCode.
      3. DatabaseError — catch-all when the status code is unrecognized.
    """
    if vendor_code is not None:
        cls = VENDOR_CODE_TO_EXCEPTION.get(vendor_code)
        if cls is not None:
            return cls
    return STATUS_TO_EXCEPTION.get(status_code, DatabaseError)


def _convert_application_error(proto_exc: ProtoApplicationException) -> Error:
    driver_exc = getattr(proto_exc, "api_error_pb", None)
    if driver_exc is None:
        return DatabaseError(str(proto_exc))

    status_code = getattr(driver_exc, "status_code", 0)
    message = getattr(driver_exc, "message", "") or ""

    # The root_cause field carries the deepest error in the chain from the
    # Rust core, which is typically the most informative for end users.
    root_cause = _get_optional_str(driver_exc, "root_cause")
    if root_cause and root_cause not in message:
        message = _append_detail(message, root_cause)

    detail = _extract_error_detail(driver_exc)
    if detail and detail not in message:
        message = _append_detail(message, detail)

    if not message:
        message = STATUS_CODE_LABELS.get(status_code, "Unknown error")

    # Prefer the Snowflake server vendor_code when the core driver provides it
    # (e.g. 1003 for syntax error, 904 for invalid identifier).
    # Fall back to the old-driver-compatible errno mapping, then to the raw
    # proto status code.
    vendor_code = _get_optional_int(driver_exc, "vendor_code")

    exc_class = _resolve_exception_class(status_code, vendor_code)
    errno = vendor_code if vendor_code is not None else STATUS_TO_ERRNO.get(status_code, status_code)

    # Prefer the server-provided sql_state; fall back to a type-derived value.
    sqlstate = _get_optional_str(driver_exc, "sql_state") or _derive_sqlstate(driver_exc)

    sfqid = _get_optional_str(driver_exc, "query_id")

    return exc_class(message, errno=errno, sqlstate=sqlstate, sfqid=sfqid)


def _get_optional_int(msg: Any, field: str) -> int | None:
    """Read an optional int32 proto field, returning None if not set."""
    try:
        if msg.HasField(field):
            return int(getattr(msg, field))
    except (ValueError, TypeError, AttributeError):
        # Field missing from the proto schema or cannot be coerced to int; treat as unset.
        pass
    return None


def _get_optional_str(msg: Any, field: str) -> str | None:
    """Read an optional string proto field, returning None if not set."""
    try:
        if msg.HasField(field):
            return str(getattr(msg, field)) or None
    except (ValueError, TypeError, AttributeError):
        # Field missing from the proto schema or cannot be coerced to string; treat as unset.
        pass
    return None


def _derive_sqlstate(driver_exception: Any) -> str | None:
    """Derive sqlstate from the error type when the proto does not carry it.

    Only login/auth errors have an obvious ANSI SQL state mapping today.
    Other error types (missing_parameter, invalid_parameter_value, etc.)
    will return ``None``; extend this function as mappings become clear.
    """
    error = getattr(driver_exception, "error", None)
    if error is None:
        return None
    error_type = error.WhichOneof("error_type")
    if error_type in ("login_error", "auth_error"):
        return "08001"  # SQLSTATE_CONNECTION_WAS_NOT_ESTABLISHED
    return None


# ---------------------------------------------------------------------------
# Transport + singleton
# ---------------------------------------------------------------------------


class ProtoTransport:
    """Async, callback-based bridge to the Rust core RPC layer.

    Each call:
      1. Creates an :class:`asyncio.Future` bound to the running event loop.
      2. Builds a C callback that resolves the Future when Rust completes.
      3. Submits the request to ``sf_core_api_call_proto_async`` (returns
         immediately — Rust spawns the work on its tokio runtime).
      4. Awaits the Future.

    Lifetime correctness is critical: the C callback object **must** outlive
    the Rust task that may invoke it. We pin it to the Future so it stays alive
    until the Future is resolved (and thus garbage-collected only after the
    awaiting coroutine resumes). Stack-locals are not safe — if the awaiting
    coroutine is cancelled, its frame is collected before Rust may have fired
    the callback.
    """

    async def handle_message(self, api: str, method: str, message: bytes) -> tuple[int, bytes]:
        loop = asyncio.get_running_loop()
        future: asyncio.Future[tuple[int, bytes]] = loop.create_future()

        # Closure captures `loop` and `future` only — no `self`.
        def on_response(
            user_data_ptr: int,
            status: int,
            response_ptr: Any,
            response_len: int,
        ) -> None:
            # Copy response bytes BEFORE returning — Rust frees the buffer next.
            # `string_at` is a single memcpy; avoids the O(n) Python __getitem__
            # loop you get from `bytes(ptr[:n])`.
            response_bytes = ctypes.string_at(response_ptr, response_len)

            def _set() -> None:
                # The Future may have been cancelled while Rust was working;
                # `set_result` would raise InvalidStateError. Guard explicitly.
                if not future.done():
                    future.set_result((status, response_bytes))

            loop.call_soon_threadsafe(_set)

        callback_ref = RESPONSE_CALLBACK(on_response)
        # Pin the callback to the Future so it cannot be garbage-collected
        # before Rust invokes it. Without this, an awaiting coroutine that
        # gets cancelled could free the callback object while Rust still
        # holds the function pointer — use-after-free / segfault.
        #
        # Note: this creates a deliberate reference cycle
        # (future -> callback_ref -> on_response closure -> future). It is
        # broken by the cycle GC after the Future resolves and the awaiting
        # coroutine drops its reference. Do not optimise this pin away
        # thinking it is redundant — the lifetime invariant matters precisely
        # in the cancellation path.
        future._proto_transport_callback_ref = callback_ref  # type: ignore[attr-defined]

        # Build a C-compatible buffer from the message bytes. (c_ubyte * n) is
        # contiguous and ctypes can pass its address directly — no extra copy
        # versus the prior `message_buf[:] = message` approach, but cleaner.
        request_buf = (ctypes.c_ubyte * len(message)).from_buffer_copy(message)

        request_id = sf_core_api_call_proto_async(
            api.encode("utf-8"),
            method.encode("utf-8"),
            ctypes.cast(request_buf, ctypes.POINTER(ctypes.c_ubyte)),
            len(message),
            callback_ref,
            None,  # user_data — unused; we capture future in the closure
        )

        try:
            status, response_bytes = await future
        except asyncio.CancelledError:
            # Tell Rust to abort the in-flight task at its next await point.
            # The callback may still fire if Rust is past that point — the
            # `future.done()` guard above makes that case a safe no-op.
            sf_core_cancel_request(request_id)
            raise

        if status in (0, 1, 2):
            return (status, response_bytes)
        raise ProtoTransportException(f"Unknown error code: {status}")


_background_loop: asyncio.AbstractEventLoop | None = None
_background_loop_thread: threading.Thread | None = None
_background_loop_lock = threading.Lock()


def _run_background_loop(loop: asyncio.AbstractEventLoop) -> None:
    """Entry point for the background event-loop thread.

    After ``loop.stop()`` is called (from the atexit handler or elsewhere),
    ``run_forever`` returns and we clean up the default executor so that no
    stale ``asyncio.to_thread`` work is leaked.
    """
    try:
        loop.run_forever()
    finally:
        try:
            loop.run_until_complete(loop.shutdown_default_executor())
        except Exception:
            pass
        loop.close()


def _shutdown_background_loop() -> None:
    """Stop the background loop and join its thread at process exit.

    Registered *before* any ``Connection._close_at_process_exit`` handler,
    so it runs *after* all connection atexit handlers (LIFO order).
    """
    global _background_loop
    loop = _background_loop
    if loop is None or not loop.is_running():
        return
    loop.call_soon_threadsafe(loop.stop)
    if _background_loop_thread is not None:
        _background_loop_thread.join(timeout=5)


def get_background_loop() -> asyncio.AbstractEventLoop:
    """Return the process-wide background event loop for sync-over-async bridging.

    The loop is created lazily on first call and runs in a daemon thread for
    the lifetime of the process. All blocking wrappers
    (:class:`DatabaseDriverBlockingClient`, :class:`BlockingImmutableCursor`,
    etc.) submit coroutines to this loop.

    An ``atexit`` handler is registered to cleanly stop the loop after all
    connection-level atexit handlers have run (LIFO ordering guarantee).
    """
    global _background_loop, _background_loop_thread
    loop = _background_loop
    if loop is not None:
        return loop
    with _background_loop_lock:
        loop = _background_loop
        if loop is not None:
            return loop
        import atexit

        loop = asyncio.new_event_loop()
        thread = threading.Thread(
            target=_run_background_loop,
            args=(loop,),
            name="sf-background-event-loop",
            daemon=True,
        )
        thread.start()
        atexit.register(_shutdown_background_loop)
        _background_loop = loop
        _background_loop_thread = thread
        return loop


class CoreDriver:
    """Process-wide facade over ``DatabaseDriverClient``.

    Lazily initializes the underlying protobuf client on first access
    (thread-safe, double-checked lock) and exposes domain-level methods that
    encapsulate all protobuf request construction so that callers never touch
    ``*Request`` objects directly.

    The held client is a :class:`DatabaseDriverBlockingClient` that wraps the
    async-first :class:`DatabaseDriverClient`. This keeps every CoreDriver
    method synchronous for existing callers while the underlying transport
    runs the FFI call asynchronously via a callback-based bridge to tokio.
    """

    def __init__(self) -> None:
        self._client: DatabaseDriverBlockingClient | None = None
        self._lock = threading.Lock()

    @property
    def client(self) -> DatabaseDriverBlockingClient:
        if self._client is not None:
            return self._client

        with self._lock:
            if self._client is None:
                async_client = DatabaseDriverClient(ProtoTransport(), error_handler=_proto_to_public_error)
                self._client = DatabaseDriverBlockingClient(async_client, get_background_loop())

        return self._client

    @client.setter
    def client(self, client: DatabaseDriverBlockingClient | None) -> None:
        self._client = client

    # =====================================================================
    # Database lifecycle
    # =====================================================================

    def database_new(self) -> DatabaseNewResponse:
        request = DatabaseNewRequest()
        return self.client.database_new(request)

    def database_init(self, db_handle: DatabaseHandle) -> DatabaseInitResponse:
        request = DatabaseInitRequest(db_handle=db_handle)
        return self.client.database_init(request)

    def database_release(self, db_handle: DatabaseHandle) -> DatabaseReleaseResponse:
        request = DatabaseReleaseRequest(db_handle=db_handle)
        return self.client.database_release(request)

    # =====================================================================
    # Connection lifecycle
    # =====================================================================

    def connection_new(self) -> ConnectionNewResponse:
        request = ConnectionNewRequest()
        return self.client.connection_new(request)

    def connection_init(
        self,
        conn_handle: ConnectionHandle,
        db_handle: DatabaseHandle,
        wrapper_identity: WrapperIdentity,
    ) -> ConnectionInitResponse:
        request = ConnectionInitRequest(
            conn_handle=conn_handle,
            db_handle=db_handle,
            wrapper_identity=wrapper_identity,
        )
        return self.client.connection_init(request)

    def connection_set_options(
        self,
        conn_handle: ConnectionHandle,
        options: dict[str, ConfigSetting],
    ) -> ConnectionSetOptionsResponse:
        request = ConnectionSetOptionsRequest(conn_handle=conn_handle, options=options)
        return self.client.connection_set_options(request)

    def connection_set_session_parameters(
        self, conn_handle: ConnectionHandle, parameters: dict[str, str]
    ) -> ConnectionSetSessionParametersResponse:
        request = ConnectionSetSessionParametersRequest(conn_handle=conn_handle, parameters=parameters)
        return self.client.connection_set_session_parameters(request)

    def connection_close(self, conn_handle: ConnectionHandle) -> ConnectionCloseResponse:
        request = ConnectionCloseRequest(conn_handle=conn_handle)
        return self.client.connection_close(request)

    def connection_release(self, conn_handle: ConnectionHandle) -> ConnectionReleaseResponse:
        request = ConnectionReleaseRequest(conn_handle=conn_handle)
        return self.client.connection_release(request)

    def connection_is_closed(self, conn_handle: ConnectionHandle) -> ConnectionIsClosedResponse:
        request = ConnectionIsClosedRequest(conn_handle=conn_handle)
        return self.client.connection_is_closed(request)

    def connection_heartbeat(self, conn_handle: ConnectionHandle) -> ConnectionHeartbeatResponse:
        request = ConnectionHeartbeatRequest(conn_handle=conn_handle)
        return self.client.connection_heartbeat(request)

    def connection_get_info(
        self,
        conn_handle: ConnectionHandle,
        include_master_token: bool = False,
    ) -> ConnectionGetInfoResponse:
        request = ConnectionGetInfoRequest(conn_handle=conn_handle, include_master_token=include_master_token)
        return self.client.connection_get_info(request)

    def connection_get_query_status(
        self, conn_handle: ConnectionHandle, query_id: str
    ) -> ConnectionGetQueryStatusResponse:
        request = ConnectionGetQueryStatusRequest(conn_handle=conn_handle, query_id=query_id)
        return self.client.connection_get_query_status(request)

    # =====================================================================
    # Connection data
    # =====================================================================

    def connection_get_result_set(self, conn_handle: ConnectionHandle, query_id: str) -> ResultSetResponse:
        request = ConnectionGetResultSetRequest(conn_handle=conn_handle, query_id=query_id)
        return self.client.connection_get_result_set(request)

    def connection_get_query_result(self, conn_handle: ConnectionHandle, query_id: str) -> ExecuteQueryResponse:
        request = ConnectionGetQueryResultRequest(conn_handle=conn_handle, query_id=query_id)
        return self.client.connection_get_query_result(request)

    def connection_abort_query(self, conn_handle: ConnectionHandle, query_id: str) -> ConnectionAbortQueryResponse:
        request = ConnectionAbortQueryRequest(conn_handle=conn_handle, query_id=query_id)
        return self.client.connection_abort_query(request)

    def connection_send_http(
        self,
        conn_handle: ConnectionHandle,
        method: str,
        url: str,
        headers: dict[str, str],
        body: bytes | None = None,
    ) -> ConnectionSendHttpResponse:
        request = ConnectionSendHttpRequest(
            conn_handle=conn_handle,
            method=method,
            url=url,
            headers=headers,
            body=body,
        )
        return self.client.connection_send_http(request)

    # =====================================================================
    # Connection tokens/params
    # =====================================================================

    def connection_request_token(
        self, conn_handle: ConnectionHandle, request_type: TokenRequestType.ValueType
    ) -> ConnectionTokenResponse:
        request = ConnectionTokenRequest(conn_handle=conn_handle, request_type=request_type)
        return self.client.connection_request_token(request)

    def connection_get_parameter(self, conn_handle: ConnectionHandle, key: str) -> ConnectionGetParameterResponse:
        request = ConnectionGetParameterRequest(conn_handle=conn_handle, key=key)
        return self.client.connection_get_parameter(request)

    def connection_get_all_parameters(self, conn_handle: ConnectionHandle) -> ConnectionGetAllParametersResponse:
        request = ConnectionGetAllParametersRequest(conn_handle=conn_handle)
        return self.client.connection_get_all_parameters(request)

    # =====================================================================
    # Statement lifecycle
    # =====================================================================

    def statement_new(self, conn_handle: ConnectionHandle) -> StatementNewResponse:
        request = StatementNewRequest(conn_handle=conn_handle)
        return self.client.statement_new(request)

    def statement_set_query(self, stmt_handle: StatementHandle, query: str) -> StatementSetSqlQueryResponse:
        request = StatementSetSqlQueryRequest(stmt_handle=stmt_handle, query=query)
        return self.client.statement_set_sql_query(request)

    def statement_release(self, stmt_handle: StatementHandle) -> StatementReleaseResponse:
        request = StatementReleaseRequest(stmt_handle=stmt_handle)
        return self.client.statement_release(request)

    def statement_set_options(
        self, stmt_handle: StatementHandle, options: dict[str, ConfigSetting]
    ) -> StatementSetOptionsResponse:
        request = StatementSetOptionsRequest(stmt_handle=stmt_handle, options=options)
        return self.client.statement_set_options(request)

    def statement_execute_query(
        self, stmt_handle: StatementHandle, bindings: QueryBindings | None = None
    ) -> ExecuteQueryResponse:
        request = StatementExecuteQueryRequest(stmt_handle=stmt_handle, bindings=bindings)
        return self.client.statement_execute_query(request)

    def statement_execute_async(
        self, stmt_handle: StatementHandle, bindings: QueryBindings | None = None
    ) -> StatementExecuteAsyncResponse:
        request = StatementExecuteAsyncRequest(stmt_handle=stmt_handle, bindings=bindings)
        return self.client.statement_execute_async(request)

    def statement_prepare(self, stmt_handle: StatementHandle) -> StatementPrepareResponse:
        request = StatementPrepareRequest(stmt_handle=stmt_handle)
        return self.client.statement_prepare(request)

    # =====================================================================
    # Result set
    # =====================================================================

    def result_set_release(self, result_set_handle: ResultSetHandle) -> ResultSetReleaseResponse:
        request = ResultSetReleaseRequest(result_set_handle=result_set_handle)
        return self.client.result_set_release(request)

    def result_set_get_stream(self, result_set_handle: ResultSetHandle) -> ResultSetGetStreamResponse:
        request = ResultSetGetStreamRequest(result_set_handle=result_set_handle)
        return self.client.result_set_get_stream(request)

    def result_set_get_chunks(self, result_set_handle: ResultSetHandle) -> ResultSetGetChunksResponse:
        request = ResultSetGetChunksRequest(result_set_handle=result_set_handle)
        return self.client.result_set_get_chunks(request)

    # =====================================================================
    # Database fetch
    # =====================================================================

    def database_fetch_chunk(
        self,
        db_handle: DatabaseHandle,
        chunk: ResultChunk,
        columns: list[ColumnMetadata],
    ) -> DatabaseFetchChunkResponse:
        request = DatabaseFetchChunkRequest(db_handle=db_handle, chunk=chunk, columns=columns)
        return self.client.database_fetch_chunk(request)

    # =====================================================================
    # Telemetry
    # =====================================================================

    def telemetry_send_api_usage(self, conn_handle: ConnectionHandle, api_method: str) -> TelemetrySendResponse:
        request = TelemetrySendApiUsageRequest(conn_handle=conn_handle, api_method=api_method)
        return self.client.telemetry_send_api_usage(request)

    def telemetry_send_wrapper_error(
        self, conn_handle: ConnectionHandle, exception_type: str, error_source: str
    ) -> TelemetrySendResponse:
        request = TelemetrySendWrapperErrorRequest(
            conn_handle=conn_handle,
            exception_type=exception_type,
            error_source=error_source,
        )
        return self.client.telemetry_send_wrapper_error(request)

    # =====================================================================
    # Config
    # =====================================================================

    def config_load_all_sections(
        self,
        config_file: str,
        connections_file: str | None = None,
    ) -> ConfigLoadAllSectionsResponse:
        request = ConfigLoadAllSectionsRequest(config_file=config_file, connections_file=connections_file)
        return self.client.config_load_all_sections(request)

    def config_get_paths(self) -> ConfigGetPathsResponse:
        request = ConfigGetPathsRequest()
        return self.client.config_get_paths(request)


core_driver = CoreDriver()

# ---------------------------------------------------------------------------
# Sync FFI close for atexit
# ---------------------------------------------------------------------------

_sync_transport = ProtoTransport()


def _call_ffi_sync(method: str, request_bytes: bytes) -> bytes:
    """Direct synchronous FFI call bypassing the async event loop.

    Used exclusively by :func:`connection_close_at_exit` so that atexit
    handlers never depend on ``asyncio.to_thread`` or the background loop
    (both are fragile during interpreter shutdown).
    """
    code, response_bytes = _sync_transport._handle_message_sync("DatabaseDriver", method, request_bytes)
    if code == 0:
        return response_bytes
    # Best-effort: swallow application/transport errors during atexit
    return b""


def connection_close_at_exit(
    conn_handle: ConnectionHandle,
    db_handle: DatabaseHandle | None,
) -> None:
    """Close a connection synchronously for atexit — bypass the async event loop.

    Performs the same steps as ``Connection.close(retry=False)`` followed by
    handle release, but calls ``sf_core_api_call_proto`` directly instead of
    routing through ``asyncio.to_thread``.  This avoids the hang/failure that
    occurs when the thread-pool executor is torn down during interpreter
    shutdown.
    """
    try:
        resp_bytes = _call_ffi_sync(
            "connection_is_closed",
            ConnectionIsClosedRequest(conn_handle=conn_handle).SerializeToString(),
        )
        if resp_bytes:
            resp = ConnectionIsClosedResponse()
            resp.ParseFromString(resp_bytes)
            if resp.is_closed:
                return
    except Exception:
        pass

    try:
        _call_ffi_sync(
            "connection_set_options",
            ConnectionSetOptionsRequest(
                conn_handle=conn_handle,
                options=create_config_settings_from_dict({LogoutOptionKeys.LOGOUT_MAX_ATTEMPTS: 1}),
            ).SerializeToString(),
        )
    except Exception:
        pass

    try:
        _call_ffi_sync(
            "connection_close",
            ConnectionCloseRequest(conn_handle=conn_handle).SerializeToString(),
        )
    except Exception:
        pass

    try:
        _call_ffi_sync(
            "connection_release",
            ConnectionReleaseRequest(conn_handle=conn_handle).SerializeToString(),
        )
    except Exception:
        pass

    if db_handle is not None:
        try:
            _call_ffi_sync(
                "database_release",
                DatabaseReleaseRequest(db_handle=db_handle).SerializeToString(),
            )
        except Exception:
            pass


class AsyncCoreDriver:
    """Async-native facade over :class:`DatabaseDriverClient`.

    The single source of truth for all protobuf RPC calls. Sync callers go
    through :class:`CoreDriver` (which bridges to this via
    :func:`get_background_loop`), async-native callers use
    ``async_core_driver`` directly.

    Lazily initializes its underlying :class:`DatabaseDriverClient` on first
    access (thread-safe, double-checked lock). Tests can inject a mock by
    assigning to the ``client`` setter.
    """

    def __init__(self) -> None:
        self._client: DatabaseDriverClient | None = None
        self._lock = threading.Lock()

    @property
    def client(self) -> DatabaseDriverClient:
        if self._client is not None:
            return self._client
        with self._lock:
            if self._client is None:
                self._client = DatabaseDriverClient(
                    ProtoTransport(),
                    error_handler=_proto_to_public_error,
                )
        return self._client

    @client.setter
    def client(self, client: DatabaseDriverClient | None) -> None:
        self._client = client

    # =====================================================================
    # Database lifecycle
    # =====================================================================

    async def database_new(self) -> DatabaseNewResponse:
        return await self.client.database_new(DatabaseNewRequest())

    async def database_init(self, db_handle: DatabaseHandle) -> DatabaseInitResponse:
        return await self.client.database_init(DatabaseInitRequest(db_handle=db_handle))

    async def database_release(self, db_handle: DatabaseHandle) -> DatabaseReleaseResponse:
        return await self.client.database_release(DatabaseReleaseRequest(db_handle=db_handle))

    # =====================================================================
    # Connection lifecycle
    # =====================================================================

    async def connection_new(self) -> ConnectionNewResponse:
        return await self.client.connection_new(ConnectionNewRequest())

    async def connection_init(
        self,
        conn_handle: ConnectionHandle,
        db_handle: DatabaseHandle,
        wrapper_identity: WrapperIdentity,
    ) -> ConnectionInitResponse:
        return await self.client.connection_init(
            ConnectionInitRequest(conn_handle=conn_handle, db_handle=db_handle, wrapper_identity=wrapper_identity)
        )

    async def connection_set_options(
        self,
        conn_handle: ConnectionHandle,
        options: dict[str, ConfigSetting],
    ) -> ConnectionSetOptionsResponse:
        return await self.client.connection_set_options(
            ConnectionSetOptionsRequest(conn_handle=conn_handle, options=options)
        )

    async def connection_set_session_parameters(
        self, conn_handle: ConnectionHandle, parameters: dict[str, str]
    ) -> ConnectionSetSessionParametersResponse:
        return await self.client.connection_set_session_parameters(
            ConnectionSetSessionParametersRequest(conn_handle=conn_handle, parameters=parameters)
        )

    async def connection_close(self, conn_handle: ConnectionHandle) -> ConnectionCloseResponse:
        return await self.client.connection_close(ConnectionCloseRequest(conn_handle=conn_handle))

    async def connection_release(self, conn_handle: ConnectionHandle) -> ConnectionReleaseResponse:
        return await self.client.connection_release(ConnectionReleaseRequest(conn_handle=conn_handle))

    async def connection_is_closed(self, conn_handle: ConnectionHandle) -> ConnectionIsClosedResponse:
        return await self.client.connection_is_closed(ConnectionIsClosedRequest(conn_handle=conn_handle))

    async def connection_heartbeat(self, conn_handle: ConnectionHandle) -> ConnectionHeartbeatResponse:
        return await self.client.connection_heartbeat(ConnectionHeartbeatRequest(conn_handle=conn_handle))

    async def connection_get_info(
        self,
        conn_handle: ConnectionHandle,
        include_master_token: bool = False,
    ) -> ConnectionGetInfoResponse:
        return await self.client.connection_get_info(
            ConnectionGetInfoRequest(conn_handle=conn_handle, include_master_token=include_master_token)
        )

    async def connection_get_query_status(
        self, conn_handle: ConnectionHandle, query_id: str
    ) -> ConnectionGetQueryStatusResponse:
        return await self.client.connection_get_query_status(
            ConnectionGetQueryStatusRequest(conn_handle=conn_handle, query_id=query_id)
        )

    # =====================================================================
    # Connection data
    # =====================================================================

    async def connection_get_result_set(self, conn_handle: ConnectionHandle, query_id: str) -> ResultSetResponse:
        return await self.client.connection_get_result_set(
            ConnectionGetResultSetRequest(conn_handle=conn_handle, query_id=query_id)
        )

    async def connection_get_query_result(self, conn_handle: ConnectionHandle, query_id: str) -> ExecuteQueryResponse:
        return await self.client.connection_get_query_result(
            ConnectionGetQueryResultRequest(conn_handle=conn_handle, query_id=query_id)
        )

    async def connection_abort_query(
        self, conn_handle: ConnectionHandle, query_id: str
    ) -> ConnectionAbortQueryResponse:
        return await self.client.connection_abort_query(
            ConnectionAbortQueryRequest(conn_handle=conn_handle, query_id=query_id)
        )

    async def connection_send_http(
        self,
        conn_handle: ConnectionHandle,
        method: str,
        url: str,
        headers: dict[str, str],
        body: bytes | None = None,
    ) -> ConnectionSendHttpResponse:
        return await self.client.connection_send_http(
            ConnectionSendHttpRequest(conn_handle=conn_handle, method=method, url=url, headers=headers, body=body)
        )

    # =====================================================================
    # Connection tokens/params
    # =====================================================================

    async def connection_request_token(
        self, conn_handle: ConnectionHandle, request_type: TokenRequestType.ValueType
    ) -> ConnectionTokenResponse:
        return await self.client.connection_request_token(
            ConnectionTokenRequest(conn_handle=conn_handle, request_type=request_type)
        )

    async def connection_get_parameter(self, conn_handle: ConnectionHandle, key: str) -> ConnectionGetParameterResponse:
        return await self.client.connection_get_parameter(
            ConnectionGetParameterRequest(conn_handle=conn_handle, key=key)
        )

    async def connection_get_all_parameters(self, conn_handle: ConnectionHandle) -> ConnectionGetAllParametersResponse:
        return await self.client.connection_get_all_parameters(
            ConnectionGetAllParametersRequest(conn_handle=conn_handle)
        )

    # =====================================================================
    # Statement lifecycle
    # =====================================================================

    async def statement_new(self, conn_handle: ConnectionHandle) -> StatementNewResponse:
        return await self.client.statement_new(StatementNewRequest(conn_handle=conn_handle))

    async def statement_set_query(self, stmt_handle: StatementHandle, query: str) -> StatementSetSqlQueryResponse:
        return await self.client.statement_set_sql_query(
            StatementSetSqlQueryRequest(stmt_handle=stmt_handle, query=query)
        )

    async def statement_release(self, stmt_handle: StatementHandle) -> StatementReleaseResponse:
        return await self.client.statement_release(StatementReleaseRequest(stmt_handle=stmt_handle))

    async def statement_set_options(
        self, stmt_handle: StatementHandle, options: dict[str, ConfigSetting]
    ) -> StatementSetOptionsResponse:
        return await self.client.statement_set_options(
            StatementSetOptionsRequest(stmt_handle=stmt_handle, options=options)
        )

    async def statement_execute_query(
        self, stmt_handle: StatementHandle, bindings: QueryBindings | None = None
    ) -> ExecuteQueryResponse:
        return await self.client.statement_execute_query(
            StatementExecuteQueryRequest(stmt_handle=stmt_handle, bindings=bindings)
        )

    async def statement_execute_async(
        self, stmt_handle: StatementHandle, bindings: QueryBindings | None = None
    ) -> StatementExecuteAsyncResponse:
        return await self.client.statement_execute_async(
            StatementExecuteAsyncRequest(stmt_handle=stmt_handle, bindings=bindings)
        )

    async def statement_prepare(self, stmt_handle: StatementHandle) -> StatementPrepareResponse:
        return await self.client.statement_prepare(StatementPrepareRequest(stmt_handle=stmt_handle))

    # =====================================================================
    # Result set
    # =====================================================================

    async def result_set_release(self, result_set_handle: ResultSetHandle) -> ResultSetReleaseResponse:
        return await self.client.result_set_release(ResultSetReleaseRequest(result_set_handle=result_set_handle))

    async def result_set_get_stream(self, result_set_handle: ResultSetHandle) -> ResultSetGetStreamResponse:
        return await self.client.result_set_get_stream(ResultSetGetStreamRequest(result_set_handle=result_set_handle))

    async def result_set_get_chunks(self, result_set_handle: ResultSetHandle) -> ResultSetGetChunksResponse:
        return await self.client.result_set_get_chunks(ResultSetGetChunksRequest(result_set_handle=result_set_handle))

    # =====================================================================
    # Database fetch
    # =====================================================================

    async def database_fetch_chunk(
        self,
        db_handle: DatabaseHandle,
        chunk: ResultChunk,
        columns: list[ColumnMetadata],
    ) -> DatabaseFetchChunkResponse:
        return await self.client.database_fetch_chunk(
            DatabaseFetchChunkRequest(db_handle=db_handle, chunk=chunk, columns=columns)
        )

    # =====================================================================
    # Telemetry
    # =====================================================================

    async def telemetry_send_api_usage(self, conn_handle: ConnectionHandle, api_method: str) -> TelemetrySendResponse:
        return await self.client.telemetry_send_api_usage(
            TelemetrySendApiUsageRequest(conn_handle=conn_handle, api_method=api_method)
        )

    async def telemetry_send_wrapper_error(
        self, conn_handle: ConnectionHandle, exception_type: str, error_source: str
    ) -> TelemetrySendResponse:
        return await self.client.telemetry_send_wrapper_error(
            TelemetrySendWrapperErrorRequest(
                conn_handle=conn_handle, exception_type=exception_type, error_source=error_source
            )
        )

    # =====================================================================
    # Config
    # =====================================================================

    async def config_load_all_sections(
        self,
        config_file: str,
        connections_file: str | None = None,
    ) -> ConfigLoadAllSectionsResponse:
        return await self.client.config_load_all_sections(
            ConfigLoadAllSectionsRequest(config_file=config_file, connections_file=connections_file)
        )

    async def config_get_paths(self) -> ConfigGetPathsResponse:
        return await self.client.config_get_paths(ConfigGetPathsRequest())


async_core_driver = AsyncCoreDriver()
