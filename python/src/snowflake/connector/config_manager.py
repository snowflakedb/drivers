"""
ConfigManager implementation that wraps the Rust sf_core config management API.

This module provides backward compatibility with the old Python Snowflake driver's
ConfigManager while using the Rust core for actual configuration management.
All file I/O, TOML parsing, and permission checks are performed by sf_core.
"""

from __future__ import annotations

import logging
import os

from collections.abc import Iterable
from pathlib import Path
from typing import Any, Callable, Literal, NamedTuple, TypeVar

from snowflake.connector._internal.api_client.client_api import (
    database_driver_client,
)
from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
    ConfigGetPathsRequest,
    ConfigLoadAllSectionsRequest,
)
from snowflake.connector._internal.protobuf_gen.proto_exception import (
    ProtoApplicationException,
)
from snowflake.connector.errors import (
    ConfigManagerError,
    ConfigSourceError,
    MissingConfigOptionError,
)


_T = TypeVar("_T")

LOGGER = logging.getLogger(__name__)


class ConfigSliceOptions(NamedTuple):
    """Class that defines settings individual configuration files."""

    check_permissions: bool = True
    only_in_slice: bool = False


class ConfigSlice(NamedTuple):
    path: Path
    options: ConfigSliceOptions
    section: str


def _parse_config_setting(setting: Any) -> Any:
    """Parse a ConfigSetting protobuf message to extract the value."""
    value_field = setting.WhichOneof("value")
    if value_field == "string_value":
        return setting.string_value
    elif value_field == "int_value":
        return setting.int_value
    elif value_field == "double_value":
        return setting.double_value
    elif value_field == "bytes_value":
        return setting.bytes_value
    elif value_field == "bool_value":
        return setting.bool_value
    return None


def _translate_core_error(
    exc: ProtoApplicationException,
    file_path: Path | None = None,
) -> ConfigManagerError:
    """Translate a Rust core DriverException into the appropriate Python error.

    Maps sf_core error messages to backward-compatible Python exception types.
    """
    msg = str(exc.message) if hasattr(exc, "message") else str(exc)

    if "parse TOML" in msg or "Failed to parse" in msg:
        display_path = str(file_path) if file_path else "unknown"
        return ConfigSourceError(f"An unknown error happened while loading '{display_path}'")

    if "Failed to read config file" in msg:
        display_path = str(file_path) if file_path else "unknown"
        return ConfigSourceError(f"An unknown error happened while loading '{display_path}'")

    return ConfigManagerError(msg)


