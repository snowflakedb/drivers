"""Result batch classes for distributed fetch.

This module provides ``ResultBatch`` objects that represent individual chunks
of a query result set.  Each batch can independently fetch and convert its
data, making them suitable for distributed processing.

These objects are pickleable for easy distribution and replication.
Note that the URLs stored in remote batches expire; the lifetime is
dictated by the Snowflake back-end (typically 6 hours).
"""

from __future__ import annotations

from collections.abc import Iterator
from enum import Enum, unique
from typing import TYPE_CHECKING, Any

from ._internal.arrow_stream_utils import (
    collect_arrow_table,
    create_row_iterator,
    create_table_iterator,
)
from ._internal.decorators import backward_compatibility
from ._internal.extras import pandas, pyarrow, requires_dependency
from ._internal.protobuf_gen.database_driver_v1_pb2 import (
    DatabaseFetchChunkRequest,
    ResultChunk,
)
from ._internal.statement_utils import get_stream_ptr
from .errors import InterfaceError


if TYPE_CHECKING:
    from pandas import DataFrame
    from pyarrow import Table

    from .connection import Connection
    from .cursor import ResultMetadata


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


class ResultBatch:
    """Represents a single chunk of a query result set.

    Each ``ResultBatch`` corresponds to what the Snowflake back-end calls
    a "result chunk".  Batches know how to retrieve their own data and
    convert it into Python-native formats.

    Fetching is lazy: the actual download/decode happens when
    :meth:`create_iter`, :meth:`to_arrow`, or :meth:`to_pandas` is called.

    These objects are pickleable for easy distribution and replication.
    After unpickling, pass a live :class:`~snowflake.connector.Connection`
    to any data-fetching method.
    """

    def __init__(
        self,
        chunk: ResultChunk,
        description: list[ResultMetadata],
        connection: Connection | None,
    ) -> None:
        self._chunk = chunk
        self._description = description
        self._connection = connection

    @classmethod
    def from_chunks(
        cls,
        chunks: list[ResultChunk] | None,
        description: list[ResultMetadata] | None,
        connection: Connection | None,
    ) -> list[ResultBatch] | None:
        """Create a list of batches from raw result chunks, or ``None`` if unavailable."""
        if chunks is None or description is None:
            return None
        return [cls(chunk=chunk, description=description, connection=connection) for chunk in chunks]

    # ------------------------------------------------------------------
    # Properties
    # ------------------------------------------------------------------

    @property
    def rowcount(self) -> int:
        raise NotImplementedError("Per-batch rowcount is not yet available.")

    @property
    def compressed_size(self) -> int | None:
        raise NotImplementedError("Per-batch compressed_size is not yet available.")

    @property
    def uncompressed_size(self) -> int | None:
        raise NotImplementedError("Per-batch uncompressed_size is not yet available.")

    @property
    def column_names(self) -> list[str]:
        return [col.name for col in self._description]

    @property
    def connection(self) -> Connection | None:
        return self._connection

    @connection.setter
    def connection(self, value: Connection | None) -> None:
        self._connection = value

    # ------------------------------------------------------------------
    # Data fetching
    # ------------------------------------------------------------------

    def _resolve_connection(self, connection: Connection | None = None) -> Connection:
        conn = connection or self._connection
        if conn is None:
            raise InterfaceError("ResultBatch is not connected to a database driver. Pass a connection argument.")
        return conn

    def _fetch_arrow_stream_ptr(self, connection: Connection) -> int:
        request = DatabaseFetchChunkRequest(
            db_handle=connection.db_handle,
            chunk=self._chunk,
        )
        response = connection.db_api.database_fetch_chunk(request)
        return get_stream_ptr(response)

    def __iter__(self) -> Iterator[tuple | dict | Exception]:
        return self.create_iter()

    def create_iter(
        self,
        connection: Connection | None = None,
        iter_unit: IterUnit | str = IterUnit.ROW_UNIT,
        structure: IterTableStructure | str = IterTableStructure.PANDAS,
        use_dict_result: bool = False,
        number_to_decimal: bool = False,
        force_microsecond_precision: bool = False,
    ) -> Iterator[tuple | dict | Exception] | Iterator[Table] | Iterator[DataFrame]:
        """Create an iterator over this batch's data.

        Args:
            connection: Optional connection override.  When *None* the
                connection set at construction time is used.
            iter_unit: :class:`IterUnit` controlling output granularity.
                ``ROW_UNIT`` (default) yields rows; ``TABLE_UNIT`` yields
                Arrow tables or Pandas DataFrames.
            structure: When *iter_unit* is ``TABLE_UNIT``,
                :attr:`IterTableStructure.ARROW` yields :class:`pyarrow.Table`
                objects, :attr:`IterTableStructure.PANDAS` (default) yields
                :class:`pandas.DataFrame` objects.
            use_dict_result: When *iter_unit* is ``ROW_UNIT``, ``True``
                yields dicts instead of tuples.
            number_to_decimal: Convert scaled NUMBER columns to
                ``Decimal`` instead of ``int``/``float``.
            force_microsecond_precision: Truncate nanosecond timestamps
                to microsecond precision.
        """
        iter_unit = IterUnit.of(iter_unit)
        structure = IterTableStructure.of(structure)

        conn = self._resolve_connection(connection)
        if iter_unit == IterUnit.TABLE_UNIT:
            if structure == IterTableStructure.PANDAS:
                return iter(
                    [
                        self.to_pandas(
                            connection=conn,
                            number_to_decimal=number_to_decimal,
                            force_microsecond_precision=force_microsecond_precision,
                        )
                    ]
                )
            return iter(
                [
                    self.to_arrow(
                        connection=conn,
                        number_to_decimal=number_to_decimal,
                        force_microsecond_precision=force_microsecond_precision,
                    )
                ]
            )

        stream_ptr = self._fetch_arrow_stream_ptr(conn)
        return create_row_iterator(stream_ptr, use_dict_result=use_dict_result)

    @requires_dependency(pyarrow)
    def to_arrow(
        self,
        connection: Connection | None = None,
        number_to_decimal: bool = False,
        force_microsecond_precision: bool = False,
    ) -> Table:
        conn = self._resolve_connection(connection)
        stream_ptr = self._fetch_arrow_stream_ptr(conn)
        return collect_arrow_table(
            create_table_iterator(
                stream_ptr,
                number_to_decimal=number_to_decimal,
                force_microsecond_precision=force_microsecond_precision,
            ),
            self._description,
        )

    @requires_dependency(pandas)
    def to_pandas(
        self,
        connection: Connection | None = None,
        number_to_decimal: bool = False,
        force_microsecond_precision: bool = False,
    ) -> DataFrame:
        return self.to_arrow(
            connection=connection,
            number_to_decimal=number_to_decimal,
            force_microsecond_precision=force_microsecond_precision,
        ).to_pandas()

    # ------------------------------------------------------------------
    # Pickle support
    # ------------------------------------------------------------------

    def __getstate__(self) -> dict[str, Any]:
        return {
            "chunk_bytes": self._chunk.SerializeToString(),
            "description": self._description,
        }

    def __setstate__(self, state: dict[str, Any]) -> None:
        chunk = ResultChunk()
        chunk.ParseFromString(state["chunk_bytes"])
        self._chunk = chunk
        self._description = state["description"]
        self._connection = None


@backward_compatibility
class ArrowResultBatch(ResultBatch):
    """Backward-compatibility wrapper around :class:`ResultBatch`.

    In the legacy connector, ``ArrowResultBatch`` was a distinct class backed
    by Arrow-format data.  In the universal driver all result batches are
    Arrow-backed, so this subclass exists solely to preserve import paths and
    ``isinstance`` checks in existing consumer code.
    """


@backward_compatibility
class JSONResultBatch(ResultBatch):
    """Backward-compatibility wrapper around :class:`ResultBatch`.

    The legacy connector used ``JSONResultBatch`` for JSON-format result
    chunks.  The universal driver always returns Arrow data, so this subclass
    exists solely to preserve import paths and ``isinstance`` checks in
    existing consumer code.  Behavior is identical to :class:`ResultBatch`.
    """


__all__ = ["IterUnit", "IterTableStructure", "ResultBatch", "ArrowResultBatch", "JSONResultBatch"]
