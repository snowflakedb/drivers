"""Async result batch classes for distributed fetch."""

from __future__ import annotations

from collections.abc import AsyncIterator
from typing import TYPE_CHECKING, Any, cast

from .._internal.api_client.client_api import async_core_driver
from .._internal.arrow_stream_async import (
    AsyncArrowStreamIterator,
    collect_arrow_table_async,
    to_pandas_async,
)
from .._internal.arrow_stream_utils import create_row_iterator, create_table_iterator
from .._internal.extras import pandas, pyarrow, requires_dependency
from .._internal.protobuf_gen.database_driver_v1_pb2 import ColumnMetadata, ResultChunk
from .._internal.result_batch import IterTableStructure, IterUnit, ResultBatchMixin
from .._internal.statement_utils import get_stream_ptr


if TYPE_CHECKING:
    from pandas import DataFrame
    from pyarrow import Table

    from ..cursor import ResultMetadata
    from .connection import Connection


class ResultBatch(ResultBatchMixin):
    """Async counterpart of :class:`~snowflake.connector.result_batch.ResultBatch`."""

    _connection: Connection | None

    def __init__(
        self,
        chunk: ResultChunk,
        description: list[ResultMetadata],
        connection: Connection | None,
        columns: list[ColumnMetadata] | None = None,
    ) -> None:
        super().__init__(chunk, description, connection, columns)

    # ------------------------------------------------------------------
    # Properties
    # ------------------------------------------------------------------

    @property
    def connection(self) -> Connection | None:
        return self._connection

    @connection.setter
    def connection(self, value: Connection | None) -> None:
        self._connection = value

    # ------------------------------------------------------------------
    # Connection resolution
    # ------------------------------------------------------------------

    def _resolve_connection(self, connection: Connection | None = None) -> Connection:
        return cast("Connection", self._require_connection(connection))

    # ------------------------------------------------------------------
    # Data fetching
    # ------------------------------------------------------------------

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

    async def populate_data(self, connection: Connection | None = None, **kwargs: Any) -> ResultBatch:
        conn = self._resolve_connection(connection)
        self._arrow_stream_ptr = await self._fetch_arrow_stream_ptr(conn)
        return self

    def __aiter__(self) -> AsyncIterator[tuple | dict | Exception]:
        return self.create_iter()

    async def create_iter(
        self,
        connection: Connection | None = None,
        iter_unit: IterUnit | str = IterUnit.ROW_UNIT,
        structure: IterTableStructure | str = IterTableStructure.PANDAS,
        use_dict_result: bool = False,
        number_to_decimal: bool = False,
        force_microsecond_precision: bool = False,
    ) -> AsyncIterator[tuple | dict | Exception] | AsyncIterator[Table] | AsyncIterator[DataFrame]:
        iter_unit = IterUnit.of(iter_unit)
        structure = IterTableStructure.of(structure)

        conn = self._resolve_connection(connection)
        if iter_unit == IterUnit.TABLE_UNIT:
            if structure == IterTableStructure.PANDAS:
                yield await self.to_pandas(
                    connection=conn,
                    number_to_decimal=number_to_decimal,
                    force_microsecond_precision=force_microsecond_precision,
                )
                return
            yield await self.to_arrow(
                connection=conn,
                number_to_decimal=number_to_decimal,
                force_microsecond_precision=force_microsecond_precision,
            )
            return

        stream_ptr = await self._take_arrow_stream_ptr(conn)
        iterator = AsyncArrowStreamIterator(
            create_row_iterator(stream_ptr, connection=conn, use_dict_result=use_dict_result)
        )
        async for row in iterator:
            yield row

    @requires_dependency(pyarrow)
    async def to_arrow(
        self,
        connection: Connection | None = None,
        number_to_decimal: bool = False,
        force_microsecond_precision: bool = False,
    ) -> Table:
        conn = self._resolve_connection(connection)
        stream_ptr = await self._take_arrow_stream_ptr(conn)
        return await collect_arrow_table_async(
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
        return await to_pandas_async(table)


__all__ = ["IterUnit", "IterTableStructure", "ResultBatch"]
