"""
Unit tests for PEP 249 Cursor class.
"""

import asyncio

from decimal import Decimal
from unittest.mock import ANY, AsyncMock, MagicMock, patch

import pytest

from snowflake.connector._internal.api_client.client_api import core_driver
from snowflake.connector._internal.binding_converters import ParamStyle, parse_stage_binding_threshold
from snowflake.connector._internal.cursor import CursorBaseMixin, QueryResult, QueryResultWaiter
from snowflake.connector._internal.errorcode import ER_INVALID_VALUE, ER_NO_PYARROW
from snowflake.connector._internal.extras import (
    MissingOptionalDependency,
)
from snowflake.connector._internal.extras import (
    check_dependency as _real_check_dependency,
)
from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
    ConnectionHandle,
    ResultSetHandle,
    StatementHandle,
)
from snowflake.connector.aio.cursor import SnowflakeCursor as AsyncSnowflakeCursor
from snowflake.connector.constants import QueryStatus, StatementParameterName
from snowflake.connector.cursor import QueryResultStats, ResultMetadataV2, SnowflakeCursor
from snowflake.connector.errors import DatabaseError, InterfaceError, ProgrammingError


@pytest.fixture(autouse=True)
def _no_native_stream_ops():
    """Prevent QueryResult from touching real native memory in unit tests."""
    with (
        patch("snowflake.connector._internal.cursor.query_result.get_stream_ptr", return_value=0),
        patch("snowflake.connector._internal.cursor.query_result.release_arrow_stream"),
    ):
        yield


@pytest.fixture
def mock_core_client():
    """Provide a MagicMock patched into core_driver.client for cursor tests."""
    mock = MagicMock()
    old = core_driver._client
    core_driver.client = mock
    yield mock
    core_driver.client = old


class MockRowIterator:
    """A mock row iterator that supports fetch_many/fetch_all like ArrowStreamIterator."""

    def __init__(self, rows):
        self._rows = list(rows)
        self._pos = 0

    def __iter__(self):
        return self

    def __next__(self):
        if self._pos >= len(self._rows):
            raise StopIteration
        row = self._rows[self._pos]
        self._pos += 1
        return row

    def fetch_many(self, size):
        result = self._rows[self._pos : self._pos + size]
        self._pos += len(result)
        return result

    def fetch_all(self):
        result = self._rows[self._pos :]
        self._pos = len(self._rows)
        return result


class TestFetchone:
    """Unit tests for Cursor.fetchone method."""

    @pytest.fixture
    def mock_connection(self):
        mock_connection = MagicMock()
        mock_connection.is_closed.return_value = False
        return mock_connection

    @pytest.fixture
    def cursor(self, mock_connection):
        """Create a cursor with a mock connection."""
        return SnowflakeCursor(mock_connection)

    def test_fetchone_returns_single_row(self, cursor):
        """Test fetchone returns a single row tuple."""
        mock_rows = [(1,), (2,), (3,)]
        mock_iterator = iter(mock_rows)
        cursor._iterator = mock_iterator

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchone()

        assert result == (1,)

    def test_fetchone_returns_none_when_exhausted(self, cursor):
        """Test fetchone returns None when no more rows."""
        mock_iterator = iter([])
        cursor._iterator = mock_iterator

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchone()

        assert result is None

    def test_fetchone_sequential_calls(self, cursor):
        """Test sequential fetchone calls return rows in order."""
        mock_rows = [(1,), (2,), (3,)]
        mock_iterator = iter(mock_rows)
        cursor._iterator = mock_iterator

        with patch.object(cursor, "_create_row_iterator"):
            first = cursor.fetchone()
            second = cursor.fetchone()
            third = cursor.fetchone()
            fourth = cursor.fetchone()

        assert first == (1,)
        assert second == (2,)
        assert third == (3,)
        assert fourth is None

    def test_fetchone_calls_create_row_iterator_if_iterator_is_none(self, cursor):
        """Test fetchone calls _create_row_iterator."""
        mock_ensure = MagicMock(return_value=iter([(1,)]))

        with patch.object(cursor, "_create_row_iterator", mock_ensure):
            cursor.fetchone()

        mock_ensure.assert_called_once()

    def test_fetchone_with_multi_column_row(self, cursor):
        """Test fetchone with multiple columns."""
        mock_rows = [(1, "hello", 3.14)]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchone()

        assert result == (1, "hello", 3.14)

    def test_fetchone_preserves_types(self, cursor):
        """Test fetchone preserves data types."""
        mock_rows = [(1, "text", Decimal("3.14"), None, True)]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_create_row_iterator"):
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

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchone()

        assert result == ()

    def test_fetchone_after_exhaustion_returns_none(self, cursor):
        """Test fetchone consistently returns None after exhaustion."""
        mock_rows = [(1,)]
        cursor._iterator = iter(mock_rows)

        with patch.object(cursor, "_create_row_iterator"):
            cursor.fetchone()  # Consume the row
            result1 = cursor.fetchone()
            result2 = cursor.fetchone()

        assert result1 is None
        assert result2 is None


class TestFetchall:
    """Unit tests for Cursor.fetchall method."""

    @pytest.fixture
    def mock_connection(self):
        mock_connection = MagicMock()
        mock_connection.is_closed.return_value = False
        return mock_connection

    @pytest.fixture
    def cursor(self, mock_connection):
        """Create a cursor with a mock connection."""
        return SnowflakeCursor(mock_connection)

    def test_fetchall_returns_all_rows(self, cursor):
        """Test fetchall returns all rows as a list."""
        cursor._iterator = MockRowIterator([(1,), (2,), (3,)])

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchall()

        assert result == [(1,), (2,), (3,)]

    def test_fetchall_returns_empty_list_when_no_rows(self, cursor):
        """Test fetchall returns empty list when no rows."""
        cursor._iterator = MockRowIterator([])

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchall()

        assert result == []

    def test_fetchall_calls_create_row_iterator_if_iterator_is_none(self, cursor):
        """Test fetchall calls _create_row_iterator."""
        mock_ensure = MagicMock(return_value=MockRowIterator([]))

        with patch.object(cursor, "_create_row_iterator", mock_ensure):
            cursor.fetchall()

        mock_ensure.assert_called_once()

    def test_fetchall_with_single_row(self, cursor):
        """Test fetchall with single row."""
        cursor._iterator = MockRowIterator([(42,)])

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchall()

        assert result == [(42,)]
        assert len(result) == 1

    def test_fetchall_with_multi_column_rows(self, cursor):
        """Test fetchall with multiple columns per row."""
        cursor._iterator = MockRowIterator(
            [
                (1, "a", 1.0),
                (2, "b", 2.0),
                (3, "c", 3.0),
            ]
        )

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchall()

        assert result == [(1, "a", 1.0), (2, "b", 2.0), (3, "c", 3.0)]

    def test_fetchall_preserves_types(self, cursor):
        """Test fetchall preserves data types in rows."""
        cursor._iterator = MockRowIterator(
            [
                (1, "text", Decimal("3.14"), None),
                (2, "more", Decimal("2.71"), True),
            ]
        )

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchall()

        assert result[0] == (1, "text", Decimal("3.14"), None)
        assert result[1] == (2, "more", Decimal("2.71"), True)
        assert isinstance(result[0][2], Decimal)
        assert isinstance(result[1][2], Decimal)

    def test_fetchall_after_partial_fetchone(self, cursor):
        """Test fetchall returns remaining rows after fetchone."""
        cursor._iterator = MockRowIterator([(1,), (2,), (3,), (4,), (5,)])

        with patch.object(cursor, "_create_row_iterator"):
            # Fetch first two rows
            cursor.fetchone()
            cursor.fetchone()
            # Fetch remaining
            result = cursor.fetchall()

        assert result == [(3,), (4,), (5,)]

    def test_fetchall_returns_empty_after_exhaustion(self, cursor):
        """Test fetchall returns empty list after all rows consumed."""
        cursor._iterator = MockRowIterator([(1,), (2,)])

        with patch.object(cursor, "_create_row_iterator"):
            cursor.fetchall()  # Consume all rows
            result = cursor.fetchall()

        assert result == []

    def test_fetchall_with_large_result_set(self, cursor):
        """Test fetchall with large number of rows."""
        cursor._iterator = MockRowIterator([(i,) for i in range(1000)])

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchall()

        assert len(result) == 1000
        assert result[0] == (0,)
        assert result[999] == (999,)

    def test_fetchall_returns_list_not_iterator(self, cursor):
        """Test fetchall returns a list, not an iterator."""
        cursor._iterator = MockRowIterator([(1,), (2,), (3,)])

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchall()

        assert isinstance(result, list)


class TestFetchmany:
    """Unit tests for Cursor.fetchmany method."""

    @pytest.fixture
    def mock_connection(self):
        """Create a mock connection for testing."""
        mock_connection = MagicMock()
        mock_connection.is_closed.return_value = False
        return mock_connection

    @pytest.fixture
    def cursor(self, mock_connection):
        """Create a cursor with a mock connection."""
        return SnowflakeCursor(mock_connection)

    def test_fetchmany_default_uses_arraysize(self, cursor):
        """Test that fetchmany() without size argument uses arraysize."""
        cursor.arraysize = 3
        cursor._iterator = MockRowIterator([(1,), (2,), (3,), (4,), (5,)])

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchmany()

        assert result == [(1,), (2,), (3,)]

    def test_fetchmany_with_explicit_size(self, cursor):
        """Test fetchmany with explicit size argument."""
        cursor._iterator = MockRowIterator([(1,), (2,), (3,), (4,), (5,)])

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchmany(2)

        assert result == [(1,), (2,)]

    def test_fetchmany_returns_fewer_rows_when_exhausted(self, cursor):
        """Test fetchmany returns fewer rows when result set is exhausted."""
        cursor._iterator = MockRowIterator([(1,), (2,)])

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchmany(5)

        assert result == [(1,), (2,)]

    def test_fetchmany_returns_empty_list_when_no_rows(self, cursor):
        """Test fetchmany returns empty list when no rows available."""
        cursor._iterator = MockRowIterator([])

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchmany(5)

        assert result == []

    def test_fetchmany_with_size_zero(self, cursor):
        """Test fetchmany(0) returns empty list without creating iterator."""
        with patch.object(cursor, "_create_row_iterator") as mock_create:
            result = cursor.fetchmany(0)

        assert result == []
        mock_create.assert_not_called()

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
        cursor._iterator = MockRowIterator([(1,), (2,), (3,), (4,), (5,)])

        with patch.object(cursor, "_create_row_iterator"):
            first_batch = cursor.fetchmany(2)
            second_batch = cursor.fetchmany(2)
            third_batch = cursor.fetchmany(2)

        assert first_batch == [(1,), (2,)]
        assert second_batch == [(3,), (4,)]
        assert third_batch == [(5,)]

    def test_fetchmany_after_exhausted_returns_empty(self, cursor):
        """Test fetchmany returns empty list after all rows consumed."""
        cursor._iterator = MockRowIterator([(1,), (2,)])

        with patch.object(cursor, "_create_row_iterator"):
            cursor.fetchmany(5)  # Consume all rows
            result = cursor.fetchmany(5)

        assert result == []

    def test_fetchmany_respects_changed_arraysize(self, cursor):
        """Test fetchmany respects dynamically changed arraysize."""
        cursor._iterator = MockRowIterator([(1,), (2,), (3,), (4,), (5,), (6,), (7,), (8,)])

        with patch.object(cursor, "_create_row_iterator"):
            cursor.arraysize = 2
            first_batch = cursor.fetchmany()

            cursor.arraysize = 4
            second_batch = cursor.fetchmany()

        assert first_batch == [(1,), (2,)]
        assert second_batch == [(3,), (4,), (5,), (6,)]

    def test_fetchmany_with_size_one(self, cursor):
        """Test fetchmany(1) returns single row list."""
        cursor._iterator = MockRowIterator([(1,), (2,), (3,)])

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchmany(1)

        assert result == [(1,)]

    def test_fetchmany_with_large_size(self, cursor):
        """Test fetchmany with size larger than available rows."""
        cursor._iterator = MockRowIterator([(i,) for i in range(10)])

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchmany(1000)

        assert result == [(i,) for i in range(10)]

    def test_fetchmany_default_arraysize_is_one(self, cursor):
        """Test that default arraysize is 1."""
        assert cursor.arraysize == 1

        cursor._iterator = MockRowIterator([(1,), (2,), (3,)])

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchmany()

        # Default arraysize is 1, so should fetch 1 row
        assert result == [(1,)]

    def test_fetchmany_with_multi_column_rows(self, cursor):
        """Test fetchmany with rows containing multiple columns."""
        cursor._iterator = MockRowIterator(
            [
                (1, "a", 1.0),
                (2, "b", 2.0),
                (3, "c", 3.0),
            ]
        )

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchmany(2)

        assert result == [(1, "a", 1.0), (2, "b", 2.0)]

    def test_fetchmany_preserves_row_types(self, cursor):
        """Test that fetchmany preserves the types in rows."""
        cursor._iterator = MockRowIterator(
            [
                (1, "text", Decimal("3.14"), None),
                (2, "more", Decimal("2.71"), True),
            ]
        )

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchmany(2)

        assert result[0] == (1, "text", Decimal("3.14"), None)
        assert result[1] == (2, "more", Decimal("2.71"), True)
        assert isinstance(result[0][2], Decimal)
        assert result[0][3] is None
        assert result[1][3] is True

    def test_fetchmany_after_partial_fetchone(self, cursor):
        """Test fetchmany returns correct rows after partial fetchone consumption."""
        cursor._iterator = MockRowIterator([(1,), (2,), (3,), (4,), (5,)])

        with patch.object(cursor, "_create_row_iterator"):
            cursor.fetchone()
            cursor.fetchone()
            result = cursor.fetchmany(2)

        assert result == [(3,), (4,)]


