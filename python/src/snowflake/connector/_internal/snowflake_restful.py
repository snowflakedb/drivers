from typing import TYPE_CHECKING


if TYPE_CHECKING:
    from snowflake.connector import Connection


class SnowflakeRestful:
    """Extend as required"""

    def __init__(self, connection: Connection) -> None:
        self._conn = connection

    @property
    def token(self) -> str:
        """Required by Python API"""
        return "TODO"
