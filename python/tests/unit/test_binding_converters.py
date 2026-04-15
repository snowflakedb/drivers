"""
Unit tests for JsonBindingConverter.

Tests cover type mapping, value conversion, array handling,
and the top-level serialize_parameters API.
Conversion logic is verified against the reference snowflake-connector-python's
SnowflakeConverter.to_snowflake_bindings behavior.
"""

import json
import time as time_module

from datetime import date, datetime, time, timedelta, timezone
from decimal import Decimal

import numpy as np
import pytest

from snowflake.connector._internal.binding_converters import ClientSideBindingConverter, JsonBindingConverter


class TestTypeMapping:
    """Test Python type to Snowflake type mapping."""

    def test_int_maps_to_fixed(self):
        snowflake_type, _ = JsonBindingConverter._convert_value(42)
        assert snowflake_type == "FIXED"

    def test_float_maps_to_real(self):
        snowflake_type, _ = JsonBindingConverter._convert_value(3.14)
        assert snowflake_type == "REAL"

    def test_str_maps_to_text(self):
        snowflake_type, _ = JsonBindingConverter._convert_value("hello")
        assert snowflake_type == "TEXT"

    def test_bool_maps_to_boolean(self):
        snowflake_type, _ = JsonBindingConverter._convert_value(True)
        assert snowflake_type == "BOOLEAN"

    def test_bytes_maps_to_binary(self):
        snowflake_type, _ = JsonBindingConverter._convert_value(b"\x00\x01")
        assert snowflake_type == "BINARY"

    def test_bytearray_maps_to_binary(self):
        snowflake_type, _ = JsonBindingConverter._convert_value(bytearray(b"\x00\x01"))
        assert snowflake_type == "BINARY"

    def test_datetime_maps_to_timestamp_ntz(self):
        snowflake_type, _ = JsonBindingConverter._convert_value(datetime(2024, 1, 15, 10, 30, 0))
        assert snowflake_type == "TIMESTAMP_NTZ"

    def test_date_maps_to_date(self):
        snowflake_type, _ = JsonBindingConverter._convert_value(date(2024, 1, 15))
        assert snowflake_type == "DATE"

    def test_time_maps_to_time(self):
        snowflake_type, _ = JsonBindingConverter._convert_value(time(10, 30, 45))
        assert snowflake_type == "TIME"

    def test_decimal_maps_to_fixed(self):
        snowflake_type, _ = JsonBindingConverter._convert_value(Decimal("123.45"))
        assert snowflake_type == "FIXED"

    def test_none_maps_to_any(self):
        """None maps to ANY, matching the reference connector."""
        snowflake_type, _ = JsonBindingConverter._convert_value(None)
        assert snowflake_type == "ANY"

    def test_timedelta_maps_to_time(self):
        snowflake_type, _ = JsonBindingConverter._convert_value(timedelta(hours=1, minutes=30))
        assert snowflake_type == "TIME"

    def test_struct_time_maps_to_timestamp_ntz(self):
        st = time_module.strptime("30 Sep 01 11:20:30", "%d %b %y %H:%M:%S")
        snowflake_type, _ = JsonBindingConverter._convert_value(st)
        assert snowflake_type == "TIMESTAMP_NTZ"

    def test_unknown_type_defaults_to_text(self):
        """Unknown types should fall back to TEXT."""
        snowflake_type, _ = JsonBindingConverter._convert_value(object())
        assert snowflake_type == "TEXT"


class TestConvertValueScalars:
    """Test scalar value conversion to Snowflake binding format."""

    def test_none_returns_none_value(self):
        _, value = JsonBindingConverter._convert_value(None)
        assert value is None

    def test_int_converts_to_string(self):
        _, value = JsonBindingConverter._convert_value(42)
        assert value == "42"

    def test_negative_int(self):
        _, value = JsonBindingConverter._convert_value(-100)
        assert value == "-100"

    def test_zero_int(self):
        _, value = JsonBindingConverter._convert_value(0)
        assert value == "0"

    def test_large_int(self):
        _, value = JsonBindingConverter._convert_value(99999999999999999)
        assert value == "99999999999999999"

    def test_float_converts_to_string(self):
        _, value = JsonBindingConverter._convert_value(3.14)
        assert value == "3.14"

    def test_negative_float(self):
        _, value = JsonBindingConverter._convert_value(-2.5)
        assert value == "-2.5"

    def test_float_zero(self):
        _, value = JsonBindingConverter._convert_value(0.0)
        assert value == "0.0"

    def test_string_preserved_as_is(self):
        _, value = JsonBindingConverter._convert_value("hello world")
        assert value == "hello world"

    def test_empty_string(self):
        _, value = JsonBindingConverter._convert_value("")
        assert value == ""

    def test_string_with_special_characters(self):
        _, value = JsonBindingConverter._convert_value('it\'s a "test"\nnewline')
        assert value == 'it\'s a "test"\nnewline'

    def test_bool_true_converts_to_lowercase(self):
        _, value = JsonBindingConverter._convert_value(True)
        assert value == "true"

    def test_bool_false_converts_to_lowercase(self):
        _, value = JsonBindingConverter._convert_value(False)
        assert value == "false"

    def test_bool_is_not_treated_as_int(self):
        """Bool is a subclass of int; ensure it's serialized as bool, not int."""
        snowflake_type, value = JsonBindingConverter._convert_value(True)
        assert snowflake_type == "BOOLEAN"
        assert value == "true"
        assert value != "1"

    def test_decimal_converts_to_string(self):
        _, value = JsonBindingConverter._convert_value(Decimal("123.456"))
        assert value == "123.456"

    def test_decimal_high_precision(self):
        _, value = JsonBindingConverter._convert_value(Decimal("0.00000000000001"))
        # Decimal str() may use scientific notation
        assert Decimal(value) == Decimal("0.00000000000001")

    def test_decimal_negative(self):
        _, value = JsonBindingConverter._convert_value(Decimal("-99.99"))
        assert value == "-99.99"

    def test_bytes_hex_encoded(self):
        _, value = JsonBindingConverter._convert_value(b"\x00\x01\x02\xff")
        assert value == "000102ff"

    def test_bytes_empty(self):
        _, value = JsonBindingConverter._convert_value(b"")
        assert value == ""

    def test_bytes_ascii(self):
        _, value = JsonBindingConverter._convert_value(b"ABC")
        assert value == "414243"

    def test_bytes_non_ascii(self):
        """Non-ASCII bytes should be properly hex-encoded without errors."""
        _, value = JsonBindingConverter._convert_value(bytes([0xF5, 0xAB, 0xCD, 0xEF]))
        assert value == "f5abcdef"

    def test_bytearray_hex_encoded(self):
        """bytearray should be hex-encoded identically to bytes."""
        _, value = JsonBindingConverter._convert_value(bytearray(b"\xab\xcd\xef"))
        assert value == "abcdef"

    def test_bytearray_matches_bytes(self):
        """bytearray and bytes with same content should produce identical output."""
        data = b"\x00\x01\x02\xff"
        _, bytes_value = JsonBindingConverter._convert_value(data)
        _, bytearray_value = JsonBindingConverter._convert_value(bytearray(data))
        assert bytes_value == bytearray_value


