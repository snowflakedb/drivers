"""NUMBER type NumPy tests for Universal Driver (Python-specific).

This module tests NUMBER type conversion to numpy types when NumPy mode is enabled.
These tests are Python-specific and not shared with other driver implementations.

CRITICAL TYPE MAPPING RULE (NumPy mode):
- scale = 0: Returns numpy.int64
- scale > 0: Returns numpy.float64

IMPORTANT: NumPy mode causes precision loss for NUMBER values!
- NUMBER supports 38-digit precision
- numpy.int64 supports only 19 digits (max: 9223372036854775807)
- numpy.float64 has only ~15 significant digits

Use standard mode (Python int/Decimal) when precision is critical.
"""

from __future__ import annotations

import pytest

from .utils import assert_floats_equal, assert_type


# NumPy is optional for these tests
np = pytest.importorskip("numpy")

# =============================================================================
# DECIMAL CONTEXT CONFIGURATION
# =============================================================================
NUMBER_PRECISION = 38


class TestNumberNumPy:
    """Test suite for NUMBER type NumPy conversion (Python-specific)."""

    @pytest.mark.skip("SNOW-2997786 - use_numpy is currently hardcoded to False in cursor")
    def test_should_cast_number_scale0_to_numpy_int64(self, cursor_with_numpy):
        # Given Snowflake client is logged in with NumPy mode enabled
        pass

        # When Query "SELECT 0::NUMBER(10,0), 123::NUMBER(10,0), -456::NUMBER(10,0),
        # 999999::NUMBER(10,0)" is executed
        sql = "SELECT 0::NUMBER(10,0), 123::NUMBER(10,0), -456::NUMBER(10,0), 999999::NUMBER(10,0)"
        cursor_with_numpy.execute(sql)
        result = cursor_with_numpy.fetchone()

        # Then All values should be returned as numpy.int64 type
        assert_type(result, np.int64)

        # And Values should match exactly [0, 123, -456, 999999]
        assert result == (0, 123, -456, 999999)

    @pytest.mark.skip("SNOW-2997786 - use_numpy is currently hardcoded to False in cursor")
    def test_should_cast_number_scale3_to_numpy_float64(self, cursor_with_numpy):
        # Given Snowflake client is logged in with NumPy mode enabled
        pass

        # When Query "SELECT 0.000::NUMBER(15,3), 123.456::NUMBER(15,3),
        # -789.012::NUMBER(15,3)" is executed
        sql = "SELECT 0.000::NUMBER(15,3), 123.456::NUMBER(15,3), -789.012::NUMBER(15,3)"
        cursor_with_numpy.execute(sql)
        result = cursor_with_numpy.fetchone()

        # Then All values should be returned as numpy.float64 type
        assert_type(result, np.float64)

        # And Values should match approximately [0.0, 123.456, -789.012] within float64 precision
        assert_floats_equal(result, (0.0, 123.456, -789.012))

    @pytest.mark.skip("SNOW-2997786 - use_numpy is currently hardcoded to False in cursor")
    def test_numpy_handles_high_precision_integers_within_int64_range(self, cursor_with_numpy):
        # Given Snowflake client is logged in with NumPy mode enabled
        pass

        # When Query with 18-digit integer (within int64 range) is executed
        sql = "SELECT 123456789012345678::NUMBER(18,0)"
        cursor_with_numpy.execute(sql)
        result = cursor_with_numpy.fetchone()

        # Then Value should be numpy.int64
        assert isinstance(result[0], np.int64)

        # And Value should match exactly
        assert result[0] == 123456789012345678
