"""BACKWARD COMPATIBILITY MODULE ONLY"""

from typing import Any

from .converter import SnowflakeConverter


class SnowflakeConverterIssue23517(SnowflakeConverter):
    def __init__(self, **kwargs: Any) -> None:
        super().__init__(**kwargs)
