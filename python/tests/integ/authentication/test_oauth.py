"""Integration tests for the Python OAuth wrapper plumbing.

Scope: connect-time kwarg validation, AUTHENTICATOR handling, and
secret redaction for the three OAuth flows (analysis_feature_oauth.md
§3 Authorization Code, §4 Client Credentials, §6 legacy pre-acquired
access token).

These tests intentionally do NOT exercise the full OAuth flows:

* ``OAUTH_AUTHORIZATION_CODE`` would spawn an OS browser and open a
  loopback listener as soon as configuration is valid.
* ``OAUTH_CLIENT_CREDENTIALS`` would perform an HTTPS token exchange
  against the configured IdP.

Happy-path coverage for both flows lives in the e2e suite
(``python/tests/e2e/authentication/test_oauth.py``) gated behind a real
Snowflake account / IdP.

Scenario step text comes verbatim from
``tests/definitions/shared/authentication/oauth.feature`` (@odbc_int /
@python_int scenarios). Python-specific behaviour (legacy alias
rewrite, deprecation warnings, ``token=`` echo guard) lives in
``test_oauth_python_specific.py`` so the shared feature file stays
language-neutral.
"""

from __future__ import annotations

import pytest

from snowflake.connector.errors import Error

from ...compatibility import IS_UNIVERSAL_DRIVER


pytestmark = pytest.mark.skipif(not IS_UNIVERSAL_DRIVER, reason="Requires universal driver")


# ---------------------------------------------------------------------------
# OAUTH_CLIENT_CREDENTIALS — required-param validation
# ---------------------------------------------------------------------------


class TestOAuthClientCredentialsRequiredParams:
    """sf_core surfaces missing-param errors synchronously, before any token exchange."""

    def test_should_fail_oauth_client_credentials_when_client_id_is_missing(self, int_test_connection_factory):
        # Given Authentication is set to OAUTH_CLIENT_CREDENTIALS without oauth_client_id
        kwargs = {
            "authenticator": "OAUTH_CLIENT_CREDENTIALS",
            "private_key_file": None,
            "oauth_client_secret": "test-client-secret",
            "oauth_token_request_url": "https://idp.example.com/oauth/token",
        }

        # When Trying to Connect
        exception = _attempt_oauth_connect(int_test_connection_factory, **kwargs)

        # Then Connection fails with a missing-parameter error citing oauth_client_id
        assert isinstance(exception, Error | Exception)
        assert "oauth_client_id" in _full_error_text(exception)

    def test_should_fail_oauth_client_credentials_when_client_secret_is_missing(self, int_test_connection_factory):
        # Given Authentication is set to OAUTH_CLIENT_CREDENTIALS without oauth_client_secret
        kwargs = {
            "authenticator": "OAUTH_CLIENT_CREDENTIALS",
            "private_key_file": None,
            "oauth_client_id": "test-client-id",
            "oauth_token_request_url": "https://idp.example.com/oauth/token",
        }

        # When Trying to Connect
        exception = _attempt_oauth_connect(int_test_connection_factory, **kwargs)

        # Then Connection fails with a missing-parameter error citing oauth_client_secret
        assert "oauth_client_secret" in _full_error_text(exception)

    def test_should_fail_oauth_client_credentials_when_token_request_url_is_missing(self, int_test_connection_factory):
        # Snowflake's GS does not mint client-credentials tokens, so the IdP token endpoint must
        # be provided up-front per analysis §4.

        # Given Authentication is set to OAUTH_CLIENT_CREDENTIALS without oauth_token_request_url
        kwargs = {
            "authenticator": "OAUTH_CLIENT_CREDENTIALS",
            "private_key_file": None,
            "oauth_client_id": "test-client-id",
            "oauth_client_secret": "test-client-secret",
        }

        # When Trying to Connect
        exception = _attempt_oauth_connect(int_test_connection_factory, **kwargs)

        # Then Connection fails with a missing-parameter error citing oauth_token_request_url
        assert "oauth_token_request_url" in _full_error_text(exception)