class TestHandleLifecycle:
    """Unit tests for statement and ResultSet handle lifecycle.

    Statement handles are scoped to the ``statement()`` context manager
    inside ``_execute`` — they are always released when execute completes.
    ResultSet handles live longer: they are held by the cursor's
    ``_ResultSetWrapper`` and released on ``reset()``, ``close()``, or
    when the next ``execute()`` replaces them.
    """

    @pytest.fixture
    def mock_connection(self, mock_core_client):
        """Create a mock connection with core_driver stubs for execute flow."""
        conn = MagicMock()
        conn.conn_handle = ConnectionHandle(id=1)
        conn.is_closed.return_value = False
        rs_counter = 0

        mock_core_client.statement_new.return_value.stmt_handle = StatementHandle(id=1)

        def make_execute_response(*_args, **_kwargs):
            nonlocal rs_counter
            rs_counter += 1
            response = MagicMock()
            response.HasField = MagicMock(return_value=False)
            response.single.result_set_handle = ResultSetHandle(id=rs_counter)
            response.single.result_descriptor.query_id = "fake-qid"
            response.single.result_descriptor.HasField = MagicMock(return_value=False)
            response.single.result_descriptor.rows_affected = 0
            response.single.result_descriptor.sql_state = "00000"
            return response

        mock_core_client.statement_execute_query.side_effect = make_execute_response

        return conn

    @pytest.fixture
    def cursor(self, mock_connection):
        """Create a cursor with the mocked connection."""
        return SnowflakeCursor(mock_connection)

    def test_statement_handle_released_after_execute(self, cursor, mock_core_client):
        """Statement handle is released within execute (context manager)."""
        cursor.execute("SELECT 1")

        mock_core_client.statement_release.assert_called_once()

    def test_result_set_handle_survives_execute(self, cursor, mock_core_client):
        """After execute(), the ResultSet handle is still held (not released)."""
        cursor.execute("SELECT 1")

        mock_core_client.result_set_release.assert_not_called()

    def test_reset_releases_result_set_handle(self, cursor, mock_core_client):
        """reset() releases the ResultSet handle held by the cursor."""
        cursor.execute("SELECT 1")
        cursor.reset()

        mock_core_client.result_set_release.assert_called_once()
        released_id = mock_core_client.result_set_release.call_args.args[0].result_set_handle.id
        assert released_id == 1

    def test_close_releases_result_set_handle(self, cursor, mock_core_client):
        """close() releases the ResultSet handle held by the cursor."""
        cursor.execute("SELECT 1")
        cursor.close()

        mock_core_client.result_set_release.assert_called_once()

    def test_sequential_executes_release_previous_result_set_handles(self, cursor, mock_core_client):
        """Each execute() releases the ResultSet handle from the previous execution."""
        n = 5
        for i in range(n):
            cursor.execute(f"SELECT {i}")

        release = mock_core_client.result_set_release
        # First n-1 handles released by replace() at the start of each subsequent _apply_result_set;
        # the last handle is still alive.
        assert release.call_count == n - 1
        released_ids = [call.args[0].result_set_handle.id for call in release.call_args_list]
        assert released_ids == list(range(1, n))

    def test_close_without_execute_does_not_release(self, cursor, mock_core_client):
        """Closing a cursor that never executed should not call release."""
        cursor.close()

        mock_core_client.result_set_release.assert_not_called()


class TestSqlstate:
    """Unit tests for Cursor.sqlstate property."""

    @pytest.fixture
    def mock_connection(self, mock_core_client):
        conn = MagicMock()
        conn.conn_handle = ConnectionHandle(id=1)
        conn.is_closed.return_value = False
        mock_core_client.statement_new.return_value.stmt_handle = StatementHandle(id=1)
        return conn

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def _stub_execute_result(self, mock_core_client, **overrides):
        """Set up the mock to return an execute result with the given overrides."""
        # Mock ResultSetDescriptor
        descriptor = MagicMock()
        descriptor.query_id = overrides.get("query_id", "test-query-id")
        descriptor.columns = overrides.get("columns", [])
        descriptor.rows_affected = overrides.get("rows_affected", 0)
        descriptor.sql_state = overrides.get("sql_state", "")
        descriptor.statement_type_id = overrides.get("statement_type_id", 0x0000)

        def has_field_impl(field_name):
            if field_name == "rows_affected":
                return overrides.get("has_rows_affected", False)
            elif field_name == "sql_state":
                return bool(overrides.get("sql_state", ""))
            elif field_name == "stats":
                return overrides.get("has_stats", False)
            elif field_name == "statement_type_id":
                return "statement_type_id" in overrides
            return False

        descriptor.HasField = MagicMock(side_effect=has_field_impl)

        # Mock ExecuteQueryResponse with single statement (ResultSetResponse)
        execute_response = MagicMock()
        execute_response.single.result_descriptor = descriptor
        execute_response.single.result_set_handle = ResultSetHandle(id=1)
        execute_response.HasField = MagicMock(side_effect=lambda f: f == "single")
        mock_core_client.statement_execute_query.return_value = execute_response

        return descriptor

    def test_sqlstate_none_before_execute(self, cursor):
        """sqlstate is None on a fresh cursor."""
        assert cursor.sqlstate is None

    def test_sqlstate_none_after_successful_execute(self, cursor, mock_core_client):
        """sqlstate is None when server returns '00000' (successful completion)."""
        self._stub_execute_result(mock_core_client, sql_state="00000")

        cursor.execute("SELECT 1")

        assert cursor.sqlstate is None

    def test_sqlstate_populated_with_error_code(self, cursor, mock_core_client):
        """sqlstate reflects non-success sql_state from execute result."""
        self._stub_execute_result(mock_core_client, sql_state="42601")

        cursor.execute("SELECT 1")

        assert cursor.sqlstate == "42601"

    def test_sqlstate_none_when_field_absent(self, cursor, mock_core_client):
        """sqlstate is None when the server does not return sql_state."""
        self._stub_execute_result(mock_core_client, sql_state="")

        cursor.execute("SELECT 1")

        assert cursor.sqlstate is None

    def test_sqlstate_updates_on_subsequent_execute(self, cursor, mock_core_client):
        """sqlstate is refreshed on every execute call."""
        # First execute with error
        self._stub_execute_result(mock_core_client, sql_state="42601")

        cursor.execute("SELECT 1")
        assert cursor.sqlstate == "42601"

        # Second execute with success
        self._stub_execute_result(mock_core_client, sql_state="00000")
        cursor.execute("SELECT 2")
        assert cursor.sqlstate is None

    def test_sqlstate_set_from_error_on_failed_execute(self, cursor, mock_core_client):
        """sqlstate is captured from PEP 249 Error when execute raises."""
        mock_core_client.statement_execute_query.side_effect = ProgrammingError("error", sqlstate="42601")

        with pytest.raises(ProgrammingError):
            cursor.execute("INVALID SQL")

        assert cursor.sqlstate == "42601"

    def test_sqlstate_set_to_none_when_error_has_no_sqlstate(self, cursor, mock_core_client):
        """sqlstate is set to None when error carries no sqlstate."""
        mock_core_client.statement_execute_query.side_effect = ProgrammingError("error", sqlstate=None)

        with pytest.raises(ProgrammingError):
            cursor.execute("INVALID SQL")

        assert cursor.sqlstate is None

    def test_sqlstate_transitions_across_success_and_failure(self, cursor, mock_core_client):
        """sqlstate updates correctly through None -> error -> None."""
        success_result = MagicMock()
        success_result.columns = []
        success_result.sql_state = "00000"

        mock_core_client.statement_execute_query.return_value.result = success_result
        mock_core_client.statement_execute_query.side_effect = None
        cursor.execute("SELECT 1")
        assert cursor.sqlstate is None

        mock_core_client.statement_execute_query.side_effect = ProgrammingError("error", sqlstate="42601")
        with pytest.raises(ProgrammingError):
            cursor.execute("INVALID SQL")
        assert cursor.sqlstate == "42601"

        mock_core_client.statement_execute_query.side_effect = None
        mock_core_client.statement_execute_query.return_value.result = success_result
        cursor.execute("SELECT 2")
        assert cursor.sqlstate is None


class TestSfqidOnFailedQuery:
    """Unit tests for cursor.sfqid propagation when execute raises."""

    @pytest.fixture
    def mock_connection(self, mock_core_client):
        conn = MagicMock()
        conn.conn_handle = ConnectionHandle(id=1)
        conn.is_closed.return_value = False
        mock_core_client.statement_new.return_value.stmt_handle = StatementHandle(id=1)
        return conn

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_sfqid_set_from_error_on_failed_execute(self, cursor, mock_core_client):
        """sfqid is captured from ProgrammingError when execute raises."""
        mock_core_client.statement_execute_query.side_effect = ProgrammingError("error", sfqid="01abc-def-12345")

        with pytest.raises(ProgrammingError):
            cursor.execute("INVALID SQL")

        assert cursor.sfqid == "01abc-def-12345"

    def test_sfqid_none_when_error_has_no_sfqid(self, cursor, mock_core_client):
        """sfqid is None when error carries no sfqid."""
        mock_core_client.statement_execute_query.side_effect = ProgrammingError("error")

        with pytest.raises(ProgrammingError):
            cursor.execute("INVALID SQL")

        assert cursor.sfqid is None


class TestQueryResultStats:
    """Unit tests for QueryResultStats NamedTuple."""

    def test_default_all_none(self):
        """All fields default to None."""
        stats = QueryResultStats()
        assert stats.num_rows_inserted is None
        assert stats.num_rows_deleted is None
        assert stats.num_rows_updated is None
        assert stats.num_dml_duplicates is None

    def test_positional_construction(self):
        """Fields can be set by position."""
        stats = QueryResultStats(10, 20, 30, 5)
        assert stats.num_rows_inserted == 10
        assert stats.num_rows_deleted == 20
        assert stats.num_rows_updated == 30
        assert stats.num_dml_duplicates == 5

    def test_keyword_construction(self):
        """Fields can be set by keyword."""
        stats = QueryResultStats(num_rows_inserted=1, num_rows_updated=2)
        assert stats.num_rows_inserted == 1
        assert stats.num_rows_deleted is None
        assert stats.num_rows_updated == 2
        assert stats.num_dml_duplicates is None

    def test_is_named_tuple(self):
        """QueryResultStats is a proper NamedTuple with tuple semantics."""
        stats = QueryResultStats(1, 2, 3, 4)
        assert isinstance(stats, tuple)
        assert len(stats) == 4
        assert stats[0] == 1
        assert stats._fields == ("num_rows_inserted", "num_rows_deleted", "num_rows_updated", "num_dml_duplicates")

    def test_equality(self):
        """Two instances with identical values are equal."""
        a = QueryResultStats(1, 2, 3, 4)
        b = QueryResultStats(1, 2, 3, 4)
        assert a == b

    def test_all_none_equality(self):
        """Default instance equals explicit all-None instance."""
        assert QueryResultStats() == QueryResultStats(None, None, None, None)

    def test_from_query_stats_all_fields_present(self):
        """from_query_stats maps all present protobuf fields."""
        mock_stats = MagicMock()
        mock_stats.num_rows_inserted = 10
        mock_stats.num_rows_deleted = 5
        mock_stats.num_rows_updated = 3
        mock_stats.num_dml_duplicates = 1
        mock_stats.HasField.return_value = True

        result = QueryResultStats.from_query_stats(mock_stats)

        assert result == QueryResultStats(10, 5, 3, 1)

    def test_from_query_stats_partial_fields(self):
        """from_query_stats returns None for absent protobuf fields."""
        mock_stats = MagicMock()
        mock_stats.num_rows_inserted = 42
        mock_stats.HasField.side_effect = lambda name: name == "num_rows_inserted"

        result = QueryResultStats.from_query_stats(mock_stats)

        assert result == QueryResultStats(
            num_rows_inserted=42, num_rows_deleted=None, num_rows_updated=None, num_dml_duplicates=None
        )

    def test_from_query_stats_no_fields_present(self):
        """from_query_stats returns all None when no fields are set."""
        mock_stats = MagicMock()
        mock_stats.HasField.return_value = False

        result = QueryResultStats.from_query_stats(mock_stats)

        assert result == QueryResultStats()

    def test_from_query_stats_zero_values(self):
        """from_query_stats preserves zero values (distinct from absent)."""
        mock_stats = MagicMock()
        mock_stats.num_rows_inserted = 0
        mock_stats.num_rows_deleted = 0
        mock_stats.num_rows_updated = 0
        mock_stats.num_dml_duplicates = 0
        mock_stats.HasField.return_value = True

        result = QueryResultStats.from_query_stats(mock_stats)

        assert result == QueryResultStats(0, 0, 0, 0)


