"""BOOLEAN type NumPy tests for Universal Driver (Python-specific)."""

from __future__ import annotations

import pytest

from .utils import assert_type


# NumPy is optional for these tests
np = pytest.importorskip("numpy")


class TestBooleanNumPy:
    """Test suite for BOOLEAN type NumPy conversion (Python-specific)."""

    @pytest.mark.skip("SNOW-2997786 - use_numpy is currently hardcoded to False in cursor")
    def test_should_cast_boolean_to_numpy_bool_type(self, cursor_with_numpy):
        # Given Snowflake client is logged in with NumPy mode enabled
        pass

        # When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN, TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
        sql = "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN, TRUE::BOOLEAN, FALSE::BOOLEAN"
        cursor_with_numpy.execute(sql)
        result = cursor_with_numpy.fetchone()

        # Then All values should be returned as numpy.bool_ type
        assert_type(result, np.bool_)

        # And Values should be [TRUE, FALSE, TRUE, FALSE]
        assert result == (True, False, True, False)
