"""Shared utilities for preparing Python values for Core's protobuf API.

create_config_setting: single value → ConfigSetting protobuf
create_config_settings_from_dict: dict of values → dict of ConfigSetting
pop_typed_kwarg: pop + validate a kwarg by expected type
"""

from __future__ import annotations

from typing import Any

from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import ConfigSetting
from snowflake.connector.errors import ProgrammingError


def create_config_setting(value: Any, *, allow_none: bool = True) -> ConfigSetting | None:
    """Create a ConfigSetting protobuf from a Python value.

    Args:
        value: Python value (bool, int, str, float, or bytes).
        allow_none: If True (default), return None for None values — they mean
            "not set" in Python kwargs and have no ConfigSetting representation.
            If False, None raises TypeError like any other unsupported type.

    Returns:
        ConfigSetting protobuf message, or None if value is None and allow_none is True.

    Raises:
        TypeError: If value type is not supported.
    """
    if value is None and allow_none:
        return None
    # Check bool before int (bool is a subclass of int in Python)
    if isinstance(value, bool):
        return ConfigSetting(bool_value=value)
    if isinstance(value, int):
        return ConfigSetting(int_value=value)
    if isinstance(value, str):
        return ConfigSetting(string_value=value)
    if isinstance(value, float):
        return ConfigSetting(double_value=value)
    if isinstance(value, bytes):
        return ConfigSetting(bytes_value=value)
    raise TypeError(
        f"Unsupported parameter type: {type(value).__name__}. Supported types: bool, int, str, float, bytes"
    )


def create_config_settings_from_dict(kwargs: dict[str, Any]) -> dict[str, ConfigSetting]:
    """Wrap a dict of Python values into ConfigSetting protobuf messages for Core.

    None values are silently skipped (they mean "not set" in Python kwargs).
    """
    return {key: setting for key, value in kwargs.items() if (setting := create_config_setting(value)) is not None}


def pop_typed_kwarg(kwargs: dict[str, Any], key: str, expected_type: type, default: Any = None) -> Any:
    """Pop a kwarg with runtime type validation.

    Returns default if key is not present. If default is None (not provided),
    None is returned and no type check is performed (optional param).
    Raises ProgrammingError if the value is present but not the expected type.

    Note: bool is a subclass of int in Python, so isinstance(True, int) is True.
    This means pop_typed_kwarg(kwargs, "port", int) accepts True as a valid int.
    We accept this Python quirk rather than adding special-case logic.
    """
    value = kwargs.pop(key, default)
    if value is not None and not isinstance(value, expected_type):
        raise ProgrammingError(f"{key} must be {expected_type.__name__}, got {type(value).__name__}")
    return value
