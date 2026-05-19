"""Shared cursor logic extracted into a mixin.

``CursorMixin`` contains everything that does **not** differ between the
sync (:class:`SnowflakeCursorBase`) and async (:class:`AsyncSnowflakeCursorBase`)
cursor hierarchies: read-only properties, PEP 249 no-ops, session-parameter
accessors, error-handler wiring, and query-preparation utilities.
"""

from __future__ import annotations

import abc
import asyncio
import ctypes
import functools
import logging

from collections.abc import Callable, Sequence
from typing import TYPE_CHECKING, Any, TypeVar, cast

from .._internal.binding_converters import (
    ClientSideBindingConverter,
    JsonBindingConverter,
    ParamStyle,
)
from .._internal.decorators import api_telemetry, pep249
from .._internal.errorcode import ER_CURSOR_IS_CLOSED, ER_INVALID_VALUE
from .._internal.extras import check_dependency, pandas, pyarrow
from .._internal.protobuf_gen.database_driver_v1_pb2 import (
    BinaryDataPtr,
    QueryBindings,
)
from ..errors import Error, ErrorValue, InterfaceError, NotSupportedError, ProgrammingError
from ._query_result import _MultiStatementQueryResultState, _QueryResult
from ._result_metadata import QueryResultStats, ResultMetadata


if TYPE_CHECKING:
    from ..connection import Connection

logger = logging.getLogger(__name__)

Row = tuple[Any, ...]
DictRow = dict[str, Any]

F = TypeVar("F", bound=Callable[..., Any])


# ------------------------------------------------------------------
# Guard decorators — async-transparent
# ------------------------------------------------------------------


def _requires_open(func: F) -> F:
    if asyncio.iscoroutinefunction(func):

        @functools.wraps(func)
        async def async_wrapper(self: CursorMixin, *args: Any, **kwargs: Any) -> Any:
            if self.is_closed():
                raise InterfaceError(msg="Cursor is closed.", errno=ER_CURSOR_IS_CLOSED)
            return await func(self, *args, **kwargs)

        return cast(F, async_wrapper)

    @functools.wraps(func)
    def wrapper(self: CursorMixin, *args: Any, **kwargs: Any) -> Any:
        if self.is_closed():
            raise InterfaceError(msg="Cursor is closed.", errno=ER_CURSOR_IS_CLOSED)
        return func(self, *args, **kwargs)

    return cast(F, wrapper)


def _requires_open_cursor_not_connection(func: F) -> F:
    """Guard that only checks ``self._closed``, ignoring the connection state.

    Unlike ``_requires_open`` (which delegates to ``is_closed()`` and therefore
    also rejects cursors whose *connection* has been closed), this decorator
    deliberately skips the connection check.  This preserves backward
    compatibility with the old driver, where fetch methods on a cursor with
    already-buffered results still worked after ``connection.close()``.
    """
    if asyncio.iscoroutinefunction(func):

        @functools.wraps(func)
        async def async_wrapper(self: CursorMixin, *args: Any, **kwargs: Any) -> Any:
            if self._closed:
                raise InterfaceError(msg="Cursor is closed.", errno=ER_CURSOR_IS_CLOSED)
            return await func(self, *args, **kwargs)

        return cast(F, async_wrapper)

    @functools.wraps(func)
    def wrapper(self: CursorMixin, *args: Any, **kwargs: Any) -> Any:
        if self._closed:
            raise InterfaceError(msg="Cursor is closed.", errno=ER_CURSOR_IS_CLOSED)
        return func(self, *args, **kwargs)

    return cast(F, wrapper)


def _with_prefetch_hook(func: F) -> F:
    """Invoke the cursor's prefetch hook (if set) before entering the wrapped method."""
    if asyncio.iscoroutinefunction(func):

        @functools.wraps(func)
        async def async_wrapper(self: CursorMixin, *args: Any, **kwargs: Any) -> Any:
            if self._prefetch_hook is not None:
                hook = self._prefetch_hook
                if asyncio.iscoroutinefunction(hook):
                    await hook()
                else:
                    hook()
            return await func(self, *args, **kwargs)

        return cast(F, async_wrapper)

    @functools.wraps(func)
    def wrapper(self: CursorMixin, *args: Any, **kwargs: Any) -> Any:
        if self._prefetch_hook is not None:
            self._prefetch_hook()
        return func(self, *args, **kwargs)

    return cast(F, wrapper)


