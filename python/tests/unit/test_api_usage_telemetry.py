"""Unit tests for api_telemetry decorator and api_usage tracking."""

import asyncio
import inspect

from io import StringIO
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from snowflake.connector._internal.decorators import _TRACKING, api_telemetry
from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
    ConnectionHandle,
    ConnectionIsClosedResponse,
    ConnectionIsExpiredResponse,
    DatabaseHandle,
    ResultSetDescriptor,
    StatementHandle,
)
from snowflake.connector.constants import SessionParameterName
from snowflake.connector.errors import InterfaceError, ProgrammingError
from tests.helpers.fixtures import (
    _make_execute_response,
    make_all_parameters_response,
    make_parameter_response,
)


def _get_parameter_side_effect(request: object):
    """Default CLIENT_TELEMETRY_ENABLED to "true" so telemetry-assertion tests in this
    file exercise their intended path by default; every other parameter stays unset,
    matching the shared ``mock_db_api`` fixture in ``tests/helpers/fixtures.py``.

    ``core_driver.connection_get_parameter`` calls ``self.client.connection_get_parameter(request)``
    with a single positional ``ConnectionGetParameterRequest``, not ``key=`` kwargs.
    """
    if request.key == SessionParameterName.CLIENT_TELEMETRY_ENABLED:
        return make_parameter_response(True)
    return make_parameter_response()


@pytest.fixture
def mock_db_api():
    """Create a mock DatabaseDriverClient patched into core_driver."""
    from snowflake.connector._internal.api_client.client_api import core_driver

    db_api = MagicMock()
    db_api.database_new.return_value = MagicMock(db_handle=DatabaseHandle(id=1))
    db_api.connection_new.return_value = MagicMock(conn_handle=ConnectionHandle(id=42))
    db_api.connection_get_parameter.side_effect = _get_parameter_side_effect
    # Mirrors connection_get_parameter's default: sf_core's connection_get_all_parameters
    # sources from the same session_parameters cache, so the bulk snapshot taken by
    # SessionParametersProxy.freeze() (on close()) must agree with the per-key default.
    db_api.connection_get_all_parameters.return_value = make_all_parameters_response(
        {SessionParameterName.CLIENT_TELEMETRY_ENABLED: True}
    )

    def _connection_is_closed(request):
        return ConnectionIsClosedResponse(is_closed=request.conn_handle.id == 0)

    db_api.connection_is_closed.side_effect = _connection_is_closed
    db_api.statement_new.return_value.stmt_handle = StatementHandle(id=1)
    db_api.statement_execute_query.return_value = _make_execute_response()
    db_api.connection_get_result_set.return_value = MagicMock(
        result_descriptor=ResultSetDescriptor(query_id="fake-qid"),
    )

    old_client = core_driver._client
    core_driver.client = db_api
    yield db_api
    core_driver.client = old_client


@pytest.fixture
def connection(mock_db_api):
    """Create a Connection with a mocked db_api."""
    from snowflake.connector.connection import Connection

    with patch("snowflake.connector._internal.cursor.query_result.get_stream_ptr", return_value=0):
        conn = Connection(user="test_user", account="test_account")
        yield conn


@pytest.fixture
def cursor(connection, mock_db_api):
    """Create a cursor from the mocked connection."""
    # Reset telemetry calls from connection setup
    mock_db_api.telemetry_send_api_usage.reset_mock()
    return connection.cursor()


@pytest.fixture(autouse=True)
def reset_tracking():
    """Ensure the _TRACKING ContextVar is reset before each test."""
    tracking_token = _TRACKING.set(True)
    yield
    _TRACKING.reset(tracking_token)


def _get_api_methods(mock_db_api):
    """Extract api_method strings from all telemetry_send_api_usage calls."""
    return [call[0][0].api_method for call in mock_db_api.telemetry_send_api_usage.call_args_list]


def _passed_arguments_for(mock_db_api, api_method):
    """Return the passed_arguments list recorded for the given api_method.

    Asserts exactly one matching call so callers get a single unambiguous list.
    """
    matches = [
        list(call[0][0].passed_arguments)
        for call in mock_db_api.telemetry_send_api_usage.call_args_list
        if call[0][0].api_method == api_method
    ]
    assert len(matches) == 1, f"expected exactly one {api_method} call, got {len(matches)}"
    return matches[0]


def _run_async(awaitable):
    """Run a coroutine/awaitable in a fresh event loop (no pytest-asyncio dependency)."""

    async def _run():
        return await awaitable

    return asyncio.run(_run())


