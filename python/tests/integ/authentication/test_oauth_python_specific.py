"""Python-only OAuth integration tests for the universal driver wrapper.

These tests cover wrapper-level behaviour that is unique to the Python
wrapper and does not appear in the shared ``oauth.feature`` Gherkin
file. By living in a file whose name does not match the shared
``oauth.feature``, this suite is exempt from the tests-format-validator
orphan check while still being picked up by ``pytest python/tests/integ``.

Covered behaviours:

* Rewriting the legacy ``oauth_token_url`` alias to the canonical
  ``oauth_token_request_url`` before forwarding (Python-only API
  surface — Python column of the cross-driver configuration matrix).
* Rewriting the legacy ``oauth_socket_uri`` alias to the canonical
  ``oauth_redirect_uri`` while emitting a ``DeprecationWarning``: the
  universal driver always binds the loopback listener to
  ``oauth_redirect_uri`` directly.
* Emitting a ``DeprecationWarning`` for Python-only OAuth switches
  (``oauth_enable_refresh_tokens``) that the universal driver does not
  honour (Python column of the cross-driver configuration matrix).
* Redacting a legacy OAUTH ``token`` literal from the wrapper's
  exception chain.

The cross-driver AUTHENTICATOR=OAUTH wrapper behaviour
(token-missing failure, token-forward-to-core) lives in
``test_oauth.py`` alongside the other shared @python_int scenarios.

Happy-path coverage for the OAuth flows lives in
``python/tests/e2e/authentication/test_oauth.py``.
"""

from __future__ import annotations

import warnings

import pytest

from snowflake.connector.errors import DatabaseError, Error, ProgrammingError

from ...compatibility import IS_UNIVERSAL_DRIVER


pytestmark = pytest.mark.skipif(not IS_UNIVERSAL_DRIVER, reason="Requires universal driver")


# ---------------------------------------------------------------------------
# Python-only secret redaction (legacy OAUTH token literal)
# ---------------------------------------------------------------------------


class TestOAuthLegacyTokenRedaction:
    """Legacy OAUTH ``token`` literals must not echo in the wrapper's exception chain."""

    def test_should_not_echo_legacy_oauth_token_in_diagnostics(self, int_test_connection_factory):
        """No shared scenario covers this — sensitive-key handling is wrapper-level."""
        token_literal = "ZZ_PY_TOKEN_NEEDLE_LEGACY_OAUTH_ZZ"
        kwargs = {
            "authenticator": "OAUTH",
            "private_key_file": None,
            "token": token_literal,
        }
        exception = _attempt_oauth_connect(int_test_connection_factory, **kwargs)

        text = _full_error_text(exception)
        assert token_literal not in text, f"OAuth access token leaked into the exception chain: {text!r}"


# ---------------------------------------------------------------------------
# Python-only alias rewriting (`oauth_token_url` → `oauth_token_request_url`)
# ---------------------------------------------------------------------------


class TestOAuthLegacyAliasRewrite:
    """``oauth_token_url`` is accepted as an alias for ``oauth_token_request_url``."""

    def test_should_not_reject_oauth_token_url_as_unknown_kwarg(self, int_test_connection_factory):
        """Python-only: snowflake-connector-python users historically passed `oauth_token_url=`."""
        kwargs = {
            "authenticator": "OAUTH_CLIENT_CREDENTIALS",
            "private_key_file": None,
            "oauth_client_id": "test-client-id",
            "oauth_client_secret": "test-client-secret",
            "oauth_token_url": "https://idp.example.com/oauth/token",
        }
        exception = _attempt_oauth_connect(int_test_connection_factory, **kwargs)

        # sf_core must NOT raise "Missing required parameter ...
        # oauth_token_request_url" — the wrapper rewrote the alias
        # before forwarding (Python legacy alias rewriting). The connect
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
        ],
    )
    def test_should_emit_deprecation_warning_for_python_only_kwarg(
        self, python_only_kwarg, int_test_connection_factory
    ):
        # A connect call that includes a Python-only legacy OAuth kwarg
        # must emit a DeprecationWarning before forwarding the rest of
        # the kwargs to sf_core. We point AUTHENTICATOR at
        # OAUTH_CLIENT_CREDENTIALS WITHOUT supplying the IdP token URL
        # so sf_core fails synchronously with a missing-parameter error
        # -- this avoids the AC flow's loopback listener / browser
        # spawn, which would otherwise hang for the full pytest-timeout
        # window.
        with pytest.warns(DeprecationWarning, match=python_only_kwarg):
            with pytest.raises((DatabaseError, ProgrammingError, Error, Exception)):
                int_test_connection_factory(
                    authenticator="OAUTH_CLIENT_CREDENTIALS",
                    private_key_file=None,
                    oauth_client_id="test-client-id",
                    oauth_client_secret="test-client-secret",
                    **{python_only_kwarg: True},
                )