class TestStats:
    """Unit tests for Cursor.stats property."""

    @pytest.fixture
    def mock_connection(self):
        return MagicMock()

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_stats_returns_all_none_before_execute(self, cursor):
        """stats returns QueryResultStats with all None fields on a fresh cursor."""
        result = cursor.stats
        assert result == QueryResultStats(None, None, None, None)

    def test_stats_returns_all_none_when_no_stats_field(self, cursor):
        """stats returns all-None when _query_result has default stats."""
        result = cursor.stats

        assert result == QueryResultStats(None, None, None, None)

    def test_stats_returns_all_fields_when_present(self, cursor):
        """stats returns all populated fields from _query_result."""
        cursor._query_result.stats = QueryResultStats(
            num_rows_inserted=10,
            num_rows_deleted=5,
            num_rows_updated=3,
            num_dml_duplicates=1,
        )

        assert cursor.stats == QueryResultStats(
            num_rows_inserted=10,
            num_rows_deleted=5,
            num_rows_updated=3,
            num_dml_duplicates=1,
        )

    def test_stats_returns_partial_fields(self, cursor):
        """stats returns None for fields not present."""
        cursor._query_result.stats = QueryResultStats(num_rows_inserted=10)

        result = cursor.stats

        assert result.num_rows_inserted == 10
        assert result.num_rows_deleted is None
        assert result.num_rows_updated is None
        assert result.num_dml_duplicates is None

    def test_stats_distinguishes_zero_from_absent(self, cursor):
        """A field present with value 0 is returned as 0, not None."""
        cursor._query_result.stats = QueryResultStats(0, 0, 0, 0)

        assert cursor.stats == QueryResultStats(0, 0, 0, 0)

    def test_stats_returns_query_result_stats_type(self, cursor):
        """stats always returns a QueryResultStats instance."""
        assert isinstance(cursor.stats, QueryResultStats)

        cursor._query_result.stats = QueryResultStats(1, 2, 3, 4)
        assert isinstance(cursor.stats, QueryResultStats)

    def test_stats_updates_on_subsequent_execute(self, cursor):
        """stats reflects the most recent _query_result."""
        cursor._query_result.stats = QueryResultStats(num_rows_inserted=5)
        assert cursor.stats.num_rows_inserted == 5

        cursor._query_result.stats = QueryResultStats(num_rows_inserted=20)
        assert cursor.stats.num_rows_inserted == 20

    def test_stats_only_insert_field(self, cursor):
        """stats correctly returns only num_rows_inserted when only that field is present."""
        cursor._query_result.stats = QueryResultStats(num_rows_inserted=42)

        result = cursor.stats
        assert result == QueryResultStats(
            num_rows_inserted=42, num_rows_deleted=None, num_rows_updated=None, num_dml_duplicates=None
        )


class TestFetchmanyArraysizeAttribute:
    """Tests for arraysize attribute interaction with fetchmany."""

    @pytest.fixture
    def mock_connection(self):
        mock_connection = MagicMock()
        mock_connection.is_closed.return_value = False
        return mock_connection

    @pytest.fixture
    def cursor(self, mock_connection):
        """Create a cursor with a mock connection."""
        return SnowflakeCursor(mock_connection)

    def test_arraysize_default(self, cursor):
        """Test that cursor has default arraysize of 1."""
        assert cursor.arraysize == 1

    def test_arraysize_is_property(self):
        """Test that arraysize is a property on the class."""
        assert isinstance(CursorBaseMixin.__dict__["arraysize"], property)

    def test_arraysize_instance_independent(self, cursor):
        """Test instance arraysize changes are independent."""
        assert cursor.arraysize == 1
        cursor.arraysize = 10
        assert cursor.arraysize == 10

    def test_fetchmany_uses_instance_arraysize(self, cursor):
        """Test fetchmany uses instance arraysize, not class attribute."""
        cursor.arraysize = 5
        cursor._iterator = MockRowIterator([(i,) for i in range(10)])

        with patch.object(cursor, "_create_row_iterator"):
            result = cursor.fetchmany()

        assert len(result) == 5


class TestRownumber:
    """Unit tests for Cursor.rownumber property."""

    @pytest.fixture
    def mock_connection(self):
        mock_connection = MagicMock()
        mock_connection.is_closed.return_value = False
        return mock_connection

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_rownumber_none_before_fetch(self, cursor):
        """rownumber is None before any rows have been fetched."""
        assert cursor.rownumber is None

    def test_rownumber_increments_with_fetchone(self, cursor):
        """rownumber increments by 1 for each fetchone call."""
        cursor._iterator = MockRowIterator([(1,), (2,), (3,)])

        with patch.object(cursor, "_create_row_iterator"):
            cursor.fetchone()
            assert cursor.rownumber == 0
            cursor.fetchone()
            assert cursor.rownumber == 1
            cursor.fetchone()
            assert cursor.rownumber == 2

    def test_rownumber_stays_after_fetchone_exhausted(self, cursor):
        """rownumber stays at last value when fetchone returns None."""
        cursor._iterator = MockRowIterator([(1,)])

        with patch.object(cursor, "_create_row_iterator"):
            cursor.fetchone()
            assert cursor.rownumber == 0
            cursor.fetchone()  # returns None
            assert cursor.rownumber == 0

    def test_rownumber_updated_by_fetchall(self, cursor):
        """rownumber reflects total rows fetched after fetchall."""
        cursor._iterator = MockRowIterator([(1,), (2,), (3,), (4,), (5,)])

        with patch.object(cursor, "_create_row_iterator"):
            cursor.fetchall()
            assert cursor.rownumber == 4

    def test_rownumber_updated_by_fetchall_after_partial_fetchone(self, cursor):
        """rownumber is correct when fetchall follows partial fetchone consumption."""
        cursor._iterator = MockRowIterator([(1,), (2,), (3,), (4,), (5,)])

        with patch.object(cursor, "_create_row_iterator"):
            cursor.fetchone()
            cursor.fetchone()
            assert cursor.rownumber == 1
            cursor.fetchall()
            assert cursor.rownumber == 4

    def test_rownumber_updated_by_fetchmany(self, cursor):
        """rownumber increments correctly through fetchmany calls."""
        cursor._iterator = MockRowIterator([(1,), (2,), (3,), (4,), (5,)])

        with patch.object(cursor, "_create_row_iterator"):
            cursor.fetchmany(3)
            assert cursor.rownumber == 2
            cursor.fetchmany(2)
            assert cursor.rownumber == 4

    def test_rownumber_fetchall_on_empty_result(self, cursor):
        """rownumber stays None when fetchall returns no rows."""
        cursor._iterator = MockRowIterator([])

        with patch.object(cursor, "_create_row_iterator"):
            cursor.fetchall()
            assert cursor.rownumber is None

    def test_rownumber_none_after_execute_resets(self, cursor):
        """rownumber resets to None after a new execute call."""
        cursor._iterator = MockRowIterator([(1,), (2,)])

        with patch.object(cursor, "_create_row_iterator"):
            cursor.fetchone()
            assert cursor.rownumber == 0

        cursor._rownumber = -1  # simulates what execute() does
        assert cursor.rownumber is None


class TestCreateRowIteratorNumpyFlag:
    """Unit tests for _create_row_iterator passing ``connection.config.numpy``.

    The numpy flag now lives on :class:`ConnectionConfig` (sourced from
    ``PARAM_DEFS``); the legacy ``connection._numpy`` attribute has been
    removed.  Cursors read ``self._connection.config.numpy`` and pass it
    through to ``create_row_iterator``.
    """

    @pytest.fixture
    def mock_connection(self):
        conn = MagicMock()
        conn.is_closed.return_value = False
        return conn

    def test_passes_numpy_true_from_connection(self, mock_connection):
        mock_connection.config.numpy = True
        cursor = SnowflakeCursor(mock_connection)
        cursor._result_set = MagicMock(get_arrow_stream_ptr=MagicMock(return_value=42))

        with patch("snowflake.connector.cursor._base.create_row_iterator") as mock_create:
            mock_create.return_value = iter([])
            cursor._create_row_iterator()

        mock_create.assert_called_once_with(
            stream_ptr=42,
            connection=mock_connection,
            use_dict_result=False,
            use_numpy=True,
        )

    def test_passes_numpy_false_from_connection(self, mock_connection):
        mock_connection.config.numpy = False
        cursor = SnowflakeCursor(mock_connection)
        cursor._result_set = MagicMock(get_arrow_stream_ptr=MagicMock(return_value=42))

        with patch("snowflake.connector.cursor._base.create_row_iterator") as mock_create:
            mock_create.return_value = iter([])
            cursor._create_row_iterator()

        mock_create.assert_called_once_with(
            stream_ptr=42,
            connection=mock_connection,
            use_dict_result=False,
            use_numpy=False,
        )


class TestCheckCanUseArrowResultset:
    """Unit tests for SnowflakeCursorBase.check_can_use_arrow_resultset."""

    @pytest.fixture
    def mock_connection(self):
        return MagicMock()

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_no_error_when_pyarrow_installed(self, cursor):
        """check_can_use_arrow_resultset does not raise when pyarrow is available."""
        with patch("snowflake.connector._internal.cursor.base.pyarrow", MagicMock()):
            cursor.check_can_use_arrow_resultset()

    def test_raises_programming_error_when_pyarrow_missing(self, cursor):
        """check_can_use_arrow_resultset raises ProgrammingError when pyarrow is not installed."""
        with patch("snowflake.connector._internal.cursor.base.pyarrow", MissingOptionalDependency(dep="pyarrow")):
            with pytest.raises(ProgrammingError) as excinfo:
                cursor.check_can_use_arrow_resultset()
            assert excinfo.value.errno == ER_NO_PYARROW
            assert "pyarrow" in str(excinfo.value)

    def test_error_message_contains_install_link(self, cursor):
        """The error message includes the documentation link for installation."""
        with patch("snowflake.connector._internal.cursor.base.pyarrow", MissingOptionalDependency(dep="pyarrow")):
            with pytest.raises(ProgrammingError, match="python-connector-pandas"):
                cursor.check_can_use_arrow_resultset()


class TestCheckCanUsePandas:
    """Unit tests for SnowflakeCursorBase.check_can_use_pandas."""

    @pytest.fixture
    def mock_connection(self):
        return MagicMock()

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_no_error_when_pandas_installed(self, cursor):
        """check_can_use_pandas does not raise when pandas is available."""
        with patch("snowflake.connector._internal.cursor.base.pandas", MagicMock()):
            cursor.check_can_use_pandas()

    def test_raises_programming_error_when_pandas_missing(self, cursor):
        """check_can_use_pandas raises ProgrammingError when pandas is not installed."""
        with patch("snowflake.connector._internal.cursor.base.pandas", MissingOptionalDependency(dep="pandas")):
            with pytest.raises(ProgrammingError) as excinfo:
                cursor.check_can_use_pandas()
            assert excinfo.value.errno == ER_NO_PYARROW
            assert "pandas" in str(excinfo.value)

    def test_error_message_contains_install_link(self, cursor):
        """The error message includes the documentation link for installation."""
        with patch("snowflake.connector._internal.cursor.base.pandas", MissingOptionalDependency(dep="pandas")):
            with pytest.raises(ProgrammingError, match="python-connector-pandas"):
                cursor.check_can_use_pandas()