@pytest.fixture
def mock_async_db_api(mock_async_db_api):
    """Layer telemetry/session-parameter defaults onto the shared ``mock_async_db_api``
    (`tests/helpers/fixtures.py`), mirroring this file's ``mock_db_api`` override so the
    frozen-snapshot path (``connection_get_all_parameters``) agrees with the per-key default.

    Also configures ``_sync_db_api`` (used by ``is_expired``, ``is_closed``, proxy reads)
    so ``async_connection`` does not need the separate ``mock_db_api`` fixture — that
    fixture would overwrite ``core_driver.client`` and drop ``connection_is_expired``.
    """
    mock_async_db_api.connection_get_all_parameters = AsyncMock(
        return_value=make_all_parameters_response({SessionParameterName.CLIENT_TELEMETRY_ENABLED: True})
    )
    sync_api = mock_async_db_api._sync_db_api
    sync_api.connection_get_parameter.side_effect = _get_parameter_side_effect
    sync_api.connection_get_all_parameters.return_value = make_all_parameters_response(
        {SessionParameterName.CLIENT_TELEMETRY_ENABLED: True}
    )
    return mock_async_db_api


@pytest.fixture
def async_connection(mock_async_db_api):
    """Create an async Connection with a mocked async db_api.

    Sync ``core_driver`` reads (``is_expired``, session-parameter proxies, etc.) go
    through ``mock_async_db_api._sync_db_api`` — see ``mock_async_db_api`` above.
    """
    from snowflake.connector.aio.connection._connection import Connection

    with patch("snowflake.connector._internal.cursor.query_result.get_stream_ptr", return_value=0):

        async def _make():
            conn = Connection(user="test_user", account="test_account")
            await conn.connect()
            return conn

        return _run_async(_make())


@pytest.fixture
def async_cursor(async_connection, mock_async_db_api):
    """Create an async cursor from the mocked async connection."""
    mock_async_db_api.telemetry_send_api_usage.reset_mock()
    return async_connection.cursor()


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

    def test_execute_stream_suppresses_during_iteration(self, connection, mock_db_api):
        """execute_stream is a generator — nested calls during iteration must be suppressed."""
        from io import StringIO

        mock_db_api.telemetry_send_api_usage.reset_mock()
        # Iterate the generator so the body actually runs
        cursors = list(connection.execute_stream(StringIO("SELECT 1; SELECT 2")))
        assert len(cursors) == 2

        methods = _get_api_methods(mock_db_api)
        assert "Connection.execute_stream" in methods
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

    def test_fetchone_does_not_send_telemetry(self, cursor, mock_db_api):
        """fetchone is a hot path — intentionally not api_telemetry-tracked."""
        mock_db_api.telemetry_send_api_usage.reset_mock()
        cursor._execute_result = MagicMock()
        cursor._iterator = iter([])
        cursor.fetchone()

        assert "SnowflakeCursor.fetchone" not in _get_api_methods(mock_db_api)

    def test_fetchmany_does_not_send_telemetry(self, cursor, mock_db_api):
        """fetchmany is a hot path — intentionally not api_telemetry-tracked."""
        mock_db_api.telemetry_send_api_usage.reset_mock()
        mock_iterator = MagicMock()
        mock_iterator.fetch_many.return_value = [(1,), (2,)]
        cursor._execute_result = MagicMock()
        cursor._iterator = mock_iterator
        cursor.fetchmany(2)

        assert "SnowflakeCursor.fetchmany" not in _get_api_methods(mock_db_api)

    def test_fetchall_does_not_send_telemetry(self, cursor, mock_db_api):
        """fetchall is a hot path — intentionally not api_telemetry-tracked."""
        mock_db_api.telemetry_send_api_usage.reset_mock()
        mock_iterator = MagicMock()
        mock_iterator.fetch_all.return_value = [(1,), (2,)]
        cursor._execute_result = MagicMock()
        cursor._iterator = mock_iterator
        cursor.fetchall()

        assert "SnowflakeCursor.fetchall" not in _get_api_methods(mock_db_api)

    def test_dict_cursor_fetchone_does_not_send_telemetry(self, connection, mock_db_api):
        """DictCursor.fetchone is a hot path — intentionally not api_telemetry-tracked."""
        from snowflake.connector.cursor import DictCursor

        mock_db_api.telemetry_send_api_usage.reset_mock()
        cur = connection.cursor(DictCursor)
        mock_db_api.telemetry_send_api_usage.reset_mock()

        cur._execute_result = MagicMock()
        cur._iterator = iter([])
        cur.fetchone()

        assert "DictCursor.fetchone" not in _get_api_methods(mock_db_api)


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
        mock_db_api.statement_execute_query.return_value = _make_execute_response()
        mock_db_api.telemetry_send_api_usage.reset_mock()
        cursor.execute("SELECT 2")

        methods = _get_api_methods(mock_db_api)
        assert "SnowflakeCursor.execute" in methods

    def test_unconsumed_generator_does_not_leak_tracking(self, connection, mock_db_api):
        """A never-iterated generator must not suppress subsequent telemetry."""
        mock_db_api.telemetry_send_api_usage.reset_mock()

        gen = connection.execute_stream(StringIO("SELECT 1"))
        del gen

        connection.cursor()

        methods = _get_api_methods(mock_db_api)
        assert "Connection.execute_stream" in methods
        assert "Connection.cursor" in methods

    def test_tracking_true_between_generator_yields(self, connection, mock_db_api):
        """Between yields, _TRACKING is True so independent calls are tracked."""
        mock_db_api.telemetry_send_api_usage.reset_mock()

        gen = connection.execute_stream(StringIO("SELECT 1; SELECT 2"))
        next(gen)

        connection.get_query_status("")

        methods = _get_api_methods(mock_db_api)
        assert "Connection.execute_stream" in methods
        assert "Connection.get_query_status" in methods

    def test_telemetry_works_after_abandoned_generator(self, connection, mock_db_api):
        """cursor() + close() send telemetry even after an abandoned generator."""
        mock_db_api.telemetry_send_api_usage.reset_mock()

        gen = connection.execute_stream(StringIO("SELECT 1"))
        del gen

        cur = connection.cursor()
        cur.close()
        connection.close()

        methods = _get_api_methods(mock_db_api)
        assert "Connection.execute_stream" in methods
        assert "Connection.cursor" in methods
        assert "SnowflakeCursor.close" in methods
        assert "Connection.close" in methods


