"""
Parameter binding serialization for Snowflake universal driver.

This module handles serialization of Python parameter bindings to JSON format
for transmission to the Rust core, following the design specified in bindingsdesign.md.

It also provides client-side binding support for pyformat/format paramstyles,
implementing escape(), quote(), and to_snowflake() functions that mirror
the reference snowflake-connector-python's SnowflakeConverter.

Conversion logic mirrors the reference snowflake-connector-python's
SnowflakeConverter.to_snowflake_bindings and Connection._process_params_qmarks.
"""

from __future__ import annotations

import binascii
import json
import time as time_module

from collections.abc import Mapping, Sequence
from datetime import date, datetime, time, timedelta, timezone
from decimal import Decimal
from enum import Enum
from typing import Any

from ..errors import ProgrammingError
from .extras import MissingOptionalDependency
from .extras import numpy as np
from .type_codes import PYTHON_TO_SNOWFLAKE_TYPE


class ParamStyle(Enum):
    """PEP 249 parameter binding style enumeration.

    Supports parsing from string and provides methods to determine
    whether the style requires client-side or server-side binding.
    """

    QMARK = "qmark"  # Server-side: ? placeholders
    NUMERIC = "numeric"  # Server-side: :1, :2 placeholders
    FORMAT = "format"  # Client-side: %s interpolation
    PYFORMAT = "pyformat"  # Client-side: %(name)s interpolation

    def __str__(self) -> str:
        return self.value

    @classmethod
    def from_string(cls, value: str) -> ParamStyle:
        """Parse ParamStyle from string value, with normalization.

        Args:
            value: Paramstyle string (case-insensitive, whitespace-trimmed)

        Returns:
            Matching ParamStyle enum value

        Raises:
            ProgrammingError: If value is not a valid paramstyle
        """
        normalized = value.strip().lower()
        for style in cls:
            if style.value == normalized:
                return style
        available = [s.value for s in cls]
        raise ProgrammingError(f"Invalid paramstyle: {value!r}. Supported: {', '.join(sorted(available))}")

    def is_client_side(self) -> bool:
        """Check if this style uses client-side SQL interpolation."""
        return self in (ParamStyle.FORMAT, ParamStyle.PYFORMAT)

    def is_server_side(self) -> bool:
        """Check if this style uses server-side binding."""
        return self in (ParamStyle.QMARK, ParamStyle.NUMERIC)

    def placeholders(self, n: int) -> str:
        """Build a comma-separated placeholder string for *n* parameters.

        Examples:
            >>> ParamStyle.QMARK.placeholders(3)
            '?, ?, ?'
            >>> ParamStyle.NUMERIC.placeholders(3)
            ':1, :2, :3'
            >>> ParamStyle.FORMAT.placeholders(2)
            '%s, %s'
        """
        if self.is_client_side():
            return ", ".join("%s" for _ in range(n))
        if self is ParamStyle.NUMERIC:
            return ", ".join(f":{i}" for i in range(1, n + 1))
        return ", ".join("?" for _ in range(n))


# Numeric types for IS_NUMERIC check (mirrors reference connector's compat.py)
_NUM_DATA_TYPES: tuple[type, ...] = (int, float, Decimal)
_NUMPY_BOOL_TYPES: tuple[type, ...] = ()
_NUMPY_FLOAT_TYPES: tuple[type, ...] = ()

if not isinstance(np, MissingOptionalDependency):
    _NUM_DATA_TYPES = _NUM_DATA_TYPES + (
        np.int8,
        np.int16,
        np.int32,
        np.int64,
        np.float16,
        np.float32,
        np.float64,
        np.uint8,
        np.uint16,
        np.uint32,
        np.uint64,
        np.bool_,
    )
    _NUMPY_BOOL_TYPES = (np.bool_,)
    _NUMPY_FLOAT_TYPES = (np.float16, np.float32, np.float64)


def _is_numeric(value: Any) -> bool:
    """Check if value is a numeric type."""
    return isinstance(value, _NUM_DATA_TYPES)


def _is_binary(value: Any) -> bool:
    """Check if value is a binary type."""
    return isinstance(value, (bytes, bytearray))


# Epoch constants (timezone-independent)
_ZERO_EPOCH_DATE = date(1970, 1, 1)
_ZERO_EPOCH = datetime.fromtimestamp(0, timezone.utc).replace(tzinfo=None)


