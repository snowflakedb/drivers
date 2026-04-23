"""DATE type NumPy tests for Universal Driver (Python-specific).

This module tests DATE type conversion to numpy.datetime64 (day resolution)
when NumPy mode is enabled. These tests are Python-specific and not shared
with other driver implementations.

In NumPy mode, DATE values are returned as numpy.datetime64[D] instead of
the default datetime.date. Day resolution ('D') is used because DATE has no
sub-day component.
"""

from __future__ import annotations

import numpy as np

from .utils import assert_type


DATE_EPOCH = np.datetime64("1970-01-01", "D")
DATE_PRE_EPOCH = np.datetime64("1969-12-31", "D")
DATE_2024_JAN = np.datetime64("2024-01-15", "D")
DATE_MIN = np.datetime64("0001-01-01", "D")
DATE_MAX = np.datetime64("9999-12-31", "D")


def _assert_datetime64_day(value) -> None:
    """Assert the value is numpy.datetime64 with day resolution."""
    assert isinstance(value, np.datetime64), f"Expected numpy.datetime64, got {type(value).__name__}"
    unit = np.datetime_data(value)[0]
    assert unit == "D", f"Expected day resolution ('D'), got '{unit}'"


class TestDateNumPy:
    """Test suite for DATE type NumPy conversion (Python-specific)."""

    def test_should_cast_usual_date_values_to_numpy_datetime64_with_day_resolution(self, cursor_with_numpy):
        # Given Snowflake client is logged in with NumPy mode enabled
        pass

        # When Query "SELECT '1970-01-01'::DATE, '1969-12-31'::DATE, '2024-01-15'::DATE" is executed
        sql = "SELECT '1970-01-01'::DATE, '1969-12-31'::DATE, '2024-01-15'::DATE"
        cursor_with_numpy.execute(sql)
        result = cursor_with_numpy.fetchone()

        # Then All values should be returned as numpy.datetime64 type with day resolution
        assert_type(result, np.datetime64)
        for v in result:
            _assert_datetime64_day(v)

        # And Values should match exactly [1970-01-01, 1969-12-31, 2024-01-15]
        assert result == (DATE_EPOCH, DATE_PRE_EPOCH, DATE_2024_JAN)

    def test_should_cast_boundary_date_values_to_numpy_datetime64_with_day_resolution(self, cursor_with_numpy):
        # Given Snowflake client is logged in with NumPy mode enabled
        pass

        # When Query "SELECT '0001-01-01'::DATE, '9999-12-31'::DATE" is executed
        sql = "SELECT '0001-01-01'::DATE, '9999-12-31'::DATE"
        cursor_with_numpy.execute(sql)
        result = cursor_with_numpy.fetchone()

        # Then All values should be returned as numpy.datetime64 type with day resolution
        assert_type(result, np.datetime64)
        for v in result:
            _assert_datetime64_day(v)

        # And Values should match exactly [0001-01-01, 9999-12-31]
        assert result == (DATE_MIN, DATE_MAX)

    def test_should_handle_null_values_for_date_with_numpy(self, cursor_with_numpy):
        # Given Snowflake client is logged in with NumPy mode enabled
        pass

        # When Query "SELECT NULL::DATE, '2024-01-15'::DATE, NULL::DATE" is executed
        sql = "SELECT NULL::DATE, '2024-01-15'::DATE, NULL::DATE"
        cursor_with_numpy.execute(sql)
        result = cursor_with_numpy.fetchone()

        # Then Non-null values should be returned as numpy.datetime64 type with day resolution
        assert_type(result, np.datetime64, can_be_none=True)
        for v in result:
            if v is not None:
                _assert_datetime64_day(v)

        # And Result should contain [NULL, 2024-01-15, NULL]
        assert result == (None, DATE_2024_JAN, None)
