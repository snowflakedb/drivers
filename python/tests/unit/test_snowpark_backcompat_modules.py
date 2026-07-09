"""Snowpark-compat surface for the backward-compatibility stub modules.

Locks in exactly what Snowpark imports from these modules so the stubs can't
drift below Snowpark's needs (see snowpark's ``mock/_telemetry.py`` and
``_internal/telemetry.py``/``server_connection.py``).
"""

import logging

from http.client import OK

import pytest

from snowflake.connector.constants import ENV_VAR_PARTNER
from snowflake.connector.network import ReauthenticationRequest
from snowflake.connector.secret_detector import SecretDetector
from snowflake.connector.telemetry import TelemetryClient, TelemetryData, TelemetryField
from snowflake.connector.telemetry_oob import TelemetryService
from snowflake.connector.wif_util import create_attestation


class TestConstantsAndCompat:
    def test_env_var_partner_value(self):
        assert ENV_VAR_PARTNER == "SF_PARTNER"

    def test_compat_ok_is_http_ok(self):
        assert OK == 200


class TestReauthenticationRequest:
    def test_carries_cause_and_is_exception(self):
        cause = ValueError("token expired")
        exc = ReauthenticationRequest(cause)
        assert exc.cause is cause
        assert isinstance(exc, Exception)

    def test_can_be_raised_and_caught(self):
        with pytest.raises(ReauthenticationRequest, match="token expired"):
            raise ReauthenticationRequest(ValueError("token expired"))


class TestSecretDetector:
    def test_mask_secrets_returns_three_tuple_and_masks_password(self):
        # Snowpark unpacks ``_, masked_text, _ = SecretDetector.mask_secrets(payload)``.
        masked, masked_text, err = SecretDetector.mask_secrets("password: hunter2")
        assert masked is True
        assert "hunter2" not in masked_text
        assert err is None

    def test_mask_secrets_none_is_unmasked(self):
        masked, _, _ = SecretDetector.mask_secrets(None)
        assert masked is False

    def test_is_a_logging_formatter(self):
        assert isinstance(SecretDetector(), logging.Formatter)


class TestTelemetryStubs:
    def test_telemetry_data_bool_constants(self):
        assert (TelemetryData.TRUE, TelemetryData.FALSE) == ("true", "false")

    def test_telemetry_field_keys(self):
        assert TelemetryField.KEY_SFQID.value == "query_id"
        assert TelemetryField.KEY_SOURCE.value == "source"
        assert TelemetryField.KEY_TYPE.value == "type"

    def test_telemetry_client_methods_are_noop(self):
        client = TelemetryClient()
        assert client.try_add_log_to_batch("x") is None
        assert client.close() is None


class TestTelemetryServiceSingleton:
    def test_get_instance_returns_singleton(self):
        assert TelemetryService.get_instance() is TelemetryService.get_instance()

    def test_subclass_get_instance_and_close(self):
        # Snowpark subclasses it: ``class LocalTestOOBTelemetryService(TelemetryService)``.
        class LocalOOB(TelemetryService):
            pass

        inst = LocalOOB.get_instance()
        assert isinstance(inst, TelemetryService)
        assert inst.close() is None


class TestWifUtil:
    def test_create_attestation_is_noop(self):
        assert create_attestation("aws", entra_resource=None) == {}
