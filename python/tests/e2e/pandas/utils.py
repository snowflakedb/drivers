"""Shared helpers for pandas type tests."""

from __future__ import annotations

import pandas as pd

from tests.compatibility import is_old_driver


is_bool = pd.api.types.is_bool_dtype
is_integer = pd.api.types.is_integer_dtype
is_float = pd.api.types.is_float_dtype
is_object = pd.api.types.is_object_dtype
is_datetime64 = pd.api.types.is_datetime64_dtype
is_datetime64_ns = pd.api.types.is_datetime64_ns_dtype
is_datetime64_tz = pd.api.types.is_datetime64tz_dtype
is_timedelta = pd.api.types.is_timedelta64_dtype
is_string = pd.api.types.is_string_dtype

NULL_FLOAT = float("nan")


def enable_decimal_mode(cursor) -> None:
    """Enable exact decimal representation for high-precision scale > 0 columns.

    Only needed for tests that assert lossless 38-digit values with scale > 0;
    all other tests rely on the default float64 conversion.
    """
    if is_old_driver():
        cursor.connection.arrow_number_to_decimal_setter = True
    else:
        cursor.connection.arrow_number_to_decimal = True


def execute_and_fetch(cursor, sql: str, params=None) -> pd.DataFrame:
    cursor.execute(sql, params)
    return cursor.fetch_pandas_all()


def execute_and_fetch_multiple_batches(cursor, sql: str, params=None) -> pd.DataFrame:
    cursor.execute(sql, params)
    batches = list(cursor.fetch_pandas_batches())
    assert batches, "expected at least one pandas batch"
    return pd.concat(batches, ignore_index=True)


def assert_dtypes(df: pd.DataFrame, expected: list) -> None:
    assert df.shape[1] == len(expected), f"Column count mismatch: {df.shape[1]} vs {len(expected)}"
    for i, (dtype, check) in enumerate(zip(df.dtypes, expected)):
        assert check(dtype), f"Column {i} ({df.columns[i]}): dtype {dtype} failed {check.__name__}"


def get_row(df: pd.DataFrame, idx: int) -> list:
    # use list() instead of .tolist(), as tolist() converts np/pandas types to native python types
    return list(df.iloc[idx])


def get_column(df: pd.DataFrame, idx: int) -> list:
    # use list() instead of .tolist(), as tolist() converts np/pandas types to native python types
    return list(df.iloc[:, idx])
