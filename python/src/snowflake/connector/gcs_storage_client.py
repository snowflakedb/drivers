"""BACKWARD COMPATIBILITY MODULE ONLY"""

from __future__ import annotations

from typing import Any


GCS_METADATA_SFC_DIGEST = "x-goog-meta-sfc-digest"


class SnowflakeGCSRestClient:
    def __init__(self, **kwargs: Any) -> None:
        raise NotImplementedError("SnowflakeGCSRestClient is not yet implemented")
