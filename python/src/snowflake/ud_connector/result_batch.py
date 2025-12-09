"""
Result batch iterators for processing Arrow data.

This module provides batch iteration over Arrow record batches,
similar to the original snowflake-connector-python result_batch module
but simplified for the universal-driver architecture.
"""

from __future__ import annotations

from typing import Any, Iterator

import pyarrow

from snowflake.ud_connector.arrow_batch_converter import PyArrowBatchConverter
from snowflake.ud_connector.arrow_context import ArrowConverterContext


class ArrowBatchIterator:
    """Iterator that processes Arrow record batches and converts them to Python tuples."""

    def __init__(
        self,
        reader: pyarrow.RecordBatchReader,
        schema: list[dict[str, Any]],
        use_dict_result: bool = False,
        arrow_context: Any = None,
    ):
        """Initialize the batch iterator.

        Args:
            reader: PyArrow RecordBatchReader to iterate over
            schema: List of column metadata dicts
            use_dict_result: If True, return dicts instead of tuples
            arrow_context: Arrow context for C++ converter (optional)
        """
        self.reader = reader
        self.schema = schema
        self.use_dict_result = use_dict_result
        # Create default context if none provided
        self.arrow_context = arrow_context if arrow_context else ArrowConverterContext()
        self._current_batch_iterator = None

    def __iter__(self) -> Iterator[tuple | dict]:
        """Return iterator over rows."""
        return self

    def __next__(self) -> tuple | dict:
        """Get next row from the batches."""
        while True:
            # If no current batch iterator or exhausted, read next batch
            if self._current_batch_iterator is None:
                try:
                    batch = self.reader.read_next_batch()

                    # Handle empty schema
                    if batch.num_columns == 0:
                        if self.use_dict_result:
                            return {}
                        else:
                            return tuple()

                    # Create C++ batch converter for this batch
                    self._current_batch_iterator = PyArrowBatchConverter(
                        batch,
                        self.arrow_context,
                        use_dict_result=self.use_dict_result,
                        use_numpy=False,
                        check_error_on_every_column=True,
                    )
                except StopIteration:
                    raise StopIteration

            # Try to get next row from current batch
            try:
                return next(self._current_batch_iterator)
            except StopIteration:
                # Batch exhausted, get next batch
                self._current_batch_iterator = None
                continue

    def fetchone(self) -> tuple | dict | None:
        """Fetch the next row."""
        try:
            return self.__next__()
        except StopIteration:
            return None

    def fetchmany(self, size: int = 1) -> list[tuple | dict]:
        """Fetch the next `size` rows.

        Args:
            size: Number of rows to fetch

        Returns:
            List of rows (tuples or dicts)
        """
        rows = []
        for _ in range(size):
            try:
                row = self.__next__()
                rows.append(row)
            except StopIteration:
                break
        return rows

    def fetchall(self) -> list[tuple | dict]:
        """Fetch all remaining rows.

        Returns:
            List of all remaining rows (tuples or dicts)
        """
        rows = []

        # Drain current batch iterator first (if any)
        if self._current_batch_iterator is not None:
            try:
                while True:
                    rows.append(next(self._current_batch_iterator))
            except StopIteration:
                pass
            self._current_batch_iterator = None

        # Read and convert following batches
        while True:
            try:
                batch = self.reader.read_next_batch()

                if batch.num_columns > 0:
                    batch_iterator = PyArrowBatchConverter(
                        batch,
                        self.arrow_context,
                        use_dict_result=self.use_dict_result,
                        use_numpy=False,
                        check_error_on_every_column=True,
                    )
                    rows.extend(list(batch_iterator))
            except StopIteration:
                break

        return rows
