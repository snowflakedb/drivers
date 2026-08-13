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
driver internals to the legacy surface. This does *not* mean access through
this path goes unwarned: ``__getattr__`` below emits an explicit
``DeprecationWarning`` (for external callers only, matching the rest of the
driver's backward-compatibility convention — see
``_internal.backward_compatibility._is_caller_external``) pointing callers at
``snowflake.connector.errors`` — the import path is what's deprecated here,
not the class itself, which the driver actively raises.
"""

from __future__ import annotations

import warnings

from typing import Any

from ._internal.backward_compatibility import _is_caller_external


__all__ = ["ReauthenticationRequest"]  # noqa: F822 - resolved lazily via __getattr__ below

_warned = False


def __getattr__(name: str) -> Any:
    if name == "ReauthenticationRequest":
        from .errors import ReauthenticationRequest

        global _warned
        if not _warned and _is_caller_external():
            _warned = True
            warnings.warn(
                "snowflake.connector.network.ReauthenticationRequest is deprecated; "
                "import ReauthenticationRequest from snowflake.connector.errors instead.",
                DeprecationWarning,
                stacklevel=2,
            )
        return ReauthenticationRequest
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")


def __dir__() -> list[str]:
    return sorted(__all__)
