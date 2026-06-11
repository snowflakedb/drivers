"""Shared result-batch mixin for sync and async result batch implementations."""

from __future__ import annotations

from enum import Enum, unique
from typing import TYPE_CHECKING, Any, TypeVar

from snowflake.connector._internal.errorhandler import ErrorHandlerMixin
from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import ColumnMetadata, ResultChunk
from snowflake.connector.errors import InterfaceError


if TYPE_CHECKING:
    from snowflake.connector._async.connection import AsyncConnection
    from snowflake.connector._internal.cursor.result_metadata import ResultMetadata
    from snowflake.connector.connection import Connection


_ResultBatchT = TypeVar("_ResultBatchT", bound="ResultBatchMixin")


@unique
class IterUnit(Enum):
    """Controls what ``ResultBatch.create_iter`` yields."""

    ROW_UNIT = "row"
    TABLE_UNIT = "table"

    @classmethod
    def of(cls, value: IterUnit | str) -> IterUnit:
        if isinstance(value, cls):
            return value
        return cls(value)


@unique
class IterTableStructure(Enum):
    """Controls what table format ``TABLE_UNIT`` iteration produces."""

    ARROW = "arrow"
    PANDAS = "pandas"

    @classmethod
    def of(cls, value: IterTableStructure | str) -> IterTableStructure:
        if isinstance(value, cls):
            return value
        return cls(value)


class ResultBatchMixin(ErrorHandlerMixin):
    """Zero-I/O result-batch members shared by sync and async batch classes."""

    _chunk: ResultChunk
    _description: list[ResultMetadata]
    _connection: Connection | AsyncConnection | None
    _columns: list[ColumnMetadata]
    _arrow_stream_ptr: int | None

    def __init__(
        self,
        chunk: ResultChunk,
        description: list[ResultMetadata],
        connection: Connection | AsyncConnection | None,
        columns: list[ColumnMetadata] | None = None,
    ) -> None:
        self._chunk = chunk
        self._description = description
        self._connection = connection
        self._columns = list(columns) if columns else []
        self._arrow_stream_ptr = None

    @classmethod
    def from_chunks(
        cls: type[_ResultBatchT],
        chunks: list[ResultChunk] | None,
        description: list[ResultMetadata] | None,
        connection: Connection | AsyncConnection | None,
        columns: list[ColumnMetadata] | None = None,
    ) -> list[_ResultBatchT] | None:
        """Create a list of batches from raw result chunks, or ``None`` if unavailable."""
        if chunks is None or description is None:
            return None
        return [cls(chunk=chunk, description=description, connection=connection, columns=columns) for chunk in chunks]

    # ------------------------------------------------------------------
    # Properties
    # ------------------------------------------------------------------

    @property
    def rowcount(self) -> int:
        return self._chunk.row_count

    @property
    def compressed_size(self) -> int | None:
        if self._chunk.HasField("remote"):
            return self._chunk.remote.compressed_size
        return None

    @property
    def uncompressed_size(self) -> int | None:
        if self._chunk.HasField("remote"):
            return self._chunk.remote.uncompressed_size
        return None

    @property
    def column_names(self) -> list[str]:
        return [col.name for col in self._description]

    @property
    def _errorhandler_connection(self) -> Connection | AsyncConnection | None:
        return self._connection

    # ------------------------------------------------------------------
    # Connection resolution
    # ------------------------------------------------------------------

    def _require_connection(
        self, connection: Connection | AsyncConnection | None = None
    ) -> Connection | AsyncConnection:
        conn = connection or self._connection
        if conn is None:
            raise InterfaceError("ResultBatch is not connected to a database driver. Pass a connection argument.")
        return conn

    # ------------------------------------------------------------------
    # Pickle support
    # ------------------------------------------------------------------

    def __getstate__(self) -> dict[str, Any]:
        return {
            "chunk_bytes": self._chunk.SerializeToString(),
            "description": self._description,
            "column_bytes": [c.SerializeToString() for c in self._columns],
        }

    def __setstate__(self, state: dict[str, Any]) -> None:
        chunk = ResultChunk()
        chunk.ParseFromString(state["chunk_bytes"])
        self._chunk = chunk
        self._description = state["description"]
        columns: list[ColumnMetadata] = []
        for raw in state.get("column_bytes", []):
            col = ColumnMetadata()
            col.ParseFromString(raw)
            columns.append(col)
        self._columns = columns
        self._connection = None
        self._arrow_stream_ptr = None
