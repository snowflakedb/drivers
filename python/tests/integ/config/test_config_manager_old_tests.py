from __future__ import annotations

import re
import stat
import tempfile

from pathlib import Path
from textwrap import dedent
from typing import Callable, Literal, Union
from uuid import uuid4

import pytest
import tomlkit

from pytest import raises

from snowflake.connector.config_manager import (
    CONFIG_MANAGER,
    ConfigManager,
    ConfigOption,
    ConfigSlice,
    ConfigSliceOptions,
)
from snowflake.connector.errors import (
    ConfigManagerError,
    ConfigSourceError,
    MissingConfigOptionError,
)
from tests.compatibility import NEW_DRIVER_ONLY


@pytest.fixture
def snowflake_home(monkeypatch):
    """
    Set up the default location of config files to [temporary_directory]/.snowflake
    """
    with tempfile.TemporaryDirectory() as tmp_dir:
        snowflake_home = Path(tmp_dir) / ".snowflake"
        snowflake_home.mkdir()
        monkeypatch.setenv("SNOWFLAKE_HOME", str(snowflake_home))
        yield snowflake_home


def tmp_files_helper(cwd: Path, to_create: files) -> None:
    for k, v in to_create.items():
        new_file = cwd / k
        if isinstance(v, str):
            new_file.touch()
            new_file.write_text(v)
        else:
            new_file.mkdir()
            tmp_files_helper(new_file, v)


files = dict[str, Union[str, Literal["files"]]]


@pytest.fixture
def tmp_files(tmp_path: Path) -> Callable[[files], Path]:
    def create_tmp_files(to_create: files) -> Path:
        tmp_files_helper(tmp_path, to_create)
        # Automatically fix file permissions
        if "config.toml" in to_create:
            (tmp_path / "config.toml").chmod(stat.S_IRUSR | stat.S_IWUSR)
        if "connections.toml" in to_create:
            (tmp_path / "connections.toml").chmod(stat.S_IRUSR | stat.S_IWUSR)
        return tmp_path

    return create_tmp_files