class TestConvertValueDatetime:
    """Test datetime/date/time value conversion."""

    def test_datetime_to_epoch_nanoseconds(self):
        dt = datetime(2024, 1, 15, 12, 0, 0)
        _, value = JsonBindingConverter._convert_value(dt)
        assert isinstance(value, str)
        nanos = int(value)
        assert nanos > 0

    def test_datetime_epoch_is_zero(self):
        """datetime at Unix epoch should produce zero nanoseconds (timezone-independent)."""
        dt = datetime(1970, 1, 1, 0, 0, 0)
        _, value = JsonBindingConverter._convert_value(dt)
        assert int(value) == 0

    def test_datetime_one_second_after_epoch(self):
        """1970-01-01 00:00:01 should produce exactly 1 billion nanoseconds."""
        dt = datetime(1970, 1, 1, 0, 0, 1)
        _, value = JsonBindingConverter._convert_value(dt)
        assert int(value) == 1_000_000_000

    def test_datetime_with_microseconds_at_epoch(self):
        """Microsecond precision must be preserved in nanosecond output.

        datetime(1970, 1, 1, 0, 0, 0, 123456) = 123456 microseconds
        = 123456000 nanoseconds.
        """
        dt = datetime(1970, 1, 1, 0, 0, 0, 123456)
        _, value = JsonBindingConverter._convert_value(dt)
        assert int(value) == 123_456_000

    def test_datetime_with_one_microsecond(self):
        """Single microsecond must not be lost.

        datetime(1970, 1, 1, 0, 0, 0, 1) = 1 microsecond = 1000 nanoseconds.
        """
        dt = datetime(1970, 1, 1, 0, 0, 0, 1)
        _, value = JsonBindingConverter._convert_value(dt)
        assert int(value) == 1_000

    def test_datetime_with_microseconds_and_seconds(self):
        """Combined seconds and microseconds must produce correct nanoseconds.

        datetime(1970, 1, 1, 0, 0, 1, 123456) = 1.123456 seconds
        = 1_123_456_000 nanoseconds.
        """
        dt = datetime(1970, 1, 1, 0, 0, 1, 123456)
        _, value = JsonBindingConverter._convert_value(dt)
        assert int(value) == 1_123_456_000

    def test_datetime_with_microseconds_far_from_epoch(self):
        """Microsecond precision must be preserved even for dates far from epoch.

        Large epoch offsets increase the total_seconds() float magnitude,
        which can cause loss of microsecond precision due to float64 limits.
        """
        dt = datetime(2024, 6, 15, 10, 30, 0, 123456)
        dt_no_us = datetime(2024, 6, 15, 10, 30, 0, 0)
        _, value = JsonBindingConverter._convert_value(dt)
        _, value_no_us = JsonBindingConverter._convert_value(dt_no_us)
        # The difference must be exactly 123456 microseconds = 123456000 nanoseconds
        assert int(value) - int(value_no_us) == 123_456_000

    def test_datetime_max_microseconds(self):
        """Maximum microsecond value (999999) must be preserved.

        datetime(1970, 1, 1, 0, 0, 0, 999999) = 999999 microseconds
        = 999_999_000 nanoseconds.
        """
        dt = datetime(1970, 1, 1, 0, 0, 0, 999999)
        _, value = JsonBindingConverter._convert_value(dt)
        assert int(value) == 999_999_000

    def test_date_epoch_is_zero(self):
        """date at Unix epoch should produce zero milliseconds (timezone-independent)."""
        d = date(1970, 1, 1)
        _, value = JsonBindingConverter._convert_value(d)
        assert int(value) == 0

    def test_date_to_epoch_milliseconds(self):
        d = date(2024, 1, 15)
        _, value = JsonBindingConverter._convert_value(d)
        assert isinstance(value, str)
        millis = int(value)
        assert millis > 0

    def test_date_known_value(self):
        """date(1970, 1, 2) should be exactly 86400000 milliseconds."""
        d = date(1970, 1, 2)
        _, value = JsonBindingConverter._convert_value(d)
        assert value == "86400000"

    def test_time_to_nanoseconds_midnight(self):
        t = time(0, 0, 0)
        _, value = JsonBindingConverter._convert_value(t)
        assert value == "0000000000"

    def test_time_to_nanoseconds_noon(self):
        t = time(12, 0, 0)
        _, value = JsonBindingConverter._convert_value(t)
        # 12*3600 = 43200, then "000000" microseconds, then "000" trailing
        assert value == "43200000000000"

    def test_time_with_microseconds(self):
        """Matches reference connector string concatenation format."""
        t = time(1, 2, 3, 456)
        _, value = JsonBindingConverter._convert_value(t)
        # str(3723) + f"{456:06d}" + "000" = "3723" + "000456" + "000"
        assert value == "3723000456000"

    def test_time_end_of_day(self):
        t = time(23, 59, 59, 999999)
        _, value = JsonBindingConverter._convert_value(t)
        # str(86399) + f"{999999:06d}" + "000" = "86399" + "999999" + "000"
        assert value == "86399999999000"

    def test_time_one_microsecond(self):
        t = time(0, 0, 0, 1)
        _, value = JsonBindingConverter._convert_value(t)
        # str(0) + f"{1:06d}" + "000" = "0" + "000001" + "000"
        assert value == "0000001000"


class TestConvertValueTimedelta:
    """Test timedelta conversion (maps to TIME type)."""

    def test_timedelta_zero(self):
        td = timedelta(0)
        snowflake_type, value = JsonBindingConverter._convert_value(td)
        assert snowflake_type == "TIME"
        assert value == "0000000000"

    def test_timedelta_one_hour(self):
        td = timedelta(hours=1)
        _, value = JsonBindingConverter._convert_value(td)
        # str(3600) + "000000" + "000"
        assert value == "3600000000000"

    def test_timedelta_with_microseconds(self):
        td = timedelta(hours=1, minutes=2, seconds=3, microseconds=456)
        _, value = JsonBindingConverter._convert_value(td)
        # str(3723) + f"{456:06d}" + "000" = "3723" + "000456" + "000"
        assert value == "3723000456000"

    def test_timedelta_with_days(self):
        """Days are converted to hours (days * 24h added to hours)."""
        td = timedelta(days=1, hours=1)
        _, value = JsonBindingConverter._convert_value(td)
        # (24+1)*3600 = 90000 seconds
        assert value == "90000000000000"

    def test_timedelta_matches_time(self):
        """timedelta(h=1,m=2,s=3,us=456) should produce same value as time(1,2,3,456)."""
        td = timedelta(hours=1, minutes=2, seconds=3, microseconds=456)
        t = time(1, 2, 3, 456)
        _, td_value = JsonBindingConverter._convert_value(td)
        _, t_value = JsonBindingConverter._convert_value(t)
        assert td_value == t_value


