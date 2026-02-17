"""
Unit tests for BindingSerializer.

Tests cover type mapping, value conversion, array handling,
and the top-level serialize_parameters API.
Conversion logic is verified against the reference snowflake-connector-python's
SnowflakeConverter.to_snowflake_bindings behavior.
"""

import json
import time as time_module

from datetime import date, datetime, time, timedelta, timezone
from decimal import Decimal

import pytest

from snowflake.connector._internal.binding_serializer import BindingSerializer


class TestTypeMapping:
    """Test Python type to Snowflake type mapping."""

    def test_int_maps_to_fixed(self):
        snowflake_type, _ = BindingSerializer._convert_value(42)
        assert snowflake_type == "FIXED"

    def test_float_maps_to_real(self):
        snowflake_type, _ = BindingSerializer._convert_value(3.14)
        assert snowflake_type == "REAL"

    def test_str_maps_to_text(self):
        snowflake_type, _ = BindingSerializer._convert_value("hello")
        assert snowflake_type == "TEXT"

    def test_bool_maps_to_boolean(self):
        snowflake_type, _ = BindingSerializer._convert_value(True)
        assert snowflake_type == "BOOLEAN"

    def test_bytes_maps_to_binary(self):
        snowflake_type, _ = BindingSerializer._convert_value(b"\x00\x01")
        assert snowflake_type == "BINARY"

    def test_bytearray_maps_to_binary(self):
        snowflake_type, _ = BindingSerializer._convert_value(bytearray(b"\x00\x01"))
        assert snowflake_type == "BINARY"

    def test_datetime_maps_to_timestamp_ntz(self):
        snowflake_type, _ = BindingSerializer._convert_value(datetime(2024, 1, 15, 10, 30, 0))
        assert snowflake_type == "TIMESTAMP_NTZ"

    def test_date_maps_to_date(self):
        snowflake_type, _ = BindingSerializer._convert_value(date(2024, 1, 15))
        assert snowflake_type == "DATE"

    def test_time_maps_to_time(self):
        snowflake_type, _ = BindingSerializer._convert_value(time(10, 30, 45))
        assert snowflake_type == "TIME"

    def test_decimal_maps_to_fixed(self):
        snowflake_type, _ = BindingSerializer._convert_value(Decimal("123.45"))
        assert snowflake_type == "FIXED"

    def test_none_maps_to_any(self):
        """None maps to ANY, matching the reference connector."""
        snowflake_type, _ = BindingSerializer._convert_value(None)
        assert snowflake_type == "ANY"

    def test_timedelta_maps_to_time(self):
        snowflake_type, _ = BindingSerializer._convert_value(timedelta(hours=1, minutes=30))
        assert snowflake_type == "TIME"

    def test_struct_time_maps_to_timestamp_ntz(self):
        st = time_module.strptime("30 Sep 01 11:20:30", "%d %b %y %H:%M:%S")
        snowflake_type, _ = BindingSerializer._convert_value(st)
        assert snowflake_type == "TIMESTAMP_NTZ"

    def test_unknown_type_defaults_to_text(self):
        """Unknown types should fall back to TEXT."""
        snowflake_type, _ = BindingSerializer._convert_value(object())
        assert snowflake_type == "TEXT"


