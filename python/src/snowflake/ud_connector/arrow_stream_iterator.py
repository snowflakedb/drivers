"""
Arrow stream iterator for processing Arrow data.

This module provides iteration over Arrow record batch streams,
converting them to Python tuples or dicts row-by-row using C++ converters.

This implementation works directly with the Arrow C Stream Interface,
without requiring pyarrow as a dependency.
"""

from __future__ import annotations

from typing import Iterator

from snowflake.ud_connector._arrow_batch_iterator import NanoarrowStreamIterator
from snowflake.ud_connector.arrow_context import ArrowConverterContext


class ArrowStreamIterator:
    """Iterator that processes Arrow record batch streams and converts them to Python tuples.

    This class wraps the Cython NanoarrowStreamIterator which directly consumes
    Arrow C Stream Interface pointers from the Rust core, without requiring pyarrow.
    """

    def __init__(
        self,
        stream_ptr: int,
        use_dict_result: bool = False,
        arrow_context: ArrowConverterContext | None = None,
        use_numpy: bool = False,
    ):
        """Initialize the stream iterator.

        Args:
            stream_ptr: Pointer to ArrowArrayStream (as integer)
            use_dict_result: If True, return dicts instead of tuples
            arrow_context: Arrow context for C++ converter (optional)
            use_numpy: If True, use numpy types for numeric data
        """
        # Create default context if none provided
        self.arrow_context = arrow_context if arrow_context else ArrowConverterContext()
        self.use_dict_result = use_dict_result
        self.use_numpy = use_numpy

        # Create the Cython stream iterator that works directly with C Stream Interface
        self._stream_iterator = NanoarrowStreamIterator(
            stream_ptr,
            self.arrow_context,
            use_dict_result=use_dict_result,
            use_numpy=use_numpy,
        )

    def __iter__(self) -> Iterator[tuple | dict]:
        """Return iterator over rows."""
        return self

    def __next__(self) -> tuple | dict:
        """Get next row from the batches."""
        return next(self._stream_iterator)

    @property
    def column_count(self) -> int:
        """Get the number of columns in the result."""
        return self._stream_iterator.column_count