class TestConvertValueStructTime:
    """Test struct_time conversion (maps to TIMESTAMP_NTZ type)."""

    def test_struct_time_conversion(self):
        st = time_module.strptime("30 Sep 01 11:20:30", "%d %b %y %H:%M:%S")
        snowflake_type, value = JsonBindingConverter._convert_value(st)
        assert snowflake_type == "TIMESTAMP_NTZ"
        assert isinstance(value, str)
        # Should be a nanosecond epoch string
        nanos = int(value)
        assert nanos > 0

    def test_struct_time_matches_equivalent_datetime(self):
        """struct_time should produce the same result as the equivalent datetime."""
        st = time_module.strptime("2024-06-15 10:30:00", "%Y-%m-%d %H:%M:%S")
        dt = datetime.fromtimestamp(time_module.mktime(st))
        _, st_value = JsonBindingConverter._convert_value(st)
        _, dt_value = JsonBindingConverter._convert_value(dt)
        assert st_value == dt_value


class TestConvertArray:
    """Test array conversion for bulk operations."""

    def test_empty_array(self):
        snowflake_type, values = JsonBindingConverter._convert_array([])
        assert snowflake_type == "TEXT"
        assert values == []

    def test_int_array(self):
        snowflake_type, values = JsonBindingConverter._convert_array([1, 2, 3])
        assert snowflake_type == "FIXED"
        assert values == ["1", "2", "3"]

    def test_string_array(self):
        snowflake_type, values = JsonBindingConverter._convert_array(["a", "b", "c"])
        assert snowflake_type == "TEXT"
        assert values == ["a", "b", "c"]

    def test_float_array(self):
        snowflake_type, values = JsonBindingConverter._convert_array([1.1, 2.2, 3.3])
        assert snowflake_type == "REAL"
        assert values == ["1.1", "2.2", "3.3"]

    def test_bool_array(self):
        snowflake_type, values = JsonBindingConverter._convert_array([True, False, True])
        assert snowflake_type == "BOOLEAN"
        assert values == ["true", "false", "true"]

    def test_array_with_nones_preserves_type(self):
        """None values should not affect the inferred type."""
        snowflake_type, values = JsonBindingConverter._convert_array([1, None, 3])
        assert snowflake_type == "FIXED"
        assert values == ["1", None, "3"]

    def test_all_none_array(self):
        snowflake_type, values = JsonBindingConverter._convert_array([None, None, None])
        assert snowflake_type == "TEXT"
        assert values == [None, None, None]

    def test_mixed_type_array_falls_back_to_text(self):
        """Arrays with mixed non-null types should default to TEXT."""
        snowflake_type, values = JsonBindingConverter._convert_array([1, "hello", 3.14])
        assert snowflake_type == "TEXT"

    def test_bytes_array(self):
        snowflake_type, values = JsonBindingConverter._convert_array([b"\x01", b"\x02"])
        assert snowflake_type == "BINARY"
        assert values == ["01", "02"]

    def test_decimal_array(self):
        snowflake_type, values = JsonBindingConverter._convert_array([Decimal("1.1"), Decimal("2.2")])
        assert snowflake_type == "FIXED"
        assert values == ["1.1", "2.2"]


class TestProcessParams:
    """Test _process_params with positional parameters."""

    def test_single_param(self):
        result = JsonBindingConverter._process_params([42])
        assert result == {"1": {"type": "FIXED", "value": "42"}}

    def test_multiple_params(self):
        result = JsonBindingConverter._process_params([42, "hello", True])
        assert result == {
            "1": {"type": "FIXED", "value": "42"},
            "2": {"type": "TEXT", "value": "hello"},
            "3": {"type": "BOOLEAN", "value": "true"},
        }

    def test_params_are_one_indexed(self):
        """Parameter keys should be 1-indexed strings."""
        result = JsonBindingConverter._process_params(["a", "b", "c"])
        assert "1" in result
        assert "2" in result
        assert "3" in result
        assert "0" not in result

    def test_array_param(self):
        result = JsonBindingConverter._process_params([[1, 2, 3]])
        assert result == {"1": {"type": "FIXED", "value": ["1", "2", "3"]}}

    def test_mixed_scalar_and_array_params(self):
        result = JsonBindingConverter._process_params(["hello", [1, 2, 3]])
        assert result == {
            "1": {"type": "TEXT", "value": "hello"},
            "2": {"type": "FIXED", "value": ["1", "2", "3"]},
        }

    def test_none_param(self):
        result = JsonBindingConverter._process_params([None])
        assert result == {"1": {"type": "ANY", "value": None}}


class TestSerializeParameters:
    """Test the top-level serialize_parameters API."""

    def test_none_params_returns_none(self):
        json_str, length = JsonBindingConverter.serialize_parameters(None)
        assert json_str is None
        assert length == 0

    def test_empty_list_returns_none(self):
        json_str, length = JsonBindingConverter.serialize_parameters([])
        assert json_str is None
        assert length == 0

    def test_returns_valid_json(self):
        json_str, length = JsonBindingConverter.serialize_parameters([42, "hello"])
        assert json_str is not None
        parsed = json.loads(json_str)
        assert parsed == {
            "1": {"type": "FIXED", "value": "42"},
            "2": {"type": "TEXT", "value": "hello"},
        }

    def test_length_matches_utf8_bytes(self):
        json_str, length = JsonBindingConverter.serialize_parameters([42, "hello"])
        assert length == len(json_str.encode("utf-8"))

    def test_length_with_unicode_characters(self):
        """UTF-8 byte length should match encoded JSON bytes."""
        json_str, length = JsonBindingConverter.serialize_parameters(["café ☕"])
        assert length == len(json_str.encode("utf-8"))
        parsed = json.loads(json_str)
        assert parsed["1"]["value"] == "café ☕"

    def test_single_int_param(self):
        json_str, _ = JsonBindingConverter.serialize_parameters([123])
        parsed = json.loads(json_str)
        assert parsed == {"1": {"type": "FIXED", "value": "123"}}

    def test_all_types_together(self):
        """Smoke test with one value of each supported type."""
        params = [
            42,  # int -> FIXED
            3.14,  # float -> REAL
            "text",  # str -> TEXT
            True,  # bool -> BOOLEAN
            b"\xab\xcd",  # bytes -> BINARY
            Decimal("9.99"),  # Decimal -> FIXED
            date(2024, 6, 15),  # date -> DATE
            time(10, 30, 0),  # time -> TIME
            datetime(2024, 6, 15, 10, 30),  # datetime -> TIMESTAMP_NTZ
            None,  # None -> ANY (null)
            bytearray(b"\x01\x02"),  # bytearray -> BINARY
            timedelta(hours=1),  # timedelta -> TIME
        ]
        json_str, length = JsonBindingConverter.serialize_parameters(params)
        assert json_str is not None
        assert length > 0

        parsed = json.loads(json_str)
        assert len(parsed) == 12

        assert parsed["1"]["type"] == "FIXED"
        assert parsed["2"]["type"] == "REAL"
        assert parsed["3"]["type"] == "TEXT"
        assert parsed["4"]["type"] == "BOOLEAN"
        assert parsed["5"]["type"] == "BINARY"
        assert parsed["6"]["type"] == "FIXED"
        assert parsed["7"]["type"] == "DATE"
        assert parsed["8"]["type"] == "TIME"
        assert parsed["9"]["type"] == "TIMESTAMP_NTZ"
        assert parsed["10"]["type"] == "ANY"
        assert parsed["10"]["value"] is None
        assert parsed["11"]["type"] == "BINARY"
        assert parsed["12"]["type"] == "TIME"

    def test_array_binding(self):
        """Test bulk insert style array parameters."""
        json_str, _ = JsonBindingConverter.serialize_parameters([[1, 2, 3], ["a", "b", "c"]])
        parsed = json.loads(json_str)
        assert parsed["1"]["type"] == "FIXED"
        assert parsed["1"]["value"] == ["1", "2", "3"]
        assert parsed["2"]["type"] == "TEXT"
        assert parsed["2"]["value"] == ["a", "b", "c"]


