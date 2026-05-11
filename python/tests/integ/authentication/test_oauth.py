"""Integration tests for the Python OAuth wrapper plumbing.

Scope: connect-time kwarg validation and AUTHENTICATOR handling for the
three OAuth flows (analysis_feature_oauth.md §3 / §4 / §6). These tests
intentionally do NOT exercise the actual OAuth flows:

* ``OAUTH_AUTHORIZATION_CODE`` would spawn an OS browser and open a
  loopback listener as soon as configuration is valid.
* ``OAUTH_CLIENT_CREDENTIALS`` would perform an HTTPS token exchange
  against the configured IdP.

Happy-path coverage for both flows lives in the e2e suite
(``python/tests/e2e/authentication/test_oauth.py``) gated behind a real
Snowflake account / IdP.

What we cover here:

* Missing-required-parameter diagnostics for OAUTH_CLIENT_CREDENTIALS,
  for legacy AUTHENTICATOR=OAUTH, and "no missing-param" guarantees for
  the legacy OAUTH happy path.
* Case-insensitive AUTHENTICATOR matching (lowercase ``oauth``).
* Unknown / typo'd OAuth-like AUTHENTICATOR rejection.
* OAuth secret redaction at the wrapper boundary (no echoing
  ``oauth_client_secret`` or ``token`` in the exception chain).
* Legacy alias rewriting (``oauth_token_url`` is accepted because the
  Python wrapper rewrites it to ``oauth_token_request_url``).

Gherkin scenarios for these test cases live in
``tests/definitions/shared/authentication/oauth.feature`` (added by
substep 6 of this stack).
"""

from __future__ import annotations

import pytest

from snowflake.connector.errors import DatabaseError, Error, ProgrammingError

from ...compatibility import IS_UNIVERSAL_DRIVER


pytestmark = pytest.mark.skipif(not IS_UNIVERSAL_DRIVER, reason="Requires universal driver")


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


# ---------------------------------------------------------------------------
# OAUTH_CLIENT_CREDENTIALS — required-param validation
# ---------------------------------------------------------------------------


class TestOAuthClientCredentialsRequiredParams:
    """sf_core surfaces missing-param errors synchronously, before any token exchange."""

    def test_should_fail_when_client_id_is_missing(self, int_test_connection_factory):
        # Given Authentication is set to OAUTH_CLIENT_CREDENTIALS without oauth_client_id
        # When Trying to Connect
        exception = _attempt_oauth_connect(
            int_test_connection_factory,
            authenticator="OAUTH_CLIENT_CREDENTIALS",
            private_key_file=None,
            oauth_client_secret="test-client-secret",
            oauth_token_request_url="https://idp.example.com/oauth/token",
        )

        # Then Connection fails with a missing-parameter error citing oauth_client_id
        assert isinstance(exception, Error | Exception)
        assert "oauth_client_id" in _full_error_text(exception)

    def test_should_fail_when_client_secret_is_missing(self, int_test_connection_factory):
        # Given Authentication is set to OAUTH_CLIENT_CREDENTIALS without oauth_client_secret
        exception = _attempt_oauth_connect(
            int_test_connection_factory,
            authenticator="OAUTH_CLIENT_CREDENTIALS",
            private_key_file=None,
            oauth_client_id="test-client-id",
            oauth_token_request_url="https://idp.example.com/oauth/token",
        )

        # Then Connection fails with a missing-parameter error citing oauth_client_secret
        assert "oauth_client_secret" in _full_error_text(exception)

    def test_should_fail_when_token_request_url_is_missing(self, int_test_connection_factory):
        # Given Authentication is set to OAUTH_CLIENT_CREDENTIALS without
        # oauth_token_request_url. Snowflake's GS does not mint
        # client-credentials tokens, so the IdP token endpoint must be
        # provided up-front (analysis §4).
        exception = _attempt_oauth_connect(
            int_test_connection_factory,
            authenticator="OAUTH_CLIENT_CREDENTIALS",
            private_key_file=None,
            oauth_client_id="test-client-id",
            oauth_client_secret="test-client-secret",
        )

        # Then Connection fails with a missing-parameter error citing oauth_token_request_url
        assert "oauth_token_request_url" in _full_error_text(exception)


# ---------------------------------------------------------------------------
# Legacy AUTHENTICATOR=OAUTH — pre-acquired access token
# ---------------------------------------------------------------------------


class TestLegacyOAuthAuthenticator:
    """Legacy AUTHENTICATOR=OAUTH requires a ``token`` kwarg and is case-insensitive."""

    def test_should_fail_when_token_is_missing(self, int_test_connection_factory):
        # Given Authentication is set to legacy OAUTH without a token
        exception = _attempt_oauth_connect(
            int_test_connection_factory,
            authenticator="OAUTH",
            private_key_file=None,
        )

        # Then Connection fails with a missing-parameter error citing token
        text = _full_error_text(exception)
        assert "token" in text.lower()

    def test_should_forward_token_to_core_without_missing_param_error(self, int_test_connection_factory):
        # Given Authentication is set to legacy OAUTH with a pre-acquired access token
        # When Trying to Connect
        exception = _attempt_oauth_connect(
            int_test_connection_factory,
            authenticator="OAUTH",
            private_key_file=None,
            token="fake.jwt.token",
        )

        # Then The wrapper forwards the token to sf_core without raising
        # a missing-parameter error for it. The connection still fails
        # because the localhost test backend rejects the token — that
        # is a *network* failure, not a *validation* failure.
        text = _full_error_text(exception).lower()
        assert "missing required parameter" not in text or "token" not in text.split("missing required parameter", 1)[1]

    def test_should_accept_lowercase_oauth_authenticator(self, int_test_connection_factory):
        # Given Authentication is set to lowercase oauth with a token
        exception = _attempt_oauth_connect(
            int_test_connection_factory,
            authenticator="oauth",
            private_key_file=None,
            token="fake.jwt.token",
        )

        # Then The wrapper does not reject the AUTHENTICATOR value as unknown
        text = _full_error_text(exception)
        assert "Invalid authenticator" not in text
        assert "Unknown authenticator" not in text