class TestWrapperErrorTelemetry:
    """Tests that ErrorHandlerMixin sends wrapper_error telemetry when a wrapped call raises."""

    @staticmethod
    def _get_wrapper_errors(mock_db_api):
        """Extract (exception_type, error_source) from all send_wrapper_error calls."""
        return [
            (call[0][0].exception_type, call[0][0].error_source)
            for call in mock_db_api.telemetry_send_wrapper_error.call_args_list
        ]

    def test_cursor_execute_error_sends_wrapper_error(self, cursor, mock_db_api):
        mock_db_api.statement_execute_query.side_effect = ProgrammingError("syntax error")

        with pytest.raises(ProgrammingError):
            cursor.execute("BAD SQL")

        assert ("ProgrammingError", "SnowflakeCursor.execute") in self._get_wrapper_errors(mock_db_api)

    def test_fetchone_closed_sends_wrapper_error(self, cursor, mock_db_api):
        """fetchone uses @simplified_error_handling but still reports wrapper_error on failure."""
        mock_db_api.telemetry_send_wrapper_error.reset_mock()
        cursor.close()

        with pytest.raises(InterfaceError):
            cursor.fetchone()

        assert ("InterfaceError", "SnowflakeCursor.fetchone") in self._get_wrapper_errors(mock_db_api)

    def test_fetchmany_closed_sends_wrapper_error(self, cursor, mock_db_api):
        """fetchmany uses @simplified_error_handling but still reports wrapper_error on failure."""
        mock_db_api.telemetry_send_wrapper_error.reset_mock()
        cursor.close()

        with pytest.raises(InterfaceError):
            cursor.fetchmany(1)

        assert ("InterfaceError", "SnowflakeCursor.fetchmany") in self._get_wrapper_errors(mock_db_api)

    def test_fetchone_success_does_not_send_wrapper_error(self, cursor, mock_db_api):
        mock_db_api.telemetry_send_wrapper_error.reset_mock()
        cursor._iterator = iter([(1,)])

        cursor.fetchone()

        assert self._get_wrapper_errors(mock_db_api) == []

    def test_connection_commit_error_reports_inner_source(self, connection, mock_db_api):
        """When commit() -> execute() raises, both the inner execute() frame and the
        outer commit() frame report — each wrapped frame that catches the exception
        reports it under its own method name, innermost first."""
        mock_db_api.telemetry_send_wrapper_error.reset_mock()
        mock_db_api.statement_execute_query.side_effect = ProgrammingError("fail")

        with pytest.raises(ProgrammingError):
            connection.commit()

        assert self._get_wrapper_errors(mock_db_api) == [
            ("ProgrammingError", "SnowflakeCursor.execute"),
            ("ProgrammingError", "Connection.commit"),
        ]

    def test_sibling_error_after_swallowed_exception_still_reports(self, connection, cursor, mock_db_api):
        """An exception raised by a nested decorated call and caught by ordinary
        (non-decorator) code inside an outer decorated call must not suppress
        reporting for a later, unrelated exception raised by a sibling decorated
        call within that same outer call."""
        mock_db_api.telemetry_send_wrapper_error.reset_mock()
        mock_db_api.statement_execute_query.side_effect = ProgrammingError("fail")

        @api_telemetry
        def outer(self):
            try:
                cursor.execute("first")
            except ProgrammingError:
                pass
            cursor.execute("second")

        with pytest.raises(ProgrammingError):
            outer(connection)

        assert self._get_wrapper_errors(mock_db_api) == [
            ("ProgrammingError", "SnowflakeCursor.execute"),
            ("ProgrammingError", "SnowflakeCursor.execute"),
        ]

    def test_non_connector_exception_reported(self, cursor, mock_db_api):
        """Non-Error exceptions (e.g. RuntimeError) are also reported."""
        mock_db_api.statement_execute_query.side_effect = RuntimeError("unexpected")

        with pytest.raises(RuntimeError):
            cursor.execute("SELECT 1")

        assert ("RuntimeError", "SnowflakeCursor.execute") in self._get_wrapper_errors(mock_db_api)

    def test_successful_call_does_not_send_wrapper_error(self, cursor, mock_db_api):
        cursor.execute("SELECT 1")
        assert self._get_wrapper_errors(mock_db_api) == []

    def test_wrapper_error_telemetry_failure_does_not_suppress_exception(self, cursor, mock_db_api):
        """If send_wrapper_error itself fails, the original exception still propagates."""
        mock_db_api.statement_execute_query.side_effect = ProgrammingError("original")
        mock_db_api.telemetry_send_wrapper_error.side_effect = RuntimeError("telemetry down")

        with pytest.raises(ProgrammingError, match="original"):
            cursor.execute("SELECT 1")

    def test_generator_error_reports_innermost_method(self, connection, mock_db_api):
        """execute_stream() is a generator method, so it is never itself wrapped by
        ErrorHandlerMixin — but the inner cursor.execute() call it drives while being
        iterated is a regular wrapped method, and reports the error from that inner
        frame."""
        mock_db_api.telemetry_send_wrapper_error.reset_mock()
        mock_db_api.statement_execute_query.side_effect = ProgrammingError("fail")

        with pytest.raises(ProgrammingError):
            list(connection.execute_stream(StringIO("SELECT 1")))

        assert self._get_wrapper_errors(mock_db_api) == [("ProgrammingError", "SnowflakeCursor.execute")]


