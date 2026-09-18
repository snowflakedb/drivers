"""General-purpose internal utilities shared across subsystems."""

from __future__ import annotations

from collections.abc import Iterable, Sequence
from typing import Any, cast

from ..errors import ProgrammingError
from .errorcode import ER_INVALID_VALUE


_ExecutemanyRow = Sequence[Any] | dict[str, Any]


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
