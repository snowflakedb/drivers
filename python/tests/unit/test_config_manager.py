"""Unit tests for the ConfigManager implementation."""

import base64

import pytest

from snowflake.connector.config_manager import ConfigManager, ConfigOption
from tests.compatibility import IS_UNIVERSAL_DRIVER


if IS_UNIVERSAL_DRIVER:
    from snowflake.connector.config_manager import _parse_setting_from_json
else:
    _parse_setting_from_json = None  # type: ignore[assignment,misc]


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


class TestParseSettingFromJson:
    """Tests for _parse_setting_from_json function (new driver only)."""

    @pytest.mark.skip_reference
    def test_string_setting(self):
        """Test parsing string setting."""
        setting = {"type": "string", "value": "test_value"}
        assert _parse_setting_from_json(setting) == "test_value"

    @pytest.mark.skip_reference
    def test_int_setting(self):
        """Test parsing int setting."""
        setting = {"type": "int", "value": 42}
        assert _parse_setting_from_json(setting) == 42

    @pytest.mark.skip_reference
    def test_double_setting(self):
        """Test parsing double setting."""
        setting = {"type": "double", "value": 3.14}
        assert _parse_setting_from_json(setting) == 3.14

    @pytest.mark.skip_reference
    def test_bytes_setting(self):
        """Test parsing bytes setting (base64 encoded)."""
        bytes_value = b"test bytes"
        setting = {
            "type": "bytes",
            "value": base64.b64encode(bytes_value).decode("utf-8"),
        }
        assert _parse_setting_from_json(setting) == bytes_value