class TestAsyncWrapperErrorTelemetry:
    """Async counterpart of TestWrapperErrorTelemetry."""

    @staticmethod
    def _get_wrapper_errors(mock_async_db_api):
        return [
            (call[0][0].exception_type, call[0][0].error_source)
            for call in mock_async_db_api.telemetry_send_wrapper_error.call_args_list
        ]

    def test_async_cursor_execute_error_sends_wrapper_error(self, async_cursor, mock_async_db_api):
        mock_async_db_api.statement_execute_query.side_effect = ProgrammingError("syntax error")

        with pytest.raises(ProgrammingError):
            _run_async(async_cursor.execute("BAD SQL"))

        assert ("ProgrammingError", "SnowflakeCursor.execute") in self._get_wrapper_errors(mock_async_db_api)

    def test_async_fetchone_closed_sends_wrapper_error(self, async_cursor, mock_async_db_api):
        mock_async_db_api.telemetry_send_wrapper_error.reset_mock()
        async_cursor.close()

        with pytest.raises(InterfaceError):
            _run_async(async_cursor.fetchone())

        assert ("InterfaceError", "SnowflakeCursor.fetchone") in self._get_wrapper_errors(mock_async_db_api)

    def test_async_connection_commit_error_reports_inner_source(self, async_connection, mock_async_db_api):
        mock_async_db_api.telemetry_send_wrapper_error.reset_mock()
        mock_async_db_api.statement_execute_query.side_effect = ProgrammingError("fail")

        with pytest.raises(ProgrammingError):
            _run_async(async_connection.commit())

        assert self._get_wrapper_errors(mock_async_db_api) == [
            ("ProgrammingError", "SnowflakeCursor.execute"),
            ("ProgrammingError", "Connection.commit"),
        ]


