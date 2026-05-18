"""Shared fixtures for pandas tests."""

from __future__ import annotations

import pytest


@pytest.fixture(autouse=True)
def _reset_arrow_number_to_decimal(connection):
    """Reset the ``arrow_number_to_decimal`` flag between tests.

    Several tests flip ``connection.arrow_number_to_decimal = True`` to get
    lossless Decimal columns for high-precision NUMBER(p,s) values. With the
    module-scoped ``connection`` fixture the flag persists across tests in
    the same module, turning every later NUMBER column into Decimal and
    failing ``is_float_dtype`` assertions. Reset to the default (False) after
    each test so the shared connection stays untainted.
    """
    yield
    try:
        connection.arrow_number_to_decimal = False
    except Exception:
        pass
    try:
        connection.arrow_number_to_decimal_setter = False
    except Exception:
        pass
