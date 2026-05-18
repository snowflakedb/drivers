"""Unit tests for the OAuth kwarg helpers + Connection.__init__ wiring.

The Python wrapper only owns parameter rewriting and log redaction for
OAuth — the actual OAuth flows live in the Rust core. These tests
therefore focus on:

* The pure ``snowflake.connector._internal.oauth`` helpers (no
  ``Connection`` involved).
* The integration points in ``Connection.__init__``: the rewritten
  kwargs land in ``connection_set_options`` as canonical names, and
  every OAuth secret is redacted in the public ``Connection.kwargs``
  view.

Cross-referenced specs:

* ``analysis_feature_oauth.md`` §9 — configuration matrix.
* ``analysis_feature_oauth.md`` §11 — logging & redaction.
"""

from __future__ import annotations

import warnings

from unittest.mock import patch

import pytest

from snowflake.connector._internal import oauth as oauth_helpers
from snowflake.connector._internal.oauth import (
    LEGACY_OAUTH_ALIASES,
    OAUTH_AUTHORIZATION_URL,
    OAUTH_CLIENT_ID,
    OAUTH_CLIENT_SECRET,
    OAUTH_DISABLE_PKCE,
    OAUTH_REDIRECT_URI,
    OAUTH_SCOPE,
    OAUTH_TOKEN_REQUEST_URL,
    PYTHON_ONLY_OAUTH_KWARGS,
    SENSITIVE_OAUTH_KWARGS,
    is_oauth_authenticator,
    is_sensitive_oauth_kwarg,
    redacted_kwargs_for_log,
    rewrite_oauth_kwargs,
)
from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import ConfigSetting
from tests.compatibility import IS_UNIVERSAL_DRIVER


pytestmark = pytest.mark.skipif(not IS_UNIVERSAL_DRIVER, reason="Requires universal driver")


# ---------------------------------------------------------------------------
# Pure-helper tests — no Connection needed.
# ---------------------------------------------------------------------------


class TestIsOauthAuthenticator:
    """``is_oauth_authenticator`` recognises every OAuth flow case-insensitively."""

    @pytest.mark.parametrize(
        "value",
        [
            "OAUTH",
            "OAUTH_AUTHORIZATION_CODE",
            "OAUTH_CLIENT_CREDENTIALS",
            "oauth",
            "oauth_authorization_code",
            "oauth_client_credentials",
            "OAuth",
            "Oauth_Authorization_Code",
        ],
    )
    def test_recognises_oauth_flows(self, value):
        assert is_oauth_authenticator(value) is True

    @pytest.mark.parametrize(
        "value",
        [
            "SNOWFLAKE_JWT",
            "USERNAME_PASSWORD_MFA",
            "EXTERNALBROWSER",
            "PROGRAMMATIC_ACCESS_TOKEN",
            "",
            None,
            123,
            ["OAUTH"],
        ],
    )
    def test_rejects_non_oauth_values(self, value):
        assert is_oauth_authenticator(value) is False


class TestIsSensitiveOauthKwarg:
    """OAuth secret detection is exact-match on the canonical name (analysis §11)."""

    @pytest.mark.parametrize("name", sorted(SENSITIVE_OAUTH_KWARGS))
    def test_flags_canonical_secrets(self, name):
        assert is_sensitive_oauth_kwarg(name) is True

    def test_oauth_secret_set_contents(self):
        # Lock down the redaction surface: anyone adding a new OAuth
        # secret has to update SENSITIVE_OAUTH_KWARGS *and* this test
        # so the wrapper redaction list cannot drift silently.
        assert SENSITIVE_OAUTH_KWARGS == frozenset({"oauth_client_secret", "token"})

    @pytest.mark.parametrize(
        "name",
        [
            "oauth_client_id",
            "oauth_redirect_uri",
            "oauth_token_request_url",
            "OAUTH_CLIENT_SECRET",  # case-sensitive on purpose — canonical names are lowercase
            "TOKEN",
            "",
            None,
            123,
        ],
    )
    def test_does_not_flag_non_secrets_or_uppercase_aliases(self, name):
        assert is_sensitive_oauth_kwarg(name) is False


class TestRedactedKwargsForLog:
    """``redacted_kwargs_for_log`` redacts OAuth secrets and only OAuth secrets."""

    def test_replaces_oauth_secret_values(self):
        kwargs = {
            "oauth_client_id": "client-123",
            "oauth_client_secret": "shhh",
            "token": "jwt.value",
            "user": "alice",
        }
        redacted = redacted_kwargs_for_log(kwargs)
        assert redacted["oauth_client_secret"] == "***"
        assert redacted["token"] == "***"
        # Non-secret OAuth kwargs and unrelated kwargs are untouched.
        assert redacted["oauth_client_id"] == "client-123"
        assert redacted["user"] == "alice"

    def test_returns_new_dict(self):
        kwargs = {"oauth_client_secret": "shhh", "user": "alice"}
        redacted = redacted_kwargs_for_log(kwargs)
        assert redacted is not kwargs
        # Original dict is untouched.
        assert kwargs["oauth_client_secret"] == "shhh"

    def test_no_secret_keys_returns_equal_copy(self):
        kwargs = {"user": "alice", "account": "acme", "oauth_client_id": "client-id"}
        redacted = redacted_kwargs_for_log(kwargs)
        assert redacted == kwargs


