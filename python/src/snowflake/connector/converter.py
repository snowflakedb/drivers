"""BACKWARD COMPATIBILITY MODULE ONLY"""

from __future__ import annotations

from typing import Any


class SnowflakeConverter:
    def __init__(self, **kwargs: Any) -> None:
        self._parameters: dict[str, str | int | bool] = {}
        self._use_numpy = kwargs.get("use_numpy", False)