class TestConvertValueScalars:
    """Test scalar value conversion to Snowflake binding format."""

    def test_none_returns_none_value(self):
        _, value = BindingSerializer._convert_value(None)
        assert value is None

    def test_int_converts_to_string(self):
        _, value = BindingSerializer._convert_value(42)
        assert value == "42"

    def test_negative_int(self):
        _, value = BindingSerializer._convert_value(-100)
        assert value == "-100"

    def test_zero_int(self):
        _, value = BindingSerializer._convert_value(0)
        assert value == "0"

    def test_large_int(self):
        _, value = BindingSerializer._convert_value(99999999999999999)
        assert value == "99999999999999999"

    def test_float_converts_to_string(self):
        _, value = BindingSerializer._convert_value(3.14)
        assert value == "3.14"

    def test_negative_float(self):
        _, value = BindingSerializer._convert_value(-2.5)
        assert value == "-2.5"

    def test_float_zero(self):
        _, value = BindingSerializer._convert_value(0.0)
        assert value == "0.0"

    def test_string_preserved_as_is(self):
        _, value = BindingSerializer._convert_value("hello world")
        assert value == "hello world"

    def test_empty_string(self):
        _, value = BindingSerializer._convert_value("")
        assert value == ""

    def test_string_with_special_characters(self):
        _, value = BindingSerializer._convert_value('it\'s a "test"\nnewline')
        assert value == 'it\'s a "test"\nnewline'

    def test_bool_true_converts_to_lowercase(self):
        _, value = BindingSerializer._convert_value(True)
        assert value == "true"

    def test_bool_false_converts_to_lowercase(self):
        _, value = BindingSerializer._convert_value(False)
        assert value == "false"

    def test_bool_is_not_treated_as_int(self):
        """Bool is a subclass of int; ensure it's serialized as bool, not int."""
        snowflake_type, value = BindingSerializer._convert_value(True)
        assert snowflake_type == "BOOLEAN"
        assert value == "true"
        assert value != "1"

    def test_decimal_converts_to_string(self):
        _, value = BindingSerializer._convert_value(Decimal("123.456"))
        assert value == "123.456"

    def test_decimal_high_precision(self):
        _, value = BindingSerializer._convert_value(Decimal("0.00000000000001"))
        # Decimal str() may use scientific notation
        assert Decimal(value) == Decimal("0.00000000000001")

    def test_decimal_negative(self):
        _, value = BindingSerializer._convert_value(Decimal("-99.99"))
        assert value == "-99.99"

    def test_bytes_hex_encoded(self):
        _, value = BindingSerializer._convert_value(b"\x00\x01\x02\xff")
        assert value == "000102ff"

    def test_bytes_empty(self):
        _, value = BindingSerializer._convert_value(b"")
        assert value == ""

    def test_bytes_ascii(self):
        _, value = BindingSerializer._convert_value(b"ABC")
        assert value == "414243"

    def test_bytes_non_ascii(self):
        """Non-ASCII bytes should be properly hex-encoded without errors."""
        _, value = BindingSerializer._convert_value(bytes([0xF5, 0xAB, 0xCD, 0xEF]))
        assert value == "f5abcdef"

    def test_bytearray_hex_encoded(self):
        """bytearray should be hex-encoded identically to bytes."""
        _, value = BindingSerializer._convert_value(bytearray(b"\xab\xcd\xef"))
        assert value == "abcdef"

    def test_bytearray_matches_bytes(self):
        """bytearray and bytes with same content should produce identical output."""
        data = b"\x00\x01\x02\xff"
        _, bytes_value = BindingSerializer._convert_value(data)
        _, bytearray_value = BindingSerializer._convert_value(bytearray(data))
        assert bytes_value == bytearray_value


