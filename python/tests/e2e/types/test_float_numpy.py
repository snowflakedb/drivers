"""FLOAT type NumPy tests for Universal Driver (Python-specific).

This module tests FLOAT type conversion to numpy.float64 when NumPy mode is enabled.
These tests are Python-specific and not shared with other driver implementations.

All type synonyms (FLOAT, FLOAT4, FLOAT8, DOUBLE, DOUBLE PRECISION, REAL) are tested.
"""

import pytest


# NumPy is optional for these tests
np = pytest.importorskip("numpy")


class TestFloatNumPy:
    """Test suite for FLOAT type NumPy conversion (Python-specific)."""

    @pytest.mark.skip("SNOW-2997742 - use_numpy is currently hardcoded to False in cursor")
    @pytest.mark.parametrize(
        "float_type",
        ["FLOAT", "FLOAT4", "FLOAT8", "DOUBLE", "DOUBLE PRECISION", "REAL"],
    )
    def test_should_cast_float_values_to_numpy_float64_for_float_and_synonyms(self, cursor_with_numpy, float_type):
        # Given Snowflake client is logged in with NumPy mode enabled
        assert not cursor_with_numpy.connection.is_closed(), "Connection should be open"

        # When Query "SELECT 0.0::<type>, 123.456::<type>, -789.012::<type>, 1.23e10::<type>" is executed
        sql = f"SELECT 0.0::{float_type}, 123.456::{float_type}, -789.012::{float_type}, 1.23e10::{float_type}"
        cursor_with_numpy.execute(sql)
        result = cursor_with_numpy.fetchone()

        # Then All values should be returned as numpy.float64 type
        expected = [0.0, 123.456, -789.012, 1.23e10]
        assert len(result) == len(expected)

        # And Values should match expected floats [0.0, 123.456, -789.012, 1.23e10]
        for actual, expect in zip(result, expected):
            assert isinstance(actual, np.float64), f"Expected numpy.float64, got {type(actual)}"
            assert np.isclose(actual, expect, rtol=1e-14), f"Expected {expect}, got {actual}"

    @pytest.mark.skip("SNOW-2997742 - use_numpy is currently hardcoded to False in cursor")
    @pytest.mark.parametrize(
        "float_type",
        ["FLOAT", "FLOAT4", "FLOAT8", "DOUBLE", "DOUBLE PRECISION", "REAL"],
    )
    def test_should_handle_special_float_values_with_numpy_for_float_and_synonyms(self, cursor_with_numpy, float_type):
        # Given Snowflake client is logged in with NumPy mode enabled
        assert not cursor_with_numpy.connection.is_closed(), "Connection should be open"

        # When Query "SELECT 'NaN'::<type>, 'inf'::<type>, '-inf'::<type>" is executed
        sql = f"SELECT 'NaN'::{float_type}, 'inf'::{float_type}, '-inf'::{float_type}"
        cursor_with_numpy.execute(sql)
        result = cursor_with_numpy.fetchone()

        # Then All values should be returned as numpy.float64 type
        assert isinstance(result[0], np.float64), f"NaN should be numpy.float64, got {type(result[0])}"
        assert isinstance(result[1], np.float64), f"inf should be numpy.float64, got {type(result[1])}"
        assert isinstance(result[2], np.float64), f"-inf should be numpy.float64, got {type(result[2])}"

        # And First value should be NaN
        assert np.isnan(result[0]), "First value should be NaN"

        # And Second value should be positive infinity
        assert np.isposinf(result[1]), "Second value should be positive infinity"

        # And Third value should be negative infinity
        assert np.isneginf(result[2]), "Third value should be negative infinity"
