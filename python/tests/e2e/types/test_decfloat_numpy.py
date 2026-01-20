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

from decimal import getcontext

import pytest


# NumPy is optional for these tests
np = pytest.importorskip("numpy")

# =============================================================================
# DECIMAL CONTEXT CONFIGURATION
# =============================================================================
DECFLOAT_PRECISION = 38


@pytest.fixture(autouse=True)
def setup_decimal_precision():
    """Set decimal context precision to 38 for all DECFLOAT tests."""
    old_prec = getcontext().prec
    getcontext().prec = DECFLOAT_PRECISION
    yield
    getcontext().prec = old_prec


class TestDecfloatNumPy:
    """Test suite for DECFLOAT type NumPy conversion (Python-specific)."""

    @pytest.mark.skip("SNOW-2997786 - use_numpy is currently hardcoded to False in cursor")
    def test_should_cast_decfloat_values_to_numpy_float64(self, cursor_with_numpy):
        # Given Snowflake client is logged in with NumPy mode enabled

        # When Query "SELECT 1.234::DECFLOAT, 123.456::DECFLOAT, -789.012::DECFLOAT" is executed
        sql = "SELECT 1.234::DECFLOAT, 123.456::DECFLOAT, -789.012::DECFLOAT"
        cursor_with_numpy.execute(sql)
        result = cursor_with_numpy.fetchone()

        # Then All values should be returned as numpy.float64 type
        expected = [1.234, 123.456, -789.012]
        assert len(result) == len(expected)
        for actual in result:
            assert isinstance(actual, np.float64), f"Expected numpy.float64, got {type(actual)}"

        # And Values should match approximately [1.234, 123.456, -789.012] within float64 precision
        for actual, expect in zip(result, expected):
            assert np.isclose(actual, expect, rtol=1e-14), f"Expected {expect}, got {actual}"

    @pytest.mark.skip("SNOW-2997786 - use_numpy is currently hardcoded to False in cursor")
    def test_numpy_handles_extreme_exponents_within_float64_range(self, cursor_with_numpy):
        # Given Snowflake client is logged in with NumPy mode enabled

        # When Query with exponents within float64 range is executed
        sql = "SELECT '1.23e100'::DECFLOAT, '9.87e-100'::DECFLOAT"
        cursor_with_numpy.execute(sql)
        result = cursor_with_numpy.fetchone()

        # Then Values should be numpy.float64
        assert isinstance(result[0], np.float64)
        assert isinstance(result[1], np.float64)

        # And Values should be approximately correct
        assert np.isclose(result[0], 1.23e100, rtol=1e-14)
        assert np.isclose(result[1], 9.87e-100, rtol=1e-14)

    @pytest.mark.skip("SNOW-2997786 - use_numpy is currently hardcoded to False in cursor")
    def test_numpy_overflows_extreme_exponents_beyond_float64_range(self, cursor_with_numpy):
        # Given Snowflake client is logged in with NumPy mode enabled

        # When Query with exponents exceeding float64 range is executed
        sql = "SELECT '1e16384'::DECFLOAT, '1e-16383'::DECFLOAT"
        cursor_with_numpy.execute(sql)
        result = cursor_with_numpy.fetchone()

        # Then e16384 exceeds float64 max (~e308) and becomes infinity
        assert np.isinf(result[0])

        # And e-16383 is below float64 min (~e-308) and becomes 0
        assert result[1] == 0.0 or np.isclose(result[1], 0.0)
