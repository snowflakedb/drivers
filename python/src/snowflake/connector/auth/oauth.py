"""BACKWARD COMPATIBILITY MODULE ONLY.

The Snowflake universal driver implements every OAuth flow in the Rust core
(`sf_core/src/rest/snowflake/oauth/`). The Python wrapper forwards OAuth
kwargs from :func:`snowflake.connector.connect` straight through to the
core via :mod:`snowflake.connector._internal.api_client`, so there is no
Python-level OAuth state machine to subclass or instantiate.

This module exists purely so that code written against the legacy
`snowflake-connector-python` package — which exposed
:class:`AuthByOAuth` here — keeps importing without an
``ImportError``. The class is an opaque marker: it inherits from
``object`` and exposes no methods. Driver code does not look at this
module at all.

Legacy ``AUTHENTICATOR=OAUTH`` + pre-acquired ``token=`` flow. The
Rust core forwards the token unchanged to Snowflake's login endpoint.
"""

from __future__ import annotations


class AuthByOAuth:
    """Backward-compatibility placeholder for the legacy ``AUTHENTICATOR=OAUTH`` auth class.

    The legacy driver instantiated this class to encapsulate the
    pre-acquired access-token login path. The universal driver
    performs that login inside the Rust core; this stub is kept only
    so legacy ``from snowflake.connector.auth.oauth import AuthByOAuth``
    imports continue to work.
    """


AuthByOauth = AuthByOAuth
