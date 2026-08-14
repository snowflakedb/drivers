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
from typing import TYPE_CHECKING, Any, cast

from ._common.extras import pandas, pyarrow, requires_dependency
from ._internal.api_client.client_api import core_driver
from ._internal.arrow_stream_utils import (
    collect_arrow_table,
    create_row_iterator,
    create_table_iterator,
)
from ._internal.backward_compatibility import install_backward_compatibility_getattr
from ._internal.decorators import backward_compatibility
from ._internal.protobuf_gen.database_driver_v1_pb2 import (
    ColumnMetadata,
    ResultChunk,
)
from ._internal.result_batch import IterTableStructure, IterUnit, ResultBatchMixin
from ._internal.statement_utils import get_stream_ptr


if TYPE_CHECKING:
    from pandas import DataFrame
    from pyarrow import Table

    from .connection import Connection
    from .cursor import ResultMetadata


class ResultBatch(ResultBatchMixin):
    """Represents a single chunk of a query result set.

    Each ``ResultBatch`` corresponds to what the Snowflake back-end calls
    a "result chunk".  Batches know how to retrieve their own data and
    convert it into Python-native formats.

    Fetching is lazy: the actual download/decode happens when
    :meth:`create_iter`, :meth:`to_arrow`, or :meth:`to_pandas` is called.

    These objects are pickleable for easy distribution and replication.
    Remote chunks use presigned URLs and can be fetched without a live
    connection. Pass an optional :class:`~snowflake.connector.Connection`
    when you need the connection-scoped HTTP client (for example after
    unpickling in a worker that opens a fresh session).
    """

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

    @property
    def _errorhandler_connection(self) -> Connection | None:
        return self._connection

    # ------------------------------------------------------------------
    # Connection resolution
    # ------------------------------------------------------------------

    def _resolve_connection(self, connection: Connection | None = None) -> Connection | None:
        return connection or self._connection

    # ------------------------------------------------------------------
    # Data fetching
    # ------------------------------------------------------------------

    def _fetch_arrow_stream_ptr(self, connection: Connection | None = None) -> int:
        conn_handle = connection.conn_handle if connection is not None else None
        response = core_driver.database_fetch_chunk(
            conn_handle=conn_handle,
            chunk=self._chunk,
            columns=self._columns,
        )
        return get_stream_ptr(response)

    def _take_arrow_stream_ptr(self, connection: Connection | None = None) -> int:
        """Return the Arrow stream pointer, fetching first if necessary."""
        if self._arrow_stream_ptr is None:
            self.populate_data(connection=connection)
        stream_ptr = cast(int, self._arrow_stream_ptr)
        self._arrow_stream_ptr = None
        return stream_ptr

    @backward_compatibility
    def populate_data(self, connection: Connection | None = None, **kwargs: Any) -> ResultBatch:
        """Pre-fetch this batch's data and store it for later consumption."""
        conn = self._resolve_connection(connection)
        self._arrow_stream_ptr = self._fetch_arrow_stream_ptr(conn)
        return self

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
        """Create an iterator over this batch's data."""
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

        stream_ptr = self._take_arrow_stream_ptr(conn)
        return create_row_iterator(stream_ptr, context=self._arrow_context, use_dict_result=use_dict_result)

    @requires_dependency(pyarrow)
    def to_arrow(
        self,
        connection: Connection | None = None,
        number_to_decimal: bool = False,
        force_microsecond_precision: bool = False,
    ) -> Table:
        conn = self._resolve_connection(connection)
        stream_ptr = self._take_arrow_stream_ptr(conn)
        return collect_arrow_table(
            create_table_iterator(
                stream_ptr,
                context=self._arrow_context,
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


@backward_compatibility
class ArrowResultBatch(ResultBatch):
    """Backward-compatibility wrapper around :class:`ResultBatch`."""


@backward_compatibility
class JSONResultBatch(ResultBatch):
    """Backward-compatibility wrapper around :class:`ResultBatch`."""


__all__ = ["IterUnit", "IterTableStructure", "ResultBatch", "ArrowResultBatch", "JSONResultBatch"]


# Must be the last statement; see ``install_backward_compatibility_getattr``.
install_backward_compatibility_getattr(__name__)
