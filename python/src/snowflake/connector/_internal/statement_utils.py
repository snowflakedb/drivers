from __future__ import annotations

from collections.abc import Generator
from contextlib import contextmanager
from typing import TYPE_CHECKING

from .protobuf_gen.database_driver_v1_pb2 import (
    DatabaseFetchChunkResponse,
    PrepareResult,
    ResultSetDescriptor,
    ResultSetResponse,
    StatementHandle,
    StatementNewRequest,
    StatementReleaseRequest,
    StatementSetSqlQueryRequest,
)
from .sqlstate import SQLSTATE_SUCCESS


if TYPE_CHECKING:
    from ..connection import Connection


def new_stmt(connection: Connection) -> StatementHandle:
    statement_request = StatementNewRequest(conn_handle=connection.conn_handle)
    stmt = connection.db_api.statement_new(request=statement_request)
    return stmt.stmt_handle


def set_query(connection: Connection, stmt_handle: StatementHandle, query: str) -> None:
    sql_query_request = StatementSetSqlQueryRequest(stmt_handle=stmt_handle, query=query)
    connection.db_api.statement_set_sql_query(sql_query_request)


def release_stmt(connection: Connection, stmt_handle: StatementHandle | None) -> StatementHandle | None:
    if stmt_handle:
        release_request = StatementReleaseRequest(stmt_handle=stmt_handle)
        connection.db_api.statement_release(release_request)
    return None


@contextmanager
def statement(connection: Connection, query: str) -> Generator[StatementHandle]:
    """Context manager that owns the full lifecycle of a statement handle.

    Allocates a new statement on the server, binds the given SQL query to it,
    and yields the ``StatementHandle`` for execution.  The statement is
    guaranteed to be released when the context exits, even if an exception
    is raised.

    Args:
        connection: Active Snowflake connection used to issue gRPC calls.
        query: SQL text to bind to the newly created statement.

    Yields:
        StatementHandle: A handle that can be passed to ``statement_execute``
        or other statement-level APIs.
    """
    stmt_handle = new_stmt(connection)
    try:
        set_query(connection, stmt_handle, query)
        yield stmt_handle
    finally:
        release_stmt(connection, stmt_handle)


def get_stream_ptr(result: DatabaseFetchChunkResponse | PrepareResult | ResultSetResponse | None) -> int:
    """Extract a C ArrowArrayStream pointer from an execute result.

    The pointer is stored as an 8-byte little-endian value inside
    ``result.stream.value``.  This function validates every step of
    the extraction and raises descriptive errors on failure, so callers
    can safely pass the returned integer to Arrow C Data Interface
    consumers (e.g. PyArrow ``RecordBatchReader.from_stream``).

    Args:
        result: The protobuf response returned by ``statement_execute``.
            Must not be ``None`` and must carry a populated ``stream``
            field.

    Returns:
        A non-zero integer representing the memory address of the
        ``ArrowArrayStream`` struct.

    Raises:
        RuntimeError: If *result* is ``None``, the stream or its value
            is missing, the value has an unexpected length, or the
            decoded pointer is null.
    """
    if result is None:
        raise RuntimeError("No query has been executed")

    if not hasattr(result, "stream") or result.stream is None:
        raise RuntimeError("Execute result does not contain a valid stream")

    if not hasattr(result.stream, "value") or result.stream.value is None:
        raise RuntimeError("Stream does not contain a valid pointer value")

    stream_value = result.stream.value
    # 8 bytes = 64-bit pointer, the size of a C ArrowArrayStream* on a 64-bit platform
    if len(stream_value) != 8:
        raise RuntimeError(f"Stream pointer value has wrong length: {len(stream_value)} (expected 8)")

    stream_ptr = int.from_bytes(stream_value, byteorder="little", signed=False)

    if stream_ptr == 0:
        raise RuntimeError("Stream pointer is null")

    return stream_ptr


def extract_sqlstate(result: PrepareResult | ResultSetDescriptor | None) -> str | None:
    """Return the SQLSTATE code from an execute result, if meaningful.

    SQLSTATE ``"00000"`` (successful completion) is normalized to ``None``
    for backwards compatibility with the legacy connector, which omits
    the code on success.

    Args:
        result: A ``PrepareResult`` or ``ResultSetDescriptor``,
            or ``None`` if no result is available.

    Returns:
        A five-character SQLSTATE string for warnings/errors, or ``None``
        on success or when *result* is ``None``.
    """
    sql_state = result.sql_state if result else None
    if sql_state and sql_state != SQLSTATE_SUCCESS:
        return sql_state
    return None


def extract_rowcount(descriptor: ResultSetDescriptor | None) -> int:
    """Return the number of rows affected from a ResultSetDescriptor.

    Returns the rows_affected value from the server if present, otherwise -1.

    Args:
        descriptor: The ResultSetDescriptor from a proto response.

    Returns:
        Row count from server, or ``-1`` when unavailable.
    """
    if not descriptor:
        return -1

    # Return rows_affected if present (for SELECT, DML, and DDL)
    if descriptor.HasField("rows_affected"):
        return descriptor.rows_affected

    return -1
