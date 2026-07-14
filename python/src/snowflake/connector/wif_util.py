"""BACKWARD COMPATIBILITY MODULE ONLY — stub for Snowpark wif_util imports."""

from __future__ import annotations

from typing import Any

from ._internal.decorators import api_telemetry, snowpark_compat
from .errors import NotSupportedError


@snowpark_compat
@api_telemetry
def create_attestation(*args: Any, **kwargs: Any) -> dict[str, Any]:
    """Raise NotSupportedError — workload identity federation is not yet implemented."""
    raise NotSupportedError("Workload identity federation is not yet supported by the Universal Driver.")
