from __future__ import annotations

from collections.abc import Generator
from contextlib import contextmanager
from typing import TYPE_CHECKING

from .protobuf_gen.database_driver_v1_pb2 import (
    ExecuteResult,
    StatementHandle,
    StatementNewRequest,
    StatementReleaseRequest,
    StatementSetSqlQueryRequest,
)


if TYPE_CHECKING:
    from ..connection import Connection


@contextmanager
def create_statement(connection: Connection, query: str) -> Generator[StatementHandle]:
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
    statement_request = StatementNewRequest(conn_handle=connection.conn_handle)
    statement = connection.db_api.statement_new(request=statement_request)
    stmt_handle = statement.stmt_handle
    sql_query_request = StatementSetSqlQueryRequest(stmt_handle=stmt_handle, query=query)
    try:
        connection.db_api.statement_set_sql_query(sql_query_request)
        yield stmt_handle
    finally:
        release_request = StatementReleaseRequest(stmt_handle=stmt_handle)
        connection.db_api.statement_release(release_request)


def extract_rowcount(result: ExecuteResult) -> int:
    """Return the number of rows affected by the executed statement.

    If the result is falsy or the ``rows_affected`` field is not present
    (e.g. for SELECT queries), returns ``-1`` following the DB-API 2.0
    convention for an indeterminate row count.

    Args:
        result: The protobuf response returned by ``statement_execute``.

    Returns:
        Non-negative row count, or ``-1`` when the count is unavailable.
    """
    if result and result.HasField("rows_affected"):
        return result.rows_affected
    return -1


def extract_sqlstate(result: ExecuteResult | None) -> str | None:
    """Return the SQLSTATE code from an execute result, if meaningful.

    SQLSTATE ``"00000"`` (successful completion) is normalised to ``None``
    for backwards compatibility with the legacy connector, which omits
    the code on success.

    Args:
        result: The protobuf response returned by ``statement_execute``,
            or ``None`` if no result is available.

    Returns:
        A five-character SQLSTATE string for warnings/errors, or ``None``
        on success or when *result* is ``None``.
    """
    sql_state = result.sql_state if result else None
    if sql_state and sql_state != "00000":
        return sql_state
    return None


def get_stream_ptr(result: ExecuteResult | None) -> int:
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