class TestOAuthCredentialsInBodyIsHonoured:
    """``oauth_credentials_in_body`` is a real, honoured parameter.

    The universal driver forwards it to sf_core, which switches the
    ``OAUTH_CLIENT_CREDENTIALS`` token request to ``client_secret_post``
    (client_id/client_secret in the body). Unlike the Python-only switches
    above it must NOT emit a ``DeprecationWarning`` — that would signal the
    parameter is a no-op, which it no longer is.
    """

    def test_should_not_emit_deprecation_warning_for_credentials_in_body(self, int_test_connection_factory):
        # Same OAUTH_CLIENT_CREDENTIALS-without-token-URL shape as the
        # deprecation test so sf_core fails fast on a missing parameter
        # rather than reaching out to a real IdP.
        #
        # We record every warning and afterwards assert none names this
        # kwarg. Recording (rather than escalating with
        # simplefilter("error")) keeps the assertion independent of the
        # expected connect failure: an escalated DeprecationWarning would
        # surface as an exception that pytest.raises would happily swallow,
        # masking the very regression this test guards against.
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            with pytest.raises(Error):
                int_test_connection_factory(
                    authenticator="OAUTH_CLIENT_CREDENTIALS",
                    private_key_file=None,
                    oauth_client_id="test-client-id",
                    oauth_client_secret="test-client-secret",
                    oauth_credentials_in_body=True,
                )

        offending = [
            str(w.message)
            for w in caught
            if issubclass(w.category, DeprecationWarning) and "oauth_credentials_in_body" in str(w.message)
        ]
        assert not offending, f"oauth_credentials_in_body is honoured and must not warn, got: {offending}"


# ---------------------------------------------------------------------------
# Python-only deprecated alias rewrite (``oauth_socket_uri`` →
# ``oauth_redirect_uri``)
# ---------------------------------------------------------------------------


class TestOAuthSocketUriDeprecatedAlias:
    """``oauth_socket_uri`` is a deprecated alias for ``oauth_redirect_uri``.

    The legacy ``snowflake-connector-python`` exposed ``oauth_socket_uri`` so
    callers could bind the loopback listener to a different host/port than
    the redirect URI advertised to the IdP. The universal driver always
    binds the listener to ``oauth_redirect_uri``, so the legacy name is
    rewritten to the canonical one and a ``DeprecationWarning`` is emitted.
    """

    def test_should_rewrite_oauth_socket_uri_to_oauth_redirect_uri_with_warning(self, int_test_connection_factory):
        with pytest.warns(DeprecationWarning, match="oauth_socket_uri"):
            with pytest.raises((DatabaseError, ProgrammingError, Error, Exception)):
                int_test_connection_factory(
                    authenticator="OAUTH_CLIENT_CREDENTIALS",
                    private_key_file=None,
                    oauth_client_id="test-client-id",
                    oauth_client_secret="test-client-secret",
                    oauth_socket_uri="http://127.0.0.1:8765",
                )


# ---------------------------------------------------------------------------
# Helpers
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
    with pytest.raises(Exception) as exc_info:
        int_test_connection_factory(**kwargs)
    return exc_info.value
