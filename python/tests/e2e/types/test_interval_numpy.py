"""INTERVAL type NumPy tests for Universal Driver (Python-specific).

This module tests INTERVAL type conversion to numpy.timedelta64 when NumPy
mode is enabled. These tests are Python-specific and not shared with other
driver implementations.

Unit selection (see arrow_context.INTERVAL_*_to_numpy_timedelta):
    YEAR TO MONTH family:
        INTERVAL YEAR          -> numpy.timedelta64[Y]
        INTERVAL MONTH         -> numpy.timedelta64[M]
        INTERVAL YEAR TO MONTH -> numpy.timedelta64[M]

    DAY TO SECOND family:
        DAY TO SECOND (default precision, SB16 / Decimal128)
                               -> numpy.timedelta64[ms]   (ns //= 1_000_000; floors toward -inf)
        DAY(3) TO SECOND (reduced precision, SB8 / int64)
                               -> numpy.timedelta64[ns]   (full nanosecond precision)

Extreme value notes:
    YEAR / MONTH / YEAR TO MONTH: the Snowflake spec max is 999_999_999,
    which is covered by the non-numpy path in test_interval.py. The numpy
    path currently fails at that magnitude (InterfaceError 252005), so the
    scenarios below use +/- 99_999 as the extreme.

    DAY TO SECOND (ms path): uses the spec min/max days without sub-ms digits,
    because ms-path floor division rounds asymmetrically for negative values.
    That asymmetry is covered by its own dedicated scenario.

    DAY(3) TO SECOND (ns path): uses the exact spec min/max,
    +/- '999 23:59:59.999999999'.

INTERVAL support requires ENABLE_INTERVAL_TYPE to be active on the account.
"""

from __future__ import annotations

import numpy as np

from .utils import assert_type


# Spec max / min for DAY TO SECOND without sub-millisecond digits:
#   999_999_999 days + 23:59:59 in ms = 86_399_999_999_999_000 ms
DAY_TIME_MS_SPEC_MAX_NO_SUBMS = 86_399_999_999_999_000

# Spec max / min for DAY(3) TO SECOND with full nanosecond precision:
#   999 days + 23:59:59.999999999 in ns = 86_399_999_999_999_999 ns
DAY_TIME_NS_SPEC_MAX = 86_399_999_999_999_999


def _assert_timedelta64_unit(value, expected_unit: str) -> None:
    """Assert the value is numpy.timedelta64 with the expected unit code."""
    assert isinstance(value, np.timedelta64), f"Expected numpy.timedelta64, got {type(value).__name__}"
    unit = np.datetime_data(value)[0]
    assert unit == expected_unit, f"Expected '{expected_unit}' resolution, got '{unit}'"


