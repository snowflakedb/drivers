"""Shared fixtures for type tests."""

from __future__ import annotations

from decimal import getcontext
from typing import Any

import pytest


# =============================================================================
# DECIMAL CONTEXT CONFIGURATION
# =============================================================================
# DECFLOAT and NUMBER types require Python's Decimal context to have 38-digit
# precision to properly represent all values without rounding
DECIMAL_PRECISION_38 = 38


@pytest.fixture(autouse=True)
def setup_decimal_precision():
    """Set decimal context precision to 38 for all type tests.

    This ensures Decimal operations don't lose precision for DECFLOAT
    and NUMBER types which support up to 38 significant digits.
    """
    old_prec = getcontext().prec
    getcontext().prec = DECIMAL_PRECISION_38
    try:
        yield
    finally:
        getcontext().prec = old_prec


@pytest.fixture
def executemany_insert(cursor):
    """Fixture for bulk-inserting rows via executemany and reading them back.

    Returns a callable:
        executemany_insert(table_name, sql, rows) -> list[Row]

    It executes cursor.executemany(sql, rows), then SELECTs all rows
    from the table ordered by the first column.
    """

    def _executemany_insert(
        table_name: str, sql: str, rows: list[tuple[Any, ...]]
    ) -> list[tuple[Any, ...]]:
        cursor.executemany(sql, rows)
        cursor.execute(f"SELECT * FROM {table_name} ORDER BY 1")
        return cursor.fetchall()

    return _executemany_insert