class TestFetchArrowBatches:
    """Unit tests for fetch_arrow_batches."""

    @pytest.fixture
    def mock_connection(self):
        mock_connection = MagicMock()
        mock_connection.is_closed.return_value = False
        return mock_connection

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    @pytest.fixture(autouse=True)
    def _patch_pyarrow(self):
        mock_pa = MagicMock()
        with (
            patch("snowflake.connector._internal.extras.check_dependency"),
            patch("snowflake.connector.cursor._base.pyarrow", new=mock_pa),
        ):
            self.pa = mock_pa
            yield

    def test_yields_tables_from_batches(self, cursor):
        batch1, batch2 = MagicMock(), MagicMock()
        table1, table2 = MagicMock(), MagicMock()
        self.pa.Table.from_batches.side_effect = [table1, table2]
        cursor._result_set = MagicMock(get_arrow_stream_ptr=MagicMock(return_value=42))

        with patch("snowflake.connector.cursor._base.create_table_iterator", return_value=iter([batch1, batch2])):
            tables = list(cursor.fetch_arrow_batches())

        assert tables == [table1, table2]
        self.pa.Table.from_batches.assert_any_call([batch1])
        self.pa.Table.from_batches.assert_any_call([batch2])

    def test_yields_nothing_for_empty_stream(self, cursor):
        cursor._result_set = MagicMock(get_arrow_stream_ptr=MagicMock(return_value=42))

        with patch("snowflake.connector.cursor._base.create_table_iterator", return_value=iter([])):
            tables = list(cursor.fetch_arrow_batches())

        assert tables == []

    def test_raises_when_pyarrow_not_installed(self, cursor):
        missing = MissingOptionalDependency(dep="pyarrow")
        with patch(
            "snowflake.connector._internal.extras.check_dependency",
            side_effect=lambda _: _real_check_dependency(missing),
        ):
            with pytest.raises(ProgrammingError, match="pyarrow"):
                list(cursor.fetch_arrow_batches())

    def test_passes_force_microsecond_precision(self, cursor, mock_connection):
        cursor._result_set = MagicMock(get_arrow_stream_ptr=MagicMock(return_value=42))

        with patch("snowflake.connector.cursor._base.create_table_iterator", return_value=iter([])) as mock_get:
            list(cursor.fetch_arrow_batches(force_microsecond_precision=True))

        mock_get.assert_called_once_with(
            stream_ptr=42, connection=mock_connection, force_microsecond_precision=True, number_to_decimal=ANY
        )


class TestFetchArrowAll:
    """Unit tests for fetch_arrow_all."""

    @pytest.fixture
    def mock_connection(self):
        mock_connection = MagicMock()
        mock_connection.is_closed.return_value = False
        return mock_connection

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    @pytest.fixture(autouse=True)
    def _patch_pyarrow(self):
        mock_pa = MagicMock()
        with (
            patch("snowflake.connector._internal.extras.check_dependency"),
            patch("snowflake.connector.cursor._base.pyarrow", new=mock_pa),
            patch("snowflake.connector._internal.arrow_stream_utils.pyarrow", new=mock_pa),
        ):
            self.pa = mock_pa
            yield

    def test_returns_concatenated_table(self, cursor):
        batch1, batch2 = MagicMock(), MagicMock()
        mock_table = MagicMock()
        self.pa.Table.from_batches.return_value = mock_table
        cursor._result_set = MagicMock(get_arrow_stream_ptr=MagicMock(return_value=42))

        with patch("snowflake.connector.cursor._base.create_table_iterator", return_value=iter([batch1, batch2])):
            result = cursor.fetch_arrow_all()

        assert result is mock_table
        self.pa.Table.from_batches.assert_called_once_with([batch1, batch2])

    def test_returns_none_for_empty_stream(self, cursor):
        mock_iterator = MagicMock()
        mock_iterator.__iter__ = MagicMock(return_value=iter([]))
        cursor._result_set = MagicMock(get_arrow_stream_ptr=MagicMock(return_value=42))

        with patch("snowflake.connector.cursor._base.create_table_iterator", return_value=mock_iterator):
            result = cursor.fetch_arrow_all()

        assert result is None

    def test_returns_empty_table_with_force_return_table(self, cursor):
        mock_empty_table = MagicMock()
        mock_schema = MagicMock()
        mock_schema.empty_table.return_value = mock_empty_table

        mock_iterator = MagicMock()
        mock_iterator.__iter__ = MagicMock(return_value=iter([]))
        mock_iterator.get_converted_schema.return_value = mock_schema
        cursor._result_set = MagicMock(get_arrow_stream_ptr=MagicMock(return_value=42))

        with patch("snowflake.connector.cursor._base.create_table_iterator", return_value=mock_iterator):
            result = cursor.fetch_arrow_all(force_return_table=True)

        assert result is mock_empty_table
        mock_iterator.get_converted_schema.assert_called_once()
        mock_schema.empty_table.assert_called_once()

    def test_returns_none_without_force_return_table(self, cursor):
        cursor._result_set = MagicMock(get_arrow_stream_ptr=MagicMock(return_value=42))

        with patch("snowflake.connector.cursor._base.create_table_iterator", return_value=iter([])):
            result = cursor.fetch_arrow_all(force_return_table=False)

        assert result is None

    def test_passes_force_microsecond_precision(self, cursor, mock_connection):
        cursor._result_set = MagicMock(get_arrow_stream_ptr=MagicMock(return_value=42))

        with patch("snowflake.connector.cursor._base.create_table_iterator", return_value=iter([])) as mock_get:
            cursor.fetch_arrow_all(force_microsecond_precision=True)

        mock_get.assert_called_once_with(
            stream_ptr=42, connection=mock_connection, force_microsecond_precision=True, number_to_decimal=ANY
        )


class TestFetchPandasBatches:
    """Unit tests for fetch_pandas_batches."""

    @pytest.fixture
    def mock_connection(self):
        mock_connection = MagicMock()
        mock_connection.is_closed.return_value = False
        return mock_connection

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    @pytest.fixture(autouse=True)
    def _patch_deps(self):
        with patch("snowflake.connector._internal.extras.check_dependency"):
            yield

    def test_yields_to_pandas_results(self, cursor):
        table1, table2 = MagicMock(), MagicMock()
        df1, df2 = MagicMock(), MagicMock()
        table1.to_pandas.return_value = df1
        table2.to_pandas.return_value = df2

        with patch.object(cursor, "fetch_arrow_batches", return_value=iter([table1, table2])):
            dfs = list(cursor.fetch_pandas_batches())

        assert dfs == [df1, df2]
        table1.to_pandas.assert_called_once()
        table2.to_pandas.assert_called_once()

    def test_raises_when_pandas_not_installed(self, cursor):
        missing = MissingOptionalDependency(dep="pandas")
        with patch(
            "snowflake.connector._internal.extras.check_dependency",
            side_effect=lambda _: _real_check_dependency(missing),
        ):
            with pytest.raises(ProgrammingError, match="pandas"):
                list(cursor.fetch_pandas_batches())


class TestFetchPandasAll:
    """Unit tests for fetch_pandas_all."""

    @pytest.fixture
    def mock_connection(self):
        mock_connection = MagicMock()
        mock_connection.is_closed.return_value = False
        return mock_connection

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    @pytest.fixture(autouse=True)
    def _patch_deps(self):
        with patch("snowflake.connector._internal.extras.check_dependency"):
            yield

    def test_returns_to_pandas_result(self, cursor):
        mock_table = MagicMock()
        mock_df = MagicMock()
        mock_table.to_pandas.return_value = mock_df

        with patch.object(cursor, "fetch_arrow_all", return_value=mock_table):
            result = cursor.fetch_pandas_all()

        assert result is mock_df
        mock_table.to_pandas.assert_called_once()

    def test_returns_empty_dataframe_for_empty_stream(self, cursor):
        mock_empty_table = MagicMock()
        mock_empty_df = MagicMock()
        mock_empty_table.to_pandas.return_value = mock_empty_df

        with patch.object(cursor, "fetch_arrow_all", return_value=mock_empty_table) as mock_fetch:
            result = cursor.fetch_pandas_all()

        assert result is mock_empty_df
        mock_fetch.assert_called_once_with(force_return_table=True)
        mock_empty_table.to_pandas.assert_called_once()

    def test_raises_when_pandas_not_installed(self, cursor):
        missing = MissingOptionalDependency(dep="pandas")
        with patch(
            "snowflake.connector._internal.extras.check_dependency",
            side_effect=lambda _: _real_check_dependency(missing),
        ):
            with pytest.raises(ProgrammingError, match="pandas"):
                cursor.fetch_pandas_all()

    def test_forwards_kwargs_to_fetch_arrow_all(self, cursor):
        mock_table = MagicMock()
        with patch.object(cursor, "fetch_arrow_all", return_value=mock_table) as mock_fetch:
            cursor.fetch_pandas_all(force_microsecond_precision=True)

        mock_fetch.assert_called_once_with(force_return_table=True, force_microsecond_precision=True)


class TestReset:
    """Unit tests for Cursor.reset method."""

    @pytest.fixture
    def mock_connection(self):
        return MagicMock()

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_reset_clears_all_state_together(self, cursor):
        """reset() frees heavy result data but preserves lightweight metadata."""
        mock_desc = [MagicMock()]
        cursor._query_result = QueryResult(
            description=mock_desc,
            sqlstate="42601",
            sfqid="abc-123",
            query="SELECT 1",
            rowcount=100,
        )
        cursor._iterator = iter([(1,)])
        cursor._binding_data = b"data"
        cursor._rownumber = 10

        cursor.reset()

        # Cleared by reset
        assert cursor._iterator is None
        assert cursor._binding_data is None
        assert cursor._query_result.rowcount is None
        # Preserved by reset (matches old driver)
        assert cursor._rownumber == 10
        assert cursor._query_result.description is mock_desc
        assert cursor._query_result.sqlstate == "42601"
        assert cursor._query_result.sfqid == "abc-123"
        assert cursor._query_result.query == "SELECT 1"

    def test_reset_is_idempotent(self, cursor):
        """Calling reset() twice produces the same state as calling it once."""
        cursor._query_result = QueryResult(rowcount=42)
        cursor._iterator = iter([(1,)])

        cursor.reset()
        cursor.reset()

        assert cursor._iterator is None
        assert cursor._query_result.rowcount is None
        assert cursor._rownumber == -1

    def test_reset_on_fresh_cursor_is_noop(self, cursor):
        """reset() on a freshly created cursor doesn't break anything."""
        cursor.reset()

        assert cursor._iterator is None
        assert cursor.sqlstate is None
        assert cursor._binding_data is None
        assert cursor._rownumber == -1
        assert cursor.rowcount is None

    def test_reset_closing_true_clears_everything_except_rowcount(self, cursor):
        """reset(closing=True) preserves rowcount in addition to the usual preserved fields."""
        mock_desc = [MagicMock()]
        cursor._query_result = QueryResult(
            description=mock_desc,
            sqlstate="42601",
            sfqid="abc-123",
            query="SELECT 1",
            rowcount=100,
        )
        cursor._iterator = iter([(1,)])
        cursor._binding_data = b"data"
        cursor._rownumber = 10

        cursor.reset(closing=True)

        # Cleared by reset
        assert cursor._iterator is None
        assert cursor._binding_data is None
        # Preserved by reset (always)
        assert cursor._rownumber == 10
        assert cursor._query_result.description is mock_desc
        assert cursor._query_result.sqlstate == "42601"
        assert cursor._query_result.sfqid == "abc-123"
        assert cursor._query_result.query == "SELECT 1"
        # Preserved by reset(closing=True) specifically
        assert cursor._query_result.rowcount == 100

    def test_reset_preserves_query_and_sfqid(self, cursor):
        """After reset(), query and sfqid are preserved (matches old driver)."""
        cursor._query_result.sfqid = "abc-123"
        cursor._query_result.query = "SELECT 1"

        assert cursor.query == "SELECT 1"
        assert cursor.sfqid == "abc-123"

        cursor.reset()

        assert cursor.query == "SELECT 1"
        assert cursor.sfqid == "abc-123"


class TestClose:
    """Unit tests for Cursor.close method."""

    @pytest.fixture
    def mock_connection(self):
        return MagicMock()

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_close_returns_true_on_success(self, cursor):
        """close() returns True when the cursor was open and is now closed."""
        assert cursor.close() is True

    def test_close_returns_false_when_already_closed(self, cursor):
        """close() returns False when the cursor was already closed."""
        cursor.close()
        assert cursor.close() is False

    def test_close_sets_closed_flag(self, cursor):
        """close() sets _closed to True."""
        cursor.close()
        assert cursor._closed is True

    def test_close_clears_messages(self, cursor):
        """close() empties the messages list."""
        cursor._messages.append((ProgrammingError, {"msg": "test"}))
        cursor.close()
        assert cursor._messages == []

    def test_close_preserves_rowcount(self, cursor):
        """close() preserves _rowcount via reset(closing=True)."""
        cursor._query_result.rowcount = 42
        cursor.close()
        assert cursor._query_result.rowcount == 42

    def test_close_clears_result_state(self, cursor):
        """close() clears result-related state via reset (except description)."""
        mock_desc = [MagicMock()]
        cursor._query_result = QueryResult(description=mock_desc)
        cursor._iterator = iter([(1,)])

        cursor.close()

        assert cursor._iterator is None
        assert cursor._query_result.description is mock_desc

    def test_close_returns_none_on_exception(self, cursor):
        """close() returns None when reset() raises an exception."""
        with patch.object(cursor, "reset", side_effect=RuntimeError("boom")):
            assert cursor.close() is None

    def test_close_exception_leaves_cursor_unclosed(self, cursor):
        """When close() fails, the cursor stays open so the caller can retry or clean up."""
        original_conn = cursor._connection
        with patch.object(cursor, "reset", side_effect=RuntimeError("boom")):
            cursor.close()

        assert cursor._closed is False
        assert cursor._connection is original_conn

    def test_close_via_context_manager(self, mock_connection):
        """Exiting a context manager calls close()."""
        with SnowflakeCursor(mock_connection) as cur:
            assert not cur._closed
        assert cur._closed is True


