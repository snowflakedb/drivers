"""
Parameter binding serialization for Snowflake universal driver.

This module handles serialization of Python parameter bindings to JSON format
for transmission to the Rust core, following the design specified in bindingsdesign.md.

Conversion logic mirrors the reference snowflake-connector-python's
SnowflakeConverter.to_snowflake_bindings and Connection._process_params_qmarks.
"""

from __future__ import annotations

import binascii
import json
import time as time_module

from collections.abc import Sequence
from datetime import date, datetime, time, timedelta, timezone
from decimal import Decimal
from typing import Any

from .type_codes import PYTHON_TO_SNOWFLAKE_TYPE


# Epoch constants (timezone-independent)
_ZERO_EPOCH_DATE = date(1970, 1, 1)
_ZERO_EPOCH = datetime.fromtimestamp(0, timezone.utc).replace(tzinfo=None)


class BindingSerializer:
    """Serializes Python parameters to Snowflake binding JSON format."""

    @staticmethod
    def _convert_datetime_to_epoch_nanoseconds(dt: datetime) -> str:
        """Convert datetime to epoch nanoseconds string.

        Uses timedelta arithmetic from a fixed epoch (timezone-independent),
        matching the reference connector's convert_datetime_to_epoch approach.
        """
        if dt.tzinfo is not None:
            # Convert tz-aware datetime to UTC, then strip tzinfo
            dt = dt.astimezone(timezone.utc).replace(tzinfo=None)
        epoch_seconds = (dt - _ZERO_EPOCH).total_seconds()
        # Format with full precision, remove dot, append "000" for nanoseconds
        return f"{epoch_seconds:f}".replace(".", "") + "000"

    @staticmethod
    def _convert_date_to_epoch_milliseconds(d: date) -> str:
        """Convert date to epoch milliseconds string.

        Uses timezone-independent timedelta arithmetic from epoch date,
        matching the reference connector's _convert_date_to_epoch_milliseconds.
        """
        return f"{(d - _ZERO_EPOCH_DATE).total_seconds():.3f}".replace(".", "")

    @staticmethod
    def _convert_time_to_nanoseconds(t: time) -> str:
        """Convert time to nanoseconds since midnight string.

        Uses string concatenation approach matching the reference connector's
        _convert_time_to_epoch_nanoseconds.
        """
        total_seconds = t.hour * 3600 + t.minute * 60 + t.second
        return str(total_seconds) + f"{t.microsecond:06d}" + "000"

    @staticmethod
    def _convert_timedelta_to_nanoseconds(td: timedelta) -> str:
        """Convert timedelta to nanoseconds string for TIME binding.

        Matches the reference connector's _timedelta_to_snowflake_bindings.
        """
        hours, remainder = divmod(td.seconds, 3600)
        mins, secs = divmod(remainder, 60)
        hours += td.days * 24
        return str(hours * 3600 + mins * 60 + secs) + f"{td.microseconds:06d}" + "000"

    @classmethod
    def serialize_parameters(cls, params: Sequence[Any] | None) -> tuple[str | None, int]:
        """
        Serialize parameters to JSON format for binding.

        Args:
            params: Parameters to serialize (sequence for positional)

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
    def _process_params(cls, params: Sequence[Any]) -> dict[str, dict[str, Any]]:
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

        Supports explicit type-hint tuples: ("SNOWFLAKE_TYPE", value).
        This mirrors the reference connector's _get_snowflake_type_and_binding.
        """
        bindings = {}

        # Positional parameters (e.g., ? or :1 style)
        for idx, value in enumerate(params):
            if isinstance(value, list):
                # Array binding for bulk operations
                snowflake_type, values = cls._convert_array(value)
                bindings[str(idx + 1)] = {"type": snowflake_type, "value": values}
            else:
                snowflake_type, snowflake_value = cls._get_type_and_binding(value)
                bindings[str(idx + 1)] = {"type": snowflake_type, "value": snowflake_value}

        return bindings

    @classmethod
    def _get_type_and_binding(cls, value: Any) -> tuple[str, Any]:
        """Resolve the Snowflake type and serialized value for a parameter.

        Supports explicit type-hint tuples ``("SNOWFLAKE_TYPE", value)`` as used
        by the reference connector (e.g. ``("DECFLOAT", Decimal("1.23"))``).
        When *value* is a 2-tuple whose first element is a string, the tuple is
        unpacked and the explicit type is used; otherwise, the type is inferred
        via ``_convert_value``.
        """
        if isinstance(value, tuple) and len(value) == 2 and isinstance(value[0], str):
            snowflake_type, raw_value = value
            _, converted = cls._convert_value(raw_value)
            return snowflake_type, converted
        return cls._convert_value(value)

    @classmethod
    def _convert_value(cls, value: Any) -> tuple[str, Any]:
        """
        Convert a Python value to Snowflake binding format.

        Returns:
            Tuple of (Snowflake type string, converted value)
        """
        if value is None:
            return "ANY", None

        type_name = value.__class__.__name__.lower()
        snowflake_type = PYTHON_TO_SNOWFLAKE_TYPE.get(type_name, "TEXT")

        # Convert value to string representation for JSON
        # Order matters: bool before int (bool is subclass of int),
        # datetime before date (datetime is subclass of date)
        if isinstance(value, bool):
            converted = str(value).lower()
        elif isinstance(value, datetime):
            # Datetime to epoch nanoseconds (must be before date check)
            converted = cls._convert_datetime_to_epoch_nanoseconds(value)
        elif isinstance(value, date):
            # Date to epoch milliseconds
            converted = cls._convert_date_to_epoch_milliseconds(value)
        elif isinstance(value, time):
            # Time to nanoseconds since midnight
            converted = cls._convert_time_to_nanoseconds(value)
        elif isinstance(value, timedelta):
            # Timedelta to nanoseconds (for TIME type)
            converted = cls._convert_timedelta_to_nanoseconds(value)
        elif isinstance(value, time_module.struct_time):
            # struct_time -> convert to datetime, then to epoch nanoseconds
            dt = datetime.fromtimestamp(time_module.mktime(value))
            converted = cls._convert_datetime_to_epoch_nanoseconds(dt)
        elif isinstance(value, (int, float)):
            converted = str(value)
        elif isinstance(value, Decimal):
            converted = str(value)
        elif isinstance(value, str):
            converted = value
        elif isinstance(value, (bytes, bytearray)):
            # Binary data - hex encode
            converted = binascii.hexlify(value).decode("utf-8")
        else:
            # For other types use string representation
            converted = str(value)

        return snowflake_type, converted

    @classmethod
    def _convert_array(cls, values: list[Any]) -> tuple[str, list[Any]]:
        """
        Convert an array of Python values to Snowflake binding format.

        Supports explicit type-hint tuples via ``_get_type_and_binding``.

        Returns:
            Tuple of (Snowflake type string, list of converted values)
        """
        if not values:
            return "TEXT", []

        # Convert all values and determine type
        converted_values = []
        types = set()

        for value in values:
            snowflake_type, converted = cls._get_type_and_binding(value)
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
