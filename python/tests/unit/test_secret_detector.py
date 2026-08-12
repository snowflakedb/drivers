"""Behavioral tests for the backward-compat ``SecretDetector`` shim.

``SecretDetector`` is imported unconditionally by Snowpark's telemetry mock; the
Universal Driver ships it only for that parity. These tests pin the masking
behavior (the reason the class exists). The once-per-process deprecation warning
on import is covered separately in ``test_backward_compatibility_warnings.py``.
"""

from __future__ import annotations

import logging
import warnings


# Resolving ``SecretDetector`` fires the backward-compat DeprecationWarning; the
# warning contract itself is asserted elsewhere, so suppress it at import here.
with warnings.catch_warnings():
    warnings.simplefilter("ignore")
    from snowflake.connector._common.secret_detector import MaskedMessageData, SecretDetector


class TestMaskSecrets:
    def test_clean_text_is_not_masked(self):
        result = SecretDetector.mask_secrets("select 1 from dual")
        assert result.is_masked is False
        assert result.masked_text == "select 1 from dual"
        assert result.error_str is None

    def test_none_text_returns_empty_result(self):
        result = SecretDetector.mask_secrets(None)
        assert result == MaskedMessageData()

    def test_password_is_masked(self):
        result = SecretDetector.mask_secrets("password=hunter2")
        assert result.is_masked is True
        assert "hunter2" not in result.masked_text
        assert SecretDetector.SECRET_STARRED_MASK_STR in result.masked_text

    def test_aws_key_is_masked(self):
        result = SecretDetector.mask_secrets("aws_key_id='AKIAIOSFODNN7EXAMPLE'")
        assert result.is_masked is True
        assert "AKIAIOSFODNN7EXAMPLE" not in result.masked_text

    def test_connection_token_is_masked(self):
        result = SecretDetector.mask_secrets("token: abcd1234efgh5678")
        assert result.is_masked is True
        assert "abcd1234efgh5678" not in result.masked_text

    def test_private_key_body_is_masked(self):
        pem = "-----BEGIN PRIVATE KEY-----\nMIIBVerySecretKeyMaterial\n-----END PRIVATE KEY-----"
        result = SecretDetector.mask_secrets(pem)
        assert result.is_masked is True
        assert "MIIBVerySecretKeyMaterial" not in result.masked_text


class TestFormatter:
    def test_format_sanitizes_log_record(self):
        formatter = SecretDetector("%(message)s")
        record = logging.LogRecord(
            name="test",
            level=logging.INFO,
            pathname=__file__,
            lineno=1,
            msg="connecting with password=hunter2",
            args=None,
            exc_info=None,
        )
        formatted = formatter.format(record)
        assert "hunter2" not in formatted
        assert SecretDetector.SECRET_STARRED_MASK_STR in formatted

    def test_is_a_logging_formatter(self):
        assert issubclass(SecretDetector, logging.Formatter)
