"""Logout parameter mapping for backward compatibility (SNOW-2314152).

Maps Python logout parameters to Core API configuration, computing final values
including phase-specific defaults and error handling strategies.
"""

from dataclasses import dataclass
from typing import Optional


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
    error_strategy: str
    logout_total_timeout_seconds: int
    max_attempts: Optional[int]  # Total attempts (1 = no retries, 3 = 2 retries)
    logout_request_timeout_seconds: Optional[int]


def map_logout_config_phase2(
    server_session_keep_alive: Optional[bool],
    enable_auto_detection: Optional[bool],
) -> LogoutConfig:
    """Map logout parameters for backward compatibility (SNOW-2314152).

    Backward-compat semantics (mirrors old Python driver):
    - server_session_keep_alive=False + auto-detection enabled → Core receives None
    - server_session_keep_alive=False + auto-detection disabled/None → Core receives False
    - server_session_keep_alive=True → Core receives True
    - server_session_keep_alive=None → Core receives None
    - enable_auto_detection: passed through as-is
    - error_strategy: BEST_EFFORT (backward compatible)

    Args:
        server_session_keep_alive: User-provided keep-alive setting (or None)
        enable_auto_detection: User-provided auto-detection setting (or None)

    Returns:
        LogoutConfig with all final values ready for Core
    """
    # Backward-compat mapping (SNOW-2314152): False + auto-detection enabled → None
    # This makes Core check the registry (legacy Python behavior)
    if server_session_keep_alive is False and enable_auto_detection:
        server_session_keep_alive = None

    return LogoutConfig(
        server_session_keep_alive=server_session_keep_alive,
        enable_logout_auto_detection=enable_auto_detection,
        error_strategy=ErrorStrategy.BEST_EFFORT,
        logout_total_timeout_seconds=15,  # 15 second total budget with 3 max attempts = ~5s per attempt
        max_attempts=3,  # 3 total attempts (2 retries) for faster failure feedback
        logout_request_timeout_seconds=5,  # 5s per request (default), dynamically adjusted to min(5s, remaining)
    )
