"""SNOW-3647715: token-based authenticators must not require ``user``.

For OAuth and PROGRAMMATIC_ACCESS_TOKEN flows the principal is encoded
in the token itself, so the connector must not reject the connect call
with ``Missing required parameter 'user'`` when ``user`` is omitted.

These tests do NOT exercise the full auth flow: the integration backend
at ``http://localhost:8090`` does not exist as a real IdP / GS, so every
connect call here is *expected to fail*. What we assert on is *which*
error path is reached — anything except a missing-``user`` error counts
as the wrapper / core having accepted the absent ``user``.
"""

from __future__ import annotations

import re

import pytest


# Two error formats existed before the SNOW-3647715 fix:
#   * `validate_settings` (the pre-flight ValidationFailed path):
#       Missing required parameter 'user'
#   * `build_auth_config` / `LoginMethod::from_settings` (the
#     ConfigError::MissingParameter path):
#       Missing required parameter: user
# Both must be absent for the regression to be considered fixed; the
# regex below catches either form so the assertion is independent of
# *which* validation path the wrapper happened to hit.
_MISSING_USER_RE = re.compile(r"Missing required parameter[: ]\s*['\"]?user['\"]?", re.IGNORECASE)


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


def _attempt_connect(int_test_connection_factory, **kwargs):
    with pytest.raises(Exception) as exc_info:
        int_test_connection_factory(**kwargs)
    return exc_info.value


def _assert_not_missing_user(exception: BaseException) -> None:
    text = _full_error_text(exception)
    assert not _MISSING_USER_RE.search(text), (
        f"Token-based auth must not reject the connect with a missing-`user` error; "
        f"the principal is encoded in the token. Got: {text!r}"
    )


class TestTokenAuthUserIsOptional:
    """``user`` must be optional whenever the principal comes from the token."""

    def test_oauth_access_token_without_user(self, int_test_connection_factory):
        # Pre-acquired bearer token (legacy AUTHENTICATOR=OAUTH).
        exception = _attempt_connect(
            int_test_connection_factory,
            authenticator="OAUTH",
            private_key_file=None,
            user=None,
            token="not-a-real-token",
        )
        _assert_not_missing_user(exception)

    def test_oauth_client_credentials_without_user(self, int_test_connection_factory):
        # Service-identity flow against an external IdP — the IdP issues
        # a token whose claims identify the Snowflake principal.
        exception = _attempt_connect(
            int_test_connection_factory,
            authenticator="OAUTH_CLIENT_CREDENTIALS",
            private_key_file=None,
            user=None,
            oauth_client_id="test-client-id",
            oauth_client_secret="test-client-secret",
            oauth_token_request_url="https://idp.example.com/oauth/token",
        )
        _assert_not_missing_user(exception)

    def test_programmatic_access_token_without_user(self, int_test_connection_factory):
        # PAT — the token literally encodes ``ALTER USER … ADD
        # PROGRAMMATIC ACCESS TOKEN`` so the principal is unambiguous.
        exception = _attempt_connect(
            int_test_connection_factory,
            authenticator="PROGRAMMATIC_ACCESS_TOKEN",
            private_key_file=None,
            user=None,
            token="not-a-real-pat",
        )
        _assert_not_missing_user(exception)
