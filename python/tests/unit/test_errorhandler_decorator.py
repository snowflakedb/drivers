"""Unit tests for the @with_errorhandler decorator and ErrorHandlerMixin."""

from __future__ import annotations

import pytest

from snowflake.connector._internal.decorators import with_errorhandler
from snowflake.connector._internal.errorhandler import ErrorHandlerMixin, _errorhandler_active, route_exception
from snowflake.connector.errors import Error, ErrorValue, InterfaceError, ProgrammingError


# ---------------------------------------------------------------------------
# Test fixtures: minimal decorated classes
# ---------------------------------------------------------------------------


@with_errorhandler
class _FakeConnection(ErrorHandlerMixin):
    """Simulates a Connection: _errorhandler_cursor is always None."""

    def __init__(self):
        self.messages: list[tuple[type[Exception], ErrorValue]] = []
        self.errorhandler = Error.default_errorhandler

    @property
    def _errorhandler_connection(self):
        return self

    @property
    def _errorhandler_cursor(self):
        return None

    def do_something(self):
        raise ProgrammingError(msg="bad SQL", errno=42)

    def do_something_else(self):
        raise ValueError("not a snowflake error")

    def succeed(self):
        return "ok"

    def calls_other_public(self):
        return self.succeed()

    def calls_failing_public(self):
        self.do_something()


@with_errorhandler
class _FakeCursor(ErrorHandlerMixin):
    """Simulates a Cursor: routes through both connection and cursor."""

    def __init__(self, connection: _FakeConnection):
        self._connection = connection
        self.messages: list[tuple[type[Exception], ErrorValue]] = []
        self.errorhandler = Error.default_errorhandler

    @property
    def _errorhandler_connection(self):
        return self._connection

    @property
    def _errorhandler_cursor(self):
        return self

    def fetch(self):
        raise InterfaceError(msg="cursor is closed", errno=1)

    def ok(self):
        return 42

    def calls_fetch(self):
        return self.fetch()


# ---------------------------------------------------------------------------
# Basic behavior
# ---------------------------------------------------------------------------


class TestWithErrorhandlerBasics:
    def test_successful_method_returns_value(self):
        conn = _FakeConnection()
        assert conn.succeed() == "ok"

    def test_error_raised_with_default_handler(self):
        conn = _FakeConnection()
        with pytest.raises(ProgrammingError, match="bad SQL"):
            conn.do_something()

    def test_messages_populated_on_error(self):
        conn = _FakeConnection()
        with pytest.raises(ProgrammingError):
            conn.do_something()
        assert len(conn.messages) == 1
        error_class, error_value = conn.messages[0]
        assert error_class is ProgrammingError
        assert error_value["msg"] == "bad SQL"
        assert error_value["errno"] == 42

    def test_non_error_exceptions_propagate_unmodified(self):
        conn = _FakeConnection()
        with pytest.raises(ValueError, match="not a snowflake error"):
            conn.do_something_else()
        assert len(conn.messages) == 0

    def test_cursor_error_records_on_both(self):
        conn = _FakeConnection()
        cursor = _FakeCursor(conn)
        with pytest.raises(InterfaceError, match="cursor is closed"):
            cursor.fetch()
        assert len(cursor.messages) == 1
        assert len(conn.messages) == 1


# ---------------------------------------------------------------------------
# Custom errorhandler (swallowing / replacing errors)
# ---------------------------------------------------------------------------


class TestCustomErrorhandler:
    def test_custom_handler_is_called_before_reraise(self):
        """A custom handler is invoked but the original error is always re-raised."""
        conn = _FakeConnection()
        observed = []
        conn.errorhandler = lambda c, cur, cls, val: observed.append((cls, val))

        with pytest.raises(ProgrammingError, match="bad SQL"):
            conn.do_something()
        assert len(observed) == 1
        assert observed[0][0] is ProgrammingError

    def test_custom_handler_can_replace_error(self):
        """A custom handler can raise a different exception."""
        conn = _FakeConnection()

        def handler(c, cur, cls, val):
            raise RuntimeError("replaced")

        conn.errorhandler = handler

        with pytest.raises(RuntimeError, match="replaced"):
            conn.do_something()

    def test_cursor_custom_handler_takes_precedence(self):
        conn = _FakeConnection()
        cursor = _FakeCursor(conn)
        captured = []
        cursor.errorhandler = lambda c, cur, cls, val: captured.append("cursor")
        conn.errorhandler = lambda c, cur, cls, val: captured.append("conn")

        with pytest.raises(InterfaceError):
            cursor.fetch()
        assert captured == ["cursor"]


# ---------------------------------------------------------------------------
# Re-entrancy (public calling public) via ContextVar
# ---------------------------------------------------------------------------


