"""BACKWARD COMPATIBILITY MODULE ONLY — stub for Snowpark wif_util imports."""

from __future__ import annotations

from typing import Any

from ._internal.decorators import snowpark_compat


@snowpark_compat
def create_attestation(*args: Any, **kwargs: Any) -> dict[str, Any]:
    """Noop stub — workload identity federation is not yet supported by the UD."""
    return {}
