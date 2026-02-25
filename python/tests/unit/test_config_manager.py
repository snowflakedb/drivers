"""Unit tests for the ConfigManager implementation."""

import pytest

from snowflake.connector.config_manager import ConfigManager, ConfigOption
from tests.compatibility import IS_UNIVERSAL_DRIVER


if IS_UNIVERSAL_DRIVER:
    from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
        ConfigSetting,
    )
    from snowflake.connector.config_manager import _parse_config_setting
else:
    _parse_config_setting = None  # type: ignore[assignment,misc]
    ConfigSetting = None  # type: ignore[assignment,misc]


class TestConfigOptionConstructor:
    """Tests for ConfigOption constructor validation."""

    def test_missing_root_manager(self):
        """Test that ConfigOption requires _root_manager."""
        with pytest.raises(TypeError, match="_root_manager cannot be None"):
            ConfigOption(
                name="test_option",
                _nest_path=["test"],
                _root_manager=None,
            )

    def test_missing_nest_path(self):
        """Test that ConfigOption requires _nest_path."""
        with pytest.raises(TypeError, match="_nest_path cannot be None"):
            ConfigOption(
                name="test_option",
                _nest_path=None,
                _root_manager=ConfigManager(name="test_manager"),
            )


class TestParseConfigSetting:
    """Tests for _parse_config_setting function (new driver only)."""

    @pytest.mark.skip_reference
    def test_string_setting(self):
        """Test parsing string setting."""
        setting = ConfigSetting(string_value="test_value")
        assert _parse_config_setting(setting) == "test_value"

    @pytest.mark.skip_reference
    def test_int_setting(self):
        """Test parsing int setting."""
        setting = ConfigSetting(int_value=42)
        assert _parse_config_setting(setting) == 42

    @pytest.mark.skip_reference
    def test_double_setting(self):
        """Test parsing double setting."""
        setting = ConfigSetting(double_value=3.14)
        assert _parse_config_setting(setting) == 3.14

    @pytest.mark.skip_reference
    def test_bytes_setting(self):
        """Test parsing bytes setting."""
        bytes_value = b"test bytes"
        setting = ConfigSetting(bytes_value=bytes_value)
        assert _parse_config_setting(setting) == bytes_value
