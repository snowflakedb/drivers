"""DECFLOAT type NumPy tests for Universal Driver (Python-specific).

This module tests DECFLOAT type conversion to numpy.float64 when NumPy mode is enabled.
These tests are Python-specific and not shared with other driver implementations.

IMPORTANT: NumPy mode causes precision loss for DECFLOAT values!
- DECFLOAT has 38-digit precision
- numpy.float64 has only ~15-digit precision
- Extreme exponents may overflow to infinity or underflow to zero

Use standard mode (Python Decimal) when precision is critical.
"""

from __future__ import annotations

import numpy as np

from .utils import assert_floats_equal, assert_type


class TestDecfloatNumPy:
    """Test suite for DECFLOAT type NumPy conversion (Python-specific)."""

    def test_should_cast_decfloat_values_to_numpy_float64(self, cursor_with_numpy):
        # Given Snowflake client is logged in with NumPy mode enabled
        pass

        # When Query "SELECT 1.234::DECFLOAT, 123.456::DECFLOAT, -789.012::DECFLOAT" is executed
        sql = "SELECT 1.234::DECFLOAT, 123.456::DECFLOAT, -789.012::DECFLOAT"
        cursor_with_numpy.execute(sql)
        result = cursor_with_numpy.fetchone()

        # Then All values should be returned as numpy.float64 type
        assert_type(result, np.float64)

        # And Values should match approximately [1.234, 123.456, -789.012] within float64 precision
        assert_floats_equal(result, (1.234, 123.456, -789.012))

    def test_numpy_handles_extreme_exponents_within_float64_range(self, cursor_with_numpy):
        # Given Snowflake client is logged in with NumPy mode enabled
        pass

        # When Query with exponents within float64 range is executed
        sql = "SELECT '1.23e100'::DECFLOAT, '9.87e-100'::DECFLOAT"
        cursor_with_numpy.execute(sql)
        result = cursor_with_numpy.fetchone()

        # Then Values should be numpy.float64
        assert_type(result, np.float64)

        # And Values should be approximately correct
        assert_floats_equal(result, (1.23e100, 9.87e-100))

    def test_numpy_overflows_extreme_exponents_beyond_float64_range(self, cursor_with_numpy):
        # Given Snowflake client is logged in with NumPy mode enabled
        pass

        # When Query with exponents exceeding float64 range is executed
        sql = "SELECT '1e16384'::DECFLOAT, '1e-16383'::DECFLOAT"
        cursor_with_numpy.execute(sql)
        result = cursor_with_numpy.fetchone()

        # Then e16384 exceeds float64 max (~e308) and becomes infinity
        assert_floats_equal(result, (float("inf"), 0.0))

        # And e-16383 is below float64 min (~e-308) and becomes 0
        assert_type(result, np.float64)