class TestResetIntegration:
    """Integration tests for reset() with other cursor methods."""

    @pytest.fixture
    def mock_connection(self, mock_core_client):
        conn = MagicMock()
        conn.conn_handle = ConnectionHandle(id=1)
        conn.is_closed.return_value = False
        mock_core_client.statement_new.return_value.stmt_handle = StatementHandle(id=1)
        execute_result = MagicMock()
        execute_result.columns = []
        execute_result.HasField = MagicMock(return_value=False)
        execute_result.sql_state = "00000"
        mock_core_client.statement_execute_query.return_value.result = execute_result
        return conn

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_close_calls_reset_with_closing_true(self, cursor):
        """close() calls reset(closing=True) to preserve rowcount."""
        cursor._query_result = QueryResult(rowcount=42)
        cursor._iterator = iter([(1,)])

        cursor.close()

        # Rowcount should be preserved
        assert cursor._query_result.rowcount == 42
        # Other state should be cleared
        assert cursor._iterator is None
        assert cursor._closed is True

    def test_execute_calls_reset_before_executing(self, cursor, mock_connection):
        """execute() calls reset() before executing to clear old state."""
        cursor._query_result = QueryResult(
            description=[MagicMock()],
            rowcount=100,
        )
        cursor._iterator = iter([(1,)])

        cursor.execute("SELECT 1")

        # Old state should have been cleared by reset()
        assert cursor._iterator is None
        assert cursor._binding_data is None

    def test_executemany_calls_reset_once_before_loop(self, cursor, mock_connection):
        """executemany() calls reset() once before the loop, not for each execute."""
        mock_connection.paramstyle = ParamStyle.PYFORMAT
        cursor._query_result.rowcount = 100

        with patch.object(cursor, "reset") as mock_reset:
            with patch.object(cursor, "_execute") as mock_execute:
                mock_execute.return_value = cursor
                cursor._query_result.rowcount = 1
                cursor.executemany("INSERT INTO t VALUES (%s)", [(1,), (2,), (3,)])

        # reset should be called once, not 3 times
        mock_reset.assert_called_once()
        # _execute should be called 3 times
        assert mock_execute.call_count == 3

    def test_execute_overwrites_sqlstate_with_new_result(self, cursor, mock_connection):
        """execute() overwrites sqlstate from the new query result."""
        cursor._query_result.sqlstate = "42601"

        cursor.execute("SELECT 1")

        assert cursor.sqlstate is None

    def test_execute_resets_description_before_new_query(self, cursor, mock_connection):
        """execute() clears old description; new one is populated from the result."""
        cursor._query_result.description = [MagicMock()]

        cursor.execute("SELECT 1")

        # Mock has no columns, so description becomes None
        assert cursor.description is None

    def test_executemany_server_side_binding_delegates_reset_to_execute(self, cursor, mock_connection):
        """executemany() with server-side (qmark) binding delegates to execute(), which performs its own reset."""
        mock_connection.paramstyle = ParamStyle.QMARK
        cursor._query_result.sqlstate = "42601"

        cursor.executemany("INSERT INTO t VALUES (?)", [(1,), (2,), (3,)])

        assert cursor.sqlstate is None

    def test_executemany_empty_params_does_not_reset(self, cursor, mock_connection):
        """executemany() with empty seq_of_parameters returns early without calling reset."""
        cursor._query_result = QueryResult(rowcount=42)

        cursor.executemany("INSERT INTO t VALUES (?)", [])

        assert cursor._query_result.rowcount == 42


class TestDescribe:
    """Unit tests for Cursor.describe method."""

    @pytest.fixture
    def mock_connection(self, mock_core_client):
        conn = MagicMock()
        conn.conn_handle = ConnectionHandle(id=1)
        conn.is_closed.return_value = False
        mock_core_client.statement_new.return_value.stmt_handle = StatementHandle(id=1)
        return conn

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def _setup_prepare(self, mock_core_client, columns=None, query_id="", query="", sql_state=None):
        result = MagicMock()
        result.columns = columns or []
        result.stream.value = (42).to_bytes(8, byteorder="little", signed=False)
        result.query_id = query_id
        result.query = query
        result.sql_state = sql_state
        mock_core_client.statement_prepare.return_value.result = result
        return result

    def test_describe_returns_column_metadata(self, cursor, mock_core_client):
        """describe() returns ResultMetadata and updates cursor.description."""
        col = MagicMock(type="FIXED", nullable=True, precision=10, scale=0)
        col.name = "COL1"
        col.HasField = lambda f: f in ("precision", "scale")
        self._setup_prepare(mock_core_client, columns=[col])

        result = cursor.describe("SELECT 1 AS COL1")

        assert result is not None
        assert len(result) == 1
        assert result[0].name == "COL1"
        assert cursor.description == result

    def test_describe_returns_none_for_no_columns(self, cursor, mock_core_client):
        """describe() returns None when the statement produces no result set."""
        self._setup_prepare(mock_core_client, columns=[])

        assert cursor.describe("INSERT INTO t VALUES (1)") is None

    def test_describe_side_effects_with_columns(self, cursor, mock_core_client):
        """describe() sets sfqid, query, sqlstate, rowcount when result has columns."""
        col = MagicMock(type="FIXED", nullable=True, precision=10, scale=0)
        col.name = "COL1"
        col.HasField = lambda f: f in ("precision", "scale")
        self._setup_prepare(
            mock_core_client,
            columns=[col],
            query_id="01abc-def",
            query="SELECT 1",
            sql_state="00000",
        )

        cursor._query_result.rowcount = 42

        cursor.describe("SELECT 1")

        assert cursor.sfqid == "01abc-def"
        assert cursor.query == "SELECT 1"
        assert cursor.sqlstate is None  # "00000" is normalized to None
        assert cursor.rowcount == 0

    def test_describe_forwards_non_success_sqlstate(self, cursor, mock_core_client):
        """describe() forwards sqlstate when it differs from '00000'."""
        col = MagicMock(type="FIXED", nullable=True, precision=10, scale=0)
        col.name = "COL1"
        col.HasField = lambda f: f in ("precision", "scale")
        self._setup_prepare(
            mock_core_client,
            columns=[col],
            sql_state="02000",
        )

        cursor.describe("SELECT 1")

        assert cursor.sqlstate == "02000"

    def test_describe_side_effects_without_columns(self, cursor, mock_core_client):
        """describe() resets state; sfqid/query/sqlstate are set from result even without columns."""
        cursor._query_result.rowcount = 42
        self._setup_prepare(mock_core_client)

        cursor.describe("SELECT 1")

        assert cursor.sfqid is None
        assert cursor.rowcount is None

    def test_describe_releases_handle_and_stream(self, cursor, mock_core_client):
        """describe() allocates/releases statement handle and releases the arrow stream."""
        self._setup_prepare(mock_core_client)

        with patch("snowflake.connector._internal.cursor.query_result.release_arrow_stream") as mock_release:
            cursor.describe("SELECT 1")

        mock_core_client.statement_new.assert_called_once()
        mock_core_client.statement_release.assert_called_once()
        mock_release.assert_called()

    def test_describe_raises_when_closed(self, cursor, mock_connection):
        """describe() raises InterfaceError on closed cursor or connection."""
        cursor.close()
        with pytest.raises(InterfaceError):
            cursor.describe("SELECT 1")

        fresh = SnowflakeCursor(mock_connection)
        mock_connection.is_closed.return_value = True
        with pytest.raises(InterfaceError):
            fresh.describe("SELECT 1")

    def test_describe_propagates_prepare_error(self, cursor, mock_core_client):
        """describe() propagates ProgrammingError and captures sqlstate."""
        mock_core_client.statement_prepare.side_effect = ProgrammingError("syntax error", sqlstate="42601")

        with pytest.raises(ProgrammingError):
            cursor.describe("INVALID SQL")

        assert cursor.sqlstate == "42601"


class TestQueryResult:
    """Unit tests for Cursor.query_result method."""

    @pytest.fixture
    def mock_connection(self, mock_core_client):
        conn = MagicMock()
        conn.conn_handle = ConnectionHandle(id=1)
        conn.is_closed.return_value = False
        return conn

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def _stub_result(self, mock_core_client, **overrides):
        """Set up the mock RPC to return a result with the given overrides."""
        # Mock ResultSetDescriptor
        descriptor = MagicMock()
        descriptor.query_id = overrides.get("query_id", "test-query-id")
        descriptor.columns = overrides.get("columns", [])
        descriptor.rows_affected = overrides.get("rows_affected", 0)
        descriptor.statement_type_id = overrides.get("statement_type_id", 0x0000)  # Default to UNKNOWN

        # Mock HasField to handle different field types
        def has_field_impl(field_name):
            if field_name == "rows_affected":
                return overrides.get("has_rows_affected", False)
            elif field_name == "sql_state":
                return bool(overrides.get("sql_state", ""))
            elif field_name == "stats":
                return overrides.get("has_stats", False)
            elif field_name == "statement_type_id":
                return "statement_type_id" in overrides
            return False

        descriptor.HasField = MagicMock(side_effect=has_field_impl)
        descriptor.sql_state = overrides.get("sql_state", "")
        descriptor.stats = overrides.get("stats", None)

        # Mock ResultSetResponse wrapping the descriptor
        result_set_response = MagicMock()
        result_set_response.result_descriptor = descriptor
        result_set_response.result_set_handle = ResultSetHandle(id=99)

        # Mock ConnectionGetQueryResultResponse with single statement
        query_result_response = MagicMock()
        query_result_response.single = result_set_response
        query_result_response.HasField = MagicMock(side_effect=lambda f: f == "single")
        mock_core_client.connection_get_query_result.return_value = query_result_response

        return descriptor

    def test_query_result_populates_cursor_state(self, cursor, mock_core_client):
        """query_result returns self, sends correct RPC args, and populates all cursor fields."""
        col = MagicMock()
        col.name = "ID"
        col.HasField = MagicMock(return_value=False)
        col.nullable = True

        self._stub_result(
            mock_core_client,
            columns=[col],
            rows_affected=42,
            has_rows_affected=True,
            sql_state="02000",
        )

        ret = cursor.query_result("01234567-abcd-ef01-0000-000000000001")

        assert ret is cursor
        assert cursor.description is not None
        assert len(cursor.description) == 1
        assert cursor.description[0].name == "ID"
        assert cursor.rowcount == 42
        assert cursor.sqlstate == "02000"

        request = mock_core_client.connection_get_query_result.call_args.args[0]
        assert request.conn_handle == ConnectionHandle(id=1)
        assert request.query_id == "01234567-abcd-ef01-0000-000000000001"

    def test_query_result_resets_prior_state(self, cursor, mock_core_client):
        """query_result clears iterator from a previous execute."""
        cursor._query_result = QueryResult(rowcount=99)
        cursor._iterator = iter([(1,)])

        self._stub_result(mock_core_client)
        cursor.query_result("qid")

        assert cursor._iterator is None

    def test_query_result_raises_on_closed_cursor_or_connection(self, cursor, mock_connection):
        """query_result raises InterfaceError when cursor or connection is closed."""
        cursor.close()
        with pytest.raises(InterfaceError):
            cursor.query_result("qid")

        fresh = SnowflakeCursor(mock_connection)
        mock_connection.is_closed.return_value = True
        with pytest.raises(InterfaceError):
            fresh.query_result("qid")

    def test_query_result_propagates_rpc_error(self, cursor, mock_core_client):
        """query_result propagates ProgrammingError from the RPC layer."""
        mock_core_client.connection_get_query_result.side_effect = ProgrammingError("Query has expired")
        with pytest.raises(ProgrammingError, match="Query has expired"):
            cursor.query_result("expired-qid")


