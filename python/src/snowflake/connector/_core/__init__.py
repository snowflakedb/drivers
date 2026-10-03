"""Validate that the installed distribution and native bridge agree."""

from .._internal._distribution_profile import FIPS_TLS_CANDIDATE
from . import sf_core_python


if not hasattr(sf_core_python, "fips_tls_enabled") or sf_core_python.fips_tls_enabled() is not FIPS_TLS_CANDIDATE:
    raise RuntimeError(
        "sf_core_python does not match the installed snowflake.connector distribution's "
        "FIPS-TLS build profile. Uninstall both snowflake-connector-python and "
        "snowflake-connector-python-fips-candidate, then reinstall only the intended wheel. "
        "The fips-tls feature identifies a build choice, not FIPS certification."
    )