class TestTelemetryEnabledGating:
    """Tests that Connection.telemetry_enabled actually gates wrapper telemetry sends,
    matching the legacy driver's AND-of-client-and-server gating (not just the client half).
    """

    def test_client_disabled_suppresses_api_usage_telemetry(self, connection, mock_db_api):
        connection.telemetry_enabled = False
        mock_db_api.telemetry_send_api_usage.reset_mock()

        connection.cursor()

        assert _get_api_methods(mock_db_api) == []

    def test_client_disabled_suppresses_wrapper_error_telemetry(self, cursor, mock_db_api):
        mock_db_api.telemetry_send_wrapper_error.reset_mock()
        cursor._connection.telemetry_enabled = False
        mock_db_api.statement_execute_query.side_effect = ProgrammingError("boom")

        with pytest.raises(ProgrammingError):
            cursor.execute("BAD SQL")

        assert mock_db_api.telemetry_send_wrapper_error.call_count == 0

    def test_server_param_disabled_suppresses_telemetry_even_when_client_enabled(self, connection, mock_db_api):
        """Regression guard: gating must consult the server half too, not just the client flag."""
        mock_db_api.telemetry_send_api_usage.reset_mock()
        assert connection._client_param_telemetry_enabled is True
        mock_db_api.connection_get_parameter.side_effect = None
        mock_db_api.connection_get_parameter.return_value = make_parameter_response(False)

        connection.cursor()

        assert _get_api_methods(mock_db_api) == []

    def test_client_enabled_and_server_enabled_sends_telemetry(self, connection, mock_db_api):
        """Baseline: with both halves on (this file's fixture default), telemetry flows."""
        mock_db_api.telemetry_send_api_usage.reset_mock()

        connection.cursor()

        assert "Connection.cursor" in _get_api_methods(mock_db_api)

    def test_re_enabling_resumes_telemetry(self, connection, mock_db_api):
        connection.telemetry_enabled = False
        mock_db_api.telemetry_send_api_usage.reset_mock()
        connection.cursor()
        assert _get_api_methods(mock_db_api) == []

        connection.telemetry_enabled = True
        mock_db_api.telemetry_send_api_usage.reset_mock()
        connection.cursor()
        assert "Connection.cursor" in _get_api_methods(mock_db_api)


class TestAsyncTelemetryEnabledGating:
    """Async counterpart of TestTelemetryEnabledGating."""

    def test_client_disabled_suppresses_api_usage_telemetry(self, async_connection, mock_async_db_api):
        async_connection.telemetry_enabled = False
        mock_async_db_api.telemetry_send_api_usage.reset_mock()

        async_connection.cursor()

        methods = [call[0][0].api_method for call in mock_async_db_api.telemetry_send_api_usage.call_args_list]
        assert methods == []

    def test_client_disabled_suppresses_wrapper_error_telemetry(self, async_cursor, mock_async_db_api):
        mock_async_db_api.telemetry_send_wrapper_error.reset_mock()
        async_cursor._connection.telemetry_enabled = False
        mock_async_db_api.statement_execute_query.side_effect = ProgrammingError("boom")

        with pytest.raises(ProgrammingError):
            _run_async(async_cursor.execute("BAD SQL"))

        assert mock_async_db_api.telemetry_send_wrapper_error.call_count == 0


class TestApiTelemetryFailureIsolation:
    """Tests that telemetry failures don't break the actual method."""

    def test_telemetry_rpc_failure_does_not_break_method(self, connection, mock_db_api):
        """If send_api_usage raises, the decorated method should still execute."""
        mock_db_api.telemetry_send_api_usage.side_effect = RuntimeError("telemetry down")

        # close() should still work despite telemetry failure
        # (send_api_usage swallows exceptions internally)
        connection.close()
        assert connection.is_closed()