class TestQueryResultWaiter:
    """Unit tests for QueryResultWaiter."""

    @pytest.fixture
    def mock_connection(self):
        conn = MagicMock()
        conn.is_still_running = MagicMock(side_effect=lambda s: s in (QueryStatus.RUNNING, QueryStatus.NO_DATA))
        return conn

    def test_returns_immediately_when_query_already_done(self, mock_connection):
        """wait() returns without sleeping when query status is SUCCESS."""
        mock_connection.get_query_status_throw_if_error.return_value = QueryStatus.SUCCESS
        waiter = QueryResultWaiter(mock_connection, "qid")

        with patch("snowflake.connector._internal.cursor.query_result_waiter.time.sleep") as mock_sleep:
            waiter.wait()

        mock_sleep.assert_not_called()

    def test_polls_until_success(self, mock_connection):
        """wait() polls with backoff until the query finishes."""
        mock_connection.get_query_status_throw_if_error.side_effect = [
            QueryStatus.RUNNING,
            QueryStatus.RUNNING,
            QueryStatus.SUCCESS,
        ]
        waiter = QueryResultWaiter(mock_connection, "qid")

        with patch("snowflake.connector._internal.cursor.query_result_waiter.time.sleep") as mock_sleep:
            waiter.wait()

        assert mock_connection.get_query_status_throw_if_error.call_count == 3
        assert mock_sleep.call_count == 2

    def test_raises_on_error_status(self, mock_connection):
        """wait() propagates ProgrammingError from get_query_status_throw_if_error."""
        mock_connection.get_query_status_throw_if_error.side_effect = ProgrammingError("Query failed")
        waiter = QueryResultWaiter(mock_connection, "qid")

        with patch("snowflake.connector._internal.cursor.query_result_waiter.time.sleep"):
            with pytest.raises(ProgrammingError, match="Query failed"):
                waiter.wait()

    def test_raises_after_no_data_max_retry(self, mock_connection):
        """wait() raises DatabaseError after too many NO_DATA responses."""
        mock_connection.get_query_status_throw_if_error.return_value = QueryStatus.NO_DATA
        waiter = QueryResultWaiter(mock_connection, "qid")

        with patch("snowflake.connector._internal.cursor.query_result_waiter.time.sleep"):
            with pytest.raises(DatabaseError, match="Cannot retrieve data"):
                waiter.wait()


class TestGetResultsFromSfqid:
    """Unit tests for Cursor.get_results_from_sfqid."""

    @pytest.fixture
    def mock_connection(self, mock_core_client):
        conn = MagicMock()
        conn.conn_handle = ConnectionHandle(id=1)
        conn.is_closed.return_value = False
        conn.get_query_status_throw_if_error.return_value = QueryStatus.SUCCESS
        conn.is_still_running.return_value = False
        result = MagicMock()
        result.columns = []
        result.HasField = MagicMock(return_value=False)
        result.sql_state = ""
        mock_core_client.connection_get_query_result.return_value.result = result
        return conn

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_sets_sfqid_eagerly(self, cursor):
        """get_results_from_sfqid sets sfqid immediately, before any fetch."""
        cursor.get_results_from_sfqid("test-qid")

        assert cursor.sfqid == "test-qid"

    def test_installs_prefetch_hook(self, cursor):
        """get_results_from_sfqid installs a prefetch hook that fires on fetch."""
        cursor.get_results_from_sfqid("test-qid")

        assert cursor._prefetch_hook is not None

    def test_prefetch_hook_fires_on_fetch(self, cursor, mock_connection):
        """First fetch triggers the hook, which calls query_result."""
        with patch("snowflake.connector._internal.cursor.query_result_waiter.time.sleep"):
            cursor.get_results_from_sfqid("test-qid")

        assert cursor._prefetch_hook is not None
        with patch.object(cursor, "query_result") as mock_qr:
            cursor._prefetch_hook()

        mock_qr.assert_called_once_with("test-qid")
        assert cursor._prefetch_hook is None

    def test_raises_on_closed_cursor(self, cursor):
        """get_results_from_sfqid raises InterfaceError when cursor is closed."""
        cursor.close()

        with pytest.raises(InterfaceError):
            cursor.get_results_from_sfqid("qid")

    def test_raises_when_query_already_failed(self, cursor, mock_connection):
        """get_results_from_sfqid raises immediately if status check returns error."""
        mock_connection.get_query_status_throw_if_error.side_effect = ProgrammingError("Query failed")

        with pytest.raises(ProgrammingError, match="Query failed"):
            cursor.get_results_from_sfqid("bad-qid")

    def test_execute_clears_pending_hook(self, cursor, mock_core_client):
        """A new execute() cancels a pending prefetch hook."""
        cursor.get_results_from_sfqid("test-qid")
        assert cursor._prefetch_hook is not None

        handle_resp = MagicMock()
        handle_resp.stmt_handle = StatementHandle(id=1)
        mock_core_client.statement_new.return_value = handle_resp
        result = MagicMock()
        result.columns = []
        result.HasField = MagicMock(return_value=False)
        result.sql_state = ""
        mock_core_client.statement_execute_query.return_value.result = result
        cursor.execute("SELECT 1")

        assert cursor._prefetch_hook is None


class TestAbortQuery:
    """Unit tests for Cursor.abort_query method."""

    @pytest.fixture
    def mock_connection(self, mock_core_client):
        conn = MagicMock()
        conn.conn_handle = ConnectionHandle(id=1)
        conn.is_closed.return_value = False
        return conn

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_abort_query_returns_true_on_success(self, cursor, mock_core_client):
        """abort_query sends correct RPC args and returns True on success."""
        mock_core_client.connection_abort_query.return_value.success = True

        result = cursor.abort_query("01234567-abcd-ef01-0000-000000000001")

        assert result is True
        request = mock_core_client.connection_abort_query.call_args.args[0]
        assert request.conn_handle == ConnectionHandle(id=1)
        assert request.query_id == "01234567-abcd-ef01-0000-000000000001"

    def test_abort_query_returns_false_on_failure(self, cursor, mock_core_client):
        """abort_query returns False when the server reports failure."""
        mock_core_client.connection_abort_query.return_value.success = False

        result = cursor.abort_query("some-qid")

        assert result is False

    def test_abort_query_does_not_mutate_cursor_state(self, cursor, mock_core_client):
        """abort_query does not modify description, rowcount, or execute_result."""
        mock_core_client.connection_abort_query.return_value.success = True

        cursor.abort_query("some-qid")

        assert cursor.description is None
        assert cursor.rowcount is None

    def test_abort_query_raises_on_closed_cursor_or_connection(self, cursor, mock_connection):
        """abort_query raises InterfaceError when cursor or connection is closed."""
        cursor.close()
        with pytest.raises(InterfaceError):
            cursor.abort_query("qid")

        fresh = SnowflakeCursor(mock_connection)
        mock_connection.is_closed.return_value = True
        with pytest.raises(InterfaceError):
            fresh.abort_query("qid")

    def test_abort_query_propagates_rpc_error(self, cursor, mock_core_client):
        """abort_query propagates ProgrammingError from the RPC layer."""
        mock_core_client.connection_abort_query.side_effect = ProgrammingError("Request failed")
        with pytest.raises(ProgrammingError, match="Request failed"):
            cursor.abort_query("bad-qid")


class TestCursorFormatQueryForLog:
    """Unit tests for cursor._format_query_for_log delegation to connection."""

    @pytest.fixture
    def mock_connection(self):
        mock_connection = MagicMock()
        mock_connection.is_closed.return_value = False
        mock_connection._format_query_for_log.return_value = "formatted"
        return mock_connection

    def test_delegates_to_connection(self, mock_connection):
        cursor = SnowflakeCursor(mock_connection)
        result = cursor._format_query_for_log("SELECT * FROM big_table")
        mock_connection._format_query_for_log.assert_called_once_with("SELECT * FROM big_table")
        assert result == "formatted"


class TestExecuteAsync:
    """Unit tests for Cursor.execute_async method."""

    @pytest.fixture
    def mock_connection(self, mock_core_client):
        conn = MagicMock()
        conn.conn_handle = ConnectionHandle(id=1)
        conn.is_closed.return_value = False
        conn.paramstyle = ParamStyle.PYFORMAT

        handle_resp = MagicMock()
        handle_resp.stmt_handle = StatementHandle(id=42)
        mock_core_client.statement_new.return_value = handle_resp

        async_resp = MagicMock()
        async_resp.query_id = "01abc-fake-query-id"
        mock_core_client.statement_execute_async.return_value = async_resp

        return conn

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def test_returns_dict_with_query_id(self, cursor):
        """execute_async returns a dict containing the queryId."""
        result = cursor.execute_async("SELECT 1")

        assert isinstance(result, dict)
        assert "queryId" in result
        assert result["queryId"] == "01abc-fake-query-id"

    def test_sets_sfqid_on_cursor(self, cursor):
        """execute_async sets sfqid so downstream callers can reference it."""
        cursor.execute_async("SELECT 1")

        assert cursor.sfqid == "01abc-fake-query-id"

    def test_calls_statement_execute_async_rpc(self, cursor, mock_core_client):
        """execute_async creates a statement and invokes the async RPC."""
        cursor.execute_async("SELECT 42")

        mock_core_client.statement_new.assert_called_once()
        mock_core_client.statement_set_sql_query.assert_called_once()
        mock_core_client.statement_execute_async.assert_called_once()
        mock_core_client.statement_release.assert_called_once()

    def test_resets_cursor_state(self, cursor):
        """execute_async resets cursor state before submission."""
        cursor._iterator = iter([(1,)])
        cursor.execute_async("SELECT 1")

        assert cursor._iterator is None

    def test_with_parameters_passes_bindings(self, cursor, mock_core_client):
        """execute_async forwards parameter bindings to the RPC request."""
        cursor._connection.paramstyle = ParamStyle.QMARK

        cursor.execute_async("SELECT ?", [42])

        request = mock_core_client.statement_execute_async.call_args.args[0]
        assert request.bindings is not None

    def test_raises_on_closed_cursor(self, cursor):
        """execute_async raises InterfaceError when cursor is closed."""
        cursor.close()

        with pytest.raises(InterfaceError):
            cursor.execute_async("SELECT 1")

    def test_propagates_rpc_error(self, cursor, mock_core_client):
        """execute_async propagates errors from the RPC layer."""
        mock_core_client.statement_execute_async.side_effect = ProgrammingError("Async submission failed")

        with pytest.raises(ProgrammingError, match="Async submission failed"):
            cursor.execute_async("SELECT 1")

    def test_handles_empty_query_id(self, cursor, mock_core_client):
        """execute_async returns None queryId when server returns empty string."""
        mock_core_client.statement_execute_async.return_value.query_id = ""

        result = cursor.execute_async("SELECT 1")

        assert result["queryId"] is None


class TestExecuteSkipUploadOnContentMatch:
    """`_skip_upload_on_content_match` is a private execute() kwarg routed
    through the per-call channel: never lands in `_statement_parameters`,
    structurally cannot bleed across calls.
    """

    @pytest.fixture
    def mock_connection(self):
        mock_connection = MagicMock()
        mock_connection.is_closed.return_value = False
        return mock_connection

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    @staticmethod
    def _capture_per_call(cursor):
        """Capture the `statement_parameters` kwarg at the moment `_execute` runs."""
        captured = {}

        def side_effect(*args, **kwargs):
            captured.update(kwargs.get("statement_parameters") or {})
            return MagicMock()

        return captured, side_effect

    def test_kwarg_true_passed_as_per_call_param(self, cursor):
        captured, side_effect = self._capture_per_call(cursor)
        with (
            patch.object(cursor, "_execute", side_effect=side_effect),
            patch.object(cursor, "reset"),
        ):
            cursor.execute("PUT file://x @s", _skip_upload_on_content_match=True)
        assert captured.get(StatementParameterName.SKIP_UPLOAD_ON_CONTENT_MATCH) is True
        # Structural guarantee: never persisted on the cursor.
        assert StatementParameterName.SKIP_UPLOAD_ON_CONTENT_MATCH not in cursor._statement_parameters

    def test_kwarg_default_passes_nothing(self, cursor):
        captured, side_effect = self._capture_per_call(cursor)
        with (
            patch.object(cursor, "_execute", side_effect=side_effect),
            patch.object(cursor, "reset"),
        ):
            cursor.execute("PUT file://x @s")
        assert StatementParameterName.SKIP_UPLOAD_ON_CONTENT_MATCH not in captured

    def test_kwarg_explicit_false_passes_nothing(self, cursor):
        # Explicit False is identical to omitting the kwarg.
        captured, side_effect = self._capture_per_call(cursor)
        with (
            patch.object(cursor, "_execute", side_effect=side_effect),
            patch.object(cursor, "reset"),
        ):
            cursor.execute("PUT file://x @s", _skip_upload_on_content_match=False)
        assert StatementParameterName.SKIP_UPLOAD_ON_CONTENT_MATCH not in captured

    def test_kwarg_does_not_persist_across_calls(self, cursor):
        # Per-call BC regression guard. With the statement_parameters channel,
        # cross-call bleed is structurally impossible.
        with (
            patch.object(cursor, "_execute"),
            patch.object(cursor, "reset"),
        ):
            cursor.execute("PUT file://a @s", _skip_upload_on_content_match=True)

        captured, side_effect = self._capture_per_call(cursor)
        with (
            patch.object(cursor, "_execute", side_effect=side_effect),
            patch.object(cursor, "reset"),
        ):
            cursor.execute("PUT file://b @s")
        assert StatementParameterName.SKIP_UPLOAD_ON_CONTENT_MATCH not in captured, (
            "second call must NOT inherit True from the first call"
        )

    def test_per_call_wins_over_sticky_on_key_collision(self, cursor):
        # Pin the merge order in `_apply_statement_parameters`: per-call
        # values override sticky values when the same key is set in both.
        # A future "tidy up" that reverses the spread direction
        # ({**per_call, **sticky}) fails this test.
        captured_options = {}

        def capture_options(*, stmt_handle, options):
            captured_options.update(options)

        cursor._statement_parameters[StatementParameterName.SKIP_UPLOAD_ON_CONTENT_MATCH] = False
        with (
            patch("snowflake.connector.cursor._base.core_driver.statement_set_options", side_effect=capture_options),
            patch("snowflake.connector.cursor._base.statement"),
            patch.object(cursor, "_execute_query", return_value=MagicMock(HasField=lambda f: False)),
            patch.object(cursor, "_apply_result_set"),
            patch.object(cursor, "reset"),
            patch.object(cursor, "_prepare_query", return_value=("PUT", None)),
        ):
            cursor.execute("PUT file://x @s", _skip_upload_on_content_match=True)

        # The merged options sent to core must reflect the per-call value (True),
        # not the sticky one (False).
        setting = captured_options.get(StatementParameterName.SKIP_UPLOAD_ON_CONTENT_MATCH)
        assert setting is not None, "skip_upload_on_content_match must be in options"
        assert setting.bool_value is True, (
            f"per-call True must override sticky False; got bool_value={setting.bool_value}"
        )