# ------------------------------------------------------------------
# Mixin
# ------------------------------------------------------------------


class CursorMixin(abc.ABC):
    """Shared state, properties, and utilities for sync and async cursors.

    Concrete cursor base classes (:class:`SnowflakeCursorBase`,
    :class:`AsyncSnowflakeCursorBase`) inherit from this mixin and add
    their own I/O methods (execute, fetch, close, reset, etc.).
    """

    def _init_cursor_mixin(self, connection: Connection) -> None:
        """Initialize shared cursor state. Called from subclass ``__init__``."""
        self._connection: Connection = connection
        self._closed: bool = False
        self._messages: list[tuple[type[Exception], ErrorValue]] = []
        self._errorhandler: Callable[..., None] = Error.default_errorhandler

        self._arraysize: int = 1

        self._query_result: _QueryResult = _QueryResult()
        self._rownumber: int = -1

        self._multi_statement: _MultiStatementQueryResultState | None = None

        self._statement_parameters: dict[str, Any] = {}

        self._binding_data: None | bytes = None
        self._prefetch_hook: Callable[..., Any] | None = None

    # ------------------------------------------------------------------
    # PEP 249 attributes
    # ------------------------------------------------------------------

    @property
    @pep249
    def connection(self) -> Connection:
        return self._connection

    @property
    @pep249
    def description(self) -> list[ResultMetadata] | None:
        return self._query_result.description

    @property
    @pep249
    def rowcount(self) -> int | None:
        return self._query_result.rowcount

    @property
    @pep249
    def arraysize(self) -> int:
        return self._arraysize

    @arraysize.setter
    def arraysize(self, value: int) -> None:
        self._arraysize = int(value)

    @property
    @pep249
    def messages(self) -> list[tuple[type[Exception], ErrorValue]]:
        return self._messages

    @messages.setter
    def messages(self, value: list[tuple[type[Exception], ErrorValue]]) -> None:
        self._messages = value

    @property
    @abc.abstractmethod
    def _use_dict_result(self) -> bool:
        """Whether fetch methods return dicts instead of tuples."""

    @property
    def query(self) -> str | None:
        return self._query_result.query

    @property
    def sfqid(self) -> str | None:
        return self._query_result.sfqid

    @property
    def stats(self) -> QueryResultStats:
        return self._query_result.stats

    @property
    @pep249
    def rownumber(self) -> int | None:
        return self._rownumber if self._rownumber >= 0 else None

    @property
    def sqlstate(self) -> str | None:
        return self._query_result.sqlstate

    @property
    def multi_statement_parent_sfqid(self) -> str | None:
        return self._multi_statement.parent_qid if self._multi_statement else None

    @property
    def multi_statement_savedIds(self) -> list[str]:
        return self._multi_statement.child_query_ids if self._multi_statement else []

    @property
    @pep249
    def lastrowid(self) -> None:
        return None

    # ------------------------------------------------------------------
    # Lifecycle
    # ------------------------------------------------------------------

    def is_closed(self) -> bool:
        return self._closed or self._connection.is_closed()

    # ------------------------------------------------------------------
    # PEP 249 no-ops
    # ------------------------------------------------------------------

    @pep249
    def setinputsizes(self, sizes: Sequence[Any]) -> None:
        return None

    @pep249
    def setoutputsize(self, size: int, column: int | None = None) -> None:
        return None

    @pep249
    @api_telemetry
    def scroll(self, value: int, mode: str = "relative") -> None:
        raise NotSupportedError("scroll is not supported")

    # ------------------------------------------------------------------
    # Statement parameters
    # ------------------------------------------------------------------

    @_requires_open
    def set_statement_parameter(self, key: str, value: Any) -> None:
        self._statement_parameters[key] = value

    @property
    def is_file_transfer(self) -> bool:
        raise NotImplementedError("is_file_transfer is not yet implemented")

    # ------------------------------------------------------------------
    # Query preparation (shared between sync and async)
    # ------------------------------------------------------------------

    def _format_query_for_log(self, query: str) -> str:
        return self._connection._format_query_for_log(query)

    def _build_query_bindings(self, parameters: Sequence[Any]) -> QueryBindings | None:
        json_str, length = JsonBindingConverter.serialize_parameters(parameters)
        if json_str is None:
            return None

        json_bytes = json_str.encode("utf-8")
        self._binding_data = json_bytes

        ptr_value = ctypes.cast(ctypes.c_char_p(json_bytes), ctypes.c_void_p).value
        if ptr_value is None:
            raise RuntimeError("Failed to obtain memory pointer for binding data")

        ptr_bytes = ptr_value.to_bytes(8, byteorder="little", signed=False)

        binary_data_ptr = BinaryDataPtr(
            value=ptr_bytes,
            length=length,
        )
        return QueryBindings(json=binary_data_ptr)

    def _prepare_query(
        self, operation: str, parameters: Sequence[Any] | dict[str, Any] | None
    ) -> tuple[str, QueryBindings | None]:
        if parameters is None:
            return operation, None

        paramstyle = self._connection.paramstyle

        if paramstyle.is_client_side():
            if paramstyle == ParamStyle.FORMAT and isinstance(parameters, dict):
                raise ProgrammingError(
                    msg="Dict parameters not supported with format paramstyle. "
                    "Use pyformat paramstyle for named parameters, or use a sequence.",
                    errno=ER_INVALID_VALUE,
                )
            query = ClientSideBindingConverter.interpolate_query(
                operation,
                parameters,
                interpolate_empty_sequences=self._connection._interpolate_empty_sequences,
            )
            return query, None
        else:
            if isinstance(parameters, dict):
                raise ProgrammingError(
                    msg="Named parameters (dict) not supported with qmark/numeric paramstyle. "
                    "Use pyformat paramstyle for named parameters.",
                    errno=ER_INVALID_VALUE,
                )
            bindings = self._build_query_bindings(parameters)
            return operation, bindings

    # ------------------------------------------------------------------
    # Session parameter accessors
    # ------------------------------------------------------------------

    @property
    def timestamp_output_format(self) -> str | None:
        return self._connection._get_session_parameter("TIMESTAMP_OUTPUT_FORMAT")

    @property
    def timestamp_ltz_output_format(self) -> str | None:
        return self._connection._get_session_parameter("TIMESTAMP_LTZ_OUTPUT_FORMAT") or self.timestamp_output_format

    @property
    def timestamp_tz_output_format(self) -> str | None:
        return self._connection._get_session_parameter("TIMESTAMP_TZ_OUTPUT_FORMAT") or self.timestamp_output_format

    @property
    def timestamp_ntz_output_format(self) -> str | None:
        return self._connection._get_session_parameter("TIMESTAMP_NTZ_OUTPUT_FORMAT") or self.timestamp_output_format

    @property
    def date_output_format(self) -> str | None:
        return self._connection._get_session_parameter("DATE_OUTPUT_FORMAT")

    @property
    def time_output_format(self) -> str | None:
        return self._connection._get_session_parameter("TIME_OUTPUT_FORMAT")

    @property
    def timezone(self) -> str | None:
        return self._connection._get_session_parameter("TIMEZONE")

    @property
    def binary_output_format(self) -> str | None:
        return self._connection._get_session_parameter("BINARY_OUTPUT_FORMAT")

    # ------------------------------------------------------------------
    # Error handling
    # ------------------------------------------------------------------

    @property
    @pep249
    def errorhandler(self) -> Callable:
        return self._errorhandler

    @errorhandler.setter
    def errorhandler(self, value: Callable | None) -> None:
        if value is None:
            raise ProgrammingError("Invalid errorhandler is specified")
        self._errorhandler = value

    @property
    def _errorhandler_connection(self) -> Connection:
        return self._connection

    # ------------------------------------------------------------------
    # Dependency checks
    # ------------------------------------------------------------------

    def check_can_use_arrow_resultset(self) -> None:
        check_dependency(pyarrow)

    def check_can_use_pandas(self) -> None:
        check_dependency(pandas)
