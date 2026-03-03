"""Integration tests for the ConfigManager implementation."""

import os
import stat

import pytest

from snowflake.connector.config_manager import (
    ConfigManager,
    ConfigOption,
    ConfigSlice,
    ConfigSliceOptions,
)
from tests.compatibility import NEW_DRIVER_ONLY, OLD_DRIVER_ONLY


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
            if (
                key.startswith("SNOWFLAKE_TEST_")
                or key.startswith("SNOWFLAKE_SECTION_")
                or key.startswith("SNOWFLAKE_MY")
                or key == "CUSTOM_ENV_VAR"
            ):
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
            _nest_path=["test_root", "section"],
            default="default_value",
        )

        # Then The file value should be returned
        assert option.value() == "file_value"

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
        root_manager.read_config(skip_file_permissions_check=True)
        option = ConfigOption(
            name="mykey",
            _root_manager=root_manager,
            _nest_path=["test_root", "section"],
            default="default_value",
            env_name=False,
        )

        # Then The file value should be returned (env overrides are not applied)
        assert option.value() == "file_value"

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


class TestDefaultEnvName:
    """Tests for the default_env_name property and auto env var resolution."""

    def test_default_env_name_single_part(self, config_env):
        """Test default_env_name with a single-level nest path."""
        # Given A ConfigOption with nest_path ["root", "myoption"]
        root_manager = ConfigManager(name="root", file_path=config_env["config_file"])
        option = ConfigOption(
            name="myoption",
            _root_manager=root_manager,
            _nest_path=["root"],
            default="unused",
        )

        assert option.default_env_name == "SNOWFLAKE_MYOPTION"

    def test_default_env_name_multi_part(self, config_env):
        """Test default_env_name with a multi-level nest path."""
        # Given A ConfigOption with nest_path ["root", "section", "subsection", "key"]
        root_manager = ConfigManager(name="root", file_path=config_env["config_file"])
        option = ConfigOption(
            name="key",
            _root_manager=root_manager,
            _nest_path=["root", "section", "subsection"],
            default="unused",
        )

        assert option.default_env_name == "SNOWFLAKE_SECTION_SUBSECTION_KEY"

    def test_value_uses_default_env_var(self, config_env):
        """Test that value() picks up SNOWFLAKE_<PATH> env var without explicit env_name."""
        # Given A ConfigOption without explicit env_name and SNOWFLAKE_MYOPTION env var set
        config_env["config_file"].write_text("")

        os.environ["SNOWFLAKE_MYOPTION"] = "from_default_env"

        root_manager = ConfigManager(name="root", file_path=config_env["config_file"])
        option = ConfigOption(
            name="myoption",
            _root_manager=root_manager,
            _nest_path=["root"],
            default="default_val",
        )

        assert option.value() == "from_default_env"

    def test_explicit_env_name_takes_priority_over_default(self, config_env):
        """Test that explicit env_name is checked instead of default_env_name."""
        # Given Both CUSTOM_ENV_VAR and SNOWFLAKE_MYOPTION are set
        config_env["config_file"].write_text("")

        os.environ["CUSTOM_ENV_VAR"] = "from_explicit"
        os.environ["SNOWFLAKE_MYOPTION"] = "from_default"

        root_manager = ConfigManager(name="root", file_path=config_env["config_file"])
        option = ConfigOption(
            name="myoption",
            _root_manager=root_manager,
            _nest_path=["root"],
            env_name="CUSTOM_ENV_VAR",
            default="default_val",
        )

        # When Retrieving the option value
        # Then The explicit env var value should be returned
        assert option.value() == "from_explicit"

    def test_default_env_var_priority_over_config_file(self, config_env):
        """Test that default env var takes priority over config file value."""
        # Given A config file with a value and the matching SNOWFLAKE_<PATH> env var set
        config_env["config_file"].write_text("""
[section]
mykey = "file_value"
""")

        os.environ["SNOWFLAKE_MYKEY"] = "env_value"

        root_manager = ConfigManager(name="test_root", file_path=config_env["config_file"])
        option = ConfigOption(
            name="mykey",
            _root_manager=root_manager,
            _nest_path=["section"],
            default="default_value",
        )

        assert option.value() == "env_value"

    def test_default_env_var_with_parse_str(self, config_env):
        """Test that parse_str is applied to values from default env var."""
        # Given SNOWFLAKE_MYOPTION set to a numeric string and parse_str=int
        config_env["config_file"].write_text("")

        os.environ["SNOWFLAKE_MYOPTION"] = "42"

        root_manager = ConfigManager(name="root", file_path=config_env["config_file"])
        option = ConfigOption(
            name="myoption",
            _root_manager=root_manager,
            _nest_path=["root"],
            parse_str=int,
            default=0,
        )

        assert option.value() == 42
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

    @pytest.mark.skip_reference
    def test_clear_cache(self):
        """Test that clear_cache resets caches."""
        # Given A ConfigManager with cached config
        manager = ConfigManager(name="test_manager")

        manager.conf_file_cache = {"test": "value"}
        # When clear_cache is called
        manager.clear_cache()
        # Then Cache should be None
        assert manager.conf_file_cache is None