class ConfigOption:
    """ConfigOption represents a flag/setting.

    This class is backward compatible with the old Python driver's ConfigOption
    but uses the Rust core for actual configuration retrieval.
    """

    def __init__(
        self,
        *,
        name: str,
        parse_str: Callable[[str], _T] | None = None,
        choices: Iterable[Any] | None = None,
        env_name: str | None | Literal[False] = None,
        default: Any | None = None,
        _root_manager: ConfigManager | None = None,
        _nest_path: list[str] | None = None,
    ) -> None:
        if _root_manager is None:
            raise TypeError("_root_manager cannot be None")
        if _nest_path is None:
            raise TypeError("_nest_path cannot be None")

        self.name = name
        self.parse_str = parse_str
        self.choices = choices
        self._nest_path = _nest_path + [name]
        self._root_manager: ConfigManager = _root_manager
        self.env_name = env_name
        self.default = default

    def value(self) -> Any:
        """Retrieve a value of option.

        Priority: Custom Environment Variable > Config File > Default
        """
        source = "configuration file"
        value = None

        if self.env_name is not False:
            env_name = self.env_name or self.default_env_name
            if env_name:
                env_var = os.environ.get(env_name)
                if env_var is not None:
                    source = "environment variable"
                    value = self.parse_str(env_var) if self.parse_str else env_var

        if value is None:
            try:
                value = self._get_config()
                source = "configuration file"
            except MissingConfigOptionError:
                if self.default is not None:
                    source = "default_value"
                    value = self.default
                else:
                    raise

        if self.choices and value not in self.choices:
            raise ConfigSourceError(f"The value of {self.option_name} read from {source} is not part of {self.choices}")

        return value

    @property
    def option_name(self) -> str:
        return ".".join(self._nest_path[1:])

    @property
    def default_env_name(self) -> str:
        pieces = [p.upper() for p in self._nest_path[1:]]
        return f"SNOWFLAKE_{'_'.join(pieces)}"

    def _get_config(self) -> Any:
        """Get value from the cached configuration loaded by sf_core."""
        all_sections = self._root_manager._get_cached_config()

        if not all_sections:
            raise MissingConfigOptionError(
                f"Configuration option '{self.option_name}' is not defined anywhere, "
                "have you forgotten to set it in a configuration file, or "
                "environmental variable?"
            )

        if len(self._nest_path) > 2:
            section_path = ".".join(self._nest_path[1:-1])
            option_name = self._nest_path[-1]

            if section_path in all_sections:
                section_settings = all_sections[section_path]
                if option_name in section_settings:
                    return section_settings[option_name]

            raise MissingConfigOptionError(
                f"Configuration option '{self.option_name}' is not defined anywhere, "
                "have you forgotten to set it in a configuration file, or "
                "environmental variable?"
            )
        else:
            if self.name == "connections":
                connections = {}
                for section_name, section_settings in all_sections.items():
                    if section_name.startswith("connections."):
                        conn_name = section_name[len("connections.") :]
                        connections[conn_name] = section_settings
                if connections:
                    return connections
                raise MissingConfigOptionError(
                    f"Configuration option '{self.option_name}' is not defined anywhere, "
                    "have you forgotten to set it in a configuration file, or "
                    "environmental variable?"
                )
            else:
                for section_name, section_settings in all_sections.items():
                    if not section_name.startswith("connections."):
                        if self.name in section_settings:
                            return section_settings[self.name]

                raise MissingConfigOptionError(
                    f"Configuration option '{self.option_name}' is not defined anywhere, "
                    "have you forgotten to set it in a configuration file, or "
                    "environmental variable?"
                )


