"""OAuth connect-kwarg helpers for the Python wrapper.

The universal driver implements every OAuth flow inside the Rust core
(``sf_core/src/rest/snowflake/oauth/``). The Python wrapper's only OAuth
responsibility is to:

* Recognise the OAuth-flavoured ``authenticator`` values so callers can
  branch consistently with the rest of the driver stack
  (``OAUTH`` / ``OAUTH_AUTHORIZATION_CODE`` / ``OAUTH_CLIENT_CREDENTIALS``).
* Forward the OAuth connect kwargs to the Rust core under the canonical
  ``oauth_*`` names registered in
  ``sf_core/src/config/param_registry.rs``.
* Translate the legacy ``snowflake-connector-python`` aliases for OAuth
  parameters into those canonical names (cross-driver configuration matrix,
  Python column).
* Redact OAuth secrets from any log line that echoes connect kwargs
  (cross-driver redaction requirement) — currently ``oauth_client_secret``
  and ``token``.

The helpers in this module are pure functions (no side-effects, no
network, no Rust core). They are wired into ``Connection.__init__`` in a
subsequent commit; this module is intentionally usable on its own so
unit tests can exercise the mapping / redaction logic without spinning
up a connection.

References:
* Cross-driver configuration matrix — canonical names per driver.
* Cross-driver logging & redaction rules.
"""

from __future__ import annotations

import warnings

from collections.abc import Mapping
from typing import Any, Final


# ---------------------------------------------------------------------------
# Canonical OAuth parameter names. These must match the
# canonical names registered in ``sf_core/src/config/param_registry.rs``;
# the Rust core resolves any aliases case-insensitively, so we only
# need to send the canonical lowercase form.
# ---------------------------------------------------------------------------

OAUTH_CLIENT_ID: Final = "oauth_client_id"
OAUTH_CLIENT_SECRET: Final = "oauth_client_secret"
OAUTH_AUTHORIZATION_URL: Final = "oauth_authorization_url"
OAUTH_TOKEN_REQUEST_URL: Final = "oauth_token_request_url"
OAUTH_REDIRECT_URI: Final = "oauth_redirect_uri"
OAUTH_SCOPE: Final = "oauth_scope"
OAUTH_ENABLE_SINGLE_USE_REFRESH_TOKENS: Final = "oauth_enable_single_use_refresh_tokens"
OAUTH_DISABLE_PKCE: Final = "oauth_disable_pkce"
OAUTH_ENABLE_DPOP: Final = "oauth_enable_dpop"
OAUTH_DISABLE_CONSOLE_LOGIN: Final = "oauth_disable_console_login"

#: All canonical OAuth parameter names recognised by the Rust core.
ALL_OAUTH_PARAMS: Final[tuple[str, ...]] = (
    OAUTH_CLIENT_ID,
    OAUTH_CLIENT_SECRET,
    OAUTH_AUTHORIZATION_URL,
    OAUTH_TOKEN_REQUEST_URL,
    OAUTH_REDIRECT_URI,
    OAUTH_SCOPE,
    OAUTH_ENABLE_SINGLE_USE_REFRESH_TOKENS,
    OAUTH_DISABLE_PKCE,
    OAUTH_ENABLE_DPOP,
    OAUTH_DISABLE_CONSOLE_LOGIN,
)

#: OAuth kwargs that contain secrets and must never appear in logs.
#:
#: Combined with the existing PWD-style redaction list in
#: ``Connection.__init__`` so a single grep target covers "things that
#: must never reach a ``tracing`` sink". ``token`` is included because
#: the legacy ``AUTHENTICATOR=OAUTH`` flow passes the access token via
#: that kwarg.
SENSITIVE_OAUTH_KWARGS: Final[frozenset[str]] = frozenset({OAUTH_CLIENT_SECRET, "token"})


# ---------------------------------------------------------------------------
# ``authenticator`` enum values that select an OAuth flow.
# Re-declared here so the Python wrapper has a single source of truth
# for "is this kwarg shape an OAuth-flavoured login?". ``sf_core`` does
# the actual case-insensitive match against ``LoginMethod``.
# ---------------------------------------------------------------------------

AUTHENTICATOR_OAUTH: Final = "OAUTH"
AUTHENTICATOR_OAUTH_AUTHORIZATION_CODE: Final = "OAUTH_AUTHORIZATION_CODE"
AUTHENTICATOR_OAUTH_CLIENT_CREDENTIALS: Final = "OAUTH_CLIENT_CREDENTIALS"

_OAUTH_AUTHENTICATORS: Final[tuple[str, ...]] = (
    AUTHENTICATOR_OAUTH,
    AUTHENTICATOR_OAUTH_AUTHORIZATION_CODE,
    AUTHENTICATOR_OAUTH_CLIENT_CREDENTIALS,
)


def is_oauth_authenticator(authenticator: Any) -> bool:
    """Return ``True`` when ``authenticator`` selects an OAuth flow (case-insensitive).

    Non-string values (``None``, integers, …) return ``False``. The
    Rust core performs its own case-insensitive match; this helper is
    only used by the wrapper to short-circuit OAuth-only logic (kwarg
    rewriting, sensitive-key redaction).
    """
    if not isinstance(authenticator, str):
        return False
    return any(authenticator.casefold() == known.casefold() for known in _OAUTH_AUTHENTICATORS)


