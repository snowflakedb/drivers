"""FLOAT type NumPy tests for Universal Driver (Python-specific).

This module tests FLOAT type conversion to numpy.float64 when NumPy mode is enabled.
These tests are Python-specific and not shared with other driver implementations.

All type synonyms (FLOAT, FLOAT4, FLOAT8, DOUBLE, DOUBLE PRECISION, REAL) are tested.
"""

from math import inf, nan

import pytest

from .utils import assert_floats_equal, assert_type


# NumPy is optional for these tests
np = pytest.importorskip("numpy")

FLOAT_TYPE_SYNONYMS = [
    "FLOAT",
    "FLOAT4",
    "FLOAT8",
    "DOUBLE",
    "DOUBLE PRECISION",
    "REAL",
]
float_type_parametrize = pytest.mark.parametrize("float_type", FLOAT_TYPE_SYNONYMS)


class TestFloatNumPy:
    """Test suite for FLOAT type NumPy conversion (Python-specific)."""

    @pytest.mark.skip("SNOW-2997786 - use_numpy is currently hardcoded to False in cursor")
    @float_type_parametrize
    def test_should_cast_float_values_to_numpy_float64_for_float_and_synonyms(self, cursor_with_numpy, float_type):
        # Given Snowflake client is logged in with NumPy mode enabled

        # When Query "SELECT 0.0::<type>, 123.456::<type>, -789.012::<type>, 1.23e10::<type>" is executed
        sql = f"SELECT 0.0::{float_type}, 123.456::{float_type}, -789.012::{float_type}, 1.23e10::{float_type}"
        cursor_with_numpy.execute(sql)
        result = cursor_with_numpy.fetchone()

        # Then All values should be returned as numpy.float64 type
        assert_type(result, np.float64)

        # And Values should match expected floats [0.0, 123.456, -789.012, 1.23e10]
        assert_floats_equal(result, (0.0, 123.456, -789.012, 1.23e10))

    @pytest.mark.skip("SNOW-2997786 - use_numpy is currently hardcoded to False in cursor")
    @float_type_parametrize
    def test_should_handle_special_float_values_with_numpy_for_float_and_synonyms(self, cursor_with_numpy, float_type):
        # Given Snowflake client is logged in with NumPy mode enabled

        # When Query "SELECT 'NaN'::<type>, 'inf'::<type>, '-inf'::<type>" is executed
        sql = f"SELECT 'NaN'::{float_type}, 'inf'::{float_type}, '-inf'::{float_type}"
        cursor_with_numpy.execute(sql)
        result = cursor_with_numpy.fetchone()

        # Then All values should be returned as numpy.float64 type
        assert_type(result, np.float64)

        # And Result should contain [NaN, positive_infinity, negative_infinity]
        assert_floats_equal(result, (nan, inf, -inf))