class TestReentrancy:
    def test_public_calling_public_success(self):
        conn = _FakeConnection()
        assert conn.calls_other_public() == "ok"

    def test_public_calling_failing_public_routes_once(self):
        """When calls_failing_public -> do_something raises, the error
        should be routed exactly once (at the calls_failing_public boundary),
        not double-handled."""
        conn = _FakeConnection()
        with pytest.raises(ProgrammingError, match="bad SQL"):
            conn.calls_failing_public()
        assert len(conn.messages) == 1

    def test_cursor_public_calling_public(self):
        conn = _FakeConnection()
        cursor = _FakeCursor(conn)
        with pytest.raises(InterfaceError, match="cursor is closed"):
            cursor.calls_fetch()
        assert len(cursor.messages) == 1
        assert len(conn.messages) == 1

    def test_context_var_is_clean_after_success(self):
        conn = _FakeConnection()
        conn.succeed()
        assert _errorhandler_active.get() is False

    def test_context_var_is_clean_after_error(self):
        conn = _FakeConnection()
        with pytest.raises(ProgrammingError):
            conn.do_something()
        assert _errorhandler_active.get() is False


# ---------------------------------------------------------------------------
# Decorator skips private methods, properties, staticmethods, classmethods
# ---------------------------------------------------------------------------


@with_errorhandler
class _MixedClass(ErrorHandlerMixin):
    def __init__(self):
        self.messages: list = []
        self.errorhandler = Error.default_errorhandler

    @property
    def _errorhandler_connection(self):
        return self

    @property
    def _errorhandler_cursor(self):
        return None

    @property
    def my_prop(self):
        raise ProgrammingError(msg="from property")

    def _private_method(self):
        raise ProgrammingError(msg="from private")

    @staticmethod
    def static_method():
        raise ProgrammingError(msg="from static")

    @classmethod
    def class_method(cls):
        raise ProgrammingError(msg="from classmethod")

    def public_method(self):
        raise ProgrammingError(msg="from public")


class TestDecoratorScoping:
    def test_property_not_wrapped(self):
        obj = _MixedClass()
        with pytest.raises(ProgrammingError, match="from property"):
            _ = obj.my_prop
        assert len(obj.messages) == 0

    def test_private_method_not_wrapped(self):
        obj = _MixedClass()
        with pytest.raises(ProgrammingError, match="from private"):
            obj._private_method()
        assert len(obj.messages) == 0

    def test_static_method_not_wrapped(self):
        with pytest.raises(ProgrammingError, match="from static"):
            _MixedClass.static_method()

    def test_classmethod_not_wrapped(self):
        with pytest.raises(ProgrammingError, match="from classmethod"):
            _MixedClass.class_method()

    def test_public_method_is_wrapped(self):
        obj = _MixedClass()
        with pytest.raises(ProgrammingError, match="from public"):
            obj.public_method()
        assert len(obj.messages) == 1


# ---------------------------------------------------------------------------
# Generator methods are not wrapped
# ---------------------------------------------------------------------------


@with_errorhandler
class _GeneratorClass(ErrorHandlerMixin):
    def __init__(self):
        self.messages: list = []
        self.errorhandler = Error.default_errorhandler

    @property
    def _errorhandler_connection(self):
        return self

    @property
    def _errorhandler_cursor(self):
        return None

    def gen(self):
        yield 1
        raise ProgrammingError(msg="mid-generator")


class TestGeneratorNotWrapped:
    def test_generator_errors_propagate_directly(self):
        obj = _GeneratorClass()
        g = obj.gen()
        assert next(g) == 1
        with pytest.raises(ProgrammingError, match="mid-generator"):
            next(g)
        assert len(obj.messages) == 0


# ---------------------------------------------------------------------------
# _route_exception
# ---------------------------------------------------------------------------


class TestRouteException:
    def test_no_handler_re_raises(self):
        exc = ProgrammingError(msg="test", errno=99)
        with pytest.raises(ProgrammingError, match="test"):
            route_exception(None, None, exc)

    def test_with_connection_handler(self):
        conn = _FakeConnection()
        captured: list[tuple] = []
        conn.errorhandler = lambda c, cur, cls, val: captured.append((cls, val))

        exc = ProgrammingError(msg="routed", errno=7)
        with pytest.raises(ProgrammingError, match="routed"):
            route_exception(conn, None, exc)
        assert len(captured) == 1
        assert captured[0][0] is ProgrammingError

    def test_with_cursor_handler(self):
        conn = _FakeConnection()
        cursor = _FakeCursor(conn)
        captured: list[tuple] = []
        cursor.errorhandler = lambda c, cur, cls, val: captured.append((cls, val))

        exc = InterfaceError(msg="cursor err")
        with pytest.raises(InterfaceError, match="cursor err"):
            route_exception(conn, cursor, exc)
        assert len(captured) == 1
        assert captured[0][0] is InterfaceError

    def test_error_value_includes_all_fields(self):
        """Verify that route_exception passes all Error fields to the handler."""
        conn = _FakeConnection()
        captured_values: list[dict] = []
        conn.errorhandler = lambda c, cur, cls, val: captured_values.append(val)

        exc = ProgrammingError(msg="full test", errno=42, sqlstate="HY000", sfqid="qid-123", query="SELECT 1")
        with pytest.raises(ProgrammingError):
            route_exception(conn, None, exc)

        assert len(captured_values) == 1
        val = captured_values[0]
        assert val["msg"] == "full test"
        assert val["errno"] == 42
        assert val["sqlstate"] == "HY000"
        assert val["sfqid"] == "qid-123"
        assert val["query"] == "SELECT 1"