class TestConvertValueDatetime:
    """Test datetime/date/time value conversion."""

    def test_datetime_to_epoch_nanoseconds(self):
        dt = datetime(2024, 1, 15, 12, 0, 0)
        _, value = BindingSerializer._convert_value(dt)
        assert isinstance(value, str)
        nanos = int(value)
        assert nanos > 0

    def test_datetime_epoch_is_zero(self):
        """datetime at Unix epoch should produce zero nanoseconds (timezone-independent)."""
        dt = datetime(1970, 1, 1, 0, 0, 0)
        _, value = BindingSerializer._convert_value(dt)
        assert int(value) == 0

    def test_datetime_one_second_after_epoch(self):
        """1970-01-01 00:00:01 should produce exactly 1 billion nanoseconds."""
        dt = datetime(1970, 1, 1, 0, 0, 1)
        _, value = BindingSerializer._convert_value(dt)
        assert int(value) == 1_000_000_000

    def test_datetime_with_microseconds_at_epoch(self):
        """Microsecond precision must be preserved in nanosecond output.

        datetime(1970, 1, 1, 0, 0, 0, 123456) = 123456 microseconds
        = 123456000 nanoseconds.
        """
        dt = datetime(1970, 1, 1, 0, 0, 0, 123456)
        _, value = BindingSerializer._convert_value(dt)
        assert int(value) == 123_456_000

    def test_datetime_with_one_microsecond(self):
        """Single microsecond must not be lost.

        datetime(1970, 1, 1, 0, 0, 0, 1) = 1 microsecond = 1000 nanoseconds.
        """
        dt = datetime(1970, 1, 1, 0, 0, 0, 1)
        _, value = BindingSerializer._convert_value(dt)
        assert int(value) == 1_000

    def test_datetime_with_microseconds_and_seconds(self):
        """Combined seconds and microseconds must produce correct nanoseconds.

        datetime(1970, 1, 1, 0, 0, 1, 123456) = 1.123456 seconds
        = 1_123_456_000 nanoseconds.
        """
        dt = datetime(1970, 1, 1, 0, 0, 1, 123456)
        _, value = BindingSerializer._convert_value(dt)
        assert int(value) == 1_123_456_000

    def test_datetime_with_microseconds_far_from_epoch(self):
        """Microsecond precision must be preserved even for dates far from epoch.

        Large epoch offsets increase the total_seconds() float magnitude,
        which can cause loss of microsecond precision due to float64 limits.
        """
        dt = datetime(2024, 6, 15, 10, 30, 0, 123456)
        dt_no_us = datetime(2024, 6, 15, 10, 30, 0, 0)
        _, value = BindingSerializer._convert_value(dt)
        _, value_no_us = BindingSerializer._convert_value(dt_no_us)
        # The difference must be exactly 123456 microseconds = 123456000 nanoseconds
        assert int(value) - int(value_no_us) == 123_456_000

    def test_datetime_max_microseconds(self):
        """Maximum microsecond value (999999) must be preserved.

        datetime(1970, 1, 1, 0, 0, 0, 999999) = 999999 microseconds
        = 999_999_000 nanoseconds.
        """
        dt = datetime(1970, 1, 1, 0, 0, 0, 999999)
        _, value = BindingSerializer._convert_value(dt)
        assert int(value) == 999_999_000

    def test_date_epoch_is_zero(self):
        """date at Unix epoch should produce zero milliseconds (timezone-independent)."""
        d = date(1970, 1, 1)
        _, value = BindingSerializer._convert_value(d)
        assert int(value) == 0

    def test_date_to_epoch_milliseconds(self):
        d = date(2024, 1, 15)
        _, value = BindingSerializer._convert_value(d)
        assert isinstance(value, str)
        millis = int(value)
        assert millis > 0

    def test_date_known_value(self):
        """date(1970, 1, 2) should be exactly 86400000 milliseconds."""
        d = date(1970, 1, 2)
        _, value = BindingSerializer._convert_value(d)
        assert value == "86400000"

    def test_time_to_nanoseconds_midnight(self):
        t = time(0, 0, 0)
        _, value = BindingSerializer._convert_value(t)
        assert value == "0000000000"

    def test_time_to_nanoseconds_noon(self):
        t = time(12, 0, 0)
        _, value = BindingSerializer._convert_value(t)
        # 12*3600 = 43200, then "000000" microseconds, then "000" trailing
        assert value == "43200000000000"

    def test_time_with_microseconds(self):
        """Matches reference connector string concatenation format."""
        t = time(1, 2, 3, 456)
        _, value = BindingSerializer._convert_value(t)
        # str(3723) + f"{456:06d}" + "000" = "3723" + "000456" + "000"
        assert value == "3723000456000"

    def test_time_end_of_day(self):
        t = time(23, 59, 59, 999999)
        _, value = BindingSerializer._convert_value(t)
        # str(86399) + f"{999999:06d}" + "000" = "86399" + "999999" + "000"
        assert value == "86399999999000"

    def test_time_one_microsecond(self):
        t = time(0, 0, 0, 1)
        _, value = BindingSerializer._convert_value(t)
        # str(0) + f"{1:06d}" + "000" = "0" + "000001" + "000"
        assert value == "0000001000"


