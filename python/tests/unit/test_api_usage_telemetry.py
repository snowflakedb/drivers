"""Unit tests for api_telemetry decorator and api_usage tracking."""

from unittest.mock import MagicMock, patch

import pytest

from snowflake.connector._internal.decorators import _TRACKING
from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
    ConnectionHandle,
    DatabaseHandle,
    ExecuteQueryResponse,
    StatementHandle,
)


@pytest.fixture
def mock_db_api():
    """Create a mock DatabaseDriverClient with minimal stubs for Connection.__init__."""
    db_api = MagicMock()
    db_api.database_new.return_value = MagicMock(db_handle=DatabaseHandle(id=1))
    db_api.connection_new.return_value = MagicMock(conn_handle=ConnectionHandle(id=42))
    db_api.connection_get_parameter.return_value = MagicMock(value="")
    # Provide a real StatementHandle so protobuf field validation passes
    db_api.statement_new.return_value.stmt_handle = StatementHandle(id=1)
    db_api.statement_execute_query.return_value = ExecuteQueryResponse()
    db_api.statement_result_chunks.return_value = MagicMock(HasField=MagicMock(return_value=False))
    return db_api


@pytest.fixture
def connection(mock_db_api):
    """Create a Connection with a mocked db_api."""
    from snowflake.connector.connection import Connection

    with patch("snowflake.connector.connection.database_driver_client", return_value=mock_db_api):
        conn = Connection(user="test_user", account="test_account")
    return conn


@pytest.fixture
def cursor(connection, mock_db_api):
    """Create a cursor from the mocked connection."""
    # Reset telemetry calls from connection setup
    mock_db_api.telemetry_send_api_usage.reset_mock()
    return connection.cursor()


@pytest.fixture(autouse=True)
def reset_tracking():
    """Ensure _TRACKING ContextVar is reset before each test."""
    token = _TRACKING.set(True)
    yield
    _TRACKING.reset(token)


def _get_api_methods(mock_db_api):
    """Extract api_method strings from all telemetry_send_api_usage calls."""
    return [call[0][0].api_method for call in mock_db_api.telemetry_send_api_usage.call_args_list]


class TestConnectionApiTelemetry:
    """Tests that Connection public methods send api_usage telemetry."""

    def test_cursor_sends_telemetry(self, connection, mock_db_api):
        mock_db_api.telemetry_send_api_usage.reset_mock()
        connection.cursor()

        methods = _get_api_methods(mock_db_api)
        assert "Connection.cursor" in methods

    def test_close_sends_telemetry(self, connection, mock_db_api):
        mock_db_api.telemetry_send_api_usage.reset_mock()
        connection.close()

        methods = _get_api_methods(mock_db_api)
        assert "Connection.close" in methods

    def test_get_autocommit_sends_telemetry(self, connection, mock_db_api):
        mock_db_api.telemetry_send_api_usage.reset_mock()
        connection.get_autocommit()

        methods = _get_api_methods(mock_db_api)
        assert "Connection.get_autocommit" in methods

    def test_commit_suppresses_inner_calls(self, connection, mock_db_api):
        """commit() calls cursor(), execute(), close() internally — only commit should be tracked."""
        mock_db_api.telemetry_send_api_usage.reset_mock()
        connection.commit()

        methods = _get_api_methods(mock_db_api)
        assert "Connection.commit" in methods
        assert "Connection.cursor" not in methods
        assert "SnowflakeCursor.execute" not in methods
        assert "SnowflakeCursor.close" not in methods

    def test_rollback_suppresses_inner_calls(self, connection, mock_db_api):
        mock_db_api.telemetry_send_api_usage.reset_mock()
        connection.rollback()

        methods = _get_api_methods(mock_db_api)
        assert "Connection.rollback" in methods
        assert "Connection.cursor" not in methods
        assert "SnowflakeCursor.execute" not in methods

    def test_execute_string_suppresses_inner_calls(self, connection, mock_db_api):
        """execute_string calls execute_stream which calls cursor() + execute() — only outermost tracked."""
        mock_db_api.telemetry_send_api_usage.reset_mock()
        connection.execute_string("SELECT 1; SELECT 2")

        methods = _get_api_methods(mock_db_api)
        assert "Connection.execute_string" in methods
        assert "Connection.execute_stream" not in methods
        assert "Connection.cursor" not in methods
        assert "SnowflakeCursor.execute" not in methods

    def test_api_method_uses_runtime_class_name(self, connection, mock_db_api):
        """api_method should be derived from type(self).__name__, not hardcoded."""
        mock_db_api.telemetry_send_api_usage.reset_mock()
        connection.close()

        req = mock_db_api.telemetry_send_api_usage.call_args[0][0]
        assert req.api_method == "Connection.close"


