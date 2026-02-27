"""BACKWARD COMPATIBILITY MODULE ONLY"""

from __future__ import annotations

from typing import Any


class StorageCredential:
    def __init__(
        self,
        credentials: dict[str, Any] | None = None,
        connection: Any = None,
        command: str = "",
    ) -> None:
        self.credentials = credentials or {}
        self.connection = connection
        self.command = command


class SnowflakeFileTransferAgent:
    def __init__(self, cursor: Any = None, command: str = "", ret: dict[str, Any] | None = None, **kwargs: Any) -> None:
        self.cursor = cursor
        self.command = command
        self.ret = ret or {}