class TestExecuteNumStatements:
    """`num_statements` is a per-call execute() kwarg: a value on call N
    must not bleed into call N+1.
    """

    @pytest.fixture
    def mock_connection(self):
        mock_connection = MagicMock()
        mock_connection.is_closed.return_value = False
        return mock_connection

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    @staticmethod
    def _capture_per_call(cursor):
        captured = {}

        def side_effect(*args, **kwargs):
            captured.update(kwargs.get("statement_parameters") or {})
            return MagicMock()

        return captured, side_effect

    def test_num_statements_passed_as_per_call_param(self, cursor):
        captured, side_effect = self._capture_per_call(cursor)
        with (
            patch.object(cursor, "_execute", side_effect=side_effect),
            patch.object(cursor, "reset"),
        ):
            cursor.execute("SELECT 1; SELECT 2; SELECT 3", num_statements=3)
        assert captured.get(StatementParameterName.MULTI_STATEMENT_COUNT) == 3
        assert StatementParameterName.MULTI_STATEMENT_COUNT not in cursor._statement_parameters

    def test_num_statements_default_passes_nothing(self, cursor):
        captured, side_effect = self._capture_per_call(cursor)
        with (
            patch.object(cursor, "_execute", side_effect=side_effect),
            patch.object(cursor, "reset"),
        ):
            cursor.execute("SELECT 1")
        assert StatementParameterName.MULTI_STATEMENT_COUNT not in captured

    def test_num_statements_does_not_persist_across_calls(self, cursor):
        # The BC-restoring regression guard. Captures what actually reaches
        # `core_driver.statement_set_options` (i.e., the merged options),
        # not just `statement_parameters` — a sticky bleed would manifest in
        # the merged dict on the second call even with `statement_parameters`
        # empty.
        # The legacy Python connector builds its statement-params dict locally
        # per execute() call and never stores it on the cursor instance, so a
        # second call without `num_statements` runs as a single statement.
        # This test pins that behaviour.
        with (
            patch.object(cursor, "_execute"),
            patch.object(cursor, "reset"),
        ):
            cursor.execute("SELECT 1; SELECT 2; SELECT 3", num_statements=3)

        # Drive the second call all the way through `_apply_statement_parameters`
        # so we can observe what it would forward to the core.
        captured_options: dict = {}

        def capture_options(*, stmt_handle, options):
            captured_options.update(options)

        with (
            patch(
                "snowflake.connector.cursor._base.core_driver.statement_set_options",
                side_effect=capture_options,
            ),
            patch("snowflake.connector.cursor._base.statement"),
            patch.object(
                cursor,
                "_execute_query",
                return_value=MagicMock(HasField=lambda f: False),
            ),
            patch.object(cursor, "_apply_result_set"),
            patch.object(cursor, "reset"),
            patch.object(cursor, "_prepare_query", return_value=("SELECT 1", None)),
        ):
            cursor.execute("SELECT 1")

        assert StatementParameterName.MULTI_STATEMENT_COUNT not in captured_options, (
            f"second call must NOT inherit MULTI_STATEMENT_COUNT=3 from the first call; got options={captured_options}"
        )


class TestStatementParameterCollection:
    """The per-call collection helper and the merged options builder that the
    sync and async cursors share on the cursor base mixin."""

    @pytest.fixture
    def cursor(self):
        mock_connection = MagicMock()
        mock_connection.is_closed.return_value = False
        return SnowflakeCursor(mock_connection)

    def test_collect_skip_upload_true(self, cursor):
        params = cursor._collect_statement_params(skip_upload_on_content_match=True)
        assert params == {StatementParameterName.SKIP_UPLOAD_ON_CONTENT_MATCH: True}

    def test_collect_default_is_empty(self, cursor):
        assert cursor._collect_statement_params(skip_upload_on_content_match=False) == {}

    def test_collect_num_statements(self, cursor):
        params = cursor._collect_statement_params(skip_upload_on_content_match=False, num_statements=3)
        assert params == {StatementParameterName.MULTI_STATEMENT_COUNT: 3}

    def test_collect_both(self, cursor):
        params = cursor._collect_statement_params(skip_upload_on_content_match=True, num_statements=2)
        assert params == {
            StatementParameterName.SKIP_UPLOAD_ON_CONTENT_MATCH: True,
            StatementParameterName.MULTI_STATEMENT_COUNT: 2,
        }

    def test_build_options_per_call_overrides_sticky(self, cursor):
        # Sticky False + per-call True for the same key: per-call must win.
        # A future reversal of the spread order ({**per_call, **sticky}) fails here.
        cursor._statement_parameters[StatementParameterName.SKIP_UPLOAD_ON_CONTENT_MATCH] = False
        options = cursor._build_statement_parameters_options(
            {StatementParameterName.SKIP_UPLOAD_ON_CONTENT_MATCH: True}
        )
        setting = options.get(StatementParameterName.SKIP_UPLOAD_ON_CONTENT_MATCH)
        assert setting is not None
        assert setting.bool_value is True

    def test_build_options_includes_sticky_when_no_per_call(self, cursor):
        cursor.set_statement_parameter(StatementParameterName.MULTI_STATEMENT_COUNT, 3)
        options = cursor._build_statement_parameters_options()
        assert StatementParameterName.MULTI_STATEMENT_COUNT in options


class TestAsyncExecuteSkipUploadOnContentMatch:
    """Async cursor parity: `_skip_upload_on_content_match` routes through the
    same per-call channel as the sync cursor and never persists on the cursor.
    """

    @pytest.fixture
    def cursor(self):
        mock_connection = MagicMock()
        # aio cursor awaits self._connection.is_closed(), so it must be async.
        mock_connection.is_closed = AsyncMock(return_value=False)
        return AsyncSnowflakeCursor(mock_connection)

    @staticmethod
    def _capture_per_call():
        captured = {}

        async def side_effect(*args, **kwargs):
            captured.update(kwargs.get("statement_parameters") or {})
            return MagicMock()

        return captured, side_effect

    def test_kwarg_true_passed_as_per_call_param(self, cursor):
        captured, side_effect = self._capture_per_call()
        with (
            patch.object(cursor, "_execute", side_effect=side_effect),
            patch.object(cursor, "reset", new=AsyncMock()),
        ):
            asyncio.run(cursor.execute("PUT file://x @s", _skip_upload_on_content_match=True))
        assert captured.get(StatementParameterName.SKIP_UPLOAD_ON_CONTENT_MATCH) is True
        assert StatementParameterName.SKIP_UPLOAD_ON_CONTENT_MATCH not in cursor._statement_parameters

    def test_kwarg_default_passes_nothing(self, cursor):
        captured, side_effect = self._capture_per_call()
        with (
            patch.object(cursor, "_execute", side_effect=side_effect),
            patch.object(cursor, "reset", new=AsyncMock()),
        ):
            asyncio.run(cursor.execute("PUT file://x @s"))
        assert StatementParameterName.SKIP_UPLOAD_ON_CONTENT_MATCH not in captured


class TestAsyncExecuteNumStatements:
    """Async cursor parity: `num_statements` on call N must not bleed into
    call N+1, matching the sync cursor.
    """

    @pytest.fixture
    def cursor(self):
        mock_connection = MagicMock()
        # aio cursor awaits self._connection.is_closed(), so it must be async.
        mock_connection.is_closed = AsyncMock(return_value=False)
        return AsyncSnowflakeCursor(mock_connection)

    @staticmethod
    def _capture_per_call():
        captured = {}

        async def side_effect(*args, **kwargs):
            captured.update(kwargs.get("statement_parameters") or {})
            return MagicMock()

        return captured, side_effect

    def test_num_statements_passed_as_per_call_param(self, cursor):
        captured, side_effect = self._capture_per_call()
        with (
            patch.object(cursor, "_execute", side_effect=side_effect),
            patch.object(cursor, "reset", new=AsyncMock()),
        ):
            asyncio.run(cursor.execute("SELECT 1; SELECT 2; SELECT 3", num_statements=3))
        assert captured.get(StatementParameterName.MULTI_STATEMENT_COUNT) == 3
        assert StatementParameterName.MULTI_STATEMENT_COUNT not in cursor._statement_parameters

    def test_num_statements_does_not_persist_across_calls(self, cursor):
        with (
            patch.object(cursor, "_execute", new=AsyncMock(return_value=MagicMock())),
            patch.object(cursor, "reset", new=AsyncMock()),
        ):
            asyncio.run(cursor.execute("SELECT 1; SELECT 2", num_statements=2))

        captured, side_effect = self._capture_per_call()
        with (
            patch.object(cursor, "_execute", side_effect=side_effect),
            patch.object(cursor, "reset", new=AsyncMock()),
        ):
            asyncio.run(cursor.execute("SELECT 99"))
        assert StatementParameterName.MULTI_STATEMENT_COUNT not in captured, "second call must NOT inherit the count"


class TestStageBindingDecision:
    """Unit tests for cursor-side stage binding threshold decision."""

    @pytest.fixture
    def cursor(self, mock_core_client):
        conn = MagicMock()
        conn.conn_handle = ConnectionHandle(id=1)
        conn.is_closed.return_value = False
        conn.paramstyle = ParamStyle.QMARK
        conn._session_parameters = {}
        mock_core_client.statement_new.return_value.stmt_handle = StatementHandle(id=1)
        execute_result = MagicMock()
        execute_result.columns = []
        execute_result.HasField = MagicMock(return_value=False)
        execute_result.sql_state = "00000"
        mock_core_client.statement_execute_query.return_value.result = execute_result
        cur = SnowflakeCursor(conn)
        yield cur
        cur.close()

    def test_should_use_csv_binding_false_for_scalar(self, cursor):
        assert not cursor._should_use_csv_binding((42,), threshold=1, query="INSERT INTO t VALUES (?)")

    def test_should_use_csv_binding_true_at_threshold(self, cursor):
        params = ([1] * 10, ["x"] * 10)
        assert cursor._should_use_csv_binding(params, threshold=20, query="INSERT INTO t VALUES (?, ?)")

    def test_should_use_csv_binding_false_below_threshold(self, cursor):
        params = ([1, 2], ["a", "b"])
        assert not cursor._should_use_csv_binding(params, threshold=100, query="INSERT INTO t VALUES (?, ?)")

    def test_should_use_csv_binding_false_for_non_insert(self, cursor):
        params = ([1] * 10, ["x"] * 10)
        assert not cursor._should_use_csv_binding(params, threshold=1, query="SELECT * FROM t WHERE id = ?")
        assert not cursor._should_use_csv_binding(params, threshold=1, query="UPDATE t SET name = ? WHERE id = ?")
        assert not cursor._should_use_csv_binding(params, threshold=1, query="DELETE FROM t WHERE id = ?")

    def test_should_use_csv_binding_false_for_empty_query(self, cursor):
        params = ([1] * 10, ["x"] * 10)
        assert not cursor._should_use_csv_binding(params, threshold=1, query="")

    def test_stage_binding_threshold_parser_defaults(self):
        assert parse_stage_binding_threshold(None) == 65280
        assert parse_stage_binding_threshold("bad") == 65280


