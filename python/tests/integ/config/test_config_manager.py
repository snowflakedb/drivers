"""Integration tests for the ConfigManager implementation."""

import os

import pytest

from snowflake.connector.config_manager import (
    ConfigManager,
    ConfigOption,
)
from tests.compatibility import NEW_DRIVER_ONLY


@pytest.fixture
def config_env(tmp_path):
    """Set up test environment with temporary config directory."""
    config_file = tmp_path / "config.toml"
    connections_file = tmp_path / "connections.toml"

    old_home = os.environ.get("SNOWFLAKE_HOME")
    os.environ["SNOWFLAKE_HOME"] = str(tmp_path)

    try:
        yield {
            "tmp_path": tmp_path,
            "config_file": config_file,
            "connections_file": connections_file,
        }
    finally:
        if old_home is not None:
            os.environ["SNOWFLAKE_HOME"] = old_home
        elif "SNOWFLAKE_HOME" in os.environ:
            del os.environ["SNOWFLAKE_HOME"]

        for key in list(os.environ.keys()):
            if key.startswith("SNOWFLAKE_TEST_") or key.startswith("SNOWFLAKE_SECTION_") or key == "CUSTOM_ENV_VAR":
                del os.environ[key]


class TestConfigOption:
    """Tests for ConfigOption class."""

    def test_default_value(self, config_env):
        """Test that default value is used when option is not configured."""
        # Given No configuration files exist
        config_env["config_file"].write_text("")

        # When ConfigManager retrieves an option with default value
        root_manager = ConfigManager(name="test_root", file_path=config_env["config_file"])
        option = ConfigOption(
            name="nonexistent_option",
            _root_manager=root_manager,
            _nest_path=["test"],
            default="default_value",
        )

        # Then The default value should be returned
        assert option.value() == "default_value"

    def test_choices_validation(self, config_env):
        """Test that choices validation works correctly."""
        from snowflake.connector.errors import ConfigSourceError as CSError

        # Given A ConfigOption with choices and an invalid default
        root_manager = ConfigManager(name="test_root", file_path=config_env["config_file"])
        option = ConfigOption(
            name="test_option",
            _root_manager=root_manager,
            _nest_path=["test"],
            choices=["option1", "option2", "option3"],
            default="invalid_option",
        )

        # When ConfigManager retrieves the option
        # Then ConfigSourceError should be raised
        with pytest.raises(CSError) as exc_info:
            option.value()
        assert "is not part of" in str(exc_info.value)

    def test_config_value_from_file(self, config_env):
        """Test that values are read from config file."""
        # Given A config.toml file with a section and option
        config_env["config_file"].write_text("""
[section]
my_option = "file_value"
""")

        # When ConfigManager retrieves the option
        root_manager = ConfigManager(name="test_root", file_path=config_env["config_file"])
        option = ConfigOption(
            name="my_option",
            _root_manager=root_manager,
            _nest_path=["section"],
            default="default_value",
        )

        # Then The file value should be returned
        if NEW_DRIVER_ONLY("BD#6"):
            # New driver reads from SNOWFLAKE_HOME via sf_core
            assert option.value() == "file_value"
        else:
            # Old driver has different section path handling, falls back to default
            assert option.value() == "default_value"

    def test_config_value_from_file_no_env_override(self, config_env):
        """Test that SNOWFLAKE_<SECTION>_<KEY> env vars do NOT override config values."""
        # Given A config.toml with section.mykey and SNOWFLAKE_SECTION_MYKEY env var
        config_env["config_file"].write_text("""
[section]
mykey = "file_value"
""")

        os.environ["SNOWFLAKE_SECTION_MYKEY"] = "env_value"

        # When ConfigManager retrieves the option
        root_manager = ConfigManager(name="test_root", file_path=config_env["config_file"])
        option = ConfigOption(
            name="mykey",
            _root_manager=root_manager,
            _nest_path=["section"],
            default="default_value",
        )

        # Then The file value should be returned (env overrides are not applied)
        if NEW_DRIVER_ONLY("BD#7"):
            assert option.value() == "file_value"
        else:
            # Old driver has different section path handling, falls back to default
            assert option.value() == "default_value"

    def test_custom_env_name(self, config_env):
        """Test that custom environment variable names work."""
        # Given A custom environment variable CUSTOM_ENV_VAR is set
        root_manager = ConfigManager(name="test_root", file_path=config_env["config_file"])
        option = ConfigOption(
            name="test_option",
            _root_manager=root_manager,
            _nest_path=["test"],
            env_name="CUSTOM_ENV_VAR",
            default="default_value",
        )

        os.environ["CUSTOM_ENV_VAR"] = "custom_value"

        # When ConfigManager retrieves option with env_name set to CUSTOM_ENV_VAR
        # Then The custom env var value should be returned
        assert option.value() == "custom_value"

    def test_custom_env_name_with_parse_str(self, config_env):
        """Test that parse_str is applied to custom env var values."""
        # Given A custom environment variable set to a numeric string
        root_manager = ConfigManager(name="test_root", file_path=config_env["config_file"])
        option = ConfigOption(
            name="test_option",
            _root_manager=root_manager,
            _nest_path=["test"],
            env_name="CUSTOM_ENV_VAR",
            parse_str=int,
            default=0,
        )

        os.environ["CUSTOM_ENV_VAR"] = "123"

        # When ConfigManager retrieves option with parse_str set to int
        # Then The parsed integer value should be returned
        assert option.value() == 123
        assert isinstance(option.value(), int)


