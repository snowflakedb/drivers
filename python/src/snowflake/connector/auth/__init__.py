"""Backward-compatibility shims for the legacy ``snowflake.connector.auth`` package.

The universal driver runs every authentication flow inside the Rust core
(``sf_core/src/rest/snowflake/``). Drivers code does not depend on
anything imported from this package — every submodule here is a
placeholder kept so legacy ``from snowflake.connector.auth.<flow> import
AuthBy<Flow>`` imports keep working.

The Rust core implements the cross-driver behavioural spec for the
OAuth flows.
"""

from __future__ import annotations

from .oauth import AuthByOAuth, AuthByOauth
from .oauth_code import AuthByOAuthCode, AuthByOauthCode
from .oauth_credentials import AuthByOAuthCredentials, AuthByOauthCredentials


__all__ = [
    "AuthByOAuth",
    "AuthByOAuthCode",
    "AuthByOAuthCredentials",
    "AuthByOauth",
    "AuthByOauthCode",
    "AuthByOauthCredentials",
]
