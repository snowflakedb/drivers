"""BACKWARD COMPATIBILITY MODULE ONLY.

The legacy ``snowflake-connector-python`` driver's HTTP/session-renewal layer
lived in ``network.py`` and defined ``ReauthenticationRequest`` there (not in
``errors.py``). Some consumers import it from this module path directly —
e.g. Snowpark's ``snowflake.snowpark._internal.server_connection`` does
``from snowflake.connector.network import ReauthenticationRequest``.

The universal driver has no equivalent HTTP/session-renewal module (that
logic lives in the Rust core); this module exists solely so that import path
keeps working. See ``errors.py`` for the actual exception hierarchy
(``ReauthenticationRequiredError`` / ``ReauthenticationRequest``) and
``python/BehaviorDifferences.yaml`` for the behavioral differences from the
legacy driver's reauthentication handling.

``ReauthenticationRequest`` is resolved lazily via module ``__getattr__``
(PEP 562) rather than a top-level ``from .errors import ...`` so this module
never rebinds the ``@backward_compatibility``-decorated class into its own
globals — a static test
(``TestNoInternalImportsOfBackwardCompatNames``) forbids any
``snowflake.connector.*`` module from doing that, since it would re-couple
driver internals to the legacy surface. One consequence: because the
resolution happens through this internal module, the one-shot deprecation
warning does not fire for callers going through this path (internal callers
are exempt from the warning by design) — only direct
``from snowflake.connector.errors import ReauthenticationRequest`` usage
warns. That's an acceptable trade-off here since the primary known consumer
of this exact path (Snowpark) is not itself deprecated usage we're trying to
flag.
"""

from __future__ import annotations

from typing import Any


__all__ = ["ReauthenticationRequest"]  # noqa: F822 - resolved lazily via __getattr__ below


def __getattr__(name: str) -> Any:
    if name == "ReauthenticationRequest":
        from .errors import ReauthenticationRequest

        return ReauthenticationRequest
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")


def __dir__() -> list[str]:
    return sorted(__all__)
