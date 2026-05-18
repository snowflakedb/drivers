"""BACKWARD COMPATIBILITY MODULE ONLY.

OAuth Authorization Code (AC) flow placeholder. The actual AC flow —
PKCE generation, loopback listener, browser launch, token exchange,
keyring caching, refresh-on-failure, and DPoP — is implemented in
``sf_core/src/rest/snowflake/oauth/authorization_code.rs`` and driven by
the connect-time kwargs registered in
``sf_core/src/config/param_registry.rs`` (canonical names ``oauth_*``).

The Python wrapper forwards those kwargs to the Rust core as-is, so
there is no Python-level AC class to subclass or replace. This module
only exists so legacy ``from snowflake.connector.auth.oauth_code import
AuthByOauthCode`` imports keep working.

The Rust core implements the full AC sequence and state machine
(cache → refresh → interactive browser leg).
"""

from __future__ import annotations


class AuthByOauthCode:
    """Backward-compatibility placeholder for the legacy AC auth class.

    The legacy driver instantiated this class to drive the AC flow.
    The universal driver runs the flow inside the Rust core; this stub
    is kept only so legacy ``from snowflake.connector.auth.oauth_code
    import AuthByOauthCode`` imports continue to work.
    """


AuthByOAuthCode = AuthByOauthCode