class ConfigManager:
    """Read a TOML configuration file with managed multi-source precedence.

    This class provides backward compatibility with the old Python driver's
    ConfigManager while using the Rust sf_core for actual file I/O,
    TOML parsing, and permission checks.
    """

    def __init__(
        self,
        *,
        name: str,
        file_path: Path | None = None,
        _slices: list[ConfigSlice] | None = None,
    ):
        if _slices is None:
            _slices = list()

        self.name = name
        self.file_path = file_path
        self._slices = _slices

        self._options: dict[str, ConfigOption] = dict()
        self._sub_managers: dict[str, ConfigManager] = dict()

        self.conf_file_cache: dict[str, Any] | None = None

        self._root_manager: ConfigManager = self
        self._nest_path = [name]

    def read_config(
        self,
        skip_file_permissions_check: bool = False,
    ) -> None:
        """Read and cache config file contents from sf_core.

        Passes file_path and slice paths to the Rust core so it reads
        the correct files. Translates core errors to Python exceptions.
        """
        if self.file_path is None:
            raise ConfigManagerError("ConfigManager is trying to read config file, but it doesn't have one")

        request = ConfigLoadAllSectionsRequest()
        request.config_file = str(self.file_path)

        connections_file = self._get_connections_file_path()
        if connections_file is not None:
            request.connections_file = str(connections_file)

        try:
            client = database_driver_client()
            response = client.config_load_all_sections(request)
        except ProtoApplicationException as e:
            msg = str(e.message) if hasattr(e, "message") else str(e)
            if "permission" in msg.lower() or "Permission denied" in msg:
                LOGGER.debug(
                    "Config file '%s' could not be read due to no permission on its parent directory",
                    self.file_path,
                )
                self.conf_file_cache = {}
                return
            raise _translate_core_error(e, self.file_path) from e

        all_sections: dict[str, Any] = {}
        for section_name, section in response.sections.items():
            section_dict: dict[str, Any] = {}
            for key, setting in section.settings.items():
                section_dict[key] = _parse_config_setting(setting)
            all_sections[section_name] = section_dict

        # Handle only_in_slice: remove sections that should only come from slice
        for slice_info in self._slices:
            _slice_path, slice_options, slice_section = slice_info
            if slice_options.only_in_slice:
                keys_to_remove = [k for k in all_sections if k == slice_section or k.startswith(f"{slice_section}.")]
                for k in keys_to_remove:
                    del all_sections[k]

        self.conf_file_cache = all_sections

    def _get_connections_file_path(self) -> Path | None:
        """Extract connections file path from slices configuration."""
        for slice_info in self._slices:
            slice_path, _slice_options, slice_section = slice_info
            if slice_section == "connections":
                return slice_path
        return None

    def _get_cached_config(self) -> dict[str, Any]:
        """Get cached config, loading from sf_core if cache is empty."""
        if self.conf_file_cache is None:
            if self.file_path is None:
                raise ConfigManagerError(f"Root manager '{self.name}' is missing file_path")
            self.read_config()
        return self.conf_file_cache or {}

    def clear_cache(self) -> None:
        self.conf_file_cache = None

    def add_option(
        self,
        *,
        option_cls: type[ConfigOption] = ConfigOption,
        **kwargs: Any,
    ) -> None:
        kwargs["_root_manager"] = self._root_manager
        kwargs["_nest_path"] = self._nest_path
        new_option = option_cls(**kwargs)
        self._check_child_conflict(new_option.name)
        self._options[new_option.name] = new_option

    def _check_child_conflict(self, name: str) -> None:
        if name in (self._options.keys() | self._sub_managers.keys()):
            raise ConfigManagerError(f"'{name}' sub-manager, or option conflicts with a child element of '{self.name}'")

    def add_submanager(self, new_child: ConfigManager) -> None:
        self._check_child_conflict(new_child.name)
        self._sub_managers[new_child.name] = new_child

        def _root_setter_helper(node: ConfigManager) -> None:
            node._root_manager = self._root_manager
            node._nest_path = self._nest_path + node._nest_path
            for sub_manager in node._sub_managers.values():
                _root_setter_helper(sub_manager)
            for option in node._options.values():
                option._root_manager = self._root_manager
                option._nest_path = self._nest_path + option._nest_path

        _root_setter_helper(new_child)

    def __getitem__(self, name: str) -> ConfigOption | ConfigManager | Any:
        if name in self._options:
            return self._options[name].value()
        if name in self._sub_managers:
            return self._sub_managers[name]
        raise ConfigSourceError(f"No ConfigManager, or ConfigOption can be found with the name '{name}'")


def _get_config_paths_from_core() -> tuple[Path, Path]:
    """Retrieve config file paths from sf_core via protobuf."""
    client = database_driver_client()
    response = client.config_get_paths(ConfigGetPathsRequest())
    return Path(response.config_file), Path(response.connections_file)


CONFIG_FILE, CONNECTIONS_FILE = _get_config_paths_from_core()

CONFIG_MANAGER = ConfigManager(
    name="CONFIG_MANAGER",
    file_path=CONFIG_FILE,
    _slices=[
        ConfigSlice(
            CONNECTIONS_FILE,
            ConfigSliceOptions(
                check_permissions=True,
            ),
            "connections",
        ),
    ],
)

CONFIG_MANAGER.add_option(
    name="connections",
    default=dict(),
)

CONFIG_MANAGER.add_option(
    name="default_connection_name",
    default="default",
)


def _get_default_connection_params() -> dict[str, Any]:
    """Get default connection parameters from configuration."""
    from snowflake.connector.errors import Error

    def_connection_name = str(CONFIG_MANAGER["default_connection_name"])
    connections: dict[str, Any] = CONFIG_MANAGER["connections"]  # type: ignore[assignment]

    if def_connection_name not in connections:
        raise Error(
            f"Default connection with name '{def_connection_name}' "
            "cannot be found, known ones are "
            f"{list(connections.keys())}"
        )

    return dict(connections[def_connection_name])


__all__ = [
    "ConfigOption",
    "ConfigManager",
    "CONFIG_MANAGER",
]
