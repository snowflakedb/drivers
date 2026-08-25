"""Shared result-batch mixin for sync and async result batch implementations."""

from __future__ import annotations

from enum import Enum, unique
from typing import TYPE_CHECKING, Any, TypeVar

from .arrow_context import ArrowConverterContext
from .errorhandler import ErrorHandlerMixin
from .protobuf_gen.database_driver_v1_pb2 import ColumnMetadata, ResultChunk


if TYPE_CHECKING:
    from ..aio.connection import Connection as AsyncConnection
    from ..connection import Connection
    from .cursor.result_metadata import ResultMetadata


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
    _arrow_context: ArrowConverterContext
    _numpy: bool
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
        self._arrow_context = (
            ArrowConverterContext.create(connection) if connection is not None else ArrowConverterContext()
        )
        self._numpy = bool(connection.config.numpy) if connection is not None else False
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
    # Pickle support
    # ------------------------------------------------------------------

    def __getstate__(self) -> dict[str, Any]:
        return {
            "chunk_bytes": self._chunk.SerializeToString(),
            "description": self._description,
            "column_bytes": [c.SerializeToString() for c in self._columns],
            "arrow_context": self._arrow_context,
            "numpy": self._numpy,
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
        self._arrow_context = state.get("arrow_context", ArrowConverterContext())
        self._numpy = state.get("numpy", False)
        self._connection = None
        self._arrow_stream_ptr = None
