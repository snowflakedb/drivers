"""
Unit tests for PEP 249 Cursor class.
"""

from decimal import Decimal
from unittest.mock import MagicMock, patch

import pytest

from snowflake.connector.cursor import SnowflakeCursor, SnowflakeCursorBase
from snowflake.connector.errors import ProgrammingError


class TestFetchone:
    """Unit tests for Cursor.fetchone method."""

    @pytest.fixture
    def mock_connection(self):
        """Create a mock connection for testing."""
        return MagicMock()

    @pytest.fixture
    def cursor(self, mock_connection):
        """Create a cursor with a mock connection."""
        return SnowflakeCursor(mock_connection)

    def test_fetchone_returns_single_row(self, cursor):
        """Test fetchone returns a single row tuple."""
        mock_rows = [(1,), (2,), (3,)]
        mock_iterator = iter(mock_rows)
        cursor._iterator = mock_iterator

        with patch.object(cursor, "_get_iterator"):
            result = cursor.fetchone()

        assert result == (1,)

    def test_fetchone_returns_none_when_exhausted(self, cursor):
        """Test fetchone returns None when no more rows."""
        mock_iterator = iter([])
        cursor._iterator = mock_iterator

        with patch.object(cursor, "_get_iterator"):
            result = cursor.fetchone()

        assert result is None

    def test_fetchone_sequential_calls(self, cursor):
        """Test sequential fetchone calls return rows in order."""
        mock_rows = [(1,), (2,), (3,)]
        mock_iterator = iter(mock_rows)
        cursor._iterator = mock_iterator

        with patch.object(cursor, "_get_iterator"):
            first = cursor.fetchone()
            second = cursor.fetchone()
            third = cursor.fetchone()
            fourth = cursor.fetchone()

        assert first == (1,)
        assert second == (2,)
        assert third == (3,)
        assert fourth is None

    def test_fetchone_calls_get_iterator_if_iterator_is_none(self, cursor):
        """Test fetchone calls _get_iterator."""
        mock_ensure = MagicMock(return_value=iter([(1,)]))

        with patch.object(cursor, "_get_iterator", mock_ensure):
            cursor.fetchone()

        mock_ensure.assert_called_once()

    def test_fetchone_with_multi_column_row(self, cursor):
        """Test fetchone with multiple columns."""
        mock_rows = [(1, "hello", 3.14)]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_get_iterator"):
            result = cursor.fetchone()

        assert result == (1, "hello", 3.14)

    def test_fetchone_preserves_types(self, cursor):
        """Test fetchone preserves data types."""
        mock_rows = [(1, "text", Decimal("3.14"), None, True)]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_get_iterator"):
            result = cursor.fetchone()

        assert result[0] == 1
        assert result[1] == "text"
        assert result[2] == Decimal("3.14")
        assert isinstance(result[2], Decimal)
        assert result[3] is None
        assert result[4] is True

    def test_fetchone_with_empty_tuple_row(self, cursor):
        """Test fetchone handles empty tuple row."""
        mock_rows = [()]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_get_iterator"):
            result = cursor.fetchone()

        assert result == ()

    def test_fetchone_after_exhaustion_returns_none(self, cursor):
        """Test fetchone consistently returns None after exhaustion."""
        mock_rows = [(1,)]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_get_iterator"):
            cursor.fetchone()  # Consume the row
            result1 = cursor.fetchone()
            result2 = cursor.fetchone()

        assert result1 is None
        assert result2 is None


