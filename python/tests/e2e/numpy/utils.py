"""Shared helpers for numpy type tests.

Fetch helpers return a list of rows (each row a ``list``) so callers can use the
same ``get_row`` / ``get_column`` shape as the pandas type suite. SQL NULLs are
``None``. There is no numpy batch API, so both fetch helpers go through
``fetchall()``, which materializes every downloaded chunk;
``execute_and_fetch_multiple_batches`` differs only in requiring a non-empty
result, mirroring the pandas suite's batch helper.
"""

from __future__ import annotations

from collections.abc import Callable, Iterable, Sequence
from datetime import datetime, time
from decimal import Decimal

import numpy as np

from tests.e2e.types.utils import iana_tz_name


NULL_FLOAT = None


def is_bool(value) -> bool:
    return isinstance(value, (bool, np.bool_))


def is_integer(value) -> bool:
    return isinstance(value, (int, np.integer)) and not isinstance(value, bool)


def is_numpy_integer(value) -> bool:
    """Native Arrow integer path under ``numpy=True`` (``FIXED_to_numpy_int64``).

    ``is_integer`` also matches Python ``int``, so it cannot tell this path from
    the decimal128 fallback. SQL INT is NUMBER(38,0); the converter follows
    Arrow's physical type, not the SQL name.
    """
    return isinstance(value, np.integer)


def is_float(value) -> bool:
    return isinstance(value, (float, np.floating)) and not isinstance(value, bool)


def is_object(value) -> bool:
    """Decimal128 fallback: Python ``int`` or ``Decimal``, not ``numpy.int64``."""
    return isinstance(value, (int, Decimal)) and not isinstance(value, bool)


def is_decimal(value) -> bool:
    return isinstance(value, Decimal)


def is_datetime64(value) -> bool:
    return isinstance(value, np.datetime64)


def is_datetime64_ns(value) -> bool:
    return isinstance(value, np.datetime64) and np.datetime_data(value)[0] == "ns"


def is_datetime64_tz(value) -> bool:
    return isinstance(value, datetime) and value.tzinfo is not None


def is_string(value) -> bool:
    return isinstance(value, str)


def is_binary(value) -> bool:
    return isinstance(value, (bytes, bytearray, memoryview))


def is_time(value) -> bool:
    return isinstance(value, time)


def execute_and_fetch(cursor, sql: str, params=None) -> list[list]:
    cursor.execute(sql, params)
    return [list(row) for row in cursor.fetchall()]


def execute_and_fetch_multiple_batches(cursor, sql: str, params=None) -> list[list]:
    rows = execute_and_fetch(cursor, sql, params)
    assert rows, "expected at least one row"
    return rows


def assert_dtypes(rows: Sequence[Sequence], expected: list[Callable]) -> None:
    assert rows, "expected at least one row"
    assert len(rows[0]) == len(expected), f"Column count mismatch: {len(rows[0])} vs {len(expected)}"
    for col_i, check in enumerate(expected):
        sample = next((row[col_i] for row in rows if row[col_i] is not None), None)
        if sample is None:
            continue
        assert check(sample), f"Column {col_i}: value {sample!r} ({type(sample).__name__}) failed {check.__name__}"


def assert_string_dtypes(rows: Sequence[Sequence]) -> None:
    assert_dtypes(rows, [is_string] * len(rows[0]))


def get_row(rows: Sequence[Sequence], idx: int) -> list:
    return list(rows[idx])


def get_column(rows: Sequence[Sequence], idx: int) -> list:
    return [row[idx] for row in rows]


def assert_datetime_type(values: Iterable, can_be_none: bool = False) -> None:
    for i, value in enumerate(values):
        if can_be_none and value is None:
            continue
        assert isinstance(value, (datetime, np.datetime64)), (
            f"Value at index {i} should be datetime or numpy.datetime64, got {type(value).__name__}"
        )


def assert_timezone(values: Iterable, expected_tz: str | None, can_be_none: bool = False) -> None:
    for i, value in enumerate(values):
        if can_be_none and value is None:
            continue
        if isinstance(value, np.datetime64):
            assert expected_tz is None, f"Value at index {i} is naive numpy.datetime64; expected tz '{expected_tz}'"
            continue
        if expected_tz:
            assert value.tzinfo is not None, f"Value at index {i} should have timezone info (tzinfo is None)"
            actual_tz = iana_tz_name(value.tzinfo)
            assert actual_tz == expected_tz, (
                f"Value at index {i}: expected tz '{expected_tz}', got '{actual_tz}' (tzinfo={value.tzinfo!r})"
            )
        else:
            assert getattr(value, "tzinfo", None) is None, (
                f"Value at index {i} should not have timezone info, got {getattr(value, 'tzinfo', None)}"
            )


def to_datetime64_ns(value) -> np.datetime64:
    if isinstance(value, np.datetime64):
        return value.astype("datetime64[ns]")
    if isinstance(value, datetime):
        return np.datetime64(value.replace(tzinfo=None), "ns")
    return np.datetime64(value, "ns")


def to_datetime64_d(value) -> np.datetime64:
    if isinstance(value, np.datetime64):
        return value.astype("datetime64[D]")
    return np.datetime64(str(value), "D")