class TestRewriteOauthKwargs:
    """``rewrite_oauth_kwargs`` maps legacy aliases and drops Python-only switches."""

    def test_canonical_kwargs_pass_through_unchanged(self):
        kwargs = {
            "oauth_client_id": "client-123",
            "oauth_client_secret": "shhh",
            "oauth_authorization_url": "https://idp/authorize",
            "oauth_token_request_url": "https://idp/token",
            "oauth_redirect_uri": "http://127.0.0.1:0",
            "oauth_scope": "session:role:R",
            "oauth_enable_single_use_refresh_tokens": True,
            "oauth_disable_pkce": False,
        }
        rewritten = rewrite_oauth_kwargs(kwargs)
        assert rewritten == kwargs

    def test_does_not_mutate_input(self):
        kwargs = {"oauth_token_url": "https://idp/token"}
        rewrite_oauth_kwargs(kwargs)
        assert kwargs == {"oauth_token_url": "https://idp/token"}

    @pytest.mark.parametrize(
        ("alias", "canonical"),
        sorted(LEGACY_OAUTH_ALIASES.items()),
    )
    def test_renames_legacy_alias_to_canonical(self, alias, canonical):
        kwargs = {alias: "https://example/token"}
        rewritten = rewrite_oauth_kwargs(kwargs)
        assert alias not in rewritten
        assert rewritten[canonical] == "https://example/token"

    def test_canonical_wins_when_both_legacy_and_canonical_provided(self):
        # Mirrors how _rewrite_mfa_params handles
        # client_request_mfa_token vs client_store_temporary_credential
        # (BD#16): canonical key takes precedence, legacy is dropped.
        kwargs = {
            "oauth_token_url": "https://legacy/token",
            "oauth_token_request_url": "https://canonical/token",
        }
        rewritten = rewrite_oauth_kwargs(kwargs)
        assert "oauth_token_url" not in rewritten
        assert rewritten["oauth_token_request_url"] == "https://canonical/token"

    @pytest.mark.parametrize("python_only", sorted(PYTHON_ONLY_OAUTH_KWARGS))
    def test_drops_python_only_kwargs_with_deprecation_warning(self, python_only):
        kwargs = {python_only: "anything", "user": "alice"}
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            rewritten = rewrite_oauth_kwargs(kwargs)
        assert python_only not in rewritten
        assert rewritten["user"] == "alice"
        # Exactly one DeprecationWarning citing the dropped kwarg.
        deprecation = [w for w in caught if w.category is DeprecationWarning]
        assert len(deprecation) == 1
        assert python_only in str(deprecation[0].message)

    def test_non_oauth_kwargs_pass_through_untouched(self):
        kwargs = {
            "user": "alice",
            "password": "hunter2",
            "account": "acme",
            "authenticator": "OAUTH_AUTHORIZATION_CODE",
            "oauth_client_id": "client-id",
        }
        rewritten = rewrite_oauth_kwargs(kwargs)
        assert rewritten == kwargs


# ---------------------------------------------------------------------------
# Connection.__init__ wiring tests — verify the helpers are reached and
# that their side-effects (sensitive-keys redaction, canonical forward)
# are visible on the public Connection surface.
# ---------------------------------------------------------------------------


def _build_connection(mock_db_api, **kwargs):
    """Construct a Connection with a mocked db_api so no Rust core is touched."""
    from snowflake.connector.connection import Connection

    with patch("snowflake.connector.connection.database_driver_client", return_value=mock_db_api):
        return Connection(**kwargs)


def _connection_set_options_request(mock_db_api):
    return mock_db_api.connection_set_options.call_args_list[0][0][0]