class TestFetchall:
    """Unit tests for Cursor.fetchall method."""

    @pytest.fixture
    def mock_connection(self):
        """Create a mock connection for testing."""
        return MagicMock()

    @pytest.fixture
    def cursor(self, mock_connection):
        """Create a cursor with a mock connection."""
        return SnowflakeCursor(mock_connection)

    def test_fetchall_returns_all_rows(self, cursor):
        """Test fetchall returns all rows as a list."""
        mock_rows = [(1,), (2,), (3,)]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_get_iterator"):
            result = cursor.fetchall()

        assert result == [(1,), (2,), (3,)]

    def test_fetchall_returns_empty_list_when_no_rows(self, cursor):
        """Test fetchall returns empty list when no rows."""
        cursor._iterator = iter([])

        with patch.object(cursor, "_get_iterator"):
            result = cursor.fetchall()

        assert result == []

    def test_fetchall_calls_get_iterator_if_iterator_is_none(self, cursor):
        """Test fetchall calls _get_iterator."""
        mock_ensure = MagicMock()

        with patch.object(cursor, "_get_iterator", mock_ensure):
            cursor.fetchall()

        mock_ensure.assert_called_once()

    def test_fetchall_with_single_row(self, cursor):
        """Test fetchall with single row."""
        mock_rows = [(42,)]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_get_iterator"):
            result = cursor.fetchall()

        assert result == [(42,)]
        assert len(result) == 1

    def test_fetchall_with_multi_column_rows(self, cursor):
        """Test fetchall with multiple columns per row."""
        mock_rows = [
            (1, "a", 1.0),
            (2, "b", 2.0),
            (3, "c", 3.0),
        ]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_get_iterator"):
            result = cursor.fetchall()

        assert result == [(1, "a", 1.0), (2, "b", 2.0), (3, "c", 3.0)]

    def test_fetchall_preserves_types(self, cursor):
        """Test fetchall preserves data types in rows."""
        mock_rows = [
            (1, "text", Decimal("3.14"), None),
            (2, "more", Decimal("2.71"), True),
        ]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_get_iterator"):
            result = cursor.fetchall()

        assert result[0] == (1, "text", Decimal("3.14"), None)
        assert result[1] == (2, "more", Decimal("2.71"), True)
        assert isinstance(result[0][2], Decimal)
        assert isinstance(result[1][2], Decimal)

    def test_fetchall_after_partial_fetchone(self, cursor):
        """Test fetchall returns remaining rows after fetchone."""
        mock_rows = [(1,), (2,), (3,), (4,), (5,)]
        mock_iterator = iter(mock_rows)
        cursor._iterator = mock_iterator

        with patch.object(cursor, "_get_iterator"):
            # Fetch first two rows
            cursor.fetchone()
            cursor.fetchone()
            # Fetch remaining
            result = cursor.fetchall()

        assert result == [(3,), (4,), (5,)]

    def test_fetchall_returns_empty_after_exhaustion(self, cursor):
        """Test fetchall returns empty list after all rows consumed."""
        mock_rows = [(1,), (2,)]
        mock_iterator = iter(mock_rows)
        cursor._iterator = mock_iterator

        with patch.object(cursor, "_get_iterator"):
            cursor.fetchall()  # Consume all rows
            result = cursor.fetchall()

        assert result == []

    def test_fetchall_with_large_result_set(self, cursor):
        """Test fetchall with large number of rows."""
        mock_rows = [(i,) for i in range(1000)]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_get_iterator"):
            result = cursor.fetchall()

        assert len(result) == 1000
        assert result[0] == (0,)
        assert result[999] == (999,)

    def test_fetchall_returns_list_not_iterator(self, cursor):
        """Test fetchall returns a list, not an iterator."""
        mock_rows = [(1,), (2,), (3,)]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_get_iterator"):
            result = cursor.fetchall()

        assert isinstance(result, list)