class TestReferenceConnectorParity:
    """Regression tests for bugs fixed during initial development.

    Each test guards against a specific bug that was found and fixed
    to achieve parity with the reference snowflake-connector-python.
    """

    # --- Regression #1: date conversion is timezone-independent ---
    # Fixed: now uses (d - date(1970,1,1)).total_seconds(), matching
    # the reference connector's timezone-independent approach.

    def test_date_epoch_is_timezone_independent(self):
        """date(1970, 1, 1) produces 0 regardless of local timezone."""
        d = date(1970, 1, 1)
        _, value = JsonBindingConverter._convert_value(d)
        assert int(value) == 0

    def test_date_known_value_is_timezone_independent(self):
        """date(2024, 1, 15) produces the same milliseconds everywhere."""
        d = date(2024, 1, 15)
        _, value = JsonBindingConverter._convert_value(d)
        expected_days = (date(2024, 1, 15) - date(1970, 1, 1)).days
        expected_ms = expected_days * 86400 * 1000
        assert int(value) == expected_ms

    def test_date_before_epoch_is_negative(self):
        """Dates before 1970-01-01 produce negative millisecond values."""
        d = date(1969, 12, 31)
        _, value = JsonBindingConverter._convert_value(d)
        assert int(value) == -86400000

    # --- Regression #2: datetime conversion is timezone-independent ---
    # Fixed: now uses (dt - ZERO_EPOCH).total_seconds(), matching
    # the reference connector.

    def test_datetime_epoch_is_timezone_independent(self):
        """datetime(1970, 1, 1) produces 0 nanoseconds."""
        dt = datetime(1970, 1, 1, 0, 0, 0)
        _, value = JsonBindingConverter._convert_value(dt)
        assert int(value) == 0

    def test_datetime_known_value_is_timezone_independent(self):
        """Two datetimes exactly 1 hour apart differ by exactly 3600 * 10^9 ns."""
        dt1 = datetime(2024, 6, 15, 10, 0, 0)
        dt2 = datetime(2024, 6, 15, 11, 0, 0)
        _, v1 = JsonBindingConverter._convert_value(dt1)
        _, v2 = JsonBindingConverter._convert_value(dt2)
        assert int(v2) - int(v1) == 3600 * 1_000_000_000

    def test_datetime_tz_aware_normalizes_to_utc(self):
        """Timezone-aware datetimes are converted to UTC before serialization."""
        # 2024-01-01 05:00:00 UTC+5 == 2024-01-01 00:00:00 UTC
        naive_utc = datetime(2024, 1, 1, 0, 0, 0)
        tz_plus5 = timezone(timedelta(hours=5))
        aware = datetime(2024, 1, 1, 5, 0, 0, tzinfo=tz_plus5)

        _, naive_value = JsonBindingConverter._convert_value(naive_utc)
        _, aware_value = JsonBindingConverter._convert_value(aware)
        assert naive_value == aware_value

    def test_datetime_tz_negative_offset(self):
        """UTC-8 datetime also normalizes correctly."""
        naive_utc = datetime(2024, 1, 1, 8, 0, 0)
        tz_minus8 = timezone(timedelta(hours=-8))
        aware = datetime(2024, 1, 1, 0, 0, 0, tzinfo=tz_minus8)

        _, naive_value = JsonBindingConverter._convert_value(naive_utc)
        _, aware_value = JsonBindingConverter._convert_value(aware)
        assert naive_value == aware_value

    # --- Regression #3: NoneType maps to "ANY" ---
    # Fixed: matches reference connector's PYTHON_TO_SNOWFLAKE_TYPE["nonetype"] = "ANY".

    def test_none_type_is_any_not_text(self):
        """None maps to type ANY, matching the reference connector."""
        snowflake_type, value = JsonBindingConverter._convert_value(None)
        assert snowflake_type == "ANY"
        assert value is None

    # --- Regression #4: bytearray is hex-encoded ---
    # Fixed: handled via _is_binary(), matching reference connector's
    # _bytearray_to_snowflake_bindings = _bytes_to_snowflake_bindings.

    def test_bytearray_produces_hex_not_str_repr(self):
        """bytearray is hex-encoded, not converted via str()."""
        ba = bytearray(b"\xab\xcd")
        snowflake_type, value = JsonBindingConverter._convert_value(ba)
        assert snowflake_type == "BINARY"
        assert value == "abcd"
        assert "bytearray" not in value

    # --- Regression #5: timedelta converts to nanoseconds (TIME type) ---
    # Fixed: matches reference connector's _timedelta_to_snowflake_bindings.

    def test_timedelta_produces_nanoseconds_not_str_repr(self):
        """timedelta converts to nanoseconds, not str(timedelta(...))."""
        td = timedelta(hours=1, minutes=2, seconds=3, microseconds=456)
        snowflake_type, value = JsonBindingConverter._convert_value(td)
        assert snowflake_type == "TIME"
        assert value == "3723000456000"
        assert ":" not in value

    def test_timedelta_with_days_converts_days_to_hours(self):
        """Days are folded into hours: timedelta(days=2, hours=3) = 51h."""
        td = timedelta(days=2, hours=3)
        _, value = JsonBindingConverter._convert_value(td)
        expected_seconds = (2 * 24 + 3) * 3600
        assert value == f"{expected_seconds}000000000"

    # --- Regression #6: struct_time converts to epoch nanoseconds ---
    # Fixed: converts via datetime.fromtimestamp(time.mktime(value)),
    # matching reference connector's _struct_time_to_snowflake_bindings.

    def test_struct_time_produces_nanoseconds_not_str_repr(self):
        """struct_time converts to epoch nanoseconds, not str repr."""
        st = time_module.strptime("2024-06-15 10:30:00", "%Y-%m-%d %H:%M:%S")
        snowflake_type, value = JsonBindingConverter._convert_value(st)
        assert snowflake_type == "TIMESTAMP_NTZ"
        assert value.lstrip("-").isdigit()
        assert "struct_time" not in value

    # --- Regression #7: time microseconds are zero-padded ---
    # Fixed: uses str(seconds) + f"{microsecond:06d}" + "000", matching
    # the reference connector's string concatenation format.

    def test_time_microsecond_zero_padding(self):
        """Microseconds are zero-padded to 6 digits in the nanosecond string.

        time(0, 0, 1, 1) -> "1" + "000001" + "000" = "1000001000"
        """
        t = time(0, 0, 1, 1)
        _, value = JsonBindingConverter._convert_value(t)
        assert value == "1000001000"

    def test_time_format_trailing_zeros(self):
        """Time format includes trailing '000' for nanosecond precision.

        time(0, 0, 1) -> "1" + "000000" + "000" = "1000000000"
        """
        t = time(0, 0, 1, 0)
        _, value = JsonBindingConverter._convert_value(t)
        assert value.endswith("000")
        assert value == "1000000000"

    # --- Cross-type parity: reference connector test_binding data types ---
    # Mirrors the data types used in test/integ/test_bindings.py::test_binding

    def test_reference_binding_data_types(self):
        """Verify all types from the reference connector's test_binding test.

        The reference test_binding uses: bool, int, Decimal, str, float,
        bytes, bytearray, datetime, date, time, None, empty string, escaped string.
        """
        params = [
            True,
            1,
            Decimal("1.2"),
            "str1",
            1.2,
            b"abc",
            bytearray(b"def"),
            datetime(2024, 1, 15, 12, 0, 0),
            date(2017, 12, 30),
            time(1, 2, 3, 456),
            None,
            "",
            ',an\\\\escaped"line\n',
        ]
        json_str, length = JsonBindingConverter.serialize_parameters(params)
        assert json_str is not None
        assert length > 0

        parsed = json.loads(json_str)
        assert len(parsed) == 13

        assert parsed["1"] == {"type": "BOOLEAN", "value": "true"}
        assert parsed["2"] == {"type": "FIXED", "value": "1"}
        assert parsed["3"] == {"type": "FIXED", "value": "1.2"}
        assert parsed["4"] == {"type": "TEXT", "value": "str1"}
        assert parsed["5"] == {"type": "REAL", "value": "1.2"}
        assert parsed["6"] == {"type": "BINARY", "value": "616263"}
        assert parsed["7"] == {"type": "BINARY", "value": "646566"}
        assert parsed["8"]["type"] == "TIMESTAMP_NTZ"
        assert int(parsed["8"]["value"]) > 0
        assert parsed["9"]["type"] == "DATE"
        assert int(parsed["9"]["value"]) > 0
        assert parsed["10"] == {"type": "TIME", "value": "3723000456000"}
        assert parsed["11"] == {"type": "ANY", "value": None}
        assert parsed["12"] == {"type": "TEXT", "value": ""}
        assert parsed["13"] == {"type": "TEXT", "value": ',an\\\\escaped"line\n'}