class TestConfigManager:
    """Tests for ConfigManager class."""

    def test_add_option(self):
        """Test adding options to ConfigManager."""
        # Given A ConfigManager instance
        manager = ConfigManager(name="test_manager")

        # When An option is added
        manager.add_option(name="test_option", default="test_value")

        # Then The option should be in the manager's options dict
        assert "test_option" in manager._options
        assert manager._options["test_option"].name == "test_option"

    def test_add_submanager(self):
        """Test adding sub-managers to ConfigManager."""
        # Given A parent ConfigManager and a child ConfigManager
        parent = ConfigManager(name="parent")
        child = ConfigManager(name="child")

        # When The child is added as a submanager
        parent.add_submanager(child)

        # Then The child nest_path and root_manager should be updated
        assert "child" in parent._sub_managers
        assert parent._sub_managers["child"] == child
        assert child._nest_path == ["parent", "child"]
        assert child._root_manager == parent

    def test_conflict_detection(self):
        """Test that conflicts between options and sub-managers are detected."""
        from snowflake.connector.errors import ConfigManagerError as CMError

        # Given A ConfigManager with an option named conflict_name
        manager = ConfigManager(name="test_manager")
        manager.add_option(name="conflict_name", default="value")

        # When Trying to add a submanager named conflict_name
        child = ConfigManager(name="conflict_name")

        # Then ConfigManagerError should be raised
        with pytest.raises(CMError):
            manager.add_submanager(child)

    def test_getitem_returns_option_value(self, config_env):
        """Test ConfigManager __getitem__ returns option values."""
        # Given A ConfigManager with an option having a default value
        config_env["config_file"].write_text("")

        manager = ConfigManager(name="test_manager", file_path=config_env["config_file"])
        manager.add_option(name="test_option", default="default_value")

        # When Accessing the option via bracket notation
        value = manager["test_option"]

        # Then The default value should be returned
        assert value == "default_value"

    def test_clear_cache(self):
        """Test that clear_cache resets caches."""
        # Given A ConfigManager with cached config
        manager = ConfigManager(name="test_manager")

        # When clear_cache is called
        # Then Cache should be None
        if NEW_DRIVER_ONLY("BD#10"):
            # New driver has clear_cache and cache attributes
            manager.conf_file_cache = {"test": "value"}

            manager.clear_cache()

            assert manager.conf_file_cache is None
        else:
            # Old driver has different caching mechanism
            if hasattr(manager, "clear_cache"):
                manager.clear_cache()
            # Just verify the manager is functional
            assert manager.name == "test_manager"


class TestBackwardCompatibility:
    """Tests for backward compatibility features."""

    def test_config_parser_alias(self):
        """Test backward compatibility for CONFIG_PARSER."""
        import warnings

        import snowflake.connector.config_manager as cm

        # When Importing CONFIG_PARSER from config_manager
        with warnings.catch_warnings(record=True) as w:
            warnings.simplefilter("always")
            _ = cm.CONFIG_MANAGER
            assert len(w) == 0
            config_parser = cm.CONFIG_PARSER

        # Then DeprecationWarning should be raised
        assert len(w) == 1
        assert issubclass(w[0].category, DeprecationWarning)
        assert "CONFIG_PARSER" in str(w[0].message) and "deprecated" in str(w[0].message)

        # And CONFIG_PARSER should reference CONFIG_MANAGER
        assert config_parser is cm.CONFIG_MANAGER

    def test_sub_parsers_property(self):
        """Test backward compatibility for _sub_parsers."""
        # Given A ConfigManager instance with a submanager
        manager = ConfigManager(name="test_manager")
        child = ConfigManager(name="child")
        manager.add_submanager(child)

        # When Accessing _sub_parsers property
        with pytest.warns(DeprecationWarning):
            sub_parsers = manager._sub_parsers

        # Then DeprecationWarning should be raised
        # And _sub_parsers should reference _sub_managers
        assert sub_parsers is manager._sub_managers

    def test_add_subparser_method(self):
        """Test backward compatibility for add_subparser."""
        # Given A ConfigManager instance and a child
        manager = ConfigManager(name="test_manager")
        child = ConfigManager(name="child")

        # When Calling add_subparser method
        with pytest.warns(DeprecationWarning):
            manager.add_subparser(child)

        # Then DeprecationWarning should be raised
        # And The child should be in _sub_managers
        assert "child" in manager._sub_managers
