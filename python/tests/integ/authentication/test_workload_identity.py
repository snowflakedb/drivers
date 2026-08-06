"""Integration tests for WIF parameter validation.

Exercises connect-time kwarg validation for WORKLOAD_IDENTITY without
requiring a live cloud identity or Snowflake account.  The integration
test backend rejects every connection attempt at the network layer;
what we assert on is *which* error path is reached (missing-param vs
invalid-value vs network-failure), which is sufficient to verify wrapper
and sf_core validation plumbing.

Scenario step text comes verbatim from
``tests/definitions/shared/authentication/workload_identity.feature``
(@python_int scenarios).
"""

from __future__ import annotations

import re

import pytest

from ...compatibility import IS_UNIVERSAL_DRIVER
from .conftest import connect_expecting_error, full_error_text


# A missing-'token' error can surface in two formats (mirrors test_token_auth_user_optional):
#   * validate_settings pre-flight:      Missing required parameter 'token'
#   * build_auth_config / ConfigError:   Missing required parameter: token
# The regex matches either form, anchored on the missing-required phrase + the 'token' param,
# so incidental "token" mentions ("invalid bearer token") and malformed-token errors cannot pass.
_MISSING_TOKEN_RE = re.compile(r"Missing required parameter[: ]\s*['\"]?token['\"]?", re.IGNORECASE)


pytestmark = pytest.mark.skipif(not IS_UNIVERSAL_DRIVER, reason="Requires universal driver")


# ---------------------------------------------------------------------------
# Required-param validation
# ---------------------------------------------------------------------------


class TestWorkloadIdentityRequiredParams:
    def test_should_fail_workload_identity_when_provider_is_missing(self, int_test_connection_factory):
        # Given Authentication is set to WORKLOAD_IDENTITY but WORKLOAD_IDENTITY_PROVIDER is absent
        kwargs = {"authenticator": "WORKLOAD_IDENTITY"}

        # When Trying to Connect
        exception = connect_expecting_error(int_test_connection_factory, **kwargs)

        # Then Connection fails with a missing-parameter error citing workload_identity_provider
        text = full_error_text(exception)
        assert "workload_identity_provider" in text.lower()

    def test_should_fail_workload_identity_when_provider_is_an_invalid_value(self, int_test_connection_factory):
        # Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is an invalid value
        kwargs = {
            "authenticator": "WORKLOAD_IDENTITY",
            "workload_identity_provider": "INVALID_CLOUD",
        }

        # When Trying to Connect
        exception = connect_expecting_error(int_test_connection_factory, **kwargs)

        # Then Connection fails with an invalid-parameter error citing workload_identity_provider
        text = full_error_text(exception)
        assert "workload_identity_provider" in text.lower()

    def test_should_fail_oidc_wif_when_token_is_missing(self, int_test_connection_factory):
        # Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is OIDC but token is absent
        kwargs = {
            "authenticator": "WORKLOAD_IDENTITY",
            "workload_identity_provider": "OIDC",
        }

        # When Trying to Connect
        exception = connect_expecting_error(int_test_connection_factory, **kwargs)

        # Then Connection fails with a missing-parameter error citing token
        text = full_error_text(exception)
        assert _MISSING_TOKEN_RE.search(text), f"Expected a missing-required-'token' error, got: {text}"

    def test_should_fail_oidc_wif_when_token_is_malformed(self, int_test_connection_factory):
        # Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is OIDC
        kwargs = {
            "authenticator": "WORKLOAD_IDENTITY",
            "workload_identity_provider": "OIDC",
        }
        # And Token is set to a malformed value that is not a valid JWT
        kwargs["token"] = "not-a-valid-jwt"

        # When Trying to Connect
        exception = connect_expecting_error(int_test_connection_factory, **kwargs)

        # Then Connection fails with an attestation error indicating a malformed token
        text = full_error_text(exception)
        assert (
            "jwt" in text.lower()
            or "malformed" in text.lower()
            or "invalid token" in text.lower()
            or "attestation" in text.lower()
        ), f"Expected malformed-JWT error, got: {text}"