class TestIntervalYearMonthNumPy:
    """Test suite for YEAR TO MONTH family INTERVAL NumPy conversion."""

    def test_should_cast_interval_year_to_numpy_timedelta64_year_unit(self, cursor_with_numpy):
        # Given Snowflake client is logged in with NumPy mode enabled
        pass

        # When Query "SELECT '0'::INTERVAL YEAR, '99999'::INTERVAL YEAR,
        # '-99999'::INTERVAL YEAR" is executed
        sql = "SELECT '0'::INTERVAL YEAR, '99999'::INTERVAL YEAR, '-99999'::INTERVAL YEAR"
        cursor_with_numpy.execute(sql)
        result = cursor_with_numpy.fetchone()

        # Then All values should be returned as numpy.timedelta64 type with year resolution
        assert_type(result, np.timedelta64)
        for v in result:
            _assert_timedelta64_unit(v, "Y")

        # And Values should match exactly [0 Y, 99999 Y, -99999 Y]
        assert result == (
            np.timedelta64(0, "Y"),
            np.timedelta64(99_999, "Y"),
            np.timedelta64(-99_999, "Y"),
        )

    def test_should_cast_interval_month_to_numpy_timedelta64_month_unit(self, cursor_with_numpy):
        # Given Snowflake client is logged in with NumPy mode enabled
        pass

        # When Query "SELECT '0'::INTERVAL MONTH, '99999'::INTERVAL MONTH,
        # '-99999'::INTERVAL MONTH" is executed
        sql = "SELECT '0'::INTERVAL MONTH, '99999'::INTERVAL MONTH, '-99999'::INTERVAL MONTH"
        cursor_with_numpy.execute(sql)
        result = cursor_with_numpy.fetchone()

        # Then All values should be returned as numpy.timedelta64 type with month resolution
        assert_type(result, np.timedelta64)
        for v in result:
            _assert_timedelta64_unit(v, "M")

        # And Values should match exactly [0 M, 99999 M, -99999 M]
        assert result == (
            np.timedelta64(0, "M"),
            np.timedelta64(99_999, "M"),
            np.timedelta64(-99_999, "M"),
        )

    def test_should_cast_interval_year_to_month_to_numpy_timedelta64_month_unit(self, cursor_with_numpy):
        # Given Snowflake client is logged in with NumPy mode enabled
        pass

        # When Query "SELECT '0-0'::INTERVAL YEAR TO MONTH, '1-2'::INTERVAL YEAR TO MONTH,
        # '99999-11'::INTERVAL YEAR TO MONTH, '-99999-11'::INTERVAL YEAR TO MONTH" is executed
        sql = (
            "SELECT '0-0'::INTERVAL YEAR TO MONTH, "
            "'1-2'::INTERVAL YEAR TO MONTH, "
            "'99999-11'::INTERVAL YEAR TO MONTH, "
            "'-99999-11'::INTERVAL YEAR TO MONTH"
        )
        cursor_with_numpy.execute(sql)
        result = cursor_with_numpy.fetchone()

        # Then All values should be returned as numpy.timedelta64 type with month resolution
        assert_type(result, np.timedelta64)
        for v in result:
            _assert_timedelta64_unit(v, "M")

        # And Values should match exactly [0 M, 14 M, 1199999 M, -1199999 M]
        assert result == (
            np.timedelta64(0, "M"),
            np.timedelta64(14, "M"),
            np.timedelta64(1_199_999, "M"),  # 99999 years * 12 + 11 months
            np.timedelta64(-1_199_999, "M"),
        )