class TestAsyncConnectionApiTelemetry:
    """Tests that aio Connection public methods send api_usage telemetry."""

    def test_cursor_sends_telemetry(self, async_connection, mock_async_db_api):
        mock_async_db_api.telemetry_send_api_usage.reset_mock()

        async def _cursor_and_drain():
            # cursor() is synchronous; on an async connection its telemetry is
            # fire-and-forget (scheduled via create_task), so it only records
            # under a running loop. Drain the scheduled task before asserting.
            async_connection.cursor()
            pending = asyncio.all_tasks() - {asyncio.current_task()}
            if pending:
                await asyncio.gather(*pending)

        _run_async(_cursor_and_drain())

        methods = _get_api_methods(mock_async_db_api)
        assert "Connection.cursor" in methods

    def test_close_sends_telemetry(self, async_connection, mock_async_db_api):
        mock_async_db_api.telemetry_send_api_usage.reset_mock()
        _run_async(async_connection.close())

        methods = _get_api_methods(mock_async_db_api)
        assert "Connection.close" in methods

    def test_commit_suppresses_inner_calls(self, async_connection, mock_async_db_api):
        mock_async_db_api.telemetry_send_api_usage.reset_mock()
        _run_async(async_connection.commit())

        methods = _get_api_methods(mock_async_db_api)
        assert "Connection.commit" in methods
        assert "Connection.cursor" not in methods
        assert "SnowflakeCursor.execute" not in methods
        assert "SnowflakeCursor.close" not in methods

    def test_rollback_suppresses_inner_calls(self, async_connection, mock_async_db_api):
        mock_async_db_api.telemetry_send_api_usage.reset_mock()
        _run_async(async_connection.rollback())

        methods = _get_api_methods(mock_async_db_api)
        assert "Connection.rollback" in methods
        assert "Connection.cursor" not in methods
        assert "SnowflakeCursor.execute" not in methods

    def test_api_method_uses_runtime_class_name(self, async_connection, mock_async_db_api):
        mock_async_db_api.telemetry_send_api_usage.reset_mock()
        _run_async(async_connection.close())

        req = mock_async_db_api.telemetry_send_api_usage.call_args[0][0]
        assert req.api_method == "Connection.close"


class TestAsyncCursorApiTelemetry:
    """Tests that aio SnowflakeCursor public methods send api_usage telemetry."""

    def test_execute_sends_telemetry(self, async_cursor, mock_async_db_api):
        _run_async(async_cursor.execute("SELECT 1"))

        methods = _get_api_methods(mock_async_db_api)
        assert "SnowflakeCursor.execute" in methods

    def test_close_sends_telemetry(self, async_cursor, mock_async_db_api):
        async def _close_and_drain():
            # close() is synchronous; on an async cursor its telemetry is
            # fire-and-forget (scheduled via create_task), so it only records
            # under a running loop. Drain the scheduled task before asserting.
            async_cursor.close()
            pending = asyncio.all_tasks() - {asyncio.current_task()}
            if pending:
                await asyncio.gather(*pending)

        _run_async(_close_and_drain())

        methods = _get_api_methods(mock_async_db_api)
        assert "SnowflakeCursor.close" in methods


class TestAsyncApiTelemetryFailureIsolation:
    """Tests that async telemetry failures don't break the decorated method."""

    def test_telemetry_rpc_failure_does_not_break_method(self, async_connection, mock_async_db_api):
        mock_async_db_api.telemetry_send_api_usage.side_effect = RuntimeError("telemetry down")

        _run_async(async_connection.close())
        assert async_connection.is_closed()


class TestPassedArgumentNames:
    """Unit tests for _passed_argument_names: names only, no values, no defaults."""

    @staticmethod
    def _names(func, *args, **kwargs):
        from snowflake.connector._internal.decorators import _passed_argument_names

        sig = inspect.signature(func)
        # The decorator binds the receiver first; mirror that by treating the
        # first positional as ``self``.
        self_obj, rest = args[0], args[1:]
        return _passed_argument_names(sig, self_obj, rest, kwargs)

    def test_only_passed_positional_and_keyword_named(self):
        def fn(self, command, parameters=None, num_statements=None): ...

        assert self._names(fn, object(), "SELECT 1") == ["command"]
        assert self._names(fn, object(), "SELECT 1", num_statements=2) == ["command", "num_statements"]

    def test_defaults_are_excluded(self):
        def fn(self, a, b=1, c=2): ...

        # b and c are left at their defaults -> omitted.
        assert self._names(fn, object(), "x") == ["a"]

    def test_explicitly_passed_value_equal_to_default_is_kept(self):
        def fn(self, a, b=None): ...

        # Caller supplied b explicitly (even though it equals the default).
        assert self._names(fn, object(), "x", b=None) == ["a", "b"]

    def test_self_is_dropped(self):
        def fn(self): ...

        assert self._names(fn, object()) == []

    def test_var_keyword_keys_are_expanded(self):
        def fn(self, **kwargs): ...

        # The var-keyword param name ("kwargs") carries no signal; expand to keys.
        names = self._names(fn, object(), account="a", user="u")
        assert "kwargs" not in names
        assert set(names) == {"account", "user"}

    def test_no_argument_values_are_captured(self):
        secret = "super-secret-password"

        def fn(self, password=None): ...

        names = self._names(fn, object(), password=secret)
        assert names == ["password"]
        assert secret not in names

    def test_binding_failure_returns_empty(self):
        def fn(self, a): ...

        # Too many positional args -> bind raises TypeError -> defensive [].
        assert self._names(fn, object(), "x", "y", "z") == []