# ---------------------------------------------------------------------------
# Negative path: invalid AUTHENTICATOR value
# ---------------------------------------------------------------------------


class TestUnknownOAuthAuthenticator:
    """Typo'd OAuth-flavoured AUTHENTICATOR values must be rejected."""

    def test_should_fail_when_authenticator_is_unknown_oauth_like_value(self, int_test_connection_factory):
        # Given Authentication is set to a typo of an OAuth flow name
        exception = _attempt_oauth_connect(
            int_test_connection_factory,
            authenticator="OAUTH_AUTHORIZATION_TYPO",
            private_key_file=None,
            oauth_client_id="test-client-id",
        )

        # Then Connection fails with an authenticator-related error.
        # Accept any of the known phrasings sf_core uses for unknown
        # authenticator values — analogous to the ODBC integration test.
        text = _full_error_text(exception).lower()
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

    def test_should_not_echo_oauth_client_secret_in_error_messages(self, int_test_connection_factory):
        # Given Authentication is set to OAUTH_CLIENT_CREDENTIALS with a distinctive client secret literal
        exception = _attempt_oauth_connect(
            int_test_connection_factory,
            authenticator="OAUTH_CLIENT_CREDENTIALS",
            private_key_file=None,
            oauth_client_id="test-client-id",
            oauth_client_secret=self.SECRET_LITERAL,
            oauth_token_request_url="https://idp.example.com/oauth/token",
        )

        # Then No exception message contains the literal client secret.
        # If the wrapper or core ever stops redacting OAuth client
        # secrets, this test catches it before the secret reaches a
        # user-visible diagnostic.
        text = _full_error_text(exception)
        assert self.SECRET_LITERAL not in text, f"OAuth client secret leaked into the exception chain: {text!r}"

    def test_should_not_echo_legacy_oauth_token_in_error_messages(self, int_test_connection_factory):
        # Given Authentication is set to legacy OAUTH with a distinctive token literal
        token_literal = "ZZ_PY_TOKEN_NEEDLE_LEGACY_OAUTH_ZZ"
        exception = _attempt_oauth_connect(
            int_test_connection_factory,
            authenticator="OAUTH",
            private_key_file=None,
            token=token_literal,
        )

        # Then No exception message contains the literal token.
        text = _full_error_text(exception)
        assert token_literal not in text, f"OAuth access token leaked into the exception chain: {text!r}"


# ---------------------------------------------------------------------------
# Wrapper-level alias rewriting (Python-specific — not exercised by sf_core
# or the ODBC layer)
# ---------------------------------------------------------------------------


class TestOAuthLegacyAliasRewrite:
    """The Python wrapper accepts ``oauth_token_url`` as an alias for the canonical key."""

    def test_should_not_reject_oauth_token_url_as_unknown_kwarg(self, int_test_connection_factory):
        # Given an OAUTH_CLIENT_CREDENTIALS connect call using the
        # legacy ``oauth_token_url`` alias (instead of the canonical
        # ``oauth_token_request_url``)
        exception = _attempt_oauth_connect(
            int_test_connection_factory,
            authenticator="OAUTH_CLIENT_CREDENTIALS",
            private_key_file=None,
            oauth_client_id="test-client-id",
            oauth_client_secret="test-client-secret",
            oauth_token_url="https://idp.example.com/oauth/token",
        )

        # Then sf_core must NOT raise "Missing required parameter ...
        # oauth_token_request_url" — the wrapper rewrote the alias
        # before forwarding (analysis §9 Python column). The connect
        # still fails because no real IdP is reachable, but for a
        # *different* reason than missing-param.
        text = _full_error_text(exception)
        assert "Missing required parameter" not in text or "oauth_token_request_url" not in text


# ---------------------------------------------------------------------------
# Python-only deprecation surface
# ---------------------------------------------------------------------------


class TestOAuthPythonOnlyKwargsDeprecation:
    """``snowflake-connector-python`` OAuth switches are silently dropped with a warning."""

    @pytest.mark.parametrize(
        "python_only_kwarg",
        [
            "oauth_enable_refresh_tokens",
            "oauth_credentials_in_body",
            "oauth_socket_uri",
        ],
    )
    def test_should_emit_deprecation_warning_for_python_only_kwarg(
        self, python_only_kwarg, int_test_connection_factory
    ):
        # Given a connect call that includes a Python-only legacy OAuth kwarg.
        # We point AUTHENTICATOR at OAUTH_CLIENT_CREDENTIALS WITHOUT supplying
        # the IdP token URL so sf_core fails synchronously with a
        # missing-parameter error -- this avoids the AC flow's loopback
        # listener / browser spawn, which would otherwise hang for the
        # full pytest-timeout window.
        with pytest.warns(DeprecationWarning, match=python_only_kwarg):
            with pytest.raises((DatabaseError, ProgrammingError, Error, Exception)):
                int_test_connection_factory(
                    authenticator="OAUTH_CLIENT_CREDENTIALS",
                    private_key_file=None,
                    oauth_client_id="test-client-id",
                    oauth_client_secret="test-client-secret",
                    **{python_only_kwarg: True},
                )