class TestConnectionForwardsCanonicalOauthOptions:
    """OAuth kwargs sent to ``Connection`` reach the Rust core under canonical names."""

    def test_canonical_oauth_kwargs_round_trip_through_connection_set_options(self, mock_db_api):
        _build_connection(
            mock_db_api,
            user="alice",
            account="acme",
            authenticator="OAUTH_AUTHORIZATION_CODE",
            oauth_client_id="client-123",
            oauth_client_secret="shhh",
            oauth_authorization_url="https://idp/authorize",
            oauth_token_request_url="https://idp/token",
            oauth_redirect_uri="http://127.0.0.1:0",
            oauth_scope="session:role:R",
            oauth_disable_pkce=False,
            oauth_enable_single_use_refresh_tokens=True,
        )
        request = _connection_set_options_request(mock_db_api)
        assert request.options[OAUTH_CLIENT_ID] == ConfigSetting(string_value="client-123")
        assert request.options[OAUTH_CLIENT_SECRET] == ConfigSetting(string_value="shhh")
        assert request.options[OAUTH_AUTHORIZATION_URL] == ConfigSetting(string_value="https://idp/authorize")
        assert request.options[OAUTH_TOKEN_REQUEST_URL] == ConfigSetting(string_value="https://idp/token")
        assert request.options[OAUTH_REDIRECT_URI] == ConfigSetting(string_value="http://127.0.0.1:0")
        assert request.options[OAUTH_SCOPE] == ConfigSetting(string_value="session:role:R")
        assert request.options[OAUTH_DISABLE_PKCE] == ConfigSetting(bool_value=False)
        assert request.options["oauth_enable_single_use_refresh_tokens"] == ConfigSetting(bool_value=True)

    def test_legacy_oauth_token_url_alias_is_rewritten_to_canonical(self, mock_db_api):
        _build_connection(
            mock_db_api,
            user="alice",
            account="acme",
            authenticator="OAUTH_CLIENT_CREDENTIALS",
            oauth_client_id="client-123",
            oauth_client_secret="shhh",
            oauth_token_url="https://idp/token",
        )
        request = _connection_set_options_request(mock_db_api)
        # Canonical key only — the legacy alias must not reach the Rust
        # core because there is no `oauth_token_url` entry in
        # `param_registry`.
        assert OAUTH_TOKEN_REQUEST_URL in request.options
        assert "oauth_token_url" not in request.options

    @pytest.mark.parametrize("python_only", sorted(PYTHON_ONLY_OAUTH_KWARGS))
    def test_python_only_oauth_kwargs_never_reach_rust_core(self, mock_db_api, python_only):
        with warnings.catch_warnings():
            # Silence the expected DeprecationWarning so the test only
            # asserts on observed behaviour.
            warnings.simplefilter("ignore", DeprecationWarning)
            _build_connection(
                mock_db_api,
                user="alice",
                account="acme",
                authenticator="OAUTH_AUTHORIZATION_CODE",
                oauth_client_id="client-123",
                **{python_only: True},
            )
        request = _connection_set_options_request(mock_db_api)
        assert python_only not in request.options


class TestConnectionKwargsRedaction:
    """``Connection.kwargs`` never echoes an OAuth secret (analysis §11)."""

    def test_oauth_client_secret_is_redacted(self, mock_db_api):
        conn = _build_connection(
            mock_db_api,
            user="alice",
            account="acme",
            authenticator="OAUTH_AUTHORIZATION_CODE",
            oauth_client_id="client-123",
            oauth_client_secret="do-not-log",
        )
        assert conn.kwargs[OAUTH_CLIENT_SECRET] == "***"
        # Public client_id is fine to leave in the kwargs view.
        assert conn.kwargs[OAUTH_CLIENT_ID] == "client-123"

    def test_token_kwarg_is_redacted_for_legacy_oauth(self, mock_db_api):
        conn = _build_connection(
            mock_db_api,
            user="alice",
            account="acme",
            authenticator="OAUTH",
            token="bearer-token-value",
        )
        assert conn.kwargs["token"] == "***"

    def test_password_redaction_still_works_alongside_oauth(self, mock_db_api):
        # OAuth redaction must extend, not replace, the pre-existing
        # PWD-style redaction list.
        conn = _build_connection(
            mock_db_api,
            user="alice",
            account="acme",
            password="hunter2",
        )
        assert conn.kwargs["password"] == "***"

    def test_repr_of_kwargs_does_not_leak_oauth_secret(self, mock_db_api):
        conn = _build_connection(
            mock_db_api,
            user="alice",
            account="acme",
            authenticator="OAUTH_AUTHORIZATION_CODE",
            oauth_client_id="client-123",
            oauth_client_secret="ultra-secret-shhh",
        )
        rendered = repr(conn.kwargs)
        assert "ultra-secret-shhh" not in rendered
        assert "***" in rendered


class TestSensitiveKeysSetClosedUnderOauth:
    """The OAuth secret set is exactly what the Connection redacts (closure invariant)."""

    def test_sensitive_kwargs_match_module_constant(self):
        # Lock down the closure: if anyone adds a new OAuth secret in
        # the helpers module but forgets to update the Connection
        # redaction set, this test will fail because the constant is
        # the single source of truth.
        assert oauth_helpers.SENSITIVE_OAUTH_KWARGS == SENSITIVE_OAUTH_KWARGS