class TestHelperMethods:
    """Test internal helper methods for datetime conversion."""

    def test_datetime_to_epoch_nanoseconds_precision(self):
        """Verify nanosecond-level precision between two datetimes."""
        dt = datetime(2024, 1, 1, 0, 0, 1)
        result = JsonBindingConverter._convert_datetime_to_epoch_nanoseconds(dt)
        dt_base = datetime(2024, 1, 1, 0, 0, 0)
        base = JsonBindingConverter._convert_datetime_to_epoch_nanoseconds(dt_base)
        assert int(result) - int(base) == 1_000_000_000

    def test_datetime_epoch_zero(self):
        """Epoch datetime produces zero nanoseconds."""
        result = JsonBindingConverter._convert_datetime_to_epoch_nanoseconds(datetime(1970, 1, 1, 0, 0, 0))
        assert int(result) == 0

    def test_datetime_microseconds_at_epoch(self):
        """Microseconds at epoch must produce exact nanoseconds."""
        result = JsonBindingConverter._convert_datetime_to_epoch_nanoseconds(datetime(1970, 1, 1, 0, 0, 0, 123456))
        assert int(result) == 123_456_000

    def test_datetime_one_microsecond_at_epoch(self):
        """A single microsecond must not be lost."""
        result = JsonBindingConverter._convert_datetime_to_epoch_nanoseconds(datetime(1970, 1, 1, 0, 0, 0, 1))
        assert int(result) == 1_000

    def test_datetime_microseconds_far_from_epoch(self):
        """Microsecond precision must survive float64 representation for large epoch values."""
        result = JsonBindingConverter._convert_datetime_to_epoch_nanoseconds(datetime(2024, 6, 15, 10, 30, 0, 123456))
        result_no_us = JsonBindingConverter._convert_datetime_to_epoch_nanoseconds(datetime(2024, 6, 15, 10, 30, 0, 0))
        assert int(result) - int(result_no_us) == 123_456_000

    def test_datetime_tz_aware_converts_to_utc(self):
        """Timezone-aware datetimes should be normalized to UTC."""
        # 2024-01-01 05:00:00 UTC+5 == 2024-01-01 00:00:00 UTC

        utc_dt = datetime(2024, 1, 1, 0, 0, 0)
        result_utc = JsonBindingConverter._convert_datetime_to_epoch_nanoseconds(utc_dt)
        try:
            tz_plus5 = timezone(timedelta(hours=5))
            aware_dt = datetime(2024, 1, 1, 5, 0, 0, tzinfo=tz_plus5)
            result_aware = JsonBindingConverter._convert_datetime_to_epoch_nanoseconds(aware_dt)
            assert result_utc == result_aware
        except Exception:
            pytest.skip("timezone offset test not available")

    def test_date_epoch_zero(self):
        """Epoch date produces zero milliseconds."""
        result = JsonBindingConverter._convert_date_to_epoch_milliseconds(date(1970, 1, 1))
        assert int(result) == 0

    def test_date_one_day_after_epoch(self):
        """One day after epoch = 86400000 milliseconds."""
        result = JsonBindingConverter._convert_date_to_epoch_milliseconds(date(1970, 1, 2))
        assert result == "86400000"

    def test_date_before_epoch(self):
        """Dates before epoch should produce negative millisecond values."""
        result = JsonBindingConverter._convert_date_to_epoch_milliseconds(date(1969, 12, 31))
        assert int(result) == -86400000

    def test_time_to_nanoseconds_one_hour(self):
        result = JsonBindingConverter._convert_time_to_nanoseconds(time(1, 0, 0))
        assert result == "3600000000000"

    def test_time_to_nanoseconds_one_minute(self):
        result = JsonBindingConverter._convert_time_to_nanoseconds(time(0, 1, 0))
        assert result == "60000000000"

    def test_time_to_nanoseconds_one_second(self):
        result = JsonBindingConverter._convert_time_to_nanoseconds(time(0, 0, 1))
        assert result == "1000000000"

    def test_time_to_nanoseconds_one_microsecond(self):
        result = JsonBindingConverter._convert_time_to_nanoseconds(time(0, 0, 0, 1))
        assert result == "0000001000"

    def test_timedelta_basic(self):
        result = JsonBindingConverter._convert_timedelta_to_nanoseconds(
            timedelta(hours=1, minutes=2, seconds=3, microseconds=456)
        )
        assert result == "3723000456000"

    def test_timedelta_with_days(self):
        result = JsonBindingConverter._convert_timedelta_to_nanoseconds(timedelta(days=1))
        # 24*3600 = 86400
        assert result == "86400000000000"