class TestParamsAliasAndForceQmark:
    """Unit tests for the legacy ``params``/``seqparams`` aliases and
    ``_force_qmark_paramstyle`` overrides on execute()/executemany().
    """

    @pytest.fixture
    def mock_connection(self, mock_core_client):
        conn = MagicMock()
        conn.conn_handle = ConnectionHandle(id=1)
        conn.is_closed.return_value = False
        conn.paramstyle = ParamStyle.PYFORMAT
        mock_core_client.statement_new.return_value.stmt_handle = StatementHandle(id=1)
        execute_result = MagicMock()
        execute_result.columns = []
        execute_result.HasField = MagicMock(return_value=False)
        execute_result.sql_state = "00000"
        mock_core_client.statement_execute_query.return_value.result = execute_result
        return conn

    @pytest.fixture
    def cursor(self, mock_connection):
        cur = SnowflakeCursor(mock_connection)
        yield cur
        cur.close()

    def test_execute_accepts_params_alias(self, cursor, mock_core_client):
        """execute(params=...) is treated as execute(parameters=...)."""
        cursor.execute("SELECT ?", params=(1,), _force_qmark_paramstyle=True)

        request = mock_core_client.statement_execute_query.call_args.args[0]
        assert request.bindings is not None

    def test_execute_rejects_both_parameters_and_params(self, cursor, mock_core_client):
        """Passing both parameters= and params= raises ProgrammingError."""
        with pytest.raises(ProgrammingError, match="both 'parameters' and 'params'") as exc_info:
            cursor.execute("SELECT ?", parameters=(1,), params=(2,))

        assert exc_info.value.errno == ER_INVALID_VALUE
        mock_core_client.statement_execute_query.assert_not_called()

    def test_force_qmark_paramstyle_overrides_pyformat(self, cursor, mock_core_client):
        """_force_qmark_paramstyle=True bypasses pyformat client-side
        interpolation: a `%s` query is sent verbatim to the server with
        bindings, instead of being interpolated locally."""
        # Connection paramstyle is pyformat (set by fixture). Without the
        # flag, "SELECT %s" with parameters=(1,) would interpolate to
        # "SELECT 1" and bindings=None.
        cursor.execute("SELECT %s", parameters=(1,), _force_qmark_paramstyle=True)

        request = mock_core_client.statement_execute_query.call_args.args[0]
        assert request.bindings is not None  # qmark path took over
        sql_request = mock_core_client.statement_set_sql_query.call_args.args[0]
        assert sql_request.query == "SELECT %s"  # SQL untouched

    def test_executemany_accepts_seqparams_alias(self, cursor, mock_core_client):
        """executemany(seqparams=...) is treated as executemany(seq_of_parameters=...)."""
        cursor.executemany(
            "INSERT INTO t VALUES (?)",
            seqparams=[(1,), (2,)],
            _force_qmark_paramstyle=True,
        )

        # Server-side array-binding path: a single execute_query call.
        assert mock_core_client.statement_execute_query.call_count == 1
        request = mock_core_client.statement_execute_query.call_args.args[0]
        assert request.bindings is not None

    def test_executemany_rejects_both_seq_aliases(self, cursor, mock_core_client):
        """Passing both seq_of_parameters and seqparams raises ProgrammingError."""
        with pytest.raises(ProgrammingError, match="both 'seq_of_parameters' and 'seqparams'") as exc_info:
            cursor.executemany(
                "INSERT INTO t VALUES (?)",
                seq_of_parameters=[(1,)],
                seqparams=[(2,)],
            )

        assert exc_info.value.errno == ER_INVALID_VALUE
        mock_core_client.statement_execute_query.assert_not_called()

    def test_executemany_force_qmark_paramstyle(self, cursor, mock_core_client):
        """_force_qmark_paramstyle=True must be threaded through executemany()
        into the recursive execute() call. Connection paramstyle is pyformat;
        without the flag, executemany() would interpolate each row
        client-side (3 individual execute_query calls, bindings=None). With
        the flag the array-binding path runs (1 call, bindings non-None,
        SQL untouched)."""
        cursor.executemany(
            "INSERT INTO t VALUES (%s)",
            [(1,), (2,), (3,)],
            _force_qmark_paramstyle=True,
        )

        assert mock_core_client.statement_execute_query.call_count == 1
        request = mock_core_client.statement_execute_query.call_args.args[0]
        assert request.bindings is not None  # flag survived to inner execute()
        sql_request = mock_core_client.statement_set_sql_query.call_args.args[0]
        assert sql_request.query == "INSERT INTO t VALUES (%s)"  # not interpolated


class TestDescribeInternal:
    """Unit tests for Cursor._describe_internal (Snowpark describe-only path).

    Mirrors TestDescribe but asserts the new-format ``ResultMetadataV2`` return
    contract that Snowpark's ``run_new_describe`` consumes.
    """

    @pytest.fixture
    def mock_connection(self, mock_core_client):
        conn = MagicMock()
        conn.conn_handle = ConnectionHandle(id=1)
        conn.is_closed.return_value = False
        mock_core_client.statement_new.return_value.stmt_handle = StatementHandle(id=1)
        return conn

    @pytest.fixture
    def cursor(self, mock_connection):
        return SnowflakeCursor(mock_connection)

    def _setup_prepare(self, mock_core_client, columns=None, query_id="", query="", sql_state=None):
        result = MagicMock()
        result.columns = columns or []
        result.stream.value = (42).to_bytes(8, byteorder="little", signed=False)
        result.query_id = query_id
        result.query = query
        result.sql_state = sql_state
        mock_core_client.statement_prepare.return_value.result = result
        return result

    @staticmethod
    def _fixed_column(name="COL1"):
        col = MagicMock(type="FIXED", nullable=True, precision=10, scale=0)
        col.name = name
        col.HasField = lambda f: f in ("precision", "scale")
        return col

    def test_returns_list_of_result_metadata_v2(self, cursor, mock_core_client):
        """_describe_internal returns a list of the real ResultMetadataV2 class."""
        self._setup_prepare(mock_core_client, columns=[self._fixed_column()])

        result = cursor._describe_internal("SELECT 1 AS COL1")

        assert result is not None
        assert len(result) == 1
        assert isinstance(result[0], ResultMetadataV2)
        assert result[0].name == "COL1"
        assert result[0].type_code == 0  # FIXED

    def test_returns_none_when_no_columns(self, cursor, mock_core_client):
        """_describe_internal returns None for a statement with no result set."""
        self._setup_prepare(mock_core_client, columns=[])

        assert cursor._describe_internal("INSERT INTO t VALUES (1)") is None

    def test_updates_cursor_description(self, cursor, mock_core_client):
        """_describe_internal updates cursor.description as an observable side effect."""
        self._setup_prepare(mock_core_client, columns=[self._fixed_column()])

        cursor._describe_internal("SELECT 1 AS COL1")

        assert cursor.description is not None
        assert cursor.description[0].name == "COL1"

    def test_rownumber_reset_when_columns_present(self, cursor, mock_core_client):
        """_describe_internal resets _rownumber to -1 when columns are returned,
        and leaves it untouched when there are none.

        Sentinel seeded at 5 to defeat the tautology: reset() preserves rownumber
        (backward-compat), so only the explicit `self._rownumber = -1` production
        line inside _describe_internal can change it from 5 → -1.
        """
        # With columns: sentinel must change to -1.
        cursor._rownumber = 5
        self._setup_prepare(mock_core_client, columns=[self._fixed_column()])
        cursor._describe_internal("SELECT 1")
        assert cursor._rownumber == -1
        assert cursor.rownumber is None  # property maps -1 → None

        # Without columns: the branch is skipped, sentinel must survive.
        cursor._rownumber = 5
        self._setup_prepare(mock_core_client, columns=[])
        cursor._describe_internal("INSERT INTO t VALUES (1)")
        assert cursor._rownumber == 5

    def test_releases_statement_handle(self, cursor, mock_core_client):
        """_describe_internal allocates and releases exactly one statement handle."""
        self._setup_prepare(mock_core_client, columns=[])

        cursor._describe_internal("SELECT 1")

        mock_core_client.statement_new.assert_called_once()
        mock_core_client.statement_release.assert_called_once()

    def test_raises_when_cursor_closed(self, cursor, mock_connection):
        """_describe_internal rejects a closed cursor (parity with describe() and legacy)."""
        cursor.close()
        with pytest.raises(InterfaceError):
            cursor._describe_internal("SELECT 1")

        fresh = SnowflakeCursor(mock_connection)
        mock_connection.is_closed.return_value = True
        with pytest.raises(InterfaceError):
            fresh._describe_internal("SELECT 1")

    def test_propagates_prepare_error(self, cursor, mock_core_client):
        """_describe_internal propagates ProgrammingError from the prepare RPC."""
        mock_core_client.statement_prepare.side_effect = ProgrammingError("syntax error", sqlstate="42601")

        with pytest.raises(ProgrammingError):
            cursor._describe_internal("INVALID SQL")

        assert cursor.sqlstate == "42601"

    def test_params_alias_forwarded_to_prepare_query(self, cursor, mock_core_client):
        """`params=` is the Snowpark alias for `parameters=`; _resolve_alias forwards it."""
        self._setup_prepare(mock_core_client, columns=[])

        with patch.object(cursor, "_prepare_query", return_value=("SELECT 1", None)) as spy:
            cursor._describe_internal("SELECT 1", params=[42])

        assert spy.call_args.args[1] == [42]


class TestAsyncDescribeInternal:
    """Async parity for _describe_internal: same ResultMetadataV2 return contract,
    same handle lifecycle, same closed-cursor rejection as the sync cursor.
    """

    @pytest.fixture
    def mock_async_core_client(self):
        from snowflake.connector._internal.api_client.client_api import async_core_driver

        client = MagicMock()
        client.statement_new = AsyncMock(return_value=MagicMock(stmt_handle=StatementHandle(id=1)))
        client.statement_set_sql_query = AsyncMock()
        client.statement_release = AsyncMock()
        client.result_set_release = AsyncMock()
        old = async_core_driver._client
        async_core_driver.client = client
        yield client
        async_core_driver.client = old

    @pytest.fixture
    def mock_connection(self):
        conn = MagicMock()
        conn.conn_handle = ConnectionHandle(id=1)
        conn.is_closed = AsyncMock(return_value=False)
        return conn

    @pytest.fixture
    def cursor(self, mock_async_core_client, mock_connection):
        return AsyncSnowflakeCursor(mock_connection)

    def _setup_prepare(self, client, columns=None):
        result = MagicMock()
        result.columns = columns or []
        result.stream.value = (42).to_bytes(8, byteorder="little", signed=False)
        result.query_id = ""
        result.query = ""
        result.sql_state = None
        client.statement_prepare = AsyncMock(return_value=MagicMock(result=result))
        return result

    @staticmethod
    def _fixed_column(name="COL1"):
        col = MagicMock(type="FIXED", nullable=True, precision=10, scale=0)
        col.name = name
        col.HasField = lambda f: f in ("precision", "scale")
        return col

    def test_returns_list_of_result_metadata_v2(self, cursor, mock_async_core_client):
        """Async _describe_internal returns a list of the real ResultMetadataV2 class."""
        self._setup_prepare(mock_async_core_client, columns=[self._fixed_column()])

        result = asyncio.run(cursor._describe_internal("SELECT 1 AS COL1"))

        assert result is not None
        assert len(result) == 1
        assert isinstance(result[0], ResultMetadataV2)
        assert result[0].name == "COL1"
        assert result[0].type_code == 0  # FIXED

    def test_returns_none_when_no_columns(self, cursor, mock_async_core_client):
        """Async _describe_internal returns None for a statement with no result set."""
        self._setup_prepare(mock_async_core_client, columns=[])

        assert asyncio.run(cursor._describe_internal("INSERT INTO t VALUES (1)")) is None

    def test_releases_statement_handle(self, cursor, mock_async_core_client):
        """Async _describe_internal allocates and releases exactly one statement handle."""
        self._setup_prepare(mock_async_core_client, columns=[])

        asyncio.run(cursor._describe_internal("SELECT 1"))

        mock_async_core_client.statement_new.assert_awaited_once()
        mock_async_core_client.statement_release.assert_awaited_once()

    def test_raises_when_cursor_closed(self, cursor, mock_connection):
        """Async _describe_internal rejects a closed cursor and a closed connection."""
        asyncio.run(cursor.close())
        with pytest.raises(InterfaceError):
            asyncio.run(cursor._describe_internal("SELECT 1"))

        fresh = AsyncSnowflakeCursor(mock_connection)
        mock_connection.is_closed = AsyncMock(return_value=True)
        with pytest.raises(InterfaceError):
            asyncio.run(fresh._describe_internal("SELECT 1"))
