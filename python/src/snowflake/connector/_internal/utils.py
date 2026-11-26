"""General-purpose internal utilities shared across subsystems."""

from __future__ import annotations

import inspect
import os
import sys
import warnings

from collections.abc import Iterable, Sequence
from types import FrameType
from typing import Any, cast

from ..errors import ProgrammingError
from .errorcode import ER_INVALID_VALUE


_CONNECTOR_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
_LEGACY_STATEMENT_PARAMS_WARNING = "'_statement_params' is deprecated; use 'statement_parameters' instead."


_ExecutemanyRow = Sequence[Any] | dict[str, Any]


def get_application_path() -> str | None:
    # Outermost frame of the calling thread, as in the old connector: reports threading.py from a
    # worker thread and "<stdin>" from a REPL.
    try:
        return inspect.stack(context=0)[-1].filename
    except Exception:
        return None


def _resolve_alias(
    canonical: object,
    alias: object,
    canonical_name: str,
    alias_name: str,
) -> object:
    """Return the resolved value from a canonical/legacy-alias pair.

    Raises ProgrammingError if both are provided.
    """
    if canonical is not None and alias is not None:
        raise ProgrammingError(
            msg=f"Cannot supply both '{canonical_name}' and '{alias_name}'; pass one only.",
            errno=ER_INVALID_VALUE,
        )
    return alias if alias is not None else canonical


def _attach_query(exc: ProgrammingError, query: str) -> None:
    """Record the SQL that was sent on a query error that does not carry it.

    The core driver does not return the query text with an error. The old
    driver set ``query`` on every server error, and callers such as Snowpark
    use it to tell which statement failed.
    """
    if exc.query is None:
        exc.query = query


def _warning_stacklevel_outside_connector() -> int:
    connector_prefix = _CONNECTOR_DIR + os.sep
    level = 1
    frame: FrameType | None = sys._getframe(1)
    while frame is not None:
        filename = frame.f_code.co_filename
        if filename and not filename.startswith("<") and not os.path.abspath(filename).startswith(connector_prefix):
            return level
        level += 1
        frame = frame.f_back
    return level


def _resolve_statement_parameters_arg(
    statement_parameters: dict[str, Any] | None,
    legacy_statement_params: dict[str, Any] | None,
) -> dict[str, Any] | None:
    """Return the per-call statement-parameter bag from the public argument.

    ``_statement_params`` is the legacy name. It warns, and it is used only
    when ``statement_parameters`` was not passed.
    """
    if legacy_statement_params is not None:
        warnings.warn(
            _LEGACY_STATEMENT_PARAMS_WARNING,
            DeprecationWarning,
            stacklevel=_warning_stacklevel_outside_connector(),
        )
        if statement_parameters is None:
            return legacy_statement_params
    return statement_parameters


def _rewrite_legacy_statement_params_kwargs(kwargs: dict[str, Any]) -> None:
    """Rewrites ``_statement_params`` in ``kwargs`` to ``statement_parameters``."""
    resolved = _resolve_statement_parameters_arg(
        kwargs.pop("statement_parameters", None),
        kwargs.pop("_statement_params", None),
    )
    if resolved is not None:
        kwargs["statement_parameters"] = resolved


def _coerce_executemany_params(seq_of_parameters: object) -> Sequence[_ExecutemanyRow]:
    """Return a random-access sequence of ``executemany`` parameter rows.

    Array binding peeks at the first row, transposes every row, then may walk the
    same collection again on the per-row fallback. Non-sequences (generators,
    ``map``, one-shot iterators) are therefore materialized once. Lists and
    tuples are left unchanged.
    """
    if seq_of_parameters is None:
        return ()
    if isinstance(seq_of_parameters, Sequence):
        return seq_of_parameters
    if not isinstance(seq_of_parameters, Iterable):
        raise ProgrammingError(
            msg=(
                "executemany() seq_of_parameters must be an iterable of parameter "
                f"sequences or dicts, got {type(seq_of_parameters).__name__}. "
                "Pass a list, tuple, or other iterable of rows."
            ),
            errno=ER_INVALID_VALUE,
        )
    return cast("Sequence[_ExecutemanyRow]", list(seq_of_parameters))
