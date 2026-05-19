"""Async result batch for distributed fetch.

``AsyncResultBatch`` mirrors :class:`~snowflake.connector.result_batch.ResultBatch`
but uses ``async_core_driver`` for chunk fetching so all I/O is non-blocking.
"""

from __future__ import annotations

from collections.abc import AsyncIterator, Iterator
from typing import TYPE_CHECKING, Any, cast

from .._internal.api_client.client_api import async_core_driver
from .._internal.arrow_stream_utils import (
    collect_arrow_table,
    create_row_iterator,
    create_table_iterator,
)
from .._internal.extras import pandas, pyarrow, requires_dependency
from .._internal.protobuf_gen.database_driver_v1_pb2 import (
    ColumnMetadata,
    ResultChunk,
)
from .._internal.statement_utils import get_stream_ptr
from ..errors import InterfaceError
from ..result_batch import IterTableStructure, IterUnit


if TYPE_CHECKING:
    from pandas import DataFrame
    from pyarrow import Table

    from ..connection import Connection
    from ..cursor import ResultMetadata


class AsyncResultBatch:
    """Async-native result batch for distributed fetch.

    Each batch corresponds to a result chunk from the Snowflake back-end.
    Data is fetched lazily via ``async_core_driver.database_fetch_chunk``
    when :meth:`populate_data`, :meth:`create_iter`, :meth:`to_arrow`, or
    :meth:`to_pandas` is awaited.
    """

    def __init__(
        self,
        chunk: ResultChunk,
        description: list[ResultMetadata],
        connection: Connection | None,
        columns: list[ColumnMetadata] | None = None,
    ) -> None:
        self._chunk = chunk
        self._description = description
        self._connection = connection
        self._columns: list[ColumnMetadata] = list(columns) if columns else []
        self._arrow_stream_ptr: int | None = None

    @classmethod
    def from_chunks(
        cls,
        chunks: list[ResultChunk] | None,
        description: list[ResultMetadata] | None,
        connection: Connection | None,
        columns: list[ColumnMetadata] | None = None,
    ) -> list[AsyncResultBatch] | None:
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
    def connection(self) -> Connection | None:
        return self._connection

    @connection.setter
    def connection(self, value: Connection | None) -> None:
        self._connection = value

    # ------------------------------------------------------------------
    # Data fetching (async)
    # ------------------------------------------------------------------

    def _resolve_connection(self, connection: Connection | None = None) -> Connection:
        conn = connection or self._connection
        if conn is None:
            raise InterfaceError("ResultBatch is not connected to a database driver. Pass a connection argument.")
        return conn

    async def _fetch_arrow_stream_ptr(self, connection: Connection) -> int:
        response = await async_core_driver.database_fetch_chunk(
            db_handle=connection.db_handle,  # type: ignore[arg-type]
            chunk=self._chunk,
            columns=self._columns,
        )
        return get_stream_ptr(response)

    async def _take_arrow_stream_ptr(self, connection: Connection) -> int:
        if self._arrow_stream_ptr is None:
            await self.populate_data(connection=connection)
        stream_ptr = cast(int, self._arrow_stream_ptr)
        self._arrow_stream_ptr = None
        return stream_ptr

    async def populate_data(self, connection: Connection | None = None, **kwargs: Any) -> AsyncResultBatch:
        """Pre-fetch this batch's data asynchronously."""
        conn = self._resolve_connection(connection)
        self._arrow_stream_ptr = await self._fetch_arrow_stream_ptr(conn)
        return self

    def __aiter__(self) -> AsyncIterator[tuple | dict | Exception]:
        return self._async_row_iter()

    async def _async_row_iter(self) -> AsyncIterator[tuple | dict | Exception]:
        """Yield rows from the batch asynchronously.

        The actual chunk fetch is async; once the Arrow stream pointer is
        obtained, row iteration is synchronous (CPU-bound decoding).
        """
        conn = self._resolve_connection()
        stream_ptr = await self._take_arrow_stream_ptr(conn)
        for row in create_row_iterator(stream_ptr, connection=conn, use_dict_result=False):
            yield row

    async def create_iter(
        self,
        connection: Connection | None = None,
        iter_unit: IterUnit | str = IterUnit.ROW_UNIT,
        structure: IterTableStructure | str = IterTableStructure.PANDAS,
        use_dict_result: bool = False,
        number_to_decimal: bool = False,
        force_microsecond_precision: bool = False,
    ) -> Iterator[tuple | dict | Exception] | Iterator[Table] | Iterator[DataFrame]:
        """Create an iterator over this batch's data.

        The chunk fetch is async; the returned iterator is synchronous
        (Arrow decoding is CPU-bound and doesn't benefit from async).
        """
        iter_unit = IterUnit.of(iter_unit)
        structure = IterTableStructure.of(structure)

        conn = self._resolve_connection(connection)
        if iter_unit == IterUnit.TABLE_UNIT:
            if structure == IterTableStructure.PANDAS:
                return iter(
                    [
                        await self.to_pandas(
                            connection=conn,
                            number_to_decimal=number_to_decimal,
                            force_microsecond_precision=force_microsecond_precision,
                        )
                    ]
                )
            return iter(
                [
                    await self.to_arrow(
                        connection=conn,
                        number_to_decimal=number_to_decimal,
                        force_microsecond_precision=force_microsecond_precision,
                    )
                ]
            )

        stream_ptr = await self._take_arrow_stream_ptr(conn)
        return create_row_iterator(stream_ptr, connection=conn, use_dict_result=use_dict_result)

    @requires_dependency(pyarrow)
    async def to_arrow(
        self,
        connection: Connection | None = None,
        number_to_decimal: bool = False,
        force_microsecond_precision: bool = False,
    ) -> Table:
        conn = self._resolve_connection(connection)
        stream_ptr = await self._take_arrow_stream_ptr(conn)
        return collect_arrow_table(
            create_table_iterator(
                stream_ptr,
                connection=conn,
                number_to_decimal=number_to_decimal,
                force_microsecond_precision=force_microsecond_precision,
            ),
            self._description,
        )

    @requires_dependency(pandas)
    async def to_pandas(
        self,
        connection: Connection | None = None,
        number_to_decimal: bool = False,
        force_microsecond_precision: bool = False,
    ) -> DataFrame:
        table = await self.to_arrow(
            connection=connection,
            number_to_decimal=number_to_decimal,
            force_microsecond_precision=force_microsecond_precision,
        )
        return table.to_pandas()

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
