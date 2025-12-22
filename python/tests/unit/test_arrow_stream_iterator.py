"""
Unit tests for ArrowStreamIterator class.

This module tests the Arrow stream iteration functionality with mocked Arrow data.
"""

from unittest.mock import MagicMock, patch, PropertyMock

import pytest


class TestArrowStreamIteratorInit:
    """Test ArrowStreamIterator initialization."""

    def test_init_with_reader(self):
        """Test initialization with a reader."""
        from snowflake.ud_connector.arrow_context import ArrowConverterContext

        # Create mock reader
        mock_reader = MagicMock()

        # Patch the import of ArrowStreamIterator
        with patch("snowflake.ud_connector.arrow_stream_iterator.PyArrowBatchIterator"):
            from snowflake.ud_connector.arrow_stream_iterator import ArrowStreamIterator

            iterator = ArrowStreamIterator(mock_reader)
            assert iterator.reader is mock_reader
            assert iterator.use_dict_result is False
            assert isinstance(iterator.arrow_context, ArrowConverterContext)
            assert iterator._current_batch_iterator is None

    def test_init_with_use_dict_result(self):
        """Test initialization with use_dict_result=True."""
        mock_reader = MagicMock()

        with patch("snowflake.ud_connector.arrow_stream_iterator.PyArrowBatchIterator"):
            from snowflake.ud_connector.arrow_stream_iterator import ArrowStreamIterator

            iterator = ArrowStreamIterator(mock_reader, use_dict_result=True)
            assert iterator.use_dict_result is True

    def test_init_with_custom_arrow_context(self):
        """Test initialization with custom arrow context."""
        from snowflake.ud_connector.arrow_context import ArrowConverterContext

        mock_reader = MagicMock()
        custom_context = ArrowConverterContext(
            session_parameters={"TIMEZONE": "America/New_York"}
        )

        with patch("snowflake.ud_connector.arrow_stream_iterator.PyArrowBatchIterator"):
            from snowflake.ud_connector.arrow_stream_iterator import ArrowStreamIterator

            iterator = ArrowStreamIterator(mock_reader, arrow_context=custom_context)
            assert iterator.arrow_context is custom_context
            assert iterator.arrow_context.timezone == "America/New_York"