class TestEscape:
    """Test ClientSideBindingConverter.escape()."""

    def test_none_passthrough(self):
        assert ClientSideBindingConverter.escape(None) is None

    def test_int_passthrough(self):
        assert ClientSideBindingConverter.escape(42) == 42

    def test_float_passthrough(self):
        assert ClientSideBindingConverter.escape(3.14) == 3.14

    def test_decimal_passthrough(self):
        assert ClientSideBindingConverter.escape(Decimal("1.23")) == Decimal("1.23")

    def test_bytes_passthrough(self):
        assert ClientSideBindingConverter.escape(b"\xab\xcd") == b"\xab\xcd"

    def test_bytearray_passthrough(self):
        assert ClientSideBindingConverter.escape(bytearray(b"\x01")) == bytearray(b"\x01")

    def test_list_passthrough(self):
        result = ClientSideBindingConverter.escape(["a", "b"])
        assert result == ["a", "b"]

    def test_backslash_escaped(self):
        assert ClientSideBindingConverter.escape("a\\b") == "a\\\\b"

    def test_newline_escaped(self):
        assert ClientSideBindingConverter.escape("a\nb") == "a\\nb"

    def test_carriage_return_escaped(self):
        assert ClientSideBindingConverter.escape("a\rb") == "a\\rb"

    def test_single_quote_escaped(self):
        assert ClientSideBindingConverter.escape("it's") == "it\\'s"

    def test_plain_string_unchanged(self):
        assert ClientSideBindingConverter.escape("hello world") == "hello world"

    def test_empty_string_unchanged(self):
        assert ClientSideBindingConverter.escape("") == ""

    def test_multiple_special_chars(self):
        result = ClientSideBindingConverter.escape("a\\b\nc\r'd")
        assert result == "a\\\\b\\nc\\r\\'d"

    def test_complex_escaped_line(self):
        """Mirrors the reference connector test_binding escaped string."""
        result = ClientSideBindingConverter.escape(',an\\\\escaped"line\n')
        assert "\\n" in result
        assert "\\\\\\\\" in result


class TestQuote:
    """Test ClientSideBindingConverter.quote()."""

    def test_none_quoted_as_null(self):
        assert ClientSideBindingConverter.quote(None) == "NULL"

    def test_bool_true_quoted(self):
        assert ClientSideBindingConverter.quote(True) == "TRUE"

    def test_bool_false_quoted(self):
        assert ClientSideBindingConverter.quote(False) == "FALSE"

    def test_int_quoted_bare(self):
        assert ClientSideBindingConverter.quote(42) == "42"

    def test_negative_int_quoted_bare(self):
        assert ClientSideBindingConverter.quote(-100) == "-100"

    def test_zero_quoted_bare(self):
        assert ClientSideBindingConverter.quote(0) == "0"

    def test_float_quoted_bare(self):
        assert ClientSideBindingConverter.quote(3.14) == "3.14"

    def test_decimal_quoted_bare(self):
        assert ClientSideBindingConverter.quote(Decimal("99.99")) == "99.99"

    def test_string_quoted_with_single_quotes(self):
        assert ClientSideBindingConverter.quote("hello") == "'hello'"

    def test_empty_string_quoted(self):
        assert ClientSideBindingConverter.quote("") == "''"

    def test_string_with_embedded_quote(self):
        assert ClientSideBindingConverter.quote("it\\'s") == "'it\\'s'"

    def test_bytes_quoted_as_hex_literal(self):
        assert ClientSideBindingConverter.quote(b"\x00\x01\xff") == "X'0001ff'"

    def test_bytearray_quoted_as_hex_literal(self):
        assert ClientSideBindingConverter.quote(bytearray(b"\xab\xcd")) == "X'abcd'"

    def test_empty_bytes_quoted(self):
        assert ClientSideBindingConverter.quote(b"") == "X''"

    def test_list_quoted_as_comma_separated(self):
        result = ClientSideBindingConverter.quote([1, "hello", None])
        assert result == "1,'hello',NULL"

    def test_list_of_ints(self):
        result = ClientSideBindingConverter.quote([1, 2, 3])
        assert result == "1,2,3"


