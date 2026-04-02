"""Logout configuration for the Python wrapper (SNOW-2314152).

LogoutConfig carries resolved logout settings with Python-specific defaults.
remap_keep_alive_phase2 applies the Phase 2 backward-compat remap.
"""

from dataclasses import dataclass
from typing import Optional


# Python backward-compat defaults (SNOW-2314152).
# These mirror the old Python driver's behavior.
# Core defaults (5s, Strict, no per-request timeout) apply when no wrapper overrides them
# (see sf_core/src/config/logout.rs::Default for LogoutConfig).
PYTHON_DEFAULT_LOGOUT_TOTAL_TIMEOUT_SECONDS = 15
PYTHON_DEFAULT_LOGOUT_MAX_ATTEMPTS = 3
PYTHON_DEFAULT_LOGOUT_REQUEST_TIMEOUT_SECONDS = 5


class LogoutOptionKeys:
    """Core API option key strings for logout configuration.

    These constants correspond to the option keys accepted by Core's
    connection_set_option_* RPCs for logout behavior.
    """

    SERVER_SESSION_KEEP_ALIVE = "server_session_keep_alive"
    ENABLE_LOGOUT_AUTO_DETECTION = "enable_logout_auto_detection"
    LOGOUT_ERROR_STRATEGY = "logout_error_strategy"
    LOGOUT_TOTAL_TIMEOUT_SECONDS = "logout_total_timeout_seconds"
    LOGOUT_MAX_ATTEMPTS = "logout_max_attempts"
    LOGOUT_REQUEST_TIMEOUT_SECONDS = "logout_request_timeout_seconds"


class ErrorStrategy:
    """String constants for the logout_error_strategy option.

    These map directly to Core's ErrorStrategy enum variants.
    Pass via connection_set_option_string("logout_error_strategy", value).
    """

    BEST_EFFORT: str = "best_effort"
    STRICT: str = "strict"


@dataclass
class LogoutConfig:
    """Final logout configuration for Core API.

    Attributes:
        server_session_keep_alive: Final value for Core (already mapped)
        enable_logout_auto_detection: Final value for Core (None = treat as False in Core)
        error_strategy: Error handling strategy string ("best_effort" or "strict")
        logout_total_timeout_seconds: Total timeout budget for logout operation (all retries)
        max_attempts: Maximum total attempts (NOT retry count: 1 = no retries, 3 = 2 retries)
        logout_request_timeout_seconds: Per-request socket timeout (None = no per-request limit)
    """

    server_session_keep_alive: Optional[bool]
    enable_logout_auto_detection: Optional[bool]
    error_strategy: str = ErrorStrategy.BEST_EFFORT
    logout_total_timeout_seconds: int = PYTHON_DEFAULT_LOGOUT_TOTAL_TIMEOUT_SECONDS
    max_attempts: Optional[int] = PYTHON_DEFAULT_LOGOUT_MAX_ATTEMPTS
    logout_request_timeout_seconds: Optional[int] = PYTHON_DEFAULT_LOGOUT_REQUEST_TIMEOUT_SECONDS


def remap_keep_alive_phase2(
    server_session_keep_alive: Optional[bool],
    enable_auto_detection: Optional[bool],
) -> Optional[bool]:
    """Phase 2 backward-compat remap for server_session_keep_alive (SNOW-2314152).

    Old Python driver: server_session_keep_alive=False (default) always checked
    _async_sfqids before logout. Phase 2 preserves this: False + True → None makes
    Core check the registry (same behavior). Phase 3: False will mean "force logout".

    Truth table:
    - False + auto_detection=True  → None (Core checks registry)
    - False + auto_detection=False → False (no remap)
    - True / None                  → pass through unchanged
    """
    if server_session_keep_alive is False and enable_auto_detection:
        return None
    return server_session_keep_alive
