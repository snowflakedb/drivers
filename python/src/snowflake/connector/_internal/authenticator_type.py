"""The ``AuthenticatorType`` enum: one member per authenticator the driver accepts.

DEPRECATED. Retained only for backward compatibility with
snowflake-connector-python, whose callers imported these values to pass to
``authenticator=``. The Universal Driver itself never reads this enum — it only
needs the raw string. New code should pass the string directly (e.g.
``authenticator="OAUTH"``); this exists so existing code keeps working.

MUST BE KEPT IN SYNC WITH THE RUST CORE. Every value below has to be a spelling
the core's connection-config parser accepts, because callers read a value from
here and hand it straight back as ``authenticator=`` — a value listed here but
rejected by the core is a broken round trip. Nothing generates this file, so the
guard is a test on the Rust side:
``sf_core::config::connection_config``'s
``recognises_every_authenticator_spelling_exposed_to_python``. It reads *this
file* and runs the real parser over every value, so adding a member here that
the core does not accept fails the Rust test suite. If you add one, add the
matching arm in ``sf_core/src/config/connection_config.rs`` too.

The reverse does not hold, deliberately: the core accepts spellings this enum
does not list (``""`` and ``"SNOWFLAKE_PASSWORD"`` for password auth,
``https://`` URLs for native Okta SSO). Only listing something the core rejects
is a bug.

Re-exported by :mod:`snowflake.connector.constants`. Unlike the flat
``*_AUTHENTICATOR`` constants in :mod:`snowflake.connector.network`, this enum
does not emit a deprecation warning on access: it's re-exported from two
modules, and the shared warn-once machinery only lets a single defining module
warn, so there's no clean way to fire exactly one warning across both re-export
paths.
"""

from __future__ import annotations

from enum import Enum


class AuthenticatorType(str, Enum):
    """Authenticator types accepted by the Universal Driver's ``authenticator=`` connection parameter."""

    def __str__(self) -> str:
        # Plain ``str``-mixed ``Enum`` members format as ``"ClassName.MEMBER"``
        # on Python < 3.11 (this package's floor); this override keeps
        # ``str(AuthenticatorType.X)`` and f-string interpolation equal to the
        # raw wire value, matching ``StrEnum`` semantics on all supported
        # versions.
        return str(self.value)

    DEFAULT = "SNOWFLAKE"
    EXTERNAL_BROWSER = "EXTERNALBROWSER"
    KEY_PAIR = "SNOWFLAKE_JWT"
    OAUTH = "OAUTH"
    OAUTH_AUTHORIZATION_CODE = "OAUTH_AUTHORIZATION_CODE"
    OAUTH_CLIENT_CREDENTIALS = "OAUTH_CLIENT_CREDENTIALS"
    USERNAME_PASSWORD_MFA = "USERNAME_PASSWORD_MFA"
    PROGRAMMATIC_ACCESS_TOKEN = "PROGRAMMATIC_ACCESS_TOKEN"
    WORKLOAD_IDENTITY = "WORKLOAD_IDENTITY"


__all__ = ["AuthenticatorType"]
