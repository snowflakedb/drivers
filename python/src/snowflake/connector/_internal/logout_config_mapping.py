"""Logout configuration for the Python wrapper (SNOW-2314152).

LogoutConfig carries resolved logout settings with Python-specific defaults.
remap_keep_alive_for_backward_compat applies the Phase 2 (SNOW-2314152) backward-compat remap.
"""

import warnings

from dataclasses import dataclass
from enum import Enum
from typing import Optional


# Python backward-compat defaults (SNOW-2314152).
# These mirror the old Python driver's behavior.
# Core defaults (5s, Strict, no per-request timeout) apply when no wrapper overrides them
# (see sf_core/src/config/logout.rs::Default for LogoutConfig).
PYTHON_DEFAULT_LOGOUT_TOTAL_TIMEOUT_SECONDS = 15
PYTHON_DEFAULT_LOGOUT_MAX_ATTEMPTS = 3
PYTHON_DEFAULT_LOGOUT_REQUEST_TIMEOUT_SECONDS = 5


class LogoutOptionKeys(str, Enum):
    """Core API option key strings for logout configuration.

    These correspond to the option keys accepted by Core's
    connection_set_option_* RPCs for logout behavior.
    """

    SERVER_SESSION_KEEP_ALIVE = "server_session_keep_alive"
    ENABLE_SERVER_SESSION_KEEP_ALIVE_AUTO_DETECTION = "enable_server_session_keep_alive_auto_detection"
    LOGOUT_ERROR_STRATEGY = "logout_error_strategy"
    LOGOUT_TOTAL_TIMEOUT_SECONDS = "logout_total_timeout_seconds"
    LOGOUT_MAX_ATTEMPTS = "logout_max_attempts"
    LOGOUT_REQUEST_TIMEOUT_SECONDS = "logout_request_timeout_seconds"


class ErrorStrategy(str, Enum):
    """Error handling strategy for logout.

    These map directly to Core's ErrorStrategy enum variants.
    """

    BEST_EFFORT = "best_effort"
    STRICT = "strict"


_UNSET = object()  # Sentinel to distinguish "not provided" from explicit values


@dataclass
class LogoutConfig:
    """Final logout configuration for Core API.

    Attributes:
        server_session_keep_alive: Final value for Core (already mapped)
        enable_server_session_keep_alive_auto_detection: Final value for Core (None = treat as False in Core)
        error_strategy: Error handling strategy string ("best_effort" or "strict")
        logout_total_timeout_seconds: Total timeout budget for logout operation (all retries)
        max_attempts: Maximum total attempts (NOT retry count: 1 = no retries, 3 = 2 retries)
        logout_request_timeout_seconds: Per-request socket timeout (None = no per-request limit)
    """

    server_session_keep_alive: Optional[bool]
    enable_server_session_keep_alive_auto_detection: Optional[bool]
    error_strategy: ErrorStrategy = ErrorStrategy.BEST_EFFORT
    logout_total_timeout_seconds: int = PYTHON_DEFAULT_LOGOUT_TOTAL_TIMEOUT_SECONDS
    max_attempts: Optional[int] = PYTHON_DEFAULT_LOGOUT_MAX_ATTEMPTS
    logout_request_timeout_seconds: Optional[int] = PYTHON_DEFAULT_LOGOUT_REQUEST_TIMEOUT_SECONDS

    @classmethod
    def from_kwargs(cls, kwargs: dict) -> "LogoutConfig":
        """Pop logout params from kwargs, apply defaults and backward-compat mapping.

        Mutates kwargs in-place (pops logout-specific keys).
        """
        from snowflake.connector.errors import ProgrammingError

        keep_alive = kwargs.pop("server_session_keep_alive", None)
        if keep_alive is not None and not isinstance(keep_alive, bool):
            raise ProgrammingError(f"server_session_keep_alive must be bool, got {type(keep_alive).__name__}")

        auto_detection = _extract_auto_detection_param(kwargs)
        keep_alive = remap_keep_alive_for_backward_compat(keep_alive, auto_detection)

        return cls(
            server_session_keep_alive=keep_alive,
            enable_server_session_keep_alive_auto_detection=auto_detection,
        )

    def to_option_dict(self) -> dict[str, "bool | int | str"]:
        """Convert to a dict suitable for ``_build_config_settings`` + ``connection_set_options``.

        ``None`` fields are omitted so Core uses its own defaults for those keys.
        """
        options: dict[str, bool | int | str] = {}
        if self.server_session_keep_alive is not None:
            options[LogoutOptionKeys.SERVER_SESSION_KEEP_ALIVE] = self.server_session_keep_alive
        if self.enable_server_session_keep_alive_auto_detection is not None:
            options[LogoutOptionKeys.ENABLE_SERVER_SESSION_KEEP_ALIVE_AUTO_DETECTION] = (
                self.enable_server_session_keep_alive_auto_detection
            )
        options[LogoutOptionKeys.LOGOUT_ERROR_STRATEGY] = self.error_strategy.value
        options[LogoutOptionKeys.LOGOUT_TOTAL_TIMEOUT_SECONDS] = self.logout_total_timeout_seconds
        if self.max_attempts is not None:
            options[LogoutOptionKeys.LOGOUT_MAX_ATTEMPTS] = self.max_attempts
        if self.logout_request_timeout_seconds is not None:
            options[LogoutOptionKeys.LOGOUT_REQUEST_TIMEOUT_SECONDS] = self.logout_request_timeout_seconds
        return options


