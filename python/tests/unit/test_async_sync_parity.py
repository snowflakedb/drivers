"""Signature parity tests between sync and async cursor / connection classes.

The sync cursor and connection classes are the source of truth. Each test here
asserts that the async counterpart exposes the same parameter names, defaults,
and kinds for every shared public method.

When a sync method is intentionally absent from the async surface (e.g.
``connect``, ``fetch_info`` on AsyncConnection), add it to the
appropriate ``_ASYNC_ONLY_*`` set below rather than adjusting the assertion.
"""

from __future__ import annotations

import inspect

from typing import Any

import pytest

from snowflake.connector.aio.connection import Connection as AsyncConnection
from snowflake.connector.aio.cursor import SnowflakeCursor as AsyncCursor
from snowflake.connector.connection import Connection as SyncConnection
from snowflake.connector.cursor import SnowflakeCursor as SyncCursor


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _method_params(cls: type, method_name: str) -> list[tuple[str, Any]]:
    """Return (name, kind) for each parameter of *method_name*, excluding ``self``.

    Intentional differences in *default values* (e.g. ``cursor_class`` defaulting
    to the sync vs. async cursor type) are not checked here; use targeted tests
    for specific parameter defaults.
    """
    method = getattr(cls, method_name)
    sig = inspect.signature(method)
    return [(name, p.kind) for name, p in sig.parameters.items() if name != "self"]


def _public_methods(cls: type) -> set[str]:
    """Return names of all public non-dunder methods defined on *cls* (any MRO level)."""
    return {name for name, obj in inspect.getmembers(cls, predicate=inspect.isfunction) if not name.startswith("_")}


# ---------------------------------------------------------------------------
# Cursor parity
# ---------------------------------------------------------------------------

# Methods present only on one side due to deliberate design decisions.
# Keep these sets lean; every addition should have a comment explaining why.
_CURSOR_SYNC_ONLY: set[str] = {
    # `download_stream` is implemented on the sync cursor only.
    "download_stream",
}
_CURSOR_ASYNC_ONLY: set[str] = set()  # none today

_CURSOR_COMMON = (_public_methods(SyncCursor) | _public_methods(AsyncCursor)) - (_CURSOR_SYNC_ONLY | _CURSOR_ASYNC_ONLY)

# Parameters that exist in the *sync* cursor but are intentionally absent from
# the async cursor. Key = method name, value = set of parameter names to skip
# when comparing. Document the reason inline.
_CURSOR_PARAM_EXCLUSIONS: dict[str, set[str]] = {
    # `params` is a deprecated alias for `parameters`; the aio implementation
    # is a fresh one and intentionally does not carry the deprecated alias.
    "execute": {"params"},
}


class TestCursorSignatureParity:
    """Every public cursor method shared by sync and async must have identical parameters."""

    @pytest.mark.parametrize("method_name", sorted(_CURSOR_COMMON))
    def test_cursor_method_parameters_match(self, method_name: str) -> None:
        excluded = _CURSOR_PARAM_EXCLUSIONS.get(method_name, set())
        sync_params = [(n, k) for n, k in _method_params(SyncCursor, method_name) if n not in excluded]
        async_params = _method_params(AsyncCursor, method_name)
        assert sync_params == async_params, (
            f"Cursor.{method_name} parameter names/kinds mismatch.\n"
            f"  sync  : {[p[0] for p in sync_params]}\n"
            f"  async : {[p[0] for p in async_params]}"
        )


# ---------------------------------------------------------------------------
# Connection parity
# ---------------------------------------------------------------------------

# Methods that intentionally exist only on AsyncConnection (not in sync).
_CONN_ASYNC_ONLY: set[str] = {
    "connect",  # async Connection has a separate I/O init step; sync connects in __init__
    "fetch_info",  # async-only convenience wrapper around _connection_info
    "snowflake_version",  # sync exposes this as a @cached_property; async exposes as a coroutine
}
# Methods that intentionally exist only on SyncConnection (not in async).
_CONN_SYNC_ONLY: set[str] = set()  # none today

_CONN_COMMON = (_public_methods(SyncConnection) | _public_methods(AsyncConnection)) - (
    _CONN_ASYNC_ONLY | _CONN_SYNC_ONLY
)


class TestConnectionSignatureParity:
    """Every shared public connection method must have identical parameters."""

    @pytest.mark.parametrize("method_name", sorted(_CONN_COMMON))
    def test_connection_method_parameters_match(self, method_name: str) -> None:
        sync_params = _method_params(SyncConnection, method_name)
        async_params = _method_params(AsyncConnection, method_name)
        assert sync_params == async_params, (
            f"Connection.{method_name} parameter names/kinds mismatch.\n"
            f"  sync  : {[p[0] for p in sync_params]}\n"
            f"  async : {[p[0] for p in async_params]}"
        )