class TestsBackwardCompatibilityForConfigManager:
    def test_incorrect_config_read(self, tmp_files):
        tmp_folder = tmp_files(
            {
                "config.toml": dedent(
                    """
                    [connections.defa
                    """
                )
            }
        )
        config_file = tmp_folder / "config.toml"
        with raises(
            ConfigSourceError,
            match=re.escape(f"An unknown error happened while loading '{str(config_file)}'"),
        ):
            ConfigManager(name="test", file_path=config_file).read_config()

    def test_simple_config_read(self, tmp_files):
        tmp_folder = tmp_files(
            {
                "config.toml": dedent(
                    """\
                    [connections.snowflake]
                    account = "snowflake"
                    user = "snowball"
                    password = "password"

                    [settings]
                    output_format = "yaml"
                    """
                )
            }
        )
        config_file = tmp_folder / "config.toml"
        TEST_PARSER = ConfigManager(
            name="test",
            file_path=config_file,
        )
        TEST_PARSER.add_option(
            name="connections",
            parse_str=tomlkit.loads,
        )
        settings_parser = ConfigManager(
            name="settings",
        )
        settings_parser.add_option(
            name="output_format",
            choices=("json", "yaml", "toml"),
        )
        TEST_PARSER.add_submanager(settings_parser)
        assert TEST_PARSER["connections"] == {
            "snowflake": {
                "account": "snowflake",
                "user": "snowball",
                "password": "password",
            }
        }
        assert TEST_PARSER["settings"]["output_format"] == "yaml"

    def test_simple_config_read_sliced(self, tmp_files):
        """Same test_simple_config_read, but rerads part of the config from another file."""
        tmp_folder = tmp_files(
            {
                "config.toml": dedent(
                    """\
                    [settings]
                    output_format = "json"
                    """
                ),
                "connections.toml": dedent(
                    """\
                    [snowflake]
                    account = "snowflake"
                    user = "snowball"
                    password = "password"
                    """
                ),
            }
        )
        TEST_PARSER = ConfigManager(
            name="root_parser",
            file_path=tmp_folder / "config.toml",
            _slices=(ConfigSlice(tmp_folder / "connections.toml", ConfigSliceOptions(), "connections"),),
        )
        TEST_PARSER.add_option(
            name="connections",
            parse_str=tomlkit.loads,
        )
        settings_parser = ConfigManager(
            name="settings",
        )
        settings_parser.add_option(
            name="output_format",
            choices=("json", "yaml", "toml"),
        )
        TEST_PARSER.add_submanager(settings_parser)
        assert TEST_PARSER["connections"] == {
            "snowflake": {
                "account": "snowflake",
                "user": "snowball",
                "password": "password",
            }
        }
        assert TEST_PARSER["settings"]["output_format"] == "json"

    def test_missing_value(self, tmp_files):
        """Test that we handle a missing configuration option gracefully."""
        tmp_folder = tmp_files(
            {
                "config.toml": dedent(
                    """\
                    [connections.snowflake]
                    account = "snowflake"
                    user = "snowball"
                    password = "password"
                    """
                ),
            }
        )
        TEST_PARSER = ConfigManager(
            name="root_parser",
            file_path=tmp_folder / "config.toml",
        )
        TEST_PARSER.add_option(
            name="connections",
        )
        settings_parser = ConfigManager(
            name="settings",
        )
        settings_parser.add_option(
            name="output_format",
            choices=("json", "yaml", "toml"),
        )
        TEST_PARSER.add_submanager(settings_parser)
        assert TEST_PARSER["connections"] == {
            "snowflake": {
                "account": "snowflake",
                "user": "snowball",
                "password": "password",
            }
        }
        with pytest.raises(
            MissingConfigOptionError,
            match=re.escape(
                "Configuration option 'settings.output_format' is not defined anywhere, "
                "have you forgotten to set it in a configuration file, or "
                "environmental variable?"
            ),
        ):
            TEST_PARSER["settings"]["output_format"]

    def test_missing_value_sliced(self, tmp_files):
        """Test that we handle a missing configuration option gracefully across multiple files."""
        tmp_folder = tmp_files(
            {
                "config.toml": dedent(
                    """\
                    [settings]
                    """
                ),
                "connections.toml": dedent(
                    """\
                    [snowflake]
                    account = "snowflake"
                    user = "snowball"
                    password = "password"
                    """
                ),
            }
        )
        TEST_PARSER = ConfigManager(
            name="root_parser",
            file_path=tmp_folder / "config.toml",
            _slices=(ConfigSlice(tmp_folder / "connections.toml", ConfigSliceOptions(), "connections"),),
        )
        TEST_PARSER.add_option(
            name="connections",
        )
        settings_parser = ConfigManager(
            name="settings",
        )
        settings_parser.add_option(
            name="output_format",
            choices=("json", "yaml", "toml"),
        )
        TEST_PARSER.add_submanager(settings_parser)
        assert TEST_PARSER["connections"] == {
            "snowflake": {
                "account": "snowflake",
                "user": "snowball",
                "password": "password",
            }
        }
        with pytest.raises(
            MissingConfigOptionError,
            match=re.escape(
                "Configuration option 'settings.output_format' is not defined anywhere, "
                "have you forgotten to set it in a configuration file, or "
                "environmental variable?"
            ),
        ):
            TEST_PARSER["settings"]["output_format"]

    def test_only_in_slice(self, tmp_files):
        tmp_folder = tmp_files(
            {
                "config.toml": dedent(
                    """\
                    [settings]
                    [connections.snowflake]
                    account = "snowflake"
                    user = "snowball"
                    password = "password"
                    """
                ),
            }
        )
        TEST_PARSER = ConfigManager(
            name="root_parser",
            file_path=tmp_folder / "config.toml",
            _slices=(
                ConfigSlice(
                    tmp_folder / "connections.toml",
                    ConfigSliceOptions(
                        only_in_slice=True,
                    ),
                    "connections",
                ),
            ),
        )
        TEST_PARSER.add_option(
            name="connections",
        )
        settings_parser = ConfigManager(
            name="settings",
        )
        settings_parser.add_option(
            name="output_format",
            choices=("json", "yaml", "toml"),
        )
        TEST_PARSER.add_submanager(settings_parser)
        with pytest.raises(
            ConfigSourceError,
            match="Configuration option 'connections' is not defined.*",
        ):
            TEST_PARSER["connections"]

    def test_simple_nesting(self, monkeypatch, tmp_path):
        c1 = ConfigManager(name="test", file_path=tmp_path / "config.toml")
        c2 = ConfigManager(name="sb")
        c3 = ConfigManager(name="sb")
        c3.add_option(name="b", parse_str=lambda e: e.lower() == "true")
        c2.add_submanager(c3)
        c1.add_submanager(c2)
        with monkeypatch.context() as m:
            m.setenv("SNOWFLAKE_SB_SB_B", "TrUe")
            assert c1["sb"]["sb"]["b"] is True

    def test_complicated_nesting(self, monkeypatch, tmp_path):
        c_file = tmp_path / "config.toml"
        c1 = ConfigManager(file_path=c_file, name="root_parser")
        c2 = ConfigManager(file_path=tmp_path / "config2.toml", name="sp")
        c2.add_option(name="b", parse_str=lambda e: e.lower() == "true")
        c1.add_submanager(c2)
        c_file.write_text(
            dedent(
                """\
                [connections.default]
                user="testuser"
                account="testaccount"
                password="testpassword"

                [sp]
                b = true
                """
            )
        )
        c_file.chmod(stat.S_IRUSR | stat.S_IWUSR)
        assert c1["sp"]["b"] is True

    def test_error_missing_file_path(self):
        with pytest.raises(
            ConfigManagerError,
            match="ConfigManager is trying to read config file, but it doesn't have one",
        ):
            ConfigManager(name="test_parser").read_config()

    def test_error_invalid_toml(self, tmp_path):
        c_file = tmp_path / "c.toml"
        c_file.write_text(
            dedent(
                """\
                invalid toml file
                """
            )
        )
        c_file.chmod(stat.S_IRUSR | stat.S_IWUSR)
        with pytest.raises(
            ConfigSourceError,
            match=re.escape(f"An unknown error happened while loading '{str(c_file)}'"),
        ):
            ConfigManager(
                name="test_parser",
                file_path=c_file,
            ).read_config()

    def test_error_child_conflict(self):
        cp = ConfigManager(name="test_parser")
        cp.add_submanager(ConfigManager(name="b"))
        with pytest.raises(
            ConfigManagerError,
            match="'b' sub-manager, or option conflicts with a child element of 'test_parser'",
        ):
            cp.add_option(name="b")

    def test_explicit_env_name(self, monkeypatch):
        rnd_string = str(uuid4())
        toml_value = dedent(
            f"""\
            text = "{rnd_string}"
            """
        )
        TEST_PARSER = ConfigManager(
            name="test_parser",
        )

        TEST_PARSER.add_option(name="connections", parse_str=tomlkit.loads, env_name="CONNECTIONS")
        with monkeypatch.context() as m:
            m.setenv("CONNECTIONS", toml_value)
            assert TEST_PARSER["connections"] == {"text": rnd_string}

    def test_error_contains(self, monkeypatch):
        tp = ConfigManager(
            name="test_parser",
        )
        tp.add_option(name="output_format", choices=("json", "csv"))
        with monkeypatch.context() as m:
            m.setenv("SNOWFLAKE_OUTPUT_FORMAT", "toml")
            with pytest.raises(
                ConfigSourceError,
                match="The value of output_format read from environment variable is not part of",
            ):
                tp["output_format"]

    def test_error_missing_item(self):
        tp = ConfigManager(
            name="test_parser",
        )
        with pytest.raises(
            ConfigSourceError,
            match="No ConfigManager, or ConfigOption can be found with the name 'asd'",
        ):
            tp["asd"]

    def test_error_missing_fp(self):
        tp = ConfigManager(
            name="test_parser",
        )
        with pytest.raises(
            ConfigManagerError,
            match="ConfigManager is trying to read config file, but it doesn't have one",
        ):
            tp.read_config()

    def test_missing_config_file(self, tmp_path):
        config_file = tmp_path / "config.toml"
        cm = ConfigManager(name="test", file_path=config_file)
        cm.add_option(name="output_format", choices=("json", "yaml"))
        with raises(
            MissingConfigOptionError,
            match="Configuration option 'output_format' is not defined anywhere.*",
        ):
            cm["output_format"]

    def test_missing_config_files_sliced(self, tmp_path):
        config_file = tmp_path / "config.toml"
        connections_file = tmp_path / "connections.toml"
        cm = ConfigManager(
            name="test",
            file_path=config_file,
            _slices=(ConfigSlice(connections_file, ConfigSliceOptions(), "connections"),),
        )
        cm.add_option(
            name="connections",
        )
        with raises(
            MissingConfigOptionError,
            match="Configuration option 'connections' is not defined anywhere.*",
        ):
            cm["connections"]

    def test_error_missing_fp_retrieve(self):
        tp = ConfigManager(
            name="test_parser",
        )
        tp.add_option(name="option")
        if NEW_DRIVER_ONLY("BD#14"):
            with pytest.raises(
                MissingConfigOptionError,
                match="Configuration option 'option' is not defined anywhere",
            ):
                tp["option"]
        else:
            with pytest.raises(
                ConfigManagerError,
                match="missing file_path",
            ):
                tp["option"]

    def test_configoption_missing_root_manager(self):
        with pytest.raises(
            TypeError,
            match="_root_manager cannot be None",
        ):
            ConfigOption(
                name="test_option",
                _nest_path=["test_option"],
                _root_manager=None,
            )

    def test_configoption_missing_nest_path(self):
        with pytest.raises(
            TypeError,
            match="_nest_path cannot be None",
        ):
            ConfigOption(
                name="test_option",
                _nest_path=None,
                _root_manager=ConfigManager(name="test_manager"),
            )

    def test_configoption_default_value(self, tmp_path, monkeypatch):
        env_name = f"SF_TEST_OPTION_{uuid4()}"
        conf_val = str(uuid4())
        cm = ConfigManager(
            name="test_manager",
            file_path=tmp_path / "config.toml",
        )
        cm.add_option(
            name="test_option",
            env_name=env_name,
            default=conf_val,
        )
        assert cm["test_option"] == conf_val
        env_value = str(uuid4())
        with monkeypatch.context() as c:
            c.setenv(env_name, env_value)
            assert cm["test_option"] == env_value

    def test_defaultconnectionname(self, tmp_path, monkeypatch):
        c_file = tmp_path / "config.toml"
        old_path = CONFIG_MANAGER.file_path
        CONFIG_MANAGER.file_path = c_file
        CONFIG_MANAGER.conf_file_cache = None
        try:
            with monkeypatch.context() as m:
                m.delenv("SNOWFLAKE_DEFAULT_CONNECTION_NAME", raising=False)
                assert CONFIG_MANAGER["default_connection_name"] == "default"
            env_val = "DEF_CONN_" + str(uuid4()).upper()
            with monkeypatch.context() as m:
                m.setenv("SNOWFLAKE_DEFAULT_CONNECTION_NAME", env_val)
                assert CONFIG_MANAGER["default_connection_name"] == env_val
            assert CONFIG_MANAGER.file_path is not None
            con_name = "conn_" + str(uuid4()).upper()
            c_file.write_text(
                dedent(
                    f"""\
                    default_connection_name = "{con_name}"
                    """
                )
            )
            c_file.chmod(stat.S_IRUSR | stat.S_IWUSR)
            # re-cache config file from disk
            CONFIG_MANAGER.file_path = c_file
            CONFIG_MANAGER.conf_file_cache = None
            assert CONFIG_MANAGER["default_connection_name"] == con_name
        finally:
            CONFIG_MANAGER.file_path = old_path
            CONFIG_MANAGER.conf_file_cache = None
