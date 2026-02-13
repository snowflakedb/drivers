"""
ConfigManager implementation that wraps the Rust sf_core config management API.

This module provides backward compatibility with the old Python Snowflake driver's
ConfigManager while using the new Rust core for actual configuration management.
"""

from __future__ import annotations

import base64
import json
import logging
import os
import warnings

from collections.abc import Iterable
from pathlib import Path
from typing import Any, Callable, Literal, NamedTuple, TypeVar

from snowflake.connector._internal.api_client.c_api import (
    sf_core_config_load_all_sections,
)
from snowflake.connector.errors import (
    ConfigManagerError,
    ConfigSourceError,
    Error,
    MissingConfigOptionError,
)


_T = TypeVar("_T")

LOGGER = logging.getLogger(__name__)

# Environment variable to skip permission warnings (backward compatibility)
SKIP_WARNING_ENV_VAR = "SF_SKIP_WARNING_FOR_READ_PERMISSIONS_ON_CONFIG_FILE"


def _should_skip_warning_for_read_permissions_on_config_file() -> bool:
    """Check if the warning should be skipped based on environment variable."""
    return os.getenv(SKIP_WARNING_ENV_VAR, "false").lower() == "true"


class ConfigSliceOptions(NamedTuple):
    """Class that defines settings individual configuration files."""

    check_permissions: bool = True
    only_in_slice: bool = False


class ConfigSlice(NamedTuple):
    path: Path
    options: ConfigSliceOptions
    section: str