class TestPassedArgumentsThroughStack:
    """End-to-end: argument names reach the TelemetrySendApiUsageRequest."""

    def test_connect_init_omits_unset_named_params(self, mock_db_api):
        """connect() must not forward defaulted connection_name/config kwargs."""
        from snowflake.connector import connect

        with patch("snowflake.connector._internal.cursor.query_result.get_stream_ptr", return_value=0):
            connect(user="test_user", account="test_account")

        init_calls = [
            list(call[0][0].passed_arguments)
            for call in mock_db_api.telemetry_send_api_usage.call_args_list
            if call[0][0].api_method == "Connection.__init__"
        ]
        assert len(init_calls) == 1
        passed = set(init_calls[0])
        assert passed == {"user", "account"}, f"unexpected Connection.__init__ api_arguments: {passed}"

    def test_connect_init_records_explicit_connection_name(self, mock_db_api):
        from snowflake.connector import connect

        with patch("snowflake.connector._internal.cursor.query_result.get_stream_ptr", return_value=0):
            connect(connection_name="myconn", user="test_user", account="test_account")

        init_calls = [
            list(call[0][0].passed_arguments)
            for call in mock_db_api.telemetry_send_api_usage.call_args_list
            if call[0][0].api_method == "Connection.__init__"
        ]
        assert len(init_calls) == 1
        assert "connection_name" in init_calls[0]

    def test_execute_records_only_passed_arguments(self, cursor, mock_db_api):
        cursor.execute("SELECT 1")
        assert _passed_arguments_for(mock_db_api, "SnowflakeCursor.execute") == ["operation"]

    def test_execute_records_extra_keyword(self, cursor, mock_db_api):
        cursor.execute("SELECT 1", num_statements=1)
        assert _passed_arguments_for(mock_db_api, "SnowflakeCursor.execute") == [
            "operation",
            "num_statements",
        ]

    def test_cursor_no_args_records_empty(self, connection, mock_db_api):
        mock_db_api.telemetry_send_api_usage.reset_mock()
        connection.cursor()
        assert _passed_arguments_for(mock_db_api, "Connection.cursor") == []


