from __future__ import annotations

from collections.abc import AsyncGenerator, Generator
from contextlib import asynccontextmanager, contextmanager

from .api_client.client_api import async_core_driver, core_driver
from .protobuf_gen.database_driver_v1_pb2 import (
    ConnectionHandle,
    DatabaseFetchChunkResponse,
    PrepareResult,
    ResultSetDescriptor,
    ResultSetGetStreamResponse,
    StatementHandle,
)
from .sqlstate import SQLSTATE_SUCCESS


@contextmanager
def statement(conn_handle: ConnectionHandle, query: str) -> Generator[StatementHandle]:
    """Context manager that owns the full lifecycle of a statement handle.

    Allocates a new statement on the server, binds the given SQL query to it,
    and yields the ``StatementHandle`` for execution.  The statement is
    guaranteed to be released when the context exits, even if an exception
    is raised.

    Args:
        conn_handle: Active Snowflake conn_handle.
        query: SQL text to bind to the newly created statement.

    Yields:
        StatementHandle: A handle that can be passed to ``statement_execute``
        or other statement-level APIs.
    """
    stmt_handle = core_driver.statement_new(conn_handle=conn_handle).stmt_handle
    try:
        core_driver.statement_set_query(stmt_handle=stmt_handle, query=query)
        yield stmt_handle
    finally:
        core_driver.statement_release(stmt_handle=stmt_handle)


@asynccontextmanager
async def async_statement(conn_handle: ConnectionHandle, query: str) -> AsyncGenerator[StatementHandle]:
    stmt_handle = (await async_core_driver.statement_new(conn_handle=conn_handle)).stmt_handle
    try:
        await async_core_driver.statement_set_query(stmt_handle=stmt_handle, query=query)
        yield stmt_handle
    finally:
        await async_core_driver.statement_release(stmt_handle=stmt_handle)


def get_stream_ptr(result: DatabaseFetchChunkResponse | PrepareResult | ResultSetGetStreamResponse | None) -> int:
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


# Snowflake statement_type_id families/classes that produce a browsable result
# set or a real update count. When the server omits rows_affected for one of
# these, the row count is genuinely unknown (-1); for every other classified
# statement (DDL, session/transaction control, etc.) the legacy connector
# reported the generic success marker 1, which we reproduce for compatibility.
# Classification mirrors the core's mask-based taxonomy (level-3 0xF000 family,
# level-2 0xFF00 class) rather than exact ids, because the concrete ids the
# server returns (e.g. ALTER SESSION 0x4100, COMMIT 0x5100) do not match named
# constants but do classify correctly by family.
_CURSOR_OR_DML_FAMILIES = frozenset(
    {
        0x1000,  # SELECT
        0x2000,  # EXPLAIN
        0x3000,  # DML (INSERT/UPDATE/DELETE/MERGE/COPY/...)
        0x7000,  # stage file operations (PUT/GET/LIST/REMOVE)
        0x9000,  # CALL
    }
)
_CURSOR_CLASSES = frozenset(
    {
        0x4400,  # SHOW
        0x4500,  # DESCRIBE
        0x4700,  # LIST_FILES
    }
)
_CURSOR_EXACT_IDS = frozenset(
    {
        0x6244,  # MANAGE_PATS: DDL-family id that returns a browsable result set
    }
)
# Level-3 (0xF000) families that produce no affected-row count but which the
# legacy connector reported as rowcount == 1. Restricting the compat fallback
# to these known families means an unrecognized/future id (e.g. 0xBEEF) is
# reported as unknown (-1) rather than a spurious success.
_NO_RESULT_FAMILIES = frozenset(
    {
        0x4000,  # SYSCMD (ALTER SESSION 0x4100, USE 0x4300, ...)
        0x5000,  # TCL (BEGIN/COMMIT/ROLLBACK, e.g. 0x5100, 0x5400)
        0x6000,  # DDL (CREATE/DROP/ALTER, e.g. 0x6100/0x6101/0x6300)
        0x8000,  # MISC_QUERY_TYPES
    }
)


def _produces_result_or_count(statement_type_id: int) -> bool:
    """Whether the statement type produces a cursor or a real update count.

    These are the statements for which an absent ``rows_affected`` means the
    count is genuinely unknown; everything else classified is a no-result
    statement that the legacy connector reported as ``1``.
    """
    if (statement_type_id & 0xF000) in _CURSOR_OR_DML_FAMILIES:
        return True
    if (statement_type_id & 0xFF00) in _CURSOR_CLASSES:
        return True
    return statement_type_id in _CURSOR_EXACT_IDS


def extract_rowcount(descriptor: ResultSetDescriptor | None) -> int:
    """Return the number of rows affected from a ResultSetDescriptor.

    Returns the server's ``rows_affected`` when present (SELECT/DML, including
    ``0``). When it is absent the statement produced no affected-row count: for
    a recognized no-result statement (DDL, session/transaction control, etc.)
    this returns ``1`` to match the legacy connector, and ``-1`` for
    cursor/DML statements with a missing count or an absent/unrecognized
    statement type.

    Args:
        descriptor: The ResultSetDescriptor from a proto response.

    Returns:
        Row count from server, ``1`` for legacy no-result successes, or ``-1``
        when unavailable.
    """
    if not descriptor:
        return -1

    if descriptor.HasField("rows_affected"):
        return descriptor.rows_affected

    if not descriptor.HasField("statement_type_id"):
        return -1
    statement_type_id = descriptor.statement_type_id
    if _produces_result_or_count(statement_type_id):
        return -1
    if (statement_type_id & 0xF000) in _NO_RESULT_FAMILIES:
        return 1
    return -1