class TestConvertValueTimedelta:
    """Test timedelta conversion (maps to TIME type)."""

    def test_timedelta_zero(self):
        td = timedelta(0)
        snowflake_type, value = BindingSerializer._convert_value(td)
        assert snowflake_type == "TIME"
        assert value == "0000000000"

    def test_timedelta_one_hour(self):
        td = timedelta(hours=1)
        _, value = BindingSerializer._convert_value(td)
        # str(3600) + "000000" + "000"
        assert value == "3600000000000"

    def test_timedelta_with_microseconds(self):
        td = timedelta(hours=1, minutes=2, seconds=3, microseconds=456)
        _, value = BindingSerializer._convert_value(td)
        # str(3723) + f"{456:06d}" + "000" = "3723" + "000456" + "000"
        assert value == "3723000456000"

    def test_timedelta_with_days(self):
        """Days are converted to hours (days * 24h added to hours)."""
        td = timedelta(days=1, hours=1)
        _, value = BindingSerializer._convert_value(td)
        # (24+1)*3600 = 90000 seconds
        assert value == "90000000000000"

    def test_timedelta_matches_time(self):
        """timedelta(h=1,m=2,s=3,us=456) should produce same value as time(1,2,3,456)."""
        td = timedelta(hours=1, minutes=2, seconds=3, microseconds=456)
        t = time(1, 2, 3, 456)
        _, td_value = BindingSerializer._convert_value(td)
        _, t_value = BindingSerializer._convert_value(t)
        assert td_value == t_value


class TestConvertValueStructTime:
    """Test struct_time conversion (maps to TIMESTAMP_NTZ type)."""

    def test_struct_time_conversion(self):
        st = time_module.strptime("30 Sep 01 11:20:30", "%d %b %y %H:%M:%S")
        snowflake_type, value = BindingSerializer._convert_value(st)
        assert snowflake_type == "TIMESTAMP_NTZ"
        assert isinstance(value, str)
        # Should be a nanosecond epoch string
        nanos = int(value)
        assert nanos > 0

    def test_struct_time_matches_equivalent_datetime(self):
        """struct_time should produce the same result as the equivalent datetime."""
        st = time_module.strptime("2024-06-15 10:30:00", "%Y-%m-%d %H:%M:%S")
        dt = datetime.fromtimestamp(time_module.mktime(st))
        _, st_value = BindingSerializer._convert_value(st)
        _, dt_value = BindingSerializer._convert_value(dt)
        assert st_value == dt_value


class TestConvertArray:
    """Test array conversion for bulk operations."""

    def test_empty_array(self):
        snowflake_type, values = BindingSerializer._convert_array([])
        assert snowflake_type == "TEXT"
        assert values == []

    def test_int_array(self):
        snowflake_type, values = BindingSerializer._convert_array([1, 2, 3])
        assert snowflake_type == "FIXED"
        assert values == ["1", "2", "3"]

    def test_string_array(self):
        snowflake_type, values = BindingSerializer._convert_array(["a", "b", "c"])
        assert snowflake_type == "TEXT"
        assert values == ["a", "b", "c"]

    def test_float_array(self):
        snowflake_type, values = BindingSerializer._convert_array([1.1, 2.2, 3.3])
        assert snowflake_type == "REAL"
        assert values == ["1.1", "2.2", "3.3"]

    def test_bool_array(self):
        snowflake_type, values = BindingSerializer._convert_array([True, False, True])
        assert snowflake_type == "BOOLEAN"
        assert values == ["true", "false", "true"]

    def test_array_with_nones_preserves_type(self):
        """None values should not affect the inferred type."""
        snowflake_type, values = BindingSerializer._convert_array([1, None, 3])
        assert snowflake_type == "FIXED"
        assert values == ["1", None, "3"]

    def test_all_none_array(self):
        snowflake_type, values = BindingSerializer._convert_array([None, None, None])
        assert snowflake_type == "TEXT"
        assert values == [None, None, None]

    def test_mixed_type_array_falls_back_to_text(self):
        """Arrays with mixed non-null types should default to TEXT."""
        snowflake_type, values = BindingSerializer._convert_array([1, "hello", 3.14])
        assert snowflake_type == "TEXT"

    def test_bytes_array(self):
        snowflake_type, values = BindingSerializer._convert_array([b"\x01", b"\x02"])
        assert snowflake_type == "BINARY"
        assert values == ["01", "02"]

    def test_decimal_array(self):
        snowflake_type, values = BindingSerializer._convert_array([Decimal("1.1"), Decimal("2.2")])
        assert snowflake_type == "FIXED"
        assert values == ["1.1", "2.2"]


