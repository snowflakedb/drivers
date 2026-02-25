"""Logout parameter mapping for backward compatibility (SNOW-2314152).

Maps Python logout parameters to Core API configuration, computing final values
including phase-specific defaults and error handling strategies.
"""

from dataclasses import dataclass
from typing import TYPE_CHECKING, Optional


if TYPE_CHECKING:
    from snowflake.connector.connection import Connection


@dataclass
class LogoutConfig:
    """Final logout configuration for Core API.

    All values are fully resolved - no additional logic needed in close().

    Attributes:
        server_session_keep_alive: Final value for Core (already mapped)
        enable_auto_detection: Final value for Core (None = treat as False in Core)
        error_strategy: Error handling strategy (BEST_EFFORT or STRICT)
    """

    server_session_keep_alive: Optional[bool]
    enable_auto_detection: Optional[bool]
    error_strategy: int


def map_logout_config_phase2(connection: "Connection") -> LogoutConfig:
    """Map logout parameters for Phase 2 backward compatibility (SNOW-2314152).

    Phase 2 semantics (backward compatible with old Python driver):
    - server_session_keep_alive=False + auto-detection enabled → Core receives None
    - server_session_keep_alive=False + auto-detection disabled/None → Core receives False
    - server_session_keep_alive=True → Core receives True
    - server_session_keep_alive=None → Core receives None
    - enable_auto_detection: passed through as-is (constructor already applied Phase 2 default)
    - error_strategy: BEST_EFFORT (backward compatible)

    Note: Constructor defaults enable_auto_detection to True for Phase 2, but if user
    explicitly passes None, it's kept as None (Core treats None as False).

    Args:
        connection: Connection instance with logout configuration

    Returns:
        LogoutConfig with all final values ready for Core
    """
    from snowflake.connector._internal.protobuf_gen import database_driver_v1_pb2

    server_session_keep_alive = connection.server_session_keep_alive
    enable_auto_detection = connection.enable_server_session_keep_alive_auto_detection

    # Phase 2 special mapping: False + auto-detection enabled → map to None
    # This makes Core check the registry (legacy Python behavior)
    if server_session_keep_alive is False and enable_auto_detection:
        server_session_keep_alive = None

    return LogoutConfig(
        server_session_keep_alive=server_session_keep_alive,
        enable_auto_detection=enable_auto_detection,
        error_strategy=database_driver_v1_pb2.ERROR_STRATEGY_BEST_EFFORT,
    )
