"""BACKWARD COMPATIBILITY MODULE ONLY.

OAuth Client Credentials (CC) flow placeholder. The actual CC flow —
``grant_type=client_credentials`` token exchange via Basic auth header,
optional client-credentials-in-body, and Snowflake login-request
forwarding — is implemented in
``sf_core/src/rest/snowflake/oauth/client_credentials.rs`` and driven by
the connect-time kwargs registered in
``sf_core/src/config/param_registry.rs`` (canonical names ``oauth_*``).

The Python wrapper forwards those kwargs to the Rust core as-is, so
there is no Python-level CC class to subclass or replace. This module
only exists so legacy ``from snowflake.connector.auth.oauth_credentials
import AuthByOauthCredentials`` imports keep working.

See ``analysis_feature_oauth.md`` §4 (CC flow) and §9 (configuration
matrix) for the full behavioural spec.
"""

from __future__ import annotations


class AuthByOauthCredentials:
    """Backward-compatibility placeholder for the legacy CC auth class.

    The legacy driver instantiated this class to drive the CC flow.
    The universal driver runs the flow inside the Rust core; this stub
    is kept only so legacy ``from snowflake.connector.auth.oauth_credentials
    import AuthByOauthCredentials`` imports continue to work.
    """


AuthByOAuthCredentials = AuthByOauthCredentials