class TestCursorApiTelemetry:
    """Tests that Cursor public methods send api_usage telemetry."""

    def test_execute_sends_telemetry(self, cursor, mock_db_api):
        cursor.execute("SELECT 1")

        methods = _get_api_methods(mock_db_api)
        assert "SnowflakeCursor.execute" in methods

    def test_close_sends_telemetry(self, cursor, mock_db_api):
        cursor.close()

        methods = _get_api_methods(mock_db_api)
        assert "SnowflakeCursor.close" in methods

    def test_fetchone_sends_telemetry(self, cursor, mock_db_api):
        # fetchone requires a prior execute — mock the iterator
        cursor._execute_result = MagicMock()
        cursor._iterator = iter([])
        cursor._fetch_mode = None
        cursor.fetchone()

        methods = _get_api_methods(mock_db_api)
        assert "SnowflakeCursor.fetchone" in methods

    def test_fetchmany_suppresses_fetchone(self, cursor, mock_db_api):
        """fetchmany() calls fetchone() internally — only fetchmany should be tracked."""
        cursor._execute_result = MagicMock()
        cursor._iterator = iter([(1,), (2,)])
        cursor._fetch_mode = None
        cursor.fetchmany(2)

        methods = _get_api_methods(mock_db_api)
        assert "SnowflakeCursor.fetchmany" in methods
        assert "SnowflakeCursor.fetchone" not in methods

    def test_dict_cursor_fetchone_uses_correct_class_name(self, connection, mock_db_api):
        from snowflake.connector.cursor import DictCursor

        mock_db_api.telemetry_send_api_usage.reset_mock()
        cur = connection.cursor(DictCursor)
        mock_db_api.telemetry_send_api_usage.reset_mock()

        cur._execute_result = MagicMock()
        cur._iterator = iter([])
        cur._fetch_mode = None
        cur.fetchone()

        methods = _get_api_methods(mock_db_api)
        assert "DictCursor.fetchone" in methods


class TestApiTelemetryResetBehavior:
    """Tests that tracking is properly reset after each call."""

    def test_tracking_resets_after_method_returns(self, connection, mock_db_api):
        """After a tracked method returns, subsequent calls should also be tracked."""
        mock_db_api.telemetry_send_api_usage.reset_mock()
        connection.close()
        connection.get_autocommit()

        methods = _get_api_methods(mock_db_api)
        assert "Connection.close" in methods
        assert "Connection.get_autocommit" in methods
        assert mock_db_api.telemetry_send_api_usage.call_count == 2

    def test_tracking_resets_after_exception(self, cursor, mock_db_api):
        """If a method raises, tracking should still reset for the next call."""
        mock_db_api.statement_execute_query.side_effect = RuntimeError("boom")

        with pytest.raises(RuntimeError):
            cursor.execute("SELECT 1")

        # Tracking should be re-enabled
        mock_db_api.statement_execute_query.side_effect = None
        mock_db_api.statement_execute_query.return_value = ExecuteQueryResponse()
        mock_db_api.telemetry_send_api_usage.reset_mock()
        cursor.execute("SELECT 2")

        methods = _get_api_methods(mock_db_api)
        assert "SnowflakeCursor.execute" in methods


class TestApiTelemetryFailureIsolation:
    """Tests that telemetry failures don't break the actual method."""

    def test_telemetry_rpc_failure_does_not_break_method(self, connection, mock_db_api):
        """If send_api_usage raises, the decorated method should still execute."""
        mock_db_api.telemetry_send_api_usage.side_effect = RuntimeError("telemetry down")

        # close() should still work despite telemetry failure
        # (send_api_usage swallows exceptions internally)
        connection.close()
        assert connection.is_closed()
