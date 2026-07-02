"""TIMESTAMP_NTZ type NumPy tests for Universal Driver (Python-specific).

This module tests TIMESTAMP_NTZ type conversion to numpy.datetime64 (nanosecond
resolution) when NumPy mode is enabled. These tests are Python-specific and not
shared with other driver implementations.

In NumPy mode, TIMESTAMP_NTZ values are returned as numpy.datetime64[ns] instead
of the default datetime.datetime. Unlike Python's datetime, which caps fractional
seconds at microseconds (6 digits), numpy.datetime64[ns] preserves the full
nanosecond precision (9 digits) supported by Snowflake.

Range note: numpy.datetime64[ns] is backed by int64 nanoseconds from the Unix
epoch, so it can only represent values between roughly 1677-09-21 and
2262-04-11. All boundary tests stay within that range.
"""

from __future__ import annotations

import numpy as np
import pytest

from .utils import assert_type


TS_2024_JAN = np.datetime64("2024-01-15T10:30:00", "ns")
TS_EPOCH = np.datetime64("1970-01-01T00:00:00", "ns")
TS_MICROSECONDS = np.datetime64("2024-01-15T10:30:00.123456", "ns")
TS_NANOSECONDS = np.datetime64("2024-01-15T10:30:00.123456789", "ns")
TS_MIN_IN_NS_RANGE = np.datetime64("1678-01-01T00:00:00", "ns")
TS_MAX_IN_NS_RANGE = np.datetime64("2261-12-31T23:59:59.999999999", "ns")


def _assert_datetime64_ns(value) -> None:
    """Assert the value is numpy.datetime64 with nanosecond resolution."""
    assert isinstance(value, np.datetime64), f"Expected numpy.datetime64, got {type(value).__name__}"
    unit = np.datetime_data(value)[0]
    assert unit == "ns", f"Expected nanosecond resolution ('ns'), got '{unit}'"


class TestTimestampNtzNumPy:
    """Test suite for TIMESTAMP_NTZ type NumPy conversion (Python-specific)."""

    @pytest.mark.parametrize(
        "input_value,expected",
        [
            ("2024-01-15 10:30:00", TS_2024_JAN),
            ("1970-01-01 00:00:00", TS_EPOCH),
            ("2024-01-15 10:30:00.123456", TS_MICROSECONDS),
        ],
        ids=["basic", "epoch", "microseconds"],
    )
    def test_should_cast_usual_timestamp_ntz_values_to_numpy_datetime64_with_nanosecond_resolution(
        self, cursor_with_numpy, input_value, expected
    ):
        # Given Snowflake client is logged in with NumPy mode enabled
        pass

        # When Query "SELECT '<input>'::TIMESTAMP_NTZ" is executed
        cursor_with_numpy.execute(f"SELECT '{input_value}'::TIMESTAMP_NTZ")
        result = cursor_with_numpy.fetchone()

        # Then Value should be numpy.datetime64 with nanosecond resolution
        _assert_datetime64_ns(result[0])

        # And Value should match exactly <expected>
        assert result[0] == expected

    def test_should_preserve_nanosecond_precision_for_timestamp_ntz_with_numpy(self, cursor_with_numpy):
        # Given Snowflake client is logged in with NumPy mode enabled
        pass

        # When Query "SELECT '2024-01-15 10:30:00.123456789'::TIMESTAMP_NTZ" is executed
        sql = "SELECT '2024-01-15 10:30:00.123456789'::TIMESTAMP_NTZ"
        cursor_with_numpy.execute(sql)
        result = cursor_with_numpy.fetchone()

        # Then Value should be numpy.datetime64 with nanosecond resolution
        _assert_datetime64_ns(result[0])

        # And Value should match exactly 2024-01-15T10:30:00.123456789
        assert result[0] == TS_NANOSECONDS

    def test_should_cast_boundary_timestamp_ntz_values_within_numpy_nanosecond_range(self, cursor_with_numpy):
        # Given Snowflake client is logged in with NumPy mode enabled
        pass

        # When Query "SELECT '1678-01-01 00:00:00'::TIMESTAMP_NTZ,
        # '2261-12-31 23:59:59.999999999'::TIMESTAMP_NTZ" is executed
        sql = "SELECT '1678-01-01 00:00:00'::TIMESTAMP_NTZ, '2261-12-31 23:59:59.999999999'::TIMESTAMP_NTZ"
        cursor_with_numpy.execute(sql)
        result = cursor_with_numpy.fetchone()

        # Then All values should be returned as numpy.datetime64 type with nanosecond resolution
        assert_type(result, np.datetime64)
        for v in result:
            _assert_datetime64_ns(v)

        # And Values should match exactly [1678-01-01T00:00:00, 2261-12-31T23:59:59.999999999]
        assert result == (TS_MIN_IN_NS_RANGE, TS_MAX_IN_NS_RANGE)

    def test_should_handle_null_values_for_timestamp_ntz_with_numpy(self, cursor_with_numpy):
        # Given Snowflake client is logged in with NumPy mode enabled
        pass

        # When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP_NTZ, NULL::TIMESTAMP_NTZ" is executed
        sql = "SELECT '2024-01-15 10:30:00'::TIMESTAMP_NTZ, NULL::TIMESTAMP_NTZ"
        cursor_with_numpy.execute(sql)
        result = cursor_with_numpy.fetchone()

        # Then Non-null values should be returned as numpy.datetime64 type with nanosecond resolution
        assert_type(result, np.datetime64, can_be_none=True)
        _assert_datetime64_ns(result[0])

        # And Result should contain [2024-01-15T10:30:00, NULL]
        assert result == (TS_2024_JAN, None)