# ---------------------------------------------------------------------------
# Legacy alias → canonical map. Drawn from the cross-driver configuration
# matrix (Python column) plus the Python-only switches that the legacy
# ``snowflake-connector-python`` exposes.
# ---------------------------------------------------------------------------

#: Direct rename mapping: ``{legacy_kwarg: canonical_kwarg}``.
#:
#: ``oauth_token_url`` is a JDBC-flavoured alias for
#: ``oauth_token_request_url`` we accept for convenience — the legacy
#: ``snowflake-connector-python`` already uses the canonical name, so
#: this is purely belt-and-suspenders for cross-driver copy-paste.
LEGACY_OAUTH_ALIASES: Final[dict[str, str]] = {
    "oauth_token_url": OAUTH_TOKEN_REQUEST_URL,
}

#: Legacy ``snowflake-connector-python`` OAuth switches that have no
#: representation in the Rust core's ``param_registry``. They are
#: silently accepted (a deprecation warning is emitted) so callers
#: migrating from the old connector don't see ``ProgrammingError`` on
#: every connect; the actual behaviour they would have controlled is
#: either always-on or covered by another canonical param.
#:
#: * ``oauth_enable_refresh_tokens`` (Python-only) — gates whether the
#:   refresh token returned by the IdP is used at all. In the universal
#:   driver this is always ``True`` — refresh tokens are used when the
#:   IdP returns one. ``client_store_temporary_credential`` is the
#:   canonical kwarg that gates *caching* the refresh token.
#: * ``oauth_credentials_in_body`` (Python-only) — toggles
#:   ``client_secret_basic`` vs ``client_secret_post`` for the CC flow.
#:   sf_core currently always uses HTTP Basic; the alternative is
#:   tracked in the OAuth feature analysis but not yet exposed here.
#: * ``oauth_socket_uri`` (Python-only) — separates the listener bind
#:   address from the redirect URI advertised to the IdP. sf_core
#:   currently binds the listener to ``oauth_redirect_uri``.
PYTHON_ONLY_OAUTH_KWARGS: Final[frozenset[str]] = frozenset(
    {
        "oauth_enable_refresh_tokens",
        "oauth_credentials_in_body",
        "oauth_socket_uri",
    }
)


def is_sensitive_oauth_kwarg(name: Any) -> bool:
    """Return ``True`` when ``name`` is an OAuth kwarg whose value must never be logged.

    Case-sensitive on purpose: connect kwargs are normalised to
    lowercase canonical names before they reach this check, and the
    Rust core's ``param_registry`` likewise stores canonical names in
    lowercase.
    """
    if not isinstance(name, str):
        return False
    return name in SENSITIVE_OAUTH_KWARGS


def redacted_kwargs_for_log(kwargs: Mapping[str, Any]) -> dict[str, Any]:
    """Return a copy of ``kwargs`` with every OAuth secret value replaced by ``"***"``.

    The redaction list is intentionally narrow — it covers only the
    OAuth secrets. ``Connection.__init__`` composes this
    with its existing redaction set for ``password`` /
    ``private_key`` / ``passcode`` to produce the final
    ``connection.kwargs`` view.

    Non-string keys (which would already be a bug) are passed through
    unchanged; only ``str`` keys participate in the redaction check.
    """
    return {key: ("***" if is_sensitive_oauth_kwarg(key) else value) for key, value in kwargs.items()}


# ---------------------------------------------------------------------------
# kwarg rewriting
# ---------------------------------------------------------------------------


def rewrite_oauth_kwargs(kwargs: dict[str, Any]) -> dict[str, Any]:
    """Translate legacy / Python-only OAuth kwargs to canonical core names.

    Performs three transformations on a **shallow copy** of ``kwargs``:

    1. Legacy aliases in :data:`LEGACY_OAUTH_ALIASES` are popped and
       re-inserted under their canonical name (e.g.
       ``oauth_token_url`` → ``oauth_token_request_url``). When both
       the alias and the canonical key are present, the canonical key
       wins and the alias is discarded — matching how
       ``_rewrite_mfa_params`` handles ``client_request_mfa_token`` vs
       ``client_store_temporary_credential``.
    2. Python-only switches in :data:`PYTHON_ONLY_OAUTH_KWARGS` are
       popped after emitting a ``DeprecationWarning`` so callers know
       the kwarg is silently dropped on the universal driver. They are
       not forwarded to the Rust core because there is no
       ``param_registry`` entry for them.
    3. All other kwargs are passed through untouched — including the
       canonical ``oauth_*`` kwargs the caller may already be using.

    The returned dict is a new object; ``kwargs`` is not mutated. Pass
    the result to ``create_config_settings_from_dict`` (or assign it
    back to your local ``kwargs`` variable) before forwarding to the
    Rust core.
    """
    rewritten: dict[str, Any] = dict(kwargs)

    for alias, canonical in LEGACY_OAUTH_ALIASES.items():
        if alias not in rewritten:
            continue
        legacy_value = rewritten.pop(alias)
        if canonical not in rewritten:
            rewritten[canonical] = legacy_value

    for python_only in PYTHON_ONLY_OAUTH_KWARGS:
        if python_only not in rewritten:
            continue
        rewritten.pop(python_only)
        warnings.warn(
            (
                f"{python_only!r} is a legacy snowflake-connector-python "
                "kwarg that has no equivalent on the universal driver and "
                "is silently ignored. See the cross-driver configuration "
                "matrix for canonical parameter names."
            ),
            DeprecationWarning,
            stacklevel=3,
        )

    return rewritten