class TestApiTelemetryFreeFunction:
    """Tests for @api_telemetry applied to module-level free functions."""

    # ── helpers ────────────────────────────────────────────────────────────────

    @staticmethod
    def _decorate(func):
        from snowflake.connector._internal.decorators import api_telemetry

        return api_telemetry(func)

    # ── synchronous ────────────────────────────────────────────────────────────

    def test_sync_method_still_works_with_positional_self(self):
        """Regression guard: existing method paths must be unaffected by the fix."""

        class Worker:
            pass

        def run(self, x: int) -> int:
            return x * 2

        worker = Worker()
        assert self._decorate(run)(worker, 5) == 10

    def test_sync_free_function_called_with_keyword_args(self):
        def compute(conn, value: int) -> int:
            return value + 1

        # Before fix: TypeError — wrapper had `self` as first param with no match
        assert self._decorate(compute)(conn=object(), value=7) == 8

    def test_sync_free_function_called_with_positional_args(self):
        def compute(conn, value: int) -> int:
            return value + 1

        assert self._decorate(compute)(object(), 7) == 8

    def test_sync_free_function_called_with_mixed_args(self):
        def compute(conn, value: int, factor: int = 1) -> int:
            return value * factor

        assert self._decorate(compute)(object(), 3, factor=4) == 12

    def test_sync_free_function_tracking_resets_after_call(self):
        def compute(conn, value: int) -> int:
            return value + 1

        self._decorate(compute)(conn=object(), value=7)
        assert _TRACKING.get() is True

    def test_sync_free_function_tracking_resets_after_exception(self):
        def broken(conn) -> None:
            raise RuntimeError("boom")

        with pytest.raises(RuntimeError):
            self._decorate(broken)(conn=object())

        assert _TRACKING.get() is True

    def test_sync_nested_free_functions_inner_tracking_suppressed(self):
        """_TRACKING is False when the inner decorated function runs."""
        tracking_states: list[bool] = []

        def inner(conn) -> None:
            tracking_states.append(_TRACKING.get())

        def outer(conn) -> None:
            self._decorate(inner)(conn=conn)

        self._decorate(outer)(conn=object())
        assert tracking_states == [False]
        assert _TRACKING.get() is True

    def test_sync_free_function_generator_yields_all_values(self):
        def produce(conn, n: int):
            yield from range(n)

        assert list(self._decorate(produce)(conn=object(), n=3)) == [0, 1, 2]

    def test_sync_free_function_generator_tracking_resets_after_exhaustion(self):
        def produce(conn, n: int):
            yield from range(n)

        list(self._decorate(produce)(object(), 3))
        assert _TRACKING.get() is True

    # ── async coroutine ────────────────────────────────────────────────────────

    def test_async_free_function_awaited_with_keyword_args(self):
        async def fetch(conn, query: str) -> list:
            return []

        assert _run_async(self._decorate(fetch)(conn=object(), query="SELECT 1")) == []

    def test_async_free_function_awaited_with_positional_args(self):
        async def fetch(conn, query: str) -> list:
            return []

        assert _run_async(self._decorate(fetch)(object(), "SELECT 1")) == []

    def test_async_free_function_tracking_resets_after_await(self):
        async def fetch(conn, query: str) -> list:
            return []

        _run_async(self._decorate(fetch)(conn=object(), query="SELECT 1"))
        assert _TRACKING.get() is True

    def test_async_free_function_tracking_resets_after_exception(self):
        async def broken(conn) -> None:
            raise RuntimeError("boom")

        with pytest.raises(RuntimeError):
            _run_async(self._decorate(broken)(conn=object()))

        assert _TRACKING.get() is True

    def test_async_nested_free_functions_inner_tracking_suppressed(self):
        tracking_states: list[bool] = []

        async def inner(conn) -> None:
            tracking_states.append(_TRACKING.get())

        async def outer(conn) -> None:
            await self._decorate(inner)(conn=conn)

        _run_async(self._decorate(outer)(conn=object()))
        assert tracking_states == [False]
        assert _TRACKING.get() is True

    # ── async generator ────────────────────────────────────────────────────────

    def test_async_generator_free_function_yields_with_keyword_args(self):
        async def stream(conn, n: int):
            for i in range(n):
                yield i

        async def collect():
            return [v async for v in self._decorate(stream)(conn=object(), n=3)]

        assert _run_async(collect()) == [0, 1, 2]

    def test_async_generator_free_function_yields_with_positional_args(self):
        async def stream(conn, n: int):
            for i in range(n):
                yield i

        async def collect():
            return [v async for v in self._decorate(stream)(object(), 3)]

        assert _run_async(collect()) == [0, 1, 2]

    def test_async_generator_free_function_tracking_resets_after_exhaustion(self):
        async def stream(conn, n: int):
            for i in range(n):
                yield i

        async def collect():
            return [v async for v in self._decorate(stream)(object(), 3)]

        _run_async(collect())
        assert _TRACKING.get() is True


class TestAsyncExpired:
    """Unit tests for ``aio.Connection.is_expired()``.

    Mirrors the sync ``TestExpired`` in test_connection.py. Now a sync method
    backed by the sync core_driver.
    """

    def test_returns_false_for_fresh_connection(self, async_connection, mock_async_db_api):
        """A fresh async connection must report is_expired() == False."""
        sync_api = mock_async_db_api._sync_db_api
        sync_api.connection_is_expired.return_value = ConnectionIsExpiredResponse(is_expired=False)
        assert async_connection.is_expired() is False
        sync_api.connection_is_expired.assert_called_once()

    def test_returns_true_when_core_reports_expired(self, async_connection, mock_async_db_api):
        """is_expired() == True is forwarded from sf_core."""
        sync_api = mock_async_db_api._sync_db_api
        sync_api.connection_is_expired.return_value = ConnectionIsExpiredResponse(is_expired=True)
        assert async_connection.is_expired() is True

    def test_returns_true_on_exception(self, async_connection, mock_async_db_api):
        """If the RPC raises, is_expired() fails closed and returns True rather than
        propagating — the connection may be unusable, so callers treat it as expired."""
        sync_api = mock_async_db_api._sync_db_api
        sync_api.connection_is_expired.side_effect = RuntimeError("handle gone")
        assert async_connection.is_expired() is True

    def test_conn_handle_none_returns_false(self, async_connection, mock_async_db_api):
        """conn_handle=None (pre-connect or post-release) must return False immediately."""
        sync_api = mock_async_db_api._sync_db_api
        async_connection.conn_handle = None
        assert async_connection.is_expired() is False
        sync_api.connection_is_expired.assert_not_called()

    def test_returns_bool(self, async_connection, mock_async_db_api):
        """is_expired() must return a plain Python bool, not a protobuf bool."""
        sync_api = mock_async_db_api._sync_db_api
        sync_api.connection_is_expired.return_value = ConnectionIsExpiredResponse(is_expired=True)
        result = async_connection.is_expired()
        assert type(result) is bool