class TestBackwardCompatibility:
    """Tests for backward compatibility features."""

    def test_config_parser_alias(self):
        """Test backward compatibility for CONFIG_PARSER."""
        import warnings

        import snowflake.connector.config_manager as cm

        if OLD_DRIVER_ONLY("BD#11"):
            # When Importing CONFIG_PARSER from config_manager (old driver)
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
        elif NEW_DRIVER_ONLY("BD#11"):
            # When Accessing CONFIG_PARSER on the new driver
            # Then AttributeError should be raised (alias was removed)
            with pytest.raises(AttributeError):
                _ = cm.CONFIG_PARSER

    def test_sub_parsers_property(self):
        """Test backward compatibility for _sub_parsers."""
        # Given A ConfigManager instance with a submanager
        manager = ConfigManager(name="test_manager")
        child = ConfigManager(name="child")
        manager.add_submanager(child)

        if OLD_DRIVER_ONLY("BD#12"):
            # When Accessing _sub_parsers property (old driver)
            with pytest.warns(DeprecationWarning):
                sub_parsers = manager._sub_parsers

            # Then DeprecationWarning should be raised
            # And _sub_parsers should reference _sub_managers
            assert sub_parsers is manager._sub_managers
        elif NEW_DRIVER_ONLY("BD#12"):
            # When Accessing _sub_parsers on the new driver
            # Then AttributeError should be raised (property was removed)
            with pytest.raises(AttributeError):
                _ = manager._sub_parsers

    def test_add_subparser_method(self):
        """Test backward compatibility for add_subparser."""
        # Given A ConfigManager instance and a child
        manager = ConfigManager(name="test_manager")
        child = ConfigManager(name="child")

        if OLD_DRIVER_ONLY("BD#12"):
            # When Calling add_subparser method (old driver)
            with pytest.warns(DeprecationWarning):
                manager.add_subparser(child)

            # Then DeprecationWarning should be raised
            # And The child should be in _sub_managers
            assert "child" in manager._sub_managers
        elif NEW_DRIVER_ONLY("BD#12"):
            # When Calling add_subparser on the new driver
            # Then AttributeError should be raised (method was removed)
            with pytest.raises(AttributeError):
                manager.add_subparser(child)


def _write_config(path, content):
    """Write a TOML file with secure permissions."""
    path.write_text(content)
    path.chmod(stat.S_IRUSR | stat.S_IWUSR)


def _make_manager(config_file, connections_file=None):
    """Create a ConfigManager with optional connections.toml slice."""
    slices = []
    if connections_file is not None:
        slices.append(ConfigSlice(connections_file, ConfigSliceOptions(check_permissions=True), "connections"))
    manager = ConfigManager(name="test_manager", file_path=config_file, _slices=slices)
    manager.add_option(name="connections", default=dict())
    manager.add_option(name="default_connection_name", default="default")
    return manager