class JsonBindingConverter:
    """Converts Python parameters to Snowflake binding JSON format for server-side binding."""

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
        snowflake_type = PYTHON_TO_SNOWFLAKE_TYPE.get(type_name)
        if snowflake_type is None:
            if isinstance(value, _NUMPY_BOOL_TYPES):
                snowflake_type = "BOOLEAN"
            elif isinstance(value, _NUMPY_FLOAT_TYPES):
                snowflake_type = "REAL"
            elif _is_numeric(value):
                snowflake_type = "FIXED"
            elif _is_binary(value):
                snowflake_type = "BINARY"
            else:
                snowflake_type = "TEXT"

        # Convert value to string representation for JSON.
        # Order matters: bool/numpy.bool_ before _is_numeric (bool is subclass
        # of int and numpy.bool_ is in _NUM_DATA_TYPES),
        # datetime before date (datetime is subclass of date).
        if isinstance(value, bool) or isinstance(value, _NUMPY_BOOL_TYPES):
            converted = str(value).lower()
        elif isinstance(value, datetime):
            converted = cls._convert_datetime_to_epoch_nanoseconds(value)
        elif isinstance(value, date):
            converted = cls._convert_date_to_epoch_milliseconds(value)
        elif isinstance(value, time):
            converted = cls._convert_time_to_nanoseconds(value)
        elif isinstance(value, timedelta):
            converted = cls._convert_timedelta_to_nanoseconds(value)
        elif isinstance(value, time_module.struct_time):
            dt = datetime.fromtimestamp(time_module.mktime(value))
            converted = cls._convert_datetime_to_epoch_nanoseconds(dt)
        elif _is_numeric(value):
            converted = str(value)
        elif isinstance(value, str):
            converted = value
        elif _is_binary(value):
            converted = binascii.hexlify(value).decode("utf-8")
        else:
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


