"""
Parameter binding serialization for Snowflake universal driver.

This module handles serialization of Python parameter bindings to JSON format
for transmission to the Rust core, following the design specified in bindingsdesign.md.
"""

from __future__ import annotations

import json

from collections.abc import Sequence
from typing import Any


class BindingSerializer:
    """Serializes Python parameters to Snowflake binding JSON format."""

    # Python type to Snowflake type mapping
    TYPE_MAP = {
        "int": "FIXED",
        "float": "REAL",
        "str": "TEXT",
        "bool": "BOOLEAN",
        "bytes": "BINARY",
        "datetime": "TIMESTAMP_NTZ",
        "date": "DATE",
        "time": "TIME",
        "decimal": "FIXED",
        "nonetype": "TEXT",  # NULL values
    }

    @classmethod
    def serialize_parameters(cls, params: Sequence[Any] | dict[str, Any] | None) -> tuple[str | None, int]:
        """
        Serialize parameters to JSON format for binding.

        Args:
            params: Parameters to serialize (sequence for positional, dict for named)

        Returns:
            Tuple of (JSON string or None, length in bytes)
        """
        if params is None or len(params) == 0:
            return None, 0

        bindings = cls._process_params(params)
        if not bindings:
            return None, 0

        json_str = json.dumps(bindings)
        return json_str, len(json_str.encode("utf-8"))

    @classmethod
    def _process_params(cls, params: Sequence[Any] | dict[str, Any]) -> dict[str, dict[str, Any]]:
        """
        Process parameters into Snowflake binding format.

        The format is:
        {
            "1": {"type": "FIXED", "value": "123"},
            "2": {"type": "TEXT", "value": "hello"}
        }

        For arrays (multi-row):
        {
            "1": {"type": "FIXED", "value": ["1", "2", "3"]},
            "2": {"type": "TEXT", "value": ["hello", "world", "foo"]}
        }
        """
        bindings = {}

        if isinstance(params, dict):
            # Named parameters (e.g., :name style)
            for key, value in params.items():
                snowflake_type, snowflake_value = cls._convert_value(value)
                bindings[str(key)] = {"type": snowflake_type, "value": snowflake_value}
        else:
            # Positional parameters (e.g., ? or :1 style)
            for idx, value in enumerate(params):
                if isinstance(value, list):
                    # Array binding for bulk operations
                    snowflake_type, values = cls._convert_array(value)
                    bindings[str(idx + 1)] = {"type": snowflake_type, "value": values}
                else:
                    snowflake_type, snowflake_value = cls._convert_value(value)
                    bindings[str(idx + 1)] = {"type": snowflake_type, "value": snowflake_value}

        return bindings

    @classmethod
    def _convert_value(cls, value: Any) -> tuple[str, Any]:
        """
        Convert a Python value to Snowflake binding format.

        Returns:
            Tuple of (Snowflake type string, converted value)
        """
        if value is None:
            return "TEXT", None

        type_name = value.__class__.__name__.lower()
        snowflake_type = cls.TYPE_MAP.get(type_name, "TEXT")

        # Convert value to string representation for JSON
        # Special handling for different types
        if isinstance(value, bool):
            # Boolean must be before int check since bool is subclass of int
            converted = str(value).lower()
        elif isinstance(value, (int, float)):
            converted = str(value)
        elif isinstance(value, str):
            converted = value
        elif isinstance(value, bytes):
            # Binary data - base64 encode
            import base64

            converted = base64.b64encode(value).decode("ascii")
        else:
            # For other types (datetime, date, time, decimal, etc.)
            # use string representation
            converted = str(value)

        return snowflake_type, converted

    @classmethod
    def _convert_array(cls, values: list[Any]) -> tuple[str, list[Any]]:
        """
        Convert an array of Python values to Snowflake binding format.

        Returns:
            Tuple of (Snowflake type string, list of converted values)
        """
        if not values:
            return "TEXT", []

        # Convert all values and determine type
        converted_values = []
        types = set()

        for value in values:
            snowflake_type, converted = cls._convert_value(value)
            converted_values.append(converted)
            if value is not None:
                types.add(snowflake_type)

        # If all non-null values have the same type, use that type
        # Otherwise, default to TEXT
        if len(types) == 1:
            snowflake_type = types.pop()
        elif len(types) == 0:
            snowflake_type = "TEXT"
        else:
            # Mixed types - use TEXT as fallback
            snowflake_type = "TEXT"

        return snowflake_type, converted_values

    # TODO: Implement stage binding decision logic in follow-up
    # When data size exceeds CLIENT_STAGE_ARRAY_BINDING_THRESHOLD (default 65280),
    # should serialize to CSV and upload to stage instead of using JSON binding.
