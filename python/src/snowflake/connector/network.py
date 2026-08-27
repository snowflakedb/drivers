"""BACKWARD COMPATIBILITY MODULE ONLY — legacy authenticator-type constants.

This module is itself deprecated and exists only for backward compatibility:
it re-exports legacy flat authenticator-type constants for drop-in parity
with snowflake-connector-python's ``network.py``. None of these are used by
the Universal Driver; they are retained so downstream consumers that import
them at module load (e.g. dbt-adapters' Workload Identity Federation support)
keep working. Each warns once on first external access, like the legacy
classes in ``errors.py`` / ``result_batch.py``. There is no non-deprecated
successor to migrate to: pass the raw authenticator string (e.g.
``"OAUTH"``) to ``authenticator=`` directly, or use
``snowflake.connector.constants.AuthenticatorType`` — also retained only
for compatibility with the enum-based style — if a typed value is more
convenient.

The values here are not written out again: each is taken from
:class:`~snowflake.connector._internal.authenticator_type.AuthenticatorType`,
which is the one place the list lives on the Python side and which carries the
note on keeping it in sync with the Rust core. Add or change a value there, not
here.
"""

from __future__ import annotations

from ._internal.backward_compatibility import install_backward_compatibility_getattr
from ._internal.decorators import backward_compatibility
from .constants import AuthenticatorType


# ``.value`` (a plain ``str``) rather than the enum member: matches the upstream
# snowflake-connector-python driver, which had these as plain string constants
# (e.g. ``DEFAULT_AUTHENTICATOR = "SNOWFLAKE"``), not enum members — so this is
# drop-in type parity, not just an implementation detail. It also sidesteps a
# subtlety in ``TestNoInternalImportsOfBackwardCompatNames``: that guard test
# flags a marked value whose ``__module__`` differs from the module it's found
# in, and an enum member inherits ``__module__`` from wherever
# ``AuthenticatorType`` is defined (``_internal.authenticator_type``, not
# ``network``). A plain ``str`` has no ``__module__`` at all, so it can't trip
# that check.
DEFAULT_AUTHENTICATOR = backward_compatibility(AuthenticatorType.DEFAULT.value)
EXTERNAL_BROWSER_AUTHENTICATOR = backward_compatibility(AuthenticatorType.EXTERNAL_BROWSER.value)
KEY_PAIR_AUTHENTICATOR = backward_compatibility(AuthenticatorType.KEY_PAIR.value)
OAUTH_AUTHENTICATOR = backward_compatibility(AuthenticatorType.OAUTH.value)
OAUTH_AUTHORIZATION_CODE = backward_compatibility(AuthenticatorType.OAUTH_AUTHORIZATION_CODE.value)
OAUTH_CLIENT_CREDENTIALS = backward_compatibility(AuthenticatorType.OAUTH_CLIENT_CREDENTIALS.value)
USR_PWD_MFA_AUTHENTICATOR = backward_compatibility(AuthenticatorType.USERNAME_PASSWORD_MFA.value)
PROGRAMMATIC_ACCESS_TOKEN = backward_compatibility(AuthenticatorType.PROGRAMMATIC_ACCESS_TOKEN.value)
WORKLOAD_IDENTITY_AUTHENTICATOR = backward_compatibility(AuthenticatorType.WORKLOAD_IDENTITY.value)

__all__ = [
    "AuthenticatorType",
    "DEFAULT_AUTHENTICATOR",
    "EXTERNAL_BROWSER_AUTHENTICATOR",
    "KEY_PAIR_AUTHENTICATOR",
    "OAUTH_AUTHENTICATOR",
    "OAUTH_AUTHORIZATION_CODE",
    "OAUTH_CLIENT_CREDENTIALS",
    "USR_PWD_MFA_AUTHENTICATOR",
    "PROGRAMMATIC_ACCESS_TOKEN",
    "WORKLOAD_IDENTITY_AUTHENTICATOR",
]

# Must be the last statement; see ``install_backward_compatibility_getattr``.
install_backward_compatibility_getattr(__name__)