class TestArrowStreamIteratorIteration:
    """Test ArrowStreamIterator iteration methods."""

    def test_iter_returns_self(self):
        """Test __iter__ returns self."""
        mock_reader = MagicMock()

        with patch("snowflake.ud_connector.arrow_stream_iterator.PyArrowBatchIterator"):
            from snowflake.ud_connector.arrow_stream_iterator import ArrowStreamIterator

            iterator = ArrowStreamIterator(mock_reader)
            assert iter(iterator) is iterator

    def test_next_with_empty_batch(self):
        """Test __next__ with an empty schema batch."""
        mock_reader = MagicMock()
        mock_batch = MagicMock()
        mock_batch.num_columns = 0
        mock_reader.read_next_batch.return_value = mock_batch

        with patch("snowflake.ud_connector.arrow_stream_iterator.PyArrowBatchIterator"):
            from snowflake.ud_connector.arrow_stream_iterator import ArrowStreamIterator

            iterator = ArrowStreamIterator(mock_reader)
            result = next(iterator)
            assert result == tuple()

    def test_next_with_empty_batch_dict_result(self):
        """Test __next__ with empty schema batch and use_dict_result=True."""
        mock_reader = MagicMock()
        mock_batch = MagicMock()
        mock_batch.num_columns = 0
        mock_reader.read_next_batch.return_value = mock_batch

        with patch("snowflake.ud_connector.arrow_stream_iterator.PyArrowBatchIterator"):
            from snowflake.ud_connector.arrow_stream_iterator import ArrowStreamIterator

            iterator = ArrowStreamIterator(mock_reader, use_dict_result=True)
            result = next(iterator)
            assert result == {}

    def test_next_reads_from_batch_iterator(self):
        """Test __next__ reads from the batch iterator."""
        mock_reader = MagicMock()
        mock_batch = MagicMock()
        mock_batch.num_columns = 2

        mock_reader.read_next_batch.return_value = mock_batch

        mock_batch_iterator = MagicMock()
        mock_batch_iterator.__next__ = MagicMock(return_value=(1, "test"))

        with patch(
            "snowflake.ud_connector.arrow_stream_iterator.PyArrowBatchIterator",
            return_value=mock_batch_iterator,
        ):
            from snowflake.ud_connector.arrow_stream_iterator import ArrowStreamIterator

            iterator = ArrowStreamIterator(mock_reader)
            result = next(iterator)
            assert result == (1, "test")

    def test_next_moves_to_next_batch_when_exhausted(self):
        """Test __next__ moves to next batch when current is exhausted."""
        mock_reader = MagicMock()
        mock_batch1 = MagicMock()
        mock_batch1.num_columns = 1
        mock_batch2 = MagicMock()
        mock_batch2.num_columns = 1

        # First call returns batch1, second returns batch2, third raises StopIteration
        mock_reader.read_next_batch.side_effect = [
            mock_batch1,
            mock_batch2,
            StopIteration,
        ]

        # First batch iterator exhausted after first call
        mock_iter1 = MagicMock()
        mock_iter1.__next__ = MagicMock(side_effect=[(1,), StopIteration])
        # Need to make it return itself when iterated
        mock_iter1.__iter__ = MagicMock(return_value=mock_iter1)

        # Second batch iterator
        mock_iter2 = MagicMock()
        mock_iter2.__next__ = MagicMock(return_value=(2,))

        with patch(
            "snowflake.ud_connector.arrow_stream_iterator.PyArrowBatchIterator",
            side_effect=[mock_iter1, mock_iter2],
        ):
            from snowflake.ud_connector.arrow_stream_iterator import ArrowStreamIterator

            iterator = ArrowStreamIterator(mock_reader)
            # First row from first batch
            result1 = next(iterator)
            assert result1 == (1,)
            # Second row from second batch
            result2 = next(iterator)
            assert result2 == (2,)

    def test_stop_iteration_when_no_more_batches(self):
        """Test StopIteration when no more batches."""
        mock_reader = MagicMock()
        mock_reader.read_next_batch.side_effect = StopIteration

        with patch("snowflake.ud_connector.arrow_stream_iterator.PyArrowBatchIterator"):
            from snowflake.ud_connector.arrow_stream_iterator import ArrowStreamIterator

            iterator = ArrowStreamIterator(mock_reader)
            with pytest.raises(StopIteration):
                next(iterator)


class TestArrowStreamIteratorFetchOne:
    """Test ArrowStreamIterator fetchone method."""

    def test_fetchone_returns_next_row(self):
        """Test fetchone returns the next row."""
        mock_reader = MagicMock()
        mock_batch = MagicMock()
        mock_batch.num_columns = 1

        mock_reader.read_next_batch.return_value = mock_batch

        mock_batch_iterator = MagicMock()
        mock_batch_iterator.__next__ = MagicMock(return_value=(42,))

        with patch(
            "snowflake.ud_connector.arrow_stream_iterator.PyArrowBatchIterator",
            return_value=mock_batch_iterator,
        ):
            from snowflake.ud_connector.arrow_stream_iterator import ArrowStreamIterator

            iterator = ArrowStreamIterator(mock_reader)
            result = iterator.fetchone()
            assert result == (42,)

    def test_fetchone_returns_none_when_exhausted(self):
        """Test fetchone returns None when iterator is exhausted."""
        mock_reader = MagicMock()
        mock_reader.read_next_batch.side_effect = StopIteration

        with patch("snowflake.ud_connector.arrow_stream_iterator.PyArrowBatchIterator"):
            from snowflake.ud_connector.arrow_stream_iterator import ArrowStreamIterator

            iterator = ArrowStreamIterator(mock_reader)
            result = iterator.fetchone()
            assert result is None