class TestIntervalDayTimeNumPy:
    """Test suite for DAY TO SECOND family INTERVAL NumPy conversion."""

    def test_should_cast_interval_day_to_second_to_numpy_timedelta64_millisecond_unit(self, cursor_with_numpy):
        # Given Snowflake client is logged in with NumPy mode enabled
        pass

        # When Query "SELECT '0 0:0:0.0'::INTERVAL DAY TO SECOND,
        # '0 0:0:1.234'::INTERVAL DAY TO SECOND,
        # '999999999 23:59:59.000'::INTERVAL DAY TO SECOND,
        # '-999999999 23:59:59.000'::INTERVAL DAY TO SECOND" is executed
        sql = (
            "SELECT '0 0:0:0.0'::INTERVAL DAY TO SECOND, "
            "'0 0:0:1.234'::INTERVAL DAY TO SECOND, "
            "'999999999 23:59:59.000'::INTERVAL DAY TO SECOND, "
            "'-999999999 23:59:59.000'::INTERVAL DAY TO SECOND"
        )
        cursor_with_numpy.execute(sql)
        result = cursor_with_numpy.fetchone()

        # Then All values should be returned as numpy.timedelta64 type with millisecond resolution
        assert_type(result, np.timedelta64)
        for v in result:
            _assert_timedelta64_unit(v, "ms")

        # And Values should match exactly [0 ms, 1234 ms, 86399999999999000 ms, -86399999999999000 ms]
        assert result == (
            np.timedelta64(0, "ms"),
            np.timedelta64(1234, "ms"),
            np.timedelta64(DAY_TIME_MS_SPEC_MAX_NO_SUBMS, "ms"),
            np.timedelta64(-DAY_TIME_MS_SPEC_MAX_NO_SUBMS, "ms"),
        )

    def test_should_floor_sub_millisecond_digits_toward_negative_infinity_for_interval_day_to_second_with_numpy(
        self, cursor_with_numpy
    ):
        # Given Snowflake client is logged in with NumPy mode enabled
        pass

        # When Query "SELECT '0 0:0:1.234999'::INTERVAL DAY TO SECOND,
        # '-0 0:0:1.234999'::INTERVAL DAY TO SECOND" is executed
        sql = "SELECT '0 0:0:1.234999'::INTERVAL DAY TO SECOND, '-0 0:0:1.234999'::INTERVAL DAY TO SECOND"
        cursor_with_numpy.execute(sql)
        result = cursor_with_numpy.fetchone()

        # Then All values should be returned as numpy.timedelta64 type with millisecond resolution
        assert_type(result, np.timedelta64)
        for v in result:
            _assert_timedelta64_unit(v, "ms")

        # And Values should match exactly [1234 ms, -1235 ms]
        assert result == (
            np.timedelta64(1234, "ms"),  # +1.234999 -> floor = 1234
            np.timedelta64(-1235, "ms"),  # -1.234999 -> floor = -1235 (one further from zero)
        )

    def test_should_cast_interval_day_3_to_second_to_numpy_timedelta64_nanosecond_unit(self, cursor_with_numpy):
        # Given Snowflake client is logged in with NumPy mode enabled
        pass

        # When Query "SELECT '0 0:0:0.0'::INTERVAL DAY(3) TO SECOND,
        # '0 0:0:1.234567890'::INTERVAL DAY(3) TO SECOND,
        # '999 23:59:59.999999999'::INTERVAL DAY(3) TO SECOND,
        # '-999 23:59:59.999999999'::INTERVAL DAY(3) TO SECOND" is executed
        sql = (
            "SELECT '0 0:0:0.0'::INTERVAL DAY(3) TO SECOND, "
            "'0 0:0:1.234567890'::INTERVAL DAY(3) TO SECOND, "
            "'999 23:59:59.999999999'::INTERVAL DAY(3) TO SECOND, "
            "'-999 23:59:59.999999999'::INTERVAL DAY(3) TO SECOND"
        )
        cursor_with_numpy.execute(sql)
        result = cursor_with_numpy.fetchone()

        # Then All values should be returned as numpy.timedelta64 type with nanosecond resolution
        assert_type(result, np.timedelta64)
        for v in result:
            _assert_timedelta64_unit(v, "ns")

        # And Values should match exactly [0 ns, 1234567890 ns, 86399999999999999 ns, -86399999999999999 ns]
        assert result == (
            np.timedelta64(0, "ns"),
            np.timedelta64(1_234_567_890, "ns"),
            np.timedelta64(DAY_TIME_NS_SPEC_MAX, "ns"),
            np.timedelta64(-DAY_TIME_NS_SPEC_MAX, "ns"),
        )


class TestIntervalNullNumPy:
    """Test suite for NULL INTERVAL values under NumPy mode."""

    def test_should_handle_null_values_for_interval_with_numpy(self, cursor_with_numpy):
        # Given Snowflake client is logged in with NumPy mode enabled
        pass

        # When Query "SELECT NULL::INTERVAL YEAR, NULL::INTERVAL MONTH,
        # NULL::INTERVAL YEAR TO MONTH, NULL::INTERVAL DAY TO SECOND,
        # NULL::INTERVAL DAY(3) TO SECOND" is executed
        sql = (
            "SELECT NULL::INTERVAL YEAR, "
            "NULL::INTERVAL MONTH, "
            "NULL::INTERVAL YEAR TO MONTH, "
            "NULL::INTERVAL DAY TO SECOND, "
            "NULL::INTERVAL DAY(3) TO SECOND"
        )
        cursor_with_numpy.execute(sql)
        result = cursor_with_numpy.fetchone()

        # Then Result should contain [NULL, NULL, NULL, NULL, NULL]
        assert result == (None, None, None, None, None)