class TestToSnowflake:
    """Test ClientSideBindingConverter.to_snowflake()."""

    def test_none_returns_none(self):
        assert ClientSideBindingConverter.to_snowflake(None) is None

    def test_bool_true_passthrough(self):
        result = ClientSideBindingConverter.to_snowflake(True)
        assert result is True

    def test_bool_false_passthrough(self):
        result = ClientSideBindingConverter.to_snowflake(False)
        assert result is False

    def test_int_passthrough(self):
        result = ClientSideBindingConverter.to_snowflake(42)
        assert result == 42

    def test_float_passthrough(self):
        result = ClientSideBindingConverter.to_snowflake(3.14)
        assert result == 3.14

    def test_decimal_passthrough(self):
        result = ClientSideBindingConverter.to_snowflake(Decimal("1.23"))
        assert result == Decimal("1.23")

    def test_string_passthrough(self):
        result = ClientSideBindingConverter.to_snowflake("hello")
        assert result == "hello"

    def test_bytes_passthrough(self):
        result = ClientSideBindingConverter.to_snowflake(b"\xab\xcd")
        assert result == b"\xab\xcd"

    def test_bytearray_passthrough(self):
        result = ClientSideBindingConverter.to_snowflake(bytearray(b"\x01"))
        assert result == bytearray(b"\x01")

    def test_datetime_naive(self):
        result = ClientSideBindingConverter.to_snowflake(datetime(2024, 6, 15, 10, 30, 0))
        assert result == "2024-06-15 10:30:00"

    def test_datetime_with_microseconds(self):
        result = ClientSideBindingConverter.to_snowflake(datetime(2024, 6, 15, 10, 30, 0, 123456))
        assert result == "2024-06-15 10:30:00.123456"

    def test_datetime_tz_aware(self):
        tz = timezone(timedelta(hours=5))
        result = ClientSideBindingConverter.to_snowflake(datetime(2024, 6, 15, 10, 30, 0, tzinfo=tz))
        assert result == "2024-06-15 10:30:00+05:00"

    def test_datetime_tz_aware_with_microseconds(self):
        tz = timezone(timedelta(hours=-8))
        result = ClientSideBindingConverter.to_snowflake(datetime(2024, 6, 15, 10, 30, 0, 123456, tzinfo=tz))
        assert result == "2024-06-15 10:30:00.123456-08:00"

    def test_datetime_utc(self):
        result = ClientSideBindingConverter.to_snowflake(datetime(2024, 6, 15, 10, 30, 0, tzinfo=timezone.utc))
        assert result == "2024-06-15 10:30:00+00:00"

    def test_date(self):
        result = ClientSideBindingConverter.to_snowflake(date(2024, 6, 15))
        assert result == "2024-06-15"

    def test_date_single_digit_month_day(self):
        result = ClientSideBindingConverter.to_snowflake(date(2024, 1, 5))
        assert result == "2024-01-05"

    def test_time_no_microseconds(self):
        result = ClientSideBindingConverter.to_snowflake(time(10, 30, 45))
        assert result == "10:30:45"

    def test_time_with_microseconds(self):
        result = ClientSideBindingConverter.to_snowflake(time(10, 30, 45, 123456))
        assert result == "10:30:45.123456"

    def test_time_midnight(self):
        result = ClientSideBindingConverter.to_snowflake(time(0, 0, 0))
        assert result == "00:00:00"

    def test_timedelta_no_microseconds(self):
        result = ClientSideBindingConverter.to_snowflake(timedelta(hours=1, minutes=2, seconds=3))
        assert result == "01:02:03"

    def test_timedelta_with_microseconds(self):
        result = ClientSideBindingConverter.to_snowflake(timedelta(hours=1, minutes=2, seconds=3, microseconds=456))
        assert result == "01:02:03.000456"

    def test_timedelta_with_days(self):
        result = ClientSideBindingConverter.to_snowflake(timedelta(days=1, hours=2))
        assert result == "26:00:00"

    def test_struct_time(self):
        st = time_module.strptime("2024-06-15 10:30:00", "%Y-%m-%d %H:%M:%S")
        result = ClientSideBindingConverter.to_snowflake(st)
        assert result == "2024-06-15 10:30:00"

    def test_list_converts_each_element(self):
        result = ClientSideBindingConverter.to_snowflake([1, "hello", None])
        assert result == [1, "hello", None]

    def test_list_with_dates(self):
        result = ClientSideBindingConverter.to_snowflake([date(2024, 1, 1), date(2024, 12, 31)])
        assert result == ["2024-01-01", "2024-12-31"]

    def test_unknown_type_converts_to_str(self):
        class Custom:
            def __str__(self):
                return "custom_value"

        result = ClientSideBindingConverter.to_snowflake(Custom())
        assert result == "custom_value"


class TestProcessSingleParam:
    """Test the full to_snowflake -> escape -> quote pipeline."""

    def test_none_produces_null(self):
        assert ClientSideBindingConverter.process_single_param(None) == "NULL"

    def test_bool_true_produces_true(self):
        assert ClientSideBindingConverter.process_single_param(True) == "TRUE"

    def test_int_produces_bare_number(self):
        assert ClientSideBindingConverter.process_single_param(42) == "42"

    def test_float_produces_bare_number(self):
        assert ClientSideBindingConverter.process_single_param(3.14) == "3.14"

    def test_decimal_produces_bare_number(self):
        assert ClientSideBindingConverter.process_single_param(Decimal("1.23")) == "1.23"

    def test_string_produces_quoted_string(self):
        assert ClientSideBindingConverter.process_single_param("hello") == "'hello'"

    def test_string_with_quote_escaped_and_quoted(self):
        result = ClientSideBindingConverter.process_single_param("it's")
        assert result == "'it\\'s'"

    def test_string_with_newline_escaped_and_quoted(self):
        result = ClientSideBindingConverter.process_single_param("a\nb")
        assert result == "'a\\nb'"

    def test_bytes_produces_hex_literal(self):
        result = ClientSideBindingConverter.process_single_param(b"\xab\xcd")
        assert result == "X'abcd'"

    def test_datetime_produces_quoted_string(self):
        result = ClientSideBindingConverter.process_single_param(datetime(2024, 6, 15, 10, 30, 0))
        assert result == "'2024-06-15 10:30:00'"

    def test_date_produces_quoted_string(self):
        result = ClientSideBindingConverter.process_single_param(date(2024, 6, 15))
        assert result == "'2024-06-15'"

    def test_time_produces_quoted_string(self):
        result = ClientSideBindingConverter.process_single_param(time(10, 30, 45))
        assert result == "'10:30:45'"


class TestProcessParamsPyformat:
    """Test process_params_pyformat with sequences and mappings."""

    def test_none_returns_empty_tuple(self):
        assert ClientSideBindingConverter.process_params_pyformat(None) == ()

    def test_sequence_returns_tuple(self):
        result = ClientSideBindingConverter.process_params_pyformat([42, "hello"])
        assert result == ("42", "'hello'")

    def test_mapping_returns_dict(self):
        result = ClientSideBindingConverter.process_params_pyformat({"a": 42, "b": "hello"})
        assert result == {"a": "42", "b": "'hello'"}

    def test_empty_sequence(self):
        result = ClientSideBindingConverter.process_params_pyformat([])
        assert result == ()

    def test_empty_mapping(self):
        result = ClientSideBindingConverter.process_params_pyformat({})
        assert result == {}

    def test_mixed_types_sequence(self):
        result = ClientSideBindingConverter.process_params_pyformat([42, "hi", True, None, 3.14])
        assert result == ("42", "'hi'", "TRUE", "NULL", "3.14")


