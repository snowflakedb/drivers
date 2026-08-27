"""Backward-compat behavior for ``snowflake.connector.network`` constants.

Every authenticator-type constant is a legacy re-export: importable, equal to
its snowflake-connector-python value, and emits a one-shot DeprecationWarning
on first external access.
"""

from __future__ import annotations

import warnings

import pytest

import snowflake.connector.constants as constants
import snowflake.connector.network as network

# Reuses the fixture already defined for the generic backward-compat helper;
# both files touch the same process-wide dedup set.
from .test_backward_compatibility_warnings import _reset_backward_compat_dedup_set  # noqa: F401


AUTH_CONSTANTS = {
    "DEFAULT_AUTHENTICATOR": "SNOWFLAKE",
    "EXTERNAL_BROWSER_AUTHENTICATOR": "EXTERNALBROWSER",
    "KEY_PAIR_AUTHENTICATOR": "SNOWFLAKE_JWT",
    "OAUTH_AUTHENTICATOR": "OAUTH",
    "OAUTH_AUTHORIZATION_CODE": "OAUTH_AUTHORIZATION_CODE",
    "OAUTH_CLIENT_CREDENTIALS": "OAUTH_CLIENT_CREDENTIALS",
    "USR_PWD_MFA_AUTHENTICATOR": "USERNAME_PASSWORD_MFA",
    "PROGRAMMATIC_ACCESS_TOKEN": "PROGRAMMATIC_ACCESS_TOKEN",
    "WORKLOAD_IDENTITY_AUTHENTICATOR": "WORKLOAD_IDENTITY",
}


@pytest.mark.parametrize("name", list(AUTH_CONSTANTS))
def test_first_external_access_warns_once(name):
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        first = getattr(network, name)
        second = getattr(network, name)  # second access: deduped

    assert first == AUTH_CONSTANTS[name]
    assert first is second
    bc_warnings = [w for w in caught if issubclass(w.category, DeprecationWarning) and name in str(w.message)]
    assert len(bc_warnings) == 1, [str(w.message) for w in caught]
    assert "backward compatibility" in str(bc_warnings[0].message)
    assert f"network.{name}" in str(bc_warnings[0].message)


def test_flat_constants_are_plain_strings():
    """Each flat constant must be a plain ``str``, not an ``AuthenticatorType``
    member: snowflake-connector-python declared these as plain strings, so
    anything stricter (``type(x) is str``, ``json.dumps`` of a containing dict,
    an ``is`` comparison against an interned literal) would behave differently
    for a drop-in consumer. It also keeps them clear of
    ``TestNoInternalImportsOfBackwardCompatNames``, which flags a marked value
    whose ``__module__`` differs from the module it is found in.
    """
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        for name in AUTH_CONSTANTS:
            value = getattr(network, name)
            assert type(value) is str, f"{name} must be a plain str, got {type(value)}"


def test_authenticator_type_members_str_as_raw_value():
    """``AuthenticatorType`` is a ``str``-mixed ``Enum``; without the
    ``__str__`` override in ``_internal/authenticator_type.py``,
    ``str(member)``/f-string interpolation would render
    ``"AuthenticatorType.MEMBER"`` on Python < 3.11 instead of the raw wire
    value, silently corrupting any downstream code that formats the
    authenticator into a request/string rather than just comparing it.
    """
    for member in network.AuthenticatorType:
        assert str(member) == member.value
        assert f"{member}" == member.value


def test_id_token_is_not_exposed_to_python():
    """``ID_TOKEN`` is wire-internal — the driver sends it when replaying a
    cached SSO token, and it is never a value a caller passes to
    ``authenticator=``. Exposing it here would advertise an unsupported
    option, so its absence is deliberate and pinned rather than incidental.
    """
    assert "ID_TOKEN" not in {member.value for member in network.AuthenticatorType}
    assert "ID_TOKEN" not in set(AUTH_CONSTANTS.values())


def test_authenticator_type_access_does_not_warn():
    """``AuthenticatorType`` is, like the flat legacy names, a
    backward-compatibility convenience — but unlike them it is never marked
    with ``@backward_compatibility``, so it must never go through the
    warn-on-access path. It's re-exported from two modules (``constants``,
    ``network``); the shared ``install_backward_compatibility_getattr``
    machinery only lets a class's *defining* module warn (see
    ``TestNoInternalImportsOfBackwardCompatNames``), so there's no way to
    mark it without either module rebinding an already-marked name. Staying
    silent is the correct, deliberate behavior here, not an oversight.
    """
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        network.AuthenticatorType  # noqa: B018

    assert not [w for w in caught if issubclass(w.category, DeprecationWarning)]


def test_authenticator_type_is_reexported_from_constants():
    """``network.py`` must import ``AuthenticatorType`` rather than redefine
    it: the class is canonically defined for ``snowflake.connector.constants``,
    and ``network`` only re-exports the same object for backward
    compatibility.
    """
    assert network.AuthenticatorType is constants.AuthenticatorType
