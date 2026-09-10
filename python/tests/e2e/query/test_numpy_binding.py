"""Numpy scalar binding with pyformat and ``numpy=True`` fetch.

Covers integers, floats, bools, and ``datetime64`` bound into INTEGER / FLOAT /
TIMESTAMP_NTZ / DATE / TIMESTAMP_LTZ / TIMESTAMP_TZ / BOOLEAN.
"""

from __future__ import annotations

import datetime
import time

import numpy as np
import pytest


_NUMPY_BINDING_CASES = [
    pytest.param(
        "America/Los_Angeles",
        "1.79769313486e+308",
        np.True_,
        np.datetime64("2005-02-25T03:30"),
        id="america_los_angeles",
    ),
    pytest.param(
        "Asia/Tokyo",
        "-1.79769313486e+308",
        np.False_,
        np.datetime64("1970-12-31T05:00:00"),
        id="asia_tokyo",
    ),
    pytest.param(
        "America/New_York",
        "-1.79769313486e+308",
        np.True_,
        np.datetime64("1969-12-31T05:00:00"),
        id="america_new_york",
    ),
    pytest.param(
        "UTC",
        "-1.79769313486e+308",
        np.False_,
        np.datetime64("1968-11-12T07:00:00.123"),
        id="utc",
    ),
]


class TestNumpyDatatypeBinding:
    @pytest.mark.parametrize("tz, float_value, numpy_bool, specific_date", _NUMPY_BINDING_CASES)
    def test_should_bind_numpy_scalars_with_pyformat(
        self, cursor_with_numpy, tmp_schema, tz, float_value, numpy_bool, specific_date
    ):
        # Given Snowflake client is logged in with numpy=True and pyformat paramstyle
        epoch_time = time.time()
        current_datetime = datetime.datetime.fromtimestamp(epoch_time)
        current_datetime64 = np.datetime64(current_datetime)
        expected_specific_date = specific_date.astype(datetime.datetime)
        table_name = f"{tmp_schema}.test_numpy_binding_{tz.replace('/', '_')}"

        cursor_with_numpy.execute(
            f"""
            CREATE OR REPLACE TEMPORARY TABLE {table_name} (
                c1  integer,
                c2  integer,
                c3  integer,
                c4  integer,
                c5  float,
                c6  float,
                c7  float,
                c8  timestamp_ntz,
                c9  date,
                c10 timestamp_ltz,
                c11 timestamp_tz,
                c12 boolean
            )
            """
        )
        cursor_with_numpy.execute(f"ALTER SESSION SET timezone='{tz}'")
        # When Numpy integer, float, datetime64, and bool scalars are bound into INSERT ... VALUES(%s)
        cursor_with_numpy.execute(
            f"""
            INSERT INTO {table_name}(
                c1, c2, c3, c4, c5, c6, c7, c8, c9, c10, c11, c12
            )
            VALUES(%s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s)
            """,
            (
                np.iinfo(np.int8).max,
                np.iinfo(np.int16).max,
                np.iinfo(np.int32).max,
                np.iinfo(np.int64).max,
                np.finfo(np.float16).max,
                np.finfo(np.float32).max,
                np.float64(float_value),
                current_datetime64,
                current_datetime64,
                current_datetime64,
                specific_date,
                numpy_bool,
            ),
        )

        rec = cursor_with_numpy.execute(
            f"""
            SELECT c1, c2, c3, c4, c5, c6, c7, c8, c9, c10, c11, c12
              FROM {table_name}
            """
        ).fetchone()

        # Then Bound numpy values round-trip through INTEGER / FLOAT / TIMESTAMP / BOOLEAN columns
        assert np.int8(rec[0]) == np.iinfo(np.int8).max
        assert np.int16(rec[1]) == np.iinfo(np.int16).max
        assert np.int32(rec[2]) == np.iinfo(np.int32).max
        assert np.int64(rec[3]) == np.iinfo(np.int64).max
        assert np.float16(rec[4]) == np.finfo(np.float16).max
        assert np.float32(rec[5]) == np.finfo(np.float32).max
        assert rec[6] == np.float64(float_value)
        assert rec[7] == current_datetime64
        assert str(rec[8]) == str(current_datetime64)[0:10]
        assert rec[9] == datetime.datetime.fromtimestamp(epoch_time, rec[9].tzinfo)
        assert rec[10] == expected_specific_date.replace(tzinfo=rec[10].tzinfo)
        assert isinstance(rec[11], bool)
        assert rec[11] == numpy_bool
        assert np.bool_(rec[11]) == numpy_bool
