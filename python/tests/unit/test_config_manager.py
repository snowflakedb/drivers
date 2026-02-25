"""Unit tests for the ConfigManager implementation."""

import os

from unittest import mock

import pytest

from snowflake.connector.config_manager import CONFIG_MANAGER, ConfigManager, ConfigOption


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

    @mock.patch.dict(os.environ, {"SNOWFLAKE_DEFAULT_CONNECTION_NAME": "test"})
    def test_get_default_connection_name_from_env(self):
        value = CONFIG_MANAGER["default_connection_name"]
        assert value == "test"