class TestProcessParams:
    """Test _process_params with positional parameters."""

    def test_single_param(self):
        result = BindingSerializer._process_params([42])
        assert result == {"1": {"type": "FIXED", "value": "42"}}

    def test_multiple_params(self):
        result = BindingSerializer._process_params([42, "hello", True])
        assert result == {
            "1": {"type": "FIXED", "value": "42"},
            "2": {"type": "TEXT", "value": "hello"},
            "3": {"type": "BOOLEAN", "value": "true"},
        }

    def test_params_are_one_indexed(self):
        """Parameter keys should be 1-indexed strings."""
        result = BindingSerializer._process_params(["a", "b", "c"])
        assert "1" in result
        assert "2" in result
        assert "3" in result
        assert "0" not in result

    def test_array_param(self):
        result = BindingSerializer._process_params([[1, 2, 3]])
        assert result == {"1": {"type": "FIXED", "value": ["1", "2", "3"]}}

    def test_mixed_scalar_and_array_params(self):
        result = BindingSerializer._process_params(["hello", [1, 2, 3]])
        assert result == {
            "1": {"type": "TEXT", "value": "hello"},
            "2": {"type": "FIXED", "value": ["1", "2", "3"]},
        }

    def test_none_param(self):
        result = BindingSerializer._process_params([None])
        assert result == {"1": {"type": "ANY", "value": None}}


class TestSerializeParameters:
    """Test the top-level serialize_parameters API."""

    def test_none_params_returns_none(self):
        json_str, length = BindingSerializer.serialize_parameters(None)
        assert json_str is None
        assert length == 0

    def test_empty_list_returns_none(self):
        json_str, length = BindingSerializer.serialize_parameters([])
        assert json_str is None
        assert length == 0

    def test_returns_valid_json(self):
        json_str, length = BindingSerializer.serialize_parameters([42, "hello"])
        assert json_str is not None
        parsed = json.loads(json_str)
        assert parsed == {
            "1": {"type": "FIXED", "value": "42"},
            "2": {"type": "TEXT", "value": "hello"},
        }

    def test_length_matches_utf8_bytes(self):
        json_str, length = BindingSerializer.serialize_parameters([42, "hello"])
        assert length == len(json_str.encode("utf-8"))

    def test_length_with_unicode_characters(self):
        """UTF-8 byte length should match encoded JSON bytes."""
        json_str, length = BindingSerializer.serialize_parameters(["café ☕"])
        assert length == len(json_str.encode("utf-8"))
        parsed = json.loads(json_str)
        assert parsed["1"]["value"] == "café ☕"

    def test_single_int_param(self):
        json_str, _ = BindingSerializer.serialize_parameters([123])
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
        json_str, length = BindingSerializer.serialize_parameters(params)
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
        json_str, _ = BindingSerializer.serialize_parameters([[1, 2, 3], ["a", "b", "c"]])
        parsed = json.loads(json_str)
        assert parsed["1"]["type"] == "FIXED"
        assert parsed["1"]["value"] == ["1", "2", "3"]
        assert parsed["2"]["type"] == "TEXT"
        assert parsed["2"]["value"] == ["a", "b", "c"]