class TestFetchmany:
    """Unit tests for Cursor.fetchmany method."""

    @pytest.fixture
    def mock_connection(self):
        """Create a mock connection for testing."""
        return MagicMock()

    @pytest.fixture
    def cursor(self, mock_connection):
        """Create a cursor with a mock connection."""
        return SnowflakeCursor(mock_connection)

    def test_fetchmany_default_uses_arraysize(self, cursor):
        """Test that fetchmany() without size argument uses arraysize."""
        cursor.arraysize = 3
        mock_rows = [(1,), (2,), (3,), (4,), (5,)]
        row_iter = iter(mock_rows)

        with patch.object(cursor, "fetchone", side_effect=lambda: next(row_iter, None)):
            result = cursor.fetchmany()

        assert result == [(1,), (2,), (3,)]

    def test_fetchmany_with_explicit_size(self, cursor):
        """Test fetchmany with explicit size argument."""
        mock_rows = [(1,), (2,), (3,), (4,), (5,)]
        row_iter = iter(mock_rows)

        with patch.object(cursor, "fetchone", side_effect=lambda: next(row_iter, None)):
            result = cursor.fetchmany(2)

        assert result == [(1,), (2,)]

    def test_fetchmany_returns_fewer_rows_when_exhausted(self, cursor):
        """Test fetchmany returns fewer rows when result set is exhausted."""
        mock_rows = [(1,), (2,)]
        row_iter = iter(mock_rows)

        with patch.object(cursor, "fetchone", side_effect=lambda: next(row_iter, None)):
            result = cursor.fetchmany(5)

        assert result == [(1,), (2,)]

    def test_fetchmany_returns_empty_list_when_no_rows(self, cursor):
        """Test fetchmany returns empty list when no rows available."""
        with patch.object(cursor, "fetchone", return_value=None):
            result = cursor.fetchmany(5)

        assert result == []

    def test_fetchmany_with_size_zero(self, cursor):
        """Test fetchmany(0) returns empty list."""
        mock_fetchone = MagicMock()
        with patch.object(cursor, "fetchone", mock_fetchone):
            result = cursor.fetchmany(0)

        assert result == []
        mock_fetchone.assert_not_called()

    def test_fetchmany_with_negative_size_raises_error(self, cursor):
        """Test fetchmany with negative size raises ProgrammingError."""
        with pytest.raises(ProgrammingError) as excinfo:
            cursor.fetchmany(-1)

        assert "The number of rows is not zero or positive number: -1" in str(excinfo.value)

    def test_fetchmany_with_negative_size_various_values(self, cursor):
        """Test fetchmany raises ProgrammingError for various negative values."""
        with pytest.raises(ProgrammingError) as excinfo:
            cursor.fetchmany(-42)

        assert "The number of rows is not zero or positive number: -42" in str(excinfo.value)

    def test_fetchmany_sequential_calls(self, cursor):
        """Test multiple sequential fetchmany calls consume rows correctly."""
        mock_rows = [(1,), (2,), (3,), (4,), (5,)]
        row_iter = iter(mock_rows)

        with patch.object(cursor, "fetchone", side_effect=lambda: next(row_iter, None)):
            first_batch = cursor.fetchmany(2)
            second_batch = cursor.fetchmany(2)
            third_batch = cursor.fetchmany(2)

        assert first_batch == [(1,), (2,)]
        assert second_batch == [(3,), (4,)]
        assert third_batch == [(5,)]

    def test_fetchmany_after_exhausted_returns_empty(self, cursor):
        """Test fetchmany returns empty list after all rows consumed."""
        mock_rows = [(1,), (2,)]
        row_iter = iter(mock_rows)

        with patch.object(cursor, "fetchone", side_effect=lambda: next(row_iter, None)):
            cursor.fetchmany(5)  # Consume all rows
            result = cursor.fetchmany(5)

        assert result == []

    def test_fetchmany_respects_changed_arraysize(self, cursor):
        """Test fetchmany respects dynamically changed arraysize."""
        mock_rows = [(1,), (2,), (3,), (4,), (5,), (6,), (7,), (8,)]
        row_iter = iter(mock_rows)

        with patch.object(cursor, "fetchone", side_effect=lambda: next(row_iter, None)):
            cursor.arraysize = 2
            first_batch = cursor.fetchmany()

            cursor.arraysize = 4
            second_batch = cursor.fetchmany()

        assert first_batch == [(1,), (2,)]
        assert second_batch == [(3,), (4,), (5,), (6,)]

    def test_fetchmany_with_size_one(self, cursor):
        """Test fetchmany(1) returns single row list."""
        mock_rows = [(1,), (2,), (3,)]
        row_iter = iter(mock_rows)

        with patch.object(cursor, "fetchone", side_effect=lambda: next(row_iter, None)):
            result = cursor.fetchmany(1)

        assert result == [(1,)]

    def test_fetchmany_with_large_size(self, cursor):
        """Test fetchmany with size larger than available rows."""
        mock_rows = [(i,) for i in range(10)]
        row_iter = iter(mock_rows)

        with patch.object(cursor, "fetchone", side_effect=lambda: next(row_iter, None)):
            result = cursor.fetchmany(1000)

        assert result == [(i,) for i in range(10)]

    def test_fetchmany_default_arraysize_is_one(self, cursor):
        """Test that default arraysize is 1."""
        assert cursor.arraysize == 1

        mock_rows = [(1,), (2,), (3,)]
        row_iter = iter(mock_rows)

        with patch.object(cursor, "fetchone", side_effect=lambda: next(row_iter, None)):
            result = cursor.fetchmany()

        # Default arraysize is 1, so should fetch 1 row
        assert result == [(1,)]

    def test_fetchmany_with_multi_column_rows(self, cursor):
        """Test fetchmany with rows containing multiple columns."""
        mock_rows = [
            (1, "a", 1.0),
            (2, "b", 2.0),
            (3, "c", 3.0),
        ]
        row_iter = iter(mock_rows)

        with patch.object(cursor, "fetchone", side_effect=lambda: next(row_iter, None)):
            result = cursor.fetchmany(2)

        assert result == [(1, "a", 1.0), (2, "b", 2.0)]

    def test_fetchmany_preserves_row_types(self, cursor):
        """Test that fetchmany preserves the types in rows."""
        mock_rows = [
            (1, "text", Decimal("3.14"), None),
            (2, "more", Decimal("2.71"), True),
        ]
        row_iter = iter(mock_rows)

        with patch.object(cursor, "fetchone", side_effect=lambda: next(row_iter, None)):
            result = cursor.fetchmany(2)

        assert result[0] == (1, "text", Decimal("3.14"), None)
        assert result[1] == (2, "more", Decimal("2.71"), True)
        assert isinstance(result[0][2], Decimal)
        assert result[0][3] is None
        assert result[1][3] is True