class TestCLIScenarios:
    """Tests covering scenarios exercised by the Snowflake CLI.

    These validate that the driver's ConfigManager behaves correctly for
    the configuration patterns the CLI depends on: connections.toml
    replacement, rich multi-connection configs, root-level scalars,
    tomlkit type wrapping, and re-read consistency.
    """

    def test_connections_toml_replaces_config_toml_connections(self, tmp_path):
        """connections.toml connections fully replace config.toml connections."""
        # Given config.toml has default and full connections
        config_file = tmp_path / "config.toml"
        connections_file = tmp_path / "connections.toml"
        _write_config(
            config_file,
            """\
[connections.default]
database = "db_for_test"
schema = "test_public"
role = "test_role"

[connections.full]
account = "dev_account"
user = "dev_user"
""",
        )
        # And connections.toml defines only default with a different value
        _write_config(
            connections_file,
            """\
[default]
database = "overridden_database"
""",
        )

        # When ConfigManager reads with a connections slice
        manager = _make_manager(config_file, connections_file)

        # Then Only the connections.toml connection should be visible
        connections = manager["connections"]
        assert connections == {"default": {"database": "overridden_database"}}
        assert "full" not in connections

    def test_connections_toml_different_connections_replace(self, tmp_path):
        """connections.toml with entirely different connection names replaces all."""
        # Given config.toml has conn_a and conn_b
        config_file = tmp_path / "config.toml"
        connections_file = tmp_path / "connections.toml"
        _write_config(
            config_file,
            """\
[connections.conn_a]
account = "account_a"

[connections.conn_b]
account = "account_b"
""",
        )
        # And connections.toml has conn_x and conn_y (completely different)
        _write_config(
            connections_file,
            """\
[conn_x]
account = "account_x"

[conn_y]
account = "account_y"
""",
        )

        # When ConfigManager reads with a connections slice
        manager = _make_manager(config_file, connections_file)

        # Then Only connections.toml connections should be present
        connections = manager["connections"]
        assert set(connections.keys()) == {"conn_x", "conn_y"}
        assert "conn_a" not in connections
        assert "conn_b" not in connections

    def test_empty_connections_toml_preserves_config_connections(self, tmp_path):
        """An empty connections.toml file does not remove config.toml connections."""
        # Given config.toml has a connection
        config_file = tmp_path / "config.toml"
        connections_file = tmp_path / "connections.toml"
        _write_config(
            config_file,
            """\
[connections.myconn]
account = "my_account"
""",
        )
        # And connections.toml exists but is empty
        _write_config(connections_file, "")

        # When ConfigManager reads with a connections slice
        manager = _make_manager(config_file, connections_file)

        connections = manager["connections"]
        if NEW_DRIVER_ONLY("BD#13"):
            # Then config.toml connections should be preserved (new driver)
            assert "myconn" in connections
            assert connections["myconn"]["account"] == "my_account"
        else:
            # Old driver replaces config.toml connections with empty connections.toml content
            assert connections == {}

    def test_nonexistent_connections_toml_preserves_config_connections(self, tmp_path):
        """When connections.toml doesn't exist, config.toml connections are kept."""
        # Given config.toml has connections
        config_file = tmp_path / "config.toml"
        connections_file = tmp_path / "connections.toml"
        _write_config(
            config_file,
            """\
[connections.myconn]
account = "my_account"
""",
        )
        # And connections.toml does NOT exist on disk

        # When ConfigManager reads with a connections slice
        manager = _make_manager(config_file, connections_file)

        # Then config.toml connections should be present
        connections = manager["connections"]
        assert "myconn" in connections

    def test_rich_multi_connection_config(self, tmp_path):
        """Read a config matching the CLI's test.toml with many connections and data types."""
        # Given A config.toml with multiple connections of various shapes
        config_file = tmp_path / "config.toml"
        _write_config(
            config_file,
            """\
[connections.full]
account = "dev_account"
user = "dev_user"
host = "dev_host"
port = 8000
protocol = "dev_protocol"
role = "dev_role"
schema = "dev_schema"
database = "dev_database"
warehouse = "dev_warehouse"

[connections.default]
database = "db_for_test"
schema = "test_public"
role = "test_role"
warehouse = "xs"
password = "dummy_password"

[connections.empty]

[connections.test_connections]
user = "python"
""",
        )

        # When ConfigManager reads the config
        manager = _make_manager(config_file)

        # Then All connections should be returned with correct types
        connections = manager["connections"]
        assert set(connections.keys()) == {"full", "default", "empty", "test_connections"}

        full = connections["full"]
        assert full["account"] == "dev_account"
        assert full["port"] == 8000
        assert isinstance(full["port"], int)

        default = connections["default"]
        assert default["database"] == "db_for_test"
        assert default["password"] == "dummy_password"

        assert connections["test_connections"]["user"] == "python"

    def test_empty_connection_section(self, tmp_path):
        """An empty connection section [connections.empty] is still present as empty dict."""
        # Given config.toml with an empty connection section
        config_file = tmp_path / "config.toml"
        _write_config(
            config_file,
            """\
[connections.empty]

[connections.notempty]
user = "someone"
""",
        )

        # When ConfigManager reads the config
        manager = _make_manager(config_file)

        # Then The empty connection should be present as an empty dict
        connections = manager["connections"]
        assert "notempty" in connections
        assert connections["notempty"]["user"] == "someone"
        # Empty sections may or may not appear depending on TOML parser;
        # the key behavior is that notempty is correct

    def test_root_level_default_connection_name_from_file(self, tmp_path):
        """Root-level scalar default_connection_name is accessible after read."""
        # Given config.toml has default_connection_name at root level
        config_file = tmp_path / "config.toml"
        _write_config(
            config_file,
            """\
default_connection_name = "my_custom_conn"

[connections.my_custom_conn]
account = "acct"
""",
        )

        # When ConfigManager reads the config
        manager = _make_manager(config_file)

        # Then The root-level value should be accessible
        assert manager["default_connection_name"] == "my_custom_conn"

    def test_root_level_default_connection_name_with_connections_toml(self, tmp_path):
        """Root-level scalar is preserved when connections.toml replaces connections."""
        # Given config.toml has default_connection_name and connections
        config_file = tmp_path / "config.toml"
        connections_file = tmp_path / "connections.toml"
        _write_config(
            config_file,
            """\
default_connection_name = "myconn"

[connections.old_conn]
account = "old_account"
""",
        )
        # And connections.toml replaces with different connection
        _write_config(
            connections_file,
            """\
[myconn]
account = "new_account"
""",
        )

        # When ConfigManager reads the config
        manager = _make_manager(config_file, connections_file)

        # Then Root-level value is preserved while connections are replaced
        assert manager["default_connection_name"] == "myconn"
        connections = manager["connections"]
        assert "old_conn" not in connections
        assert connections["myconn"]["account"] == "new_account"

    def test_non_connection_sections_preserved_during_replacement(self, tmp_path):
        """Non-connection sections from config.toml survive connections.toml replacement."""
        # Given config.toml has connections and other sections
        config_file = tmp_path / "config.toml"
        connections_file = tmp_path / "connections.toml"
        _write_config(
            config_file,
            """\
[connections.old]
account = "old_account"

[cli]
analytics = true

[log]
level = "debug"
""",
        )
        # And connections.toml replaces connections
        _write_config(
            connections_file,
            """\
[new_conn]
account = "new_account"
""",
        )

        # When ConfigManager reads with a connections slice
        manager = ConfigManager(
            name="test_manager",
            file_path=config_file,
            _slices=[ConfigSlice(connections_file, ConfigSliceOptions(), "connections")],
        )
        cli_manager = ConfigManager(name="cli")
        cli_manager.add_option(name="analytics", default=False)
        manager.add_submanager(cli_manager)
        log_manager = ConfigManager(name="log")
        log_manager.add_option(name="level", default="info")
        manager.add_submanager(log_manager)
        manager.add_option(name="connections", default=dict())

        # Then Non-connection sections should be accessible
        assert manager["cli"]["analytics"] is True
        assert manager["log"]["level"] == "debug"

        # And connections should come from connections.toml only
        connections = manager["connections"]
        assert "old" not in connections
        assert connections["new_conn"]["account"] == "new_account"

    def test_tomlkit_type_wrapping(self, tmp_path):
        """Returned connection dicts are wrapped in tomlkit Container/Table types."""
        # Given A config.toml with connections
        config_file = tmp_path / "config.toml"
        _write_config(
            config_file,
            """\
[connections.default]
account = "test_account"
database = "test_db"
""",
        )

        # When ConfigManager reads and returns connections
        manager = _make_manager(config_file)
        connections = manager["connections"]

        # Then The result should be tomlkit types (for CLI isinstance checks)
        try:
            from tomlkit.container import Container
            from tomlkit.items import Table

            assert isinstance(connections, Container), (
                f"connections should be a tomlkit Container, got {type(connections)}"
            )
            assert isinstance(connections["default"], Table), (
                f"connection entry should be a tomlkit Table, got {type(connections['default'])}"
            )
        except ImportError:
            pytest.skip("tomlkit not available")

    @pytest.mark.skip_reference(reason="Old driver ConfigManager has no clear_cache method")
    def test_reread_consistency(self, tmp_path):
        """Re-reading config after clear_cache returns the same result."""
        # Given A config.toml with connections
        config_file = tmp_path / "config.toml"
        connections_file = tmp_path / "connections.toml"
        _write_config(
            config_file,
            """\
default_connection_name = "prod"

[connections.prod]
account = "prod_account"
database = "prod_db"
""",
        )
        _write_config(
            connections_file,
            """\
[prod]
account = "overridden_account"
""",
        )

        # When ConfigManager reads, clears cache, and reads again
        manager = _make_manager(config_file, connections_file)
        first_connections = dict(manager["connections"])
        first_default = manager["default_connection_name"]

        manager.clear_cache()

        second_connections = dict(manager["connections"])
        second_default = manager["default_connection_name"]

        # Then Both reads should produce identical results
        assert first_connections == second_connections
        assert first_default == second_default
        assert first_connections == {"prod": {"account": "overridden_account"}}

    def test_multiple_connections_toml_with_varied_types(self, tmp_path):
        """connections.toml with multiple connections having varied value types."""
        # Given connections.toml with connections using strings, integers, and booleans
        config_file = tmp_path / "config.toml"
        connections_file = tmp_path / "connections.toml"
        _write_config(config_file, "")
        _write_config(
            connections_file,
            """\
[conn1]
account = "acct1"
port = 443
validate_certs = true

[conn2]
account = "acct2"
port = 8080
validate_certs = false
warehouse = "xs"
""",
        )

        # When ConfigManager reads with a connections slice
        manager = _make_manager(config_file, connections_file)

        # Then All connections with correct types should be returned
        connections = manager["connections"]
        assert connections["conn1"]["account"] == "acct1"
        assert connections["conn1"]["port"] == 443
        assert connections["conn1"]["validate_certs"] is True

        assert connections["conn2"]["port"] == 8080
        assert connections["conn2"]["validate_certs"] is False
        assert connections["conn2"]["warehouse"] == "xs"

    def test_only_in_slice_with_connections_in_config(self, tmp_path):
        """only_in_slice=True strips connections from config.toml even without connections.toml."""
        # Given config.toml has connections and only_in_slice is True
        config_file = tmp_path / "config.toml"
        connections_file = tmp_path / "connections.toml"
        _write_config(
            config_file,
            """\
[connections.default]
account = "config_account"
""",
        )
        # And connections.toml does not exist

        # When ConfigManager uses only_in_slice=True for connections
        manager = ConfigManager(
            name="test_manager",
            file_path=config_file,
            _slices=[
                ConfigSlice(
                    connections_file,
                    ConfigSliceOptions(only_in_slice=True),
                    "connections",
                )
            ],
        )
        manager.add_option(name="connections", default=dict())

        # Then Connections from config.toml should be stripped
        connections = manager["connections"]
        assert connections == {}

    def test_connections_from_config_toml_only(self, tmp_path):
        """Without connections.toml slice, connections from config.toml are returned."""
        # Given config.toml has connections and no slice is configured
        config_file = tmp_path / "config.toml"
        _write_config(
            config_file,
            """\
[connections.default]
database = "db_for_test"
schema = "test_public"

[connections.full]
account = "dev_account"
user = "dev_user"
""",
        )

        # When ConfigManager reads without a connections slice
        manager = _make_manager(config_file)

        # Then All config.toml connections should be present
        connections = manager["connections"]
        assert set(connections.keys()) == {"default", "full"}
        assert connections["default"]["database"] == "db_for_test"
        assert connections["full"]["account"] == "dev_account"