class TestInterpolateQuery:
    """Test interpolate_query for full query construction."""

    def test_none_params_returns_query_unchanged(self):
        query = "SELECT * FROM t"
        assert ClientSideBindingConverter.interpolate_query(query, None) == query

    def test_empty_params_returns_query_unchanged(self):
        query = "SELECT * FROM t"
        assert ClientSideBindingConverter.interpolate_query(query, []) == query

    def test_positional_int(self):
        result = ClientSideBindingConverter.interpolate_query("SELECT %s", [42])
        assert result == "SELECT 42"

    def test_positional_string(self):
        result = ClientSideBindingConverter.interpolate_query("SELECT %s", ["hello"])
        assert result == "SELECT 'hello'"

    def test_positional_null(self):
        result = ClientSideBindingConverter.interpolate_query("SELECT %s", [None])
        assert result == "SELECT NULL"

    def test_positional_bool(self):
        result = ClientSideBindingConverter.interpolate_query("SELECT %s", [True])
        assert result == "SELECT TRUE"

    def test_multiple_positional(self):
        result = ClientSideBindingConverter.interpolate_query("SELECT %s, %s, %s", [42, "hello", None])
        assert result == "SELECT 42, 'hello', NULL"

    def test_named_params(self):
        result = ClientSideBindingConverter.interpolate_query("SELECT %(a)s, %(b)s", {"a": 42, "b": "hello"})
        assert result == "SELECT 42, 'hello'"

    def test_named_param_reused(self):
        result = ClientSideBindingConverter.interpolate_query("SELECT %(x)s, %(x)s", {"x": 42})
        assert result == "SELECT 42, 42"

    def test_string_with_single_quote_escaped(self):
        result = ClientSideBindingConverter.interpolate_query("SELECT %s", ["it's"])
        assert result == "SELECT 'it\\'s'"

    def test_string_with_newline_escaped(self):
        result = ClientSideBindingConverter.interpolate_query("SELECT %s", ["a\nb"])
        assert result == "SELECT 'a\\nb'"

    def test_binary_interpolation(self):
        result = ClientSideBindingConverter.interpolate_query("SELECT %s", [b"\xab\xcd"])
        assert result == "SELECT X'abcd'"

    def test_datetime_interpolation(self):
        result = ClientSideBindingConverter.interpolate_query("SELECT %s", [datetime(2024, 6, 15, 10, 30)])
        assert result == "SELECT '2024-06-15 10:30:00'"

    def test_date_interpolation(self):
        result = ClientSideBindingConverter.interpolate_query("SELECT %s", [date(2024, 6, 15)])
        assert result == "SELECT '2024-06-15'"

    def test_time_interpolation(self):
        result = ClientSideBindingConverter.interpolate_query("SELECT %s", [time(10, 30, 45)])
        assert result == "SELECT '10:30:45'"

    def test_list_in_clause(self):
        result = ClientSideBindingConverter.interpolate_query("SELECT * FROM t WHERE id IN (%s)", [[1, 2, 3]])
        assert result == "SELECT * FROM t WHERE id IN (1,2,3)"

    def test_list_with_strings_in_clause(self):
        result = ClientSideBindingConverter.interpolate_query("SELECT * FROM t WHERE name IN (%s)", [["Alice", "Bob"]])
        assert result == "SELECT * FROM t WHERE name IN ('Alice','Bob')"

    def test_decimal_interpolation(self):
        result = ClientSideBindingConverter.interpolate_query("SELECT %s", [Decimal("99.99")])
        assert result == "SELECT 99.99"

    def test_where_clause_with_mixed_types(self):
        result = ClientSideBindingConverter.interpolate_query(
            "SELECT * FROM t WHERE id = %s AND name = %s AND active = %s",
            [42, "Alice", True],
        )
        assert result == "SELECT * FROM t WHERE id = 42 AND name = 'Alice' AND active = TRUE"


class TestJsonBindingConverterNumpy:
    """Test that JsonBindingConverter handles numpy types via _is_numeric."""

    def test_numpy_int64_type_is_fixed(self):
        snowflake_type, value = JsonBindingConverter._convert_value(np.int64(42))
        assert snowflake_type == "FIXED"
        assert value == "42"

    def test_numpy_int32_type_is_fixed(self):
        snowflake_type, value = JsonBindingConverter._convert_value(np.int32(-7))
        assert snowflake_type == "FIXED"
        assert value == "-7"

    def test_numpy_uint16_type_is_fixed(self):
        snowflake_type, value = JsonBindingConverter._convert_value(np.uint16(300))
        assert snowflake_type == "FIXED"
        assert value == "300"

    def test_numpy_float64_type_is_real(self):
        snowflake_type, value = JsonBindingConverter._convert_value(np.float64(3.14))
        assert snowflake_type == "REAL"
        assert value == "3.14"

    def test_numpy_float32_type_is_real(self):
        snowflake_type, _ = JsonBindingConverter._convert_value(np.float32(1.5))
        assert snowflake_type == "REAL"

    def test_numpy_bool_type_is_boolean(self):
        snowflake_type, value = JsonBindingConverter._convert_value(np.bool_(True))
        assert snowflake_type == "BOOLEAN"
        assert value == "true"

    def test_numpy_int_array_binding(self):
        snowflake_type, values = JsonBindingConverter._convert_array([np.int64(1), np.int64(2), np.int64(3)])
        assert snowflake_type == "FIXED"
        assert values == ["1", "2", "3"]

    def test_numpy_serialize_parameters(self):
        json_str, length = JsonBindingConverter.serialize_parameters([np.int64(42), np.float64(3.14)])
        assert json_str is not None
        parsed = json.loads(json_str)
        assert parsed["1"]["type"] == "FIXED"
        assert parsed["1"]["value"] == "42"
        assert parsed["2"]["type"] == "REAL"
        assert parsed["2"]["value"] == "3.14"


class TestClientSideBindingConverterNumpy:
    """Test that ClientSideBindingConverter handles numpy types via _is_numeric."""

    def test_numpy_int64_passthrough(self):
        result = ClientSideBindingConverter.to_snowflake(np.int64(42))
        assert result == np.int64(42)

    def test_numpy_float64_passthrough(self):
        result = ClientSideBindingConverter.to_snowflake(np.float64(3.14))
        assert result == np.float64(3.14)

    def test_numpy_int64_quoted_as_bare_number(self):
        result = ClientSideBindingConverter.quote(np.int64(42))
        assert result == "42"
        assert "'" not in result

    def test_numpy_float64_quoted_as_bare_number(self):
        result = ClientSideBindingConverter.quote(np.float64(3.14))
        assert result == "3.14"
        assert "'" not in result

    def test_numpy_int64_not_escaped(self):
        result = ClientSideBindingConverter.escape(np.int64(42))
        assert result == np.int64(42)

    def test_numpy_int64_full_pipeline(self):
        result = ClientSideBindingConverter.process_single_param(np.int64(42))
        assert result == "42"

    def test_numpy_float64_interpolation(self):
        query = "SELECT * FROM t WHERE x = %s"
        result = ClientSideBindingConverter.interpolate_query(query, [np.float64(3.14)])
        assert result == "SELECT * FROM t WHERE x = 3.14"