class TestFetchmanyArraysizeAttribute:
    """Tests for arraysize attribute interaction with fetchmany."""

    @pytest.fixture
    def mock_connection(self):
        """Create a mock connection for testing."""
        return MagicMock()

    @pytest.fixture
    def cursor(self, mock_connection):
        """Create a cursor with a mock connection."""
        return SnowflakeCursor(mock_connection)

    def test_arraysize_default(self, cursor):
        """Test that cursor has default arraysize of 1."""
        assert cursor.arraysize == 1

    def test_arraysize_is_property(self):
        """Test that arraysize is a property on the class."""
        assert isinstance(SnowflakeCursorBase.__dict__["arraysize"], property)

    def test_arraysize_instance_independent(self, cursor):
        """Test instance arraysize changes are independent."""
        assert cursor.arraysize == 1
        cursor.arraysize = 10
        assert cursor.arraysize == 10

    def test_fetchmany_uses_instance_arraysize(self, cursor):
        """Test fetchmany uses instance arraysize, not class attribute."""
        cursor.arraysize = 5
        mock_rows = [(i,) for i in range(10)]
        row_iter = iter(mock_rows)

        with patch.object(cursor, "fetchone", side_effect=lambda: next(row_iter, None)):
            result = cursor.fetchmany()

        assert len(result) == 5