class TestArrowStreamIteratorFetchMany:
    """Test ArrowStreamIterator fetchmany method."""

    def test_fetchmany_returns_requested_rows(self):
        """Test fetchmany returns requested number of rows."""
        mock_reader = MagicMock()
        mock_batch = MagicMock()
        mock_batch.num_columns = 1

        mock_reader.read_next_batch.return_value = mock_batch

        mock_batch_iterator = MagicMock()
        # Return 3 rows then StopIteration
        mock_batch_iterator.__next__ = MagicMock(
            side_effect=[(1,), (2,), (3,), StopIteration]
        )

        with patch(
            "snowflake.ud_connector.arrow_stream_iterator.PyArrowBatchIterator",
            return_value=mock_batch_iterator,
        ):
            from snowflake.ud_connector.arrow_stream_iterator import ArrowStreamIterator

            iterator = ArrowStreamIterator(mock_reader)
            result = iterator.fetchmany(2)
            assert result == [(1,), (2,)]

    def test_fetchmany_returns_less_when_exhausted(self):
        """Test fetchmany returns fewer rows if iterator exhausts."""
        mock_reader = MagicMock()
        mock_batch = MagicMock()
        mock_batch.num_columns = 1

        # First returns batch, second raises StopIteration
        mock_reader.read_next_batch.side_effect = [mock_batch, StopIteration]

        mock_batch_iterator = MagicMock()
        # Only 1 row then batch exhausted
        mock_batch_iterator.__next__ = MagicMock(side_effect=[(1,), StopIteration])

        with patch(
            "snowflake.ud_connector.arrow_stream_iterator.PyArrowBatchIterator",
            return_value=mock_batch_iterator,
        ):
            from snowflake.ud_connector.arrow_stream_iterator import ArrowStreamIterator

            iterator = ArrowStreamIterator(mock_reader)
            result = iterator.fetchmany(5)
            # Should return just the 1 available row
            assert result == [(1,)]

    def test_fetchmany_default_size_is_one(self):
        """Test fetchmany with default size returns 1 row."""
        mock_reader = MagicMock()
        mock_batch = MagicMock()
        mock_batch.num_columns = 1

        mock_reader.read_next_batch.return_value = mock_batch

        mock_batch_iterator = MagicMock()
        mock_batch_iterator.__next__ = MagicMock(side_effect=[(1,), (2,), (3,)])

        with patch(
            "snowflake.ud_connector.arrow_stream_iterator.PyArrowBatchIterator",
            return_value=mock_batch_iterator,
        ):
            from snowflake.ud_connector.arrow_stream_iterator import ArrowStreamIterator

            iterator = ArrowStreamIterator(mock_reader)
            result = iterator.fetchmany()
            assert result == [(1,)]


