from __future__ import annotations

import threading

from typing import Any

from snowflake.connector._internal.status_codes import (
    STATUS_CODE_LABELS,
    STATUS_TO_ERRNO,
    STATUS_TO_EXCEPTION,
    VENDOR_CODE_TO_EXCEPTION,
)
from snowflake.connector.errors import DatabaseError, Error, OperationalError

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
from ..protobuf_gen.database_driver_v1_services import AsyncDatabaseDriverClient, DatabaseDriverClient
from ..protobuf_gen.proto_exception import (
    ProtoApplicationException,
    ProtoTransportException,
)
from .bridge import ProtoTransport


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
# CoreDriver facade (process-wide singleton)
# ---------------------------------------------------------------------------


class CoreDriver:
    """Process-wide facade over ``DatabaseDriverClient``.

    Lazily initializes the underlying protobuf client on first access
    (thread-safe, double-checked lock) and exposes domain-level methods that
    encapsulate all protobuf request construction so that callers never touch
    ``*Request`` objects directly.
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
                self._client = DatabaseDriverClient(ProtoTransport(), error_handler=_proto_to_public_error)

        return self._client

    @client.setter
    def client(self, client: DatabaseDriverClient | None) -> None:
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


core_driver: CoreDriver = CoreDriver()


# ---------------------------------------------------------------------------
# async CoreDriver facade (process-wide singleton)
# ---------------------------------------------------------------------------


class AsyncCoreDriver:
    """Async-native facade over :class:`DatabaseDriverClient`.

    Mirrors :class:`CoreDriver` but exposes ``async def`` methods. Will
    eventually replace ``CoreDriver`` once the codebase is fully async-first;
    for now both coexist — sync callers go through ``core_driver``,
    async-native callers go through ``async_core_driver``.

    Both facades share the process-wide :data:`_proto_transport`.

    Lazily initializes its underlying :class:`DatabaseDriverClient` on first
    access (thread-safe, double-checked lock). Tests can inject a mock by
    assigning to the ``client`` setter.

    Currently, exposes only the subset of methods the cursor layer needs;
    extend as new async callers appear.
    """

    def __init__(self) -> None:
        self._client: AsyncDatabaseDriverClient | None = None
        self._lock = threading.Lock()

    @property
    def client(self) -> AsyncDatabaseDriverClient:
        if self._client is not None:
            return self._client
        with self._lock:
            if self._client is None:
                self._client = AsyncDatabaseDriverClient(
                    ProtoTransport(),
                    error_handler=_proto_to_public_error,
                )
        return self._client

    @client.setter
    def client(self, client: AsyncDatabaseDriverClient | None) -> None:
        self._client = client

    # =====================================================================
    # Statement lifecycle (cursor execute path)
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
    # Connection result-set access (multi-statement / async-query paths)
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

    # =====================================================================
    # Result-set streaming
    # =====================================================================

    async def result_set_get_stream(self, result_set_handle: ResultSetHandle) -> ResultSetGetStreamResponse:
        return await self.client.result_set_get_stream(ResultSetGetStreamRequest(result_set_handle=result_set_handle))

    async def result_set_get_chunks(self, result_set_handle: ResultSetHandle) -> ResultSetGetChunksResponse:
        return await self.client.result_set_get_chunks(ResultSetGetChunksRequest(result_set_handle=result_set_handle))

    async def result_set_release(self, result_set_handle: ResultSetHandle) -> ResultSetReleaseResponse:
        return await self.client.result_set_release(ResultSetReleaseRequest(result_set_handle=result_set_handle))

    async def database_fetch_chunk(
        self,
        db_handle: DatabaseHandle,
        chunk: ResultChunk,
        columns: list[ColumnMetadata],
    ) -> DatabaseFetchChunkResponse:
        return await self.client.database_fetch_chunk(
            DatabaseFetchChunkRequest(db_handle=db_handle, chunk=chunk, columns=columns)
        )


async_core_driver: AsyncCoreDriver = AsyncCoreDriver()