def _parse_setting_from_json(setting_json: dict[str, Any]) -> Any:
    """Parse a setting from the JSON format returned by Rust."""
    setting_type = setting_json.get("type")
    value = setting_json.get("value")

    if setting_type == "bytes" and isinstance(value, str):
        # Decode base64 encoded bytes
        return base64.b64decode(value)

    return value


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
        """Create a config option that can read values from different sources."""
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

        This function implements order of precedence between different sources.
        Priority: Custom Environment Variable > Config File (with env overrides) > Default

        Note: Default environment variable overrides (SNOWFLAKE_<KEY>) are handled
        by sf_core when loading config. Custom env_name and env_name=False are
        Python wrapper features for backward compatibility.
        """
        source = "configuration file"
        value = None

        # Check for custom environment variable (Python-specific feature)
        if self.env_name is not False and self.env_name is not None:
            # Custom env var name specified - check it first
            env_var = os.environ.get(self.env_name)
            if env_var is not None:
                source = "environment variable"
                value = self.parse_str(env_var) if self.parse_str else env_var

        if value is None:
            try:
                # Load config - sf_core handles default env overrides (SNOWFLAKE_<KEY>)
                # unless env_name=False, in which case we skip env overrides
                apply_env_overrides = self.env_name is not False
                value = self._get_config(apply_env_overrides=apply_env_overrides)
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
        """User-friendly name of the config option. Includes self._nest_path."""
        return ".".join(self._nest_path[1:])

    @property
    def default_env_name(self) -> str:
        """The default environmental variable name for this option."""
        pieces = [p.upper() for p in self._nest_path[1:]]
        return f"SNOWFLAKE_{'_'.join(pieces)}"

    def _get_config(self, apply_env_overrides: bool = True) -> Any:
        """Get value from the cached configuration.

        Uses the root manager's cache, loading from Rust core if cache is empty.

        Args:
            apply_env_overrides: If True, use config with SNOWFLAKE_<KEY> env overrides.
        """
        # Get cached config from root manager
        all_sections = self._root_manager._get_cached_config(apply_env_overrides)

        if not all_sections:
            raise MissingConfigOptionError(
                f"Configuration option '{self.option_name}' is not defined anywhere. "
                "No configuration files were found or they could not be loaded. "
                "Ensure that config files exist in the SNOWFLAKE_HOME directory "
                "or set the value via an environment variable."
            )

        # Build the path to the config option
        if len(self._nest_path) > 2:
            # This is a nested option, need to find the section
            section_path = ".".join(self._nest_path[1:-1])
            option_name = self._nest_path[-1]

            if section_path in all_sections:
                section_settings = all_sections[section_path]
                if option_name in section_settings:
                    return _parse_setting_from_json(section_settings[option_name])

            raise MissingConfigOptionError(f"Configuration option '{self.option_name}' is not defined anywhere")
        else:
            # For top-level options
            if self._nest_path[0] == "connections":
                # Extract all connections from sections with "connections." prefix
                connections = {}
                for section_name, section_settings in all_sections.items():
                    if section_name.startswith("connections."):
                        conn_name = section_name[len("connections.") :]
                        connections[conn_name] = section_settings
                return connections
            else:
                # Find the option in the appropriate section (non-connection sections)
                for section_name, section_settings in all_sections.items():
                    if not section_name.startswith("connections."):
                        if self.name in section_settings:
                            return _parse_setting_from_json(section_settings[self.name])

                raise MissingConfigOptionError(f"Configuration option '{self.option_name}' is not defined anywhere")


class ConfigManager:
    """Read a TOML configuration file with managed multi-source precedence.

    This class provides backward compatibility with the old Python driver's ConfigManager
    while using the Rust sf_core for actual configuration management.
    """

    def __init__(
        self,
        *,
        name: str,
        file_path: Path | None = None,
        _slices: list[ConfigSlice] | None = None,
    ):
        """Create a new ConfigManager."""
        if _slices is None:
            _slices = list()

        self.name = name
        self.file_path = file_path
        self._slices = _slices

        # Objects holding sub-managers and options
        self._options: dict[str, ConfigOption] = dict()
        self._sub_managers: dict[str, ConfigManager] = dict()

        # Cache for configuration loaded from Rust core
        # conf_file_cache: config with env overrides applied (default)
        # _conf_file_cache_raw: config without env overrides (for env_name=False)
        self.conf_file_cache: dict[str, Any] | None = None
        self._conf_file_cache_raw: dict[str, Any] | None = None

        # Information necessary to be able to nest elements
        self._root_manager: ConfigManager = self
        self._nest_path = [name]

    def read_config(
        self,
        skip_file_permissions_check: bool = False,
        apply_env_overrides: bool = True,
    ) -> None:
        """Read and cache config file contents from Rust core.

        This method loads configuration from sf_core and caches it to avoid
        repeated calls for each config value lookup.

        Args:
            skip_file_permissions_check: Ignored (handled by Rust core).
            apply_env_overrides: If True, cache config with env overrides.
                If False, cache raw config without env overrides.
        """
        code, result = sf_core_config_load_all_sections(apply_env_overrides)

        if code != 0:
            LOGGER.debug("Failed to load config from sf_core: %s", result.decode("utf-8", errors="replace"))
            # Set empty cache to indicate we tried but failed
            if apply_env_overrides:
                self.conf_file_cache = {}
            else:
                self._conf_file_cache_raw = {}
            return

        try:
            all_sections = json.loads(result)
        except json.JSONDecodeError:
            LOGGER.debug("Failed to parse config response: %s", result.decode("utf-8", errors="replace"))
            if apply_env_overrides:
                self.conf_file_cache = {}
            else:
                self._conf_file_cache_raw = {}
            return

        if apply_env_overrides:
            self.conf_file_cache = all_sections
        else:
            self._conf_file_cache_raw = all_sections

    def _get_cached_config(self, apply_env_overrides: bool = True) -> dict[str, Any]:
        """Get cached config, loading from Rust core if cache is empty.

        Args:
            apply_env_overrides: If True, return config with env overrides.

        Returns:
            Dictionary of all config sections.
        """
        cache = self.conf_file_cache if apply_env_overrides else self._conf_file_cache_raw
        if cache is None:
            self.read_config(apply_env_overrides=apply_env_overrides)
            cache = self.conf_file_cache if apply_env_overrides else self._conf_file_cache_raw
        return cache or {}

    def clear_cache(self) -> None:
        """Clear the configuration cache, forcing a reload on next access."""
        self.conf_file_cache = None
        self._conf_file_cache_raw = None

    def add_option(
        self,
        *,
        option_cls: type[ConfigOption] = ConfigOption,
        **kwargs: Any,
    ) -> None:
        """Add a ConfigOption to this ConfigManager."""
        kwargs["_root_manager"] = self._root_manager
        kwargs["_nest_path"] = self._nest_path
        new_option = option_cls(**kwargs)
        self._check_child_conflict(new_option.name)
        self._options[new_option.name] = new_option

    def _check_child_conflict(self, name: str) -> None:
        """Check if a sub-manager, or ConfigOption conflicts with given name."""
        if name in (self._options.keys() | self._sub_managers.keys()):
            raise ConfigManagerError(f"'{name}' sub-manager, or option conflicts with a child element of '{self.name}'")

    def add_submanager(self, new_child: ConfigManager) -> None:
        """Nest another ConfigManager under this one."""
        self._check_child_conflict(new_child.name)
        self._sub_managers[new_child.name] = new_child

        def _root_setter_helper(node: ConfigManager) -> None:
            # Deal with ConfigManagers
            node._root_manager = self._root_manager
            node._nest_path = self._nest_path + node._nest_path
            for sub_manager in node._sub_managers.values():
                _root_setter_helper(sub_manager)
            # Deal with ConfigOptions
            for option in node._options.values():
                option._root_manager = self._root_manager
                option._nest_path = self._nest_path + option._nest_path

        _root_setter_helper(new_child)

    def add_subparser(self, new_child: ConfigManager) -> None:
        """Add a sub-manager (deprecated, use add_submanager instead)."""
        warnings.warn(
            "add_subparser is deprecated, use add_submanager instead",
            DeprecationWarning,
            stacklevel=2,
        )
        self.add_submanager(new_child)

    @property
    def _sub_parsers(self) -> dict[str, ConfigManager]:
        """Deprecated: Use _sub_managers instead."""
        warnings.warn(
            "_sub_parsers is deprecated, use _sub_managers instead",
            DeprecationWarning,
            stacklevel=2,
        )
        return self._sub_managers

    def __getitem__(self, name: str) -> ConfigOption | ConfigManager | Any:
        """Get either sub-manager, or option in this manager with name."""
        if name in self._options:
            return self._options[name].value()
        if name not in self._sub_managers:
            # Special handling for connections
            if self.name == "CONFIG_MANAGER" and name == "connections":
                # Use cached config
                all_sections = self._get_cached_config(apply_env_overrides=True)
                # Extract connections from sections with "connections." prefix
                connections = {}
                for section_name, section_settings in all_sections.items():
                    if section_name.startswith("connections."):
                        conn_name = section_name[len("connections.") :]
                        connections[conn_name] = section_settings
                return connections

            raise KeyError(f"No ConfigManager, or ConfigOption can be found with the name '{name}'")
        return self._sub_managers[name]


# Default configuration paths (backward compatibility)
CONFIG_FILE = Path.home() / ".snowflake" / "config.toml"
CONNECTIONS_FILE = Path.home() / ".snowflake" / "connections.toml"

# Create the root CONFIG_MANAGER (backward compatibility)
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

# Add default options (backward compatibility)
CONFIG_MANAGER.add_option(
    name="connections",
    parse_str=json.loads,
    default=dict(),
)

CONFIG_MANAGER.add_option(
    name="default_connection_name",
    default="default",
)


def _get_default_connection_params() -> dict[str, Any]:
    """Get default connection parameters from configuration."""
    def_connection_name = str(CONFIG_MANAGER["default_connection_name"])
    connections: dict[str, Any] = CONFIG_MANAGER["connections"]  # type: ignore[assignment]

    if def_connection_name not in connections:
        raise Error(
            f"Default connection with name '{def_connection_name}' "
            "cannot be found, known ones are "
            f"{list(connections.keys())}"
        )

    # Parse the connection settings
    conn_settings: dict[str, Any] = connections[def_connection_name]
    result: dict[str, Any] = {}

    for key, setting in conn_settings.items():
        result[key] = _parse_setting_from_json(setting)

    return result


# Deprecated alias for backward compatibility
def __getattr__(name: str) -> Any:
    if name == "CONFIG_PARSER":
        warnings.warn(
            "CONFIG_PARSER is deprecated, use CONFIG_MANAGER instead",
            DeprecationWarning,
            stacklevel=2,
        )
        return CONFIG_MANAGER
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")


# Export public API
__all__ = [
    "ConfigOption",
    "ConfigManager",
    "CONFIG_MANAGER",
]