class ClientSideBindingConverter:
    """Converts Python values for client-side SQL interpolation (pyformat/format styles).

    This class mirrors the reference snowflake-connector-python's SnowflakeConverter
    for client-side binding, implementing escape(), quote(), and to_snowflake() methods.
    """

    @staticmethod
    def escape(value: Any) -> Any:
        """Escape special characters in string values for SQL interpolation.

        Mirrors reference connector's SnowflakeConverter.escape() method.
        """
        if isinstance(value, list):
            return value
        if value is None or _is_numeric(value) or _is_binary(value):
            return value
        res = value
        res = res.replace("\\", "\\\\")
        res = res.replace("\n", "\\n")
        res = res.replace("\r", "\\r")
        res = res.replace("\047", "\134\047")  # single quotes: ' -> \'
        return res

    @classmethod
    def quote(cls, value: Any) -> str:
        """Quote a value for SQL interpolation.

        Mirrors reference connector's SnowflakeConverter.quote() method.
        """
        if isinstance(value, list):
            # Quote each item in the list and join with commas
            # Apply escape() to each item since to_snowflake() already converted them
            return ",".join(cls.quote(cls.escape(item)) for item in value)
        if value is None:
            return "NULL"
        elif isinstance(value, bool):
            return "TRUE" if value else "FALSE"
        elif _is_numeric(value):
            return str(value)
        elif _is_binary(value):
            # Binary literal syntax: X'hex_value'
            return "X'{}'".format(binascii.hexlify(value).decode("ascii"))
        return f"'{value}'"

    @classmethod
    def to_snowflake(cls, value: Any) -> Any:
        """Convert Python value to Snowflake-compatible format for client-side binding.

        Mirrors reference connector's SnowflakeConverter.to_snowflake() method.
        """
        if value is None:
            return None
        elif isinstance(value, bool):
            return value
        elif _is_numeric(value):
            return value
        elif isinstance(value, str):
            return value
        elif _is_binary(value):
            return value
        elif isinstance(value, datetime):
            return cls._datetime_to_snowflake(value)
        elif isinstance(value, date):
            return cls._date_to_snowflake(value)
        elif isinstance(value, time):
            return cls._time_to_snowflake(value)
        elif isinstance(value, timedelta):
            return cls._timedelta_to_snowflake(value)
        elif isinstance(value, time_module.struct_time):
            return cls._struct_time_to_snowflake(value)
        elif isinstance(value, list):
            # List for IN clause - convert each element
            return [cls.to_snowflake(v) for v in value]
        else:
            # For other types, convert to string
            return str(value)

    @staticmethod
    def _datetime_to_snowflake(value: datetime) -> str:
        """Convert datetime to Snowflake string format."""
        tzinfo_value = value.tzinfo
        if tzinfo_value:
            # Get UTC offset
            td = tzinfo_value.utcoffset(value)
            if td is None:
                td = timedelta(0)
            sign = "+" if td >= timedelta(0) else "-"
            td_secs = int(td.total_seconds())
            h, m = divmod(abs(td_secs // 60), 60)
            if value.microsecond:
                return (
                    f"{value.year:d}-{value.month:02d}-{value.day:02d} "
                    f"{value.hour:02d}:{value.minute:02d}:{value.second:02d}."
                    f"{value.microsecond:06d}{sign}{h:02d}:{m:02d}"
                )
            return (
                f"{value.year:d}-{value.month:02d}-{value.day:02d} "
                f"{value.hour:02d}:{value.minute:02d}:{value.second:02d}"
                f"{sign}{h:02d}:{m:02d}"
            )
        else:
            if value.microsecond:
                return (
                    f"{value.year:d}-{value.month:02d}-{value.day:02d} "
                    f"{value.hour:02d}:{value.minute:02d}:{value.second:02d}."
                    f"{value.microsecond:06d}"
                )
            return (
                f"{value.year:d}-{value.month:02d}-{value.day:02d} "
                f"{value.hour:02d}:{value.minute:02d}:{value.second:02d}"
            )

    @staticmethod
    def _date_to_snowflake(value: date) -> str:
        """Convert date to Snowflake string format."""
        return f"{value.year:d}-{value.month:02d}-{value.day:02d}"

    @staticmethod
    def _time_to_snowflake(value: time) -> str:
        """Convert time to Snowflake string format."""
        if value.microsecond:
            return f"{value.hour:02d}:{value.minute:02d}:{value.second:02d}.{value.microsecond:06d}"
        return f"{value.hour:02d}:{value.minute:02d}:{value.second:02d}"

    @staticmethod
    def _timedelta_to_snowflake(value: timedelta) -> str:
        """Convert timedelta to Snowflake string format."""
        hours, remainder = divmod(value.seconds, 3600)
        mins, secs = divmod(remainder, 60)
        hours += value.days * 24
        if value.microseconds:
            return f"{hours:02d}:{mins:02d}:{secs:02d}.{value.microseconds:06d}"
        return f"{hours:02d}:{mins:02d}:{secs:02d}"

    @staticmethod
    def _struct_time_to_snowflake(value: time_module.struct_time) -> str:
        """Convert struct_time to Snowflake string format."""
        return (
            f"{value.tm_year:d}-{value.tm_mon:02d}-{value.tm_mday:02d} "
            f"{value.tm_hour:02d}:{value.tm_min:02d}:{value.tm_sec:02d}"
        )

    @classmethod
    def process_single_param(cls, param: Any) -> Any:
        """Process a single parameter for client-side binding.

        Applies to_snowflake -> escape -> quote transformation.
        """
        return cls.quote(cls.escape(cls.to_snowflake(param)))

    @classmethod
    def process_params_pyformat(
        cls, params: Sequence[Any] | Mapping[str, Any] | None
    ) -> dict[str, Any] | tuple[Any, ...]:
        """Process parameters for pyformat/format style binding.

        Args:
            params: Parameters as sequence (for %s) or mapping (for %(name)s)

        Returns:
            Processed parameters ready for % string interpolation
        """
        if params is None:
            return ()

        if isinstance(params, Mapping):
            # Named parameters: %(name)s
            return {key: cls.process_single_param(value) for key, value in params.items()}
        else:
            # Positional parameters: %s
            return tuple(cls.process_single_param(param) for param in params)

    @classmethod
    def interpolate_query(cls, query: str, params: Sequence[Any] | Mapping[str, Any] | None) -> str:
        """Interpolate parameters into query using Python % formatting.

        This is the main entry point for client-side binding, mirroring
        the reference connector's _preprocess_pyformat_query method.

        Args:
            query: SQL query with %s or %(name)s placeholders
            params: Parameters to interpolate

        Returns:
            Query string with parameters interpolated
        """
        if params is None or (not isinstance(params, Mapping) and len(params) == 0):
            return query

        processed_params = cls.process_params_pyformat(params)

        if processed_params:
            return query % processed_params
        return query
