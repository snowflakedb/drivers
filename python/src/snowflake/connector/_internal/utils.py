"""General-purpose internal utilities shared across subsystems."""

from __future__ import annotations

from ..errors import ProgrammingError
from .errorcode import ER_INVALID_VALUE


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
