"""BACKWARD COMPATIBILITY MODULE ONLY"""

import logging
from typing import NamedTuple


class MaskedMessageData(NamedTuple):
    is_masked: bool
    masked_text: str
    error_str: str | None


class SecretDetector(logging.Formatter):
    @staticmethod
    def mask_secrets(text: str) -> MaskedMessageData:
        return MaskedMessageData(is_masked=False, masked_text=text, error_str=None)