# ---------------------------------------------------------------------------
# AUTHENTICATOR value handling (case-insensitivity, unknown values)
# ---------------------------------------------------------------------------


class TestOAuthAuthenticatorValue:
    """The wrapper accepts case-insensitive OAuth authenticator values and rejects typos."""

    def test_should_accept_lowercase_oauth_authenticator_value(self, int_test_connection_factory):
        # Given Authentication is set to lowercase oauth with a TOKEN
        kwargs = {
            "authenticator": "oauth",
            "private_key_file": None,
            "token": "fake.jwt.token",
        }

        # When Trying to Connect
        exception = _attempt_oauth_connect(int_test_connection_factory, **kwargs)

        # Then The wrapper does not reject the AUTHENTICATOR value as unknown
        text = _full_error_text(exception)
        assert "Invalid authenticator" not in text
        assert "Unknown authenticator" not in text

    def test_should_fail_when_authenticator_is_an_unknown_o_auth_like_value(self, int_test_connection_factory):
        # Given Authentication is set to a typo of an OAuth flow name
        kwargs = {
            "authenticator": "OAUTH_AUTHORIZATION_TYPO",
            "private_key_file": None,
            "oauth_client_id": "test-client-id",
        }

        # When Trying to Connect
        exception = _attempt_oauth_connect(int_test_connection_factory, **kwargs)
        text = _full_error_text(exception).lower()

        # Then Connection fails with an authenticator-related error
        #
        # Accept any of the known phrasings sf_core uses for unknown
        # authenticator values — analogous to the ODBC integration test.
        assert any(
            needle in text
            for needle in (
                "invalid authenticator",
                "unknown authenticator",
                "oauth_authorization_typo",
                "authenticator",
            )
        )


# ---------------------------------------------------------------------------
# Secret redaction at the wrapper boundary
# ---------------------------------------------------------------------------


class TestOAuthSecretRedaction:
    """No driver-emitted error message echoes an OAuth secret literal (analysis §11)."""

    SECRET_LITERAL = "ZZ_PY_SECRET_NEEDLE_OAUTH_CC_ZZ"

    def test_should_not_echo_oauth_client_secret_in_diagnostics(self, int_test_connection_factory):
        # Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with a distinctive client secret literal
        kwargs = {
            "authenticator": "OAUTH_CLIENT_CREDENTIALS",
            "private_key_file": None,
            "oauth_client_id": "test-client-id",
            "oauth_client_secret": self.SECRET_LITERAL,
            "oauth_token_request_url": "https://idp.example.com/oauth/token",
        }

        # When Trying to Connect
        exception = _attempt_oauth_connect(int_test_connection_factory, **kwargs)
        text = _full_error_text(exception)

        # Then No diagnostic record contains the literal client secret
        #
        # Catches regressions where the wrapper or core stops redacting
        # OAuth client secrets before the value reaches a user-visible
        # diagnostic.
        assert self.SECRET_LITERAL not in text, f"OAuth client secret leaked into the exception chain: {text!r}"


# ---------------------------------------------------------------------------
# Helpers — flatten the full exception chain (cause / context) into a
# single text blob so a single `in` check covers nested error wrappers.
# ---------------------------------------------------------------------------


def _full_error_text(exception: BaseException) -> str:
    parts: list[str] = []
    current: BaseException | None = exception
    seen: set[int] = set()
    while current is not None and id(current) not in seen:
        seen.add(id(current))
        parts.append(repr(current))
        parts.append(str(current))
        current = current.__cause__ or current.__context__
    return "\n".join(parts)


def _attempt_oauth_connect(int_test_connection_factory, **kwargs):
    """Try to connect with OAuth kwargs and return the raised exception.

    The integration test backend listens on ``http://localhost:8090`` so
    every OAuth connect call ultimately fails — what we assert on is
    *which* error path was reached (missing-param vs invalid-value vs
    network), which is enough to verify the wrapper-level plumbing.
    """
    with pytest.raises(Exception) as exc_info:
        int_test_connection_factory(**kwargs)
    return exc_info.value