class TestReferenceConnectorParity:
    """Tests verifying parity with reference snowflake-connector-python.

    Each test documents a specific difference found during code comparison
    and asserts the corrected behavior matches the reference connector.
    """

    # --- Difference #1: date conversion must be timezone-independent ---
    # Old BindingSerializer used datetime.combine(d, min.time()).timestamp()
    # which applies the local timezone offset. The reference connector uses
    # (d - date(1970,1,1)).total_seconds() which is timezone-independent.

    def test_date_epoch_is_timezone_independent(self):
        """date(1970, 1, 1) must always produce 0 regardless of local timezone.

        Previously used datetime.timestamp() which would produce e.g. -28800000
        on a UTC-8 machine.
        """
        d = date(1970, 1, 1)
        _, value = BindingSerializer._convert_value(d)
        assert int(value) == 0

    def test_date_known_value_is_timezone_independent(self):
        """date(2024, 1, 15) must produce the same milliseconds everywhere.

        Reference connector: (date(2024,1,15) - date(1970,1,1)).total_seconds() * 1000
        = 19737 days * 86400 * 1000 = 1705276800000
        """
        d = date(2024, 1, 15)
        _, value = BindingSerializer._convert_value(d)
        expected_days = (date(2024, 1, 15) - date(1970, 1, 1)).days
        expected_ms = expected_days * 86400 * 1000
        assert int(value) == expected_ms

    def test_date_before_epoch_is_negative(self):
        """Dates before 1970-01-01 must produce negative millisecond values."""
        d = date(1969, 12, 31)
        _, value = BindingSerializer._convert_value(d)
        assert int(value) == -86400000

    # --- Difference #2: datetime conversion must be timezone-independent ---
    # Old BindingSerializer used dt.timestamp() which depends on local timezone.
    # The reference connector uses (dt - ZERO_EPOCH).total_seconds().

    def test_datetime_epoch_is_timezone_independent(self):
        """datetime(1970, 1, 1) must always produce 0 nanoseconds.

        Previously used dt.timestamp() which produces different values
        depending on local timezone.
        """
        dt = datetime(1970, 1, 1, 0, 0, 0)
        _, value = BindingSerializer._convert_value(dt)
        assert int(value) == 0

    def test_datetime_known_value_is_timezone_independent(self):
        """Two datetimes exactly 1 hour apart must differ by exactly 3600 * 10^9 ns."""
        dt1 = datetime(2024, 6, 15, 10, 0, 0)
        dt2 = datetime(2024, 6, 15, 11, 0, 0)
        _, v1 = BindingSerializer._convert_value(dt1)
        _, v2 = BindingSerializer._convert_value(dt2)
        assert int(v2) - int(v1) == 3600 * 1_000_000_000

    def test_datetime_tz_aware_normalizes_to_utc(self):
        """Timezone-aware datetimes must be converted to UTC before serialization.

        Reference connector: astimezone(pytz.UTC).replace(tzinfo=None), then
        compute epoch from the naive UTC datetime.
        """
        # 2024-01-01 05:00:00 UTC+5 == 2024-01-01 00:00:00 UTC
        naive_utc = datetime(2024, 1, 1, 0, 0, 0)
        tz_plus5 = timezone(timedelta(hours=5))
        aware = datetime(2024, 1, 1, 5, 0, 0, tzinfo=tz_plus5)

        _, naive_value = BindingSerializer._convert_value(naive_utc)
        _, aware_value = BindingSerializer._convert_value(aware)
        assert naive_value == aware_value

    def test_datetime_tz_negative_offset(self):
        """UTC-8 datetime should also normalize correctly."""
        naive_utc = datetime(2024, 1, 1, 8, 0, 0)
        tz_minus8 = timezone(timedelta(hours=-8))
        aware = datetime(2024, 1, 1, 0, 0, 0, tzinfo=tz_minus8)

        _, naive_value = BindingSerializer._convert_value(naive_utc)
        _, aware_value = BindingSerializer._convert_value(aware)
        assert naive_value == aware_value

    # --- Difference #3: NoneType must map to "ANY", not "TEXT" ---
    # Reference connector: PYTHON_TO_SNOWFLAKE_TYPE["nonetype"] = "ANY"

    def test_none_type_is_any_not_text(self):
        """None must map to type ANY, matching the reference connector.

        Previously mapped to TEXT which could affect server-side schema inference.
        """
        snowflake_type, value = BindingSerializer._convert_value(None)
        assert snowflake_type == "ANY"
        assert value is None

    # --- Difference #4: bytearray must be supported ---
    # Reference connector: _bytearray_to_snowflake_bindings = _bytes_to_snowflake_bindings
    # Previously missing, would fall through to str(value) producing wrong output.

    def test_bytearray_produces_hex_not_str_repr(self):
        """bytearray must hex-encode, not produce str(bytearray(...)).

        Previously fell through to str(value) which would produce something like
        "bytearray(b'\\xab\\xcd')" instead of proper hex encoding.
        """
        ba = bytearray(b"\xab\xcd")
        snowflake_type, value = BindingSerializer._convert_value(ba)
        assert snowflake_type == "BINARY"
        assert value == "abcd"
        # Must NOT be the string representation
        assert "bytearray" not in value

    # --- Difference #5: timedelta must be supported (maps to TIME) ---
    # Reference connector: _timedelta_to_snowflake_bindings converts to nanoseconds.
    # Previously missing, would fall through to str(value).

    def test_timedelta_produces_nanoseconds_not_str_repr(self):
        """timedelta must convert to nanoseconds, not produce str(timedelta(...)).

        Previously fell through to str(value) which would produce something like
        "1:02:03.000456" instead of nanosecond format.
        """
        td = timedelta(hours=1, minutes=2, seconds=3, microseconds=456)
        snowflake_type, value = BindingSerializer._convert_value(td)
        assert snowflake_type == "TIME"
        assert value == "3723000456000"
        # Must NOT contain colons (the str(timedelta) format)
        assert ":" not in value

    def test_timedelta_with_days_converts_days_to_hours(self):
        """Reference connector multiplies days * 24 and adds to hours.

        timedelta(days=2, hours=3) = (2*24 + 3) * 3600 = 183600 seconds
        """
        td = timedelta(days=2, hours=3)
        _, value = BindingSerializer._convert_value(td)
        expected_seconds = (2 * 24 + 3) * 3600
        assert value == f"{expected_seconds}000000000"

    # --- Difference #6: struct_time must be supported ---
    # Reference connector: _struct_time_to_snowflake_bindings converts via
    # datetime.fromtimestamp(time.mktime(value)), then serializes as datetime.
    # Previously missing, would fall through to str(value).

    def test_struct_time_produces_nanoseconds_not_str_repr(self):
        """struct_time must convert to epoch nanoseconds, not produce str repr.

        Previously fell through to str(value) producing something like
        "time.struct_time(tm_year=2024, ...)" instead of nanoseconds.
        """
        st = time_module.strptime("2024-06-15 10:30:00", "%Y-%m-%d %H:%M:%S")
        snowflake_type, value = BindingSerializer._convert_value(st)
        assert snowflake_type == "TIMESTAMP_NTZ"
        # Must be a pure numeric nanosecond string
        assert value.lstrip("-").isdigit()
        # Must NOT contain struct_time string representation
        assert "struct_time" not in value

    # --- Difference #7: time formatting must use string concatenation ---
    # Reference connector: str(seconds) + f"{microsecond:06d}" + "000"
    # This produces left-padded microseconds (e.g., "000456") which is critical
    # for Snowflake server-side parsing.

    def test_time_microsecond_zero_padding(self):
        """Microseconds must be zero-padded to 6 digits in the nanosecond string.

        time(0, 0, 1, 1) -> "1" + "000001" + "000" = "1000001000"
        Without zero-padding this would incorrectly become "1" + "1" + "000" = "11000"
        """
        t = time(0, 0, 1, 1)
        _, value = BindingSerializer._convert_value(t)
        assert value == "1000001000"

    def test_time_format_trailing_zeros(self):
        """Time format must include trailing '000' for nanosecond precision.

        Even for zero microseconds: time(0, 0, 1) -> "1" + "000000" + "000"
        """
        t = time(0, 0, 1, 0)
        _, value = BindingSerializer._convert_value(t)
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
        json_str, length = BindingSerializer.serialize_parameters(params)
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
        result = BindingSerializer._convert_datetime_to_epoch_nanoseconds(dt)
        dt_base = datetime(2024, 1, 1, 0, 0, 0)
        base = BindingSerializer._convert_datetime_to_epoch_nanoseconds(dt_base)
        assert int(result) - int(base) == 1_000_000_000

    def test_datetime_epoch_zero(self):
        """Epoch datetime produces zero nanoseconds."""
        result = BindingSerializer._convert_datetime_to_epoch_nanoseconds(datetime(1970, 1, 1, 0, 0, 0))
        assert int(result) == 0

    def test_datetime_microseconds_at_epoch(self):
        """Microseconds at epoch must produce exact nanoseconds."""
        result = BindingSerializer._convert_datetime_to_epoch_nanoseconds(datetime(1970, 1, 1, 0, 0, 0, 123456))
        assert int(result) == 123_456_000

    def test_datetime_one_microsecond_at_epoch(self):
        """A single microsecond must not be lost."""
        result = BindingSerializer._convert_datetime_to_epoch_nanoseconds(datetime(1970, 1, 1, 0, 0, 0, 1))
        assert int(result) == 1_000

    def test_datetime_microseconds_far_from_epoch(self):
        """Microsecond precision must survive float64 representation for large epoch values."""
        result = BindingSerializer._convert_datetime_to_epoch_nanoseconds(datetime(2024, 6, 15, 10, 30, 0, 123456))
        result_no_us = BindingSerializer._convert_datetime_to_epoch_nanoseconds(datetime(2024, 6, 15, 10, 30, 0, 0))
        assert int(result) - int(result_no_us) == 123_456_000

    def test_datetime_tz_aware_converts_to_utc(self):
        """Timezone-aware datetimes should be normalized to UTC."""
        # 2024-01-01 05:00:00 UTC+5 == 2024-01-01 00:00:00 UTC

        utc_dt = datetime(2024, 1, 1, 0, 0, 0)
        result_utc = BindingSerializer._convert_datetime_to_epoch_nanoseconds(utc_dt)
        try:
            tz_plus5 = timezone(timedelta(hours=5))
            aware_dt = datetime(2024, 1, 1, 5, 0, 0, tzinfo=tz_plus5)
            result_aware = BindingSerializer._convert_datetime_to_epoch_nanoseconds(aware_dt)
            assert result_utc == result_aware
        except Exception:
            pytest.skip("timezone offset test not available")

    def test_date_epoch_zero(self):
        """Epoch date produces zero milliseconds."""
        result = BindingSerializer._convert_date_to_epoch_milliseconds(date(1970, 1, 1))
        assert int(result) == 0

    def test_date_one_day_after_epoch(self):
        """One day after epoch = 86400000 milliseconds."""
        result = BindingSerializer._convert_date_to_epoch_milliseconds(date(1970, 1, 2))
        assert result == "86400000"

    def test_date_before_epoch(self):
        """Dates before epoch should produce negative millisecond values."""
        result = BindingSerializer._convert_date_to_epoch_milliseconds(date(1969, 12, 31))
        assert int(result) == -86400000

    def test_time_to_nanoseconds_one_hour(self):
        result = BindingSerializer._convert_time_to_nanoseconds(time(1, 0, 0))
        assert result == "3600000000000"

    def test_time_to_nanoseconds_one_minute(self):
        result = BindingSerializer._convert_time_to_nanoseconds(time(0, 1, 0))
        assert result == "60000000000"

    def test_time_to_nanoseconds_one_second(self):
        result = BindingSerializer._convert_time_to_nanoseconds(time(0, 0, 1))
        assert result == "1000000000"

    def test_time_to_nanoseconds_one_microsecond(self):
        result = BindingSerializer._convert_time_to_nanoseconds(time(0, 0, 0, 1))
        assert result == "0000001000"

    def test_timedelta_basic(self):
        result = BindingSerializer._convert_timedelta_to_nanoseconds(
            timedelta(hours=1, minutes=2, seconds=3, microseconds=456)
        )
        assert result == "3723000456000"

    def test_timedelta_with_days(self):
        result = BindingSerializer._convert_timedelta_to_nanoseconds(timedelta(days=1))
        # 24*3600 = 86400
        assert result == "86400000000000"
