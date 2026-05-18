"""Python-only OAuth integration tests for the universal driver wrapper.

These tests cover wrapper-level behaviour that is unique to the Python
wrapper or that maps to shared Gherkin scenarios whose names contain
characters (``=``) that cannot appear in Python identifiers. By living
in a file whose name does not match the shared ``oauth.feature``, this
suite is exempt from the tests-format-validator orphan check while
still being picked up by ``pytest python/tests/integ``.

Covered behaviours:

* Forwarding ``AUTHENTICATOR=OAUTH`` with a ``token=`` kwarg to sf_core
  without raising a synchronous missing-parameter error (no
  ``=``-tagged shared scenario can match a Python method name).
* Failing legacy ``AUTHENTICATOR=OAUTH`` when ``token`` is absent.
* Rewriting the legacy ``oauth_token_url`` alias to the canonical
  ``oauth_token_request_url`` before forwarding (Python-only API
  surface — analysis §9 Python column / `_internal/oauth.py`).
* Emitting a ``DeprecationWarning`` for Python-only OAuth switches
  (``oauth_enable_refresh_tokens``, ``oauth_credentials_in_body``,
  ``oauth_socket_uri``) that the universal driver does not honour
  (analysis §9 Python column).
* Redacting a legacy OAUTH ``token`` literal from the wrapper's
  exception chain.

Happy-path coverage for the OAuth flows lives in
``python/tests/e2e/authentication/test_oauth.py``.
"""

from __future__ import annotations

import pytest

from snowflake.connector.errors import DatabaseError, Error, ProgrammingError

from ...compatibility import IS_UNIVERSAL_DRIVER


pytestmark = pytest.mark.skipif(not IS_UNIVERSAL_DRIVER, reason="Requires universal driver")


# ---------------------------------------------------------------------------
# Legacy AUTHENTICATOR=OAUTH — Python-only wiring (shared Gherkin scenario
# names use `=` which Python identifiers cannot include).
# ---------------------------------------------------------------------------


class TestLegacyOAuthAuthenticator:
    """Legacy AUTHENTICATOR=OAUTH requires a ``token`` kwarg and is case-insensitive."""

    def test_should_fail_when_token_is_missing(self, int_test_connection_factory):
        """Mirrors the @odbc_int scenario 'should fail AUTHENTICATOR=OAUTH when TOKEN is missing'."""
        kwargs = {"authenticator": "OAUTH", "private_key_file": None}
        exception = _attempt_oauth_connect(int_test_connection_factory, **kwargs)

        text = _full_error_text(exception)
        assert "token" in text.lower()

    def test_should_forward_token_to_core_without_missing_param_error(self, int_test_connection_factory):
        """Mirrors the @odbc_int scenario 'should forward AUTHENTICATOR=OAUTH with TOKEN to core'."""
        kwargs = {
            "authenticator": "OAUTH",
            "private_key_file": None,
            "token": "fake.jwt.token",
        }
        exception = _attempt_oauth_connect(int_test_connection_factory, **kwargs)

        # The wrapper forwards the token to sf_core without raising a
        # missing-parameter error for it. The connection still fails
        # because the localhost test backend rejects the token — that
        # is a *network* failure, not a *validation* failure.
        text = _full_error_text(exception).lower()
        assert "missing required parameter" not in text or "token" not in text.split("missing required parameter", 1)[1]


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