class TestArrowStreamIteratorFetchAll:
    """Test ArrowStreamIterator fetchall method."""

    def test_fetchall_returns_all_rows(self):
        """Test fetchall returns all remaining rows."""
        mock_reader = MagicMock()
        mock_batch = MagicMock()
        mock_batch.num_columns = 1

        # First returns batch, second raises StopIteration
        mock_reader.read_next_batch.side_effect = [mock_batch, StopIteration]

        mock_batch_iterator = MagicMock()
        mock_batch_iterator.__iter__ = MagicMock(return_value=iter([(1,), (2,), (3,)]))

        with patch(
            "snowflake.ud_connector.arrow_stream_iterator.PyArrowBatchIterator",
            return_value=mock_batch_iterator,
        ):
            from snowflake.ud_connector.arrow_stream_iterator import ArrowStreamIterator

            iterator = ArrowStreamIterator(mock_reader)
            result = iterator.fetchall()
            assert result == [(1,), (2,), (3,)]

    def test_fetchall_with_multiple_batches(self):
        """Test fetchall across multiple batches."""
        mock_reader = MagicMock()
        mock_batch1 = MagicMock()
        mock_batch1.num_columns = 1
        mock_batch2 = MagicMock()
        mock_batch2.num_columns = 1

        # Returns batch1, batch2, then StopIteration
        mock_reader.read_next_batch.side_effect = [
            mock_batch1,
            mock_batch2,
            StopIteration,
        ]

        mock_iter1 = MagicMock()
        mock_iter1.__iter__ = MagicMock(return_value=iter([(1,), (2,)]))

        mock_iter2 = MagicMock()
        mock_iter2.__iter__ = MagicMock(return_value=iter([(3,), (4,)]))

        with patch(
            "snowflake.ud_connector.arrow_stream_iterator.PyArrowBatchIterator",
            side_effect=[mock_iter1, mock_iter2],
        ):
            from snowflake.ud_connector.arrow_stream_iterator import ArrowStreamIterator

            iterator = ArrowStreamIterator(mock_reader)
            result = iterator.fetchall()
            assert result == [(1,), (2,), (3,), (4,)]

    def test_fetchall_empty_result(self):
        """Test fetchall with no rows."""
        mock_reader = MagicMock()
        mock_reader.read_next_batch.side_effect = StopIteration

        with patch("snowflake.ud_connector.arrow_stream_iterator.PyArrowBatchIterator"):
            from snowflake.ud_connector.arrow_stream_iterator import ArrowStreamIterator

            iterator = ArrowStreamIterator(mock_reader)
            result = iterator.fetchall()
            assert result == []

    def test_fetchall_read_remaining_rows(self):
        """Test fetchall reads remaining rows from all batches."""
        mock_reader = MagicMock()
        mock_batch1 = MagicMock()
        mock_batch1.num_columns = 1
        mock_batch2 = MagicMock()
        mock_batch2.num_columns = 1

        # Returns batch1, batch2, then StopIteration
        mock_reader.read_next_batch.side_effect = [
            mock_batch1,
            mock_batch2,
            StopIteration,
        ]

        mock_iter1 = MagicMock()
        mock_iter1.__next__ = MagicMock(side_effect=[(1,), (2,), StopIteration])
        mock_iter1.__iter__ = MagicMock(return_value=mock_iter1)

        mock_iter2 = MagicMock()
        mock_iter2.__next__ = MagicMock(side_effect=[(3,), (4,), (5,), StopIteration])
        mock_iter2.__iter__ = MagicMock(return_value=mock_iter2)

        with patch(
            "snowflake.ud_connector.arrow_stream_iterator.PyArrowBatchIterator",
            side_effect=[mock_iter1, mock_iter2],
        ):
            from snowflake.ud_connector.arrow_stream_iterator import ArrowStreamIterator

            iterator = ArrowStreamIterator(mock_reader)
            result_one = iterator.fetchone()
            assert result_one == (1,)

            result_rest = iterator.fetchall()
            assert result_rest == [(2,), (3,), (4,), (5,)]


class TestArrowStreamIteratorWithDictResult:
    """Test ArrowStreamIterator with use_dict_result=True."""

    def test_fetchone_returns_dict(self):
        """Test fetchone returns dict when use_dict_result=True."""
        mock_reader = MagicMock()
        mock_batch = MagicMock()
        mock_batch.num_columns = 2

        mock_reader.read_next_batch.return_value = mock_batch

        mock_batch_iterator = MagicMock()
        mock_batch_iterator.__next__ = MagicMock(
            return_value={"col1": 1, "col2": "test"}
        )

        with patch(
            "snowflake.ud_connector.arrow_stream_iterator.PyArrowBatchIterator",
            return_value=mock_batch_iterator,
        ):
            from snowflake.ud_connector.arrow_stream_iterator import ArrowStreamIterator

            iterator = ArrowStreamIterator(mock_reader, use_dict_result=True)
            result = iterator.fetchone()
            assert result == {"col1": 1, "col2": "test"}
