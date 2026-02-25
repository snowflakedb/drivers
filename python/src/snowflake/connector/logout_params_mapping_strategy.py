"""Strategy pattern for logout parameter mapping across phases (SNOW-2314152).

Phase 2: server_session_keep_alive=False respects auto-detection (backward compatible)
Phase 3: Pass parameters directly to Core without mapping
"""

from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, Optional


if TYPE_CHECKING:
    from snowflake.connector.connection import Connection


class LogoutParamsMappingStrategy(ABC):
    """Abstract strategy for mapping Python logout parameters to Core parameters."""

    @abstractmethod
    def map_server_session_keep_alive(self, connection: "Connection") -> Optional[bool]:
        """Map Python server_session_keep_alive parameter to Core parameter.

        Args:
            connection: Connection instance with logout configuration

        Returns:
            Mapped value to pass to Core
        """
        pass


class Phase2LogoutParamsMappingStrategy(LogoutParamsMappingStrategy):
    """Phase 2 mapping: server_session_keep_alive=False respects auto-detection.

    Phase 2 semantics (backward compatible with old Python driver):
    - server_session_keep_alive=False + auto-detection enabled → Core receives None
    - server_session_keep_alive=False + auto-detection disabled → Core receives False
    - server_session_keep_alive=True → Core receives True
    - server_session_keep_alive=None → Core receives None

    This allows False to still respect the auto-detection registry check,
    preserving legacy Python behavior.
    """

    def map_server_session_keep_alive(self, connection: "Connection") -> Optional[bool]:
        # Extract parameters from connection
        server_session_keep_alive = connection.server_session_keep_alive
        enable_auto_detection = connection.enable_server_session_keep_alive_auto_detection

        # Default auto-detection to True for backward compatibility
        effective_enable_auto = enable_auto_detection if enable_auto_detection is not None else True

        # Phase 2 mapping logic
        if server_session_keep_alive is False and effective_enable_auto:
            # Special case: False + auto-detection enabled → map to None
            # This makes Core check the registry (legacy Python behavior)
            return None

        # All other cases: pass through as-is
        return server_session_keep_alive


class Phase3LogoutParamsMappingStrategy(LogoutParamsMappingStrategy):
    """Phase 3 mapping: Pass parameters directly to Core without transformation.

    Phase 3 semantics (simplified):
    - server_session_keep_alive=False → Core receives False (force logout)
    - server_session_keep_alive=True → Core receives True (never logout)
    - server_session_keep_alive=None → Core receives None (delegate to auto-detection)

    No special mapping logic - what Python receives is what Core gets.
    """

    def map_server_session_keep_alive(self, connection: "Connection") -> Optional[bool]:
        # No mapping - pass through directly
        return connection.server_session_keep_alive