def _extract_auto_detection_param(kwargs: dict) -> Optional[bool]:
    """Pop and parse enable_server_session_keep_alive_auto_detection from kwargs.

    If not provided, defaults to True and emits a FutureWarning:
    the default will change to None in a future version (SNOW-2314152).
    """
    from snowflake.connector.errors import ProgrammingError

    raw = kwargs.pop("enable_server_session_keep_alive_auto_detection", _UNSET)
    if raw is _UNSET:
        warnings.warn(
            "enable_server_session_keep_alive_auto_detection was not set and defaults "
            "to True. In a future version, the default will change to None. "
            "Please provide an explicit value: "
            "True = check for running async queries before logout (queries are preserved); "
            "False/None = always send logout on close (async queries may be terminated by server). "
            "Logout behavior can also be overridden with server_session_keep_alive. "
            "See the connection parameter docs for more info.",
            FutureWarning,
            stacklevel=6,
        )
        return True
    if raw is not None and not isinstance(raw, bool):
        raise ProgrammingError(
            f"enable_server_session_keep_alive_auto_detection must be bool or None, got {type(raw).__name__}"
        )
    return raw


def remap_keep_alive_for_backward_compat(
    server_session_keep_alive: Optional[bool],
    enable_auto_detection: Optional[bool],
) -> Optional[bool]:
    """Phase 2 backward-compat remap for server_session_keep_alive (SNOW-2314152).

    Old Python driver: server_session_keep_alive=False (default) always checked
    _async_sfqids before logout. Phase 2 (SNOW-2314152) preserves this: False + True → None makes
    Core check the registry (same behavior). Phase 3 (SNOW-2314152): False will mean "force logout".

    Truth table:
    - False + auto_detection=True  → None (Core checks registry) + deprecation warning
    - False + auto_detection=False → False (no remap, no warning — same meaning in Phase 3 (SNOW-2314152))
    - True / None                  → pass through unchanged
    """
    if server_session_keep_alive is False and enable_auto_detection:
        warnings.warn(
            "server_session_keep_alive=False currently respects auto-detection "
            "(async query registry is checked before logout). In a future version, "
            "False will mean 'always logout' without registry check. "
            "To keep current behavior, use server_session_keep_alive=None with "
            "enable_server_session_keep_alive_auto_detection=True.",
            FutureWarning,
            stacklevel=5,
        )
        return None
    return server_session_keep_alive
