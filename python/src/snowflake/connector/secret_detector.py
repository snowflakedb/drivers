"""BACKWARD COMPATIBILITY MODULE ONLY — secret masking stubs."""

from __future__ import annotations

import logging
import re

from typing import NamedTuple


class MaskedMessageData(NamedTuple):
    masked: bool = False
    masked_text: str | None = None
    err_str: str | None = None


class SecretDetector(logging.Formatter):
    """Logging formatter that masks secrets in log messages.

    Minimal stub — masks common patterns (passwords, tokens, AWS keys).
    Matches the interface of snowflake-connector-python SecretDetector.
    """

    SECRET_STARRED_MASK_STR = "****"

    _PASSWORD_RE = re.compile(
        r"(password|pwd)([\'\"\s:=]+)([^\s\'\"]{1,})",
        flags=re.IGNORECASE,
    )
    _TOKEN_RE = re.compile(
        r"(token|assertion content)([\'\"\s:=]+)([a-z0-9=/_\-\+\.]{8,})",
        flags=re.IGNORECASE,
    )
    _AWS_KEY_RE = re.compile(
        r"(aws_key_id|aws_secret_key|access_key_id|secret_access_key)\s*=\s*'([^']+)'",
        flags=re.IGNORECASE,
    )

    @staticmethod
    def mask_secrets(text: str | None) -> MaskedMessageData:
        if text is None:
            return MaskedMessageData()
        try:
            masked_text = SecretDetector._PASSWORD_RE.sub(r"\1\2" + SecretDetector.SECRET_STARRED_MASK_STR, text)
            masked_text = SecretDetector._TOKEN_RE.sub(r"\1\2" + SecretDetector.SECRET_STARRED_MASK_STR, masked_text)
            masked_text = SecretDetector._AWS_KEY_RE.sub(
                r"\1=" + f"'{SecretDetector.SECRET_STARRED_MASK_STR}'", masked_text
            )
            masked = masked_text != text
        except Exception as ex:
            return MaskedMessageData(True, str(ex), str(ex))
        return MaskedMessageData(masked, masked_text, None)

    def format(self, record: logging.LogRecord) -> str:
        try:
            raw = super().format(record)
            data = SecretDetector.mask_secrets(raw)
            return data.masked_text or raw
        except Exception:
            return super().format(record)
