"""
Unit tests for ArrowConverterContext class.

This module tests the Python helper functions used for Arrow conversions.
"""

import decimal
import sys
from datetime import datetime, timedelta, timezone

import pytest
import pytz

from snowflake.ud_connector._internal.arrow_context import (
    ArrowConverterContext,
    ZERO_EPOCH,
    PARAMETER_TIMEZONE,
    _generate_tzinfo_from_tzoffset,
    interval_year_month_to_string,
)


class TestHelperFunctions:
    """Test module-level helper functions."""

    @pytest.mark.parametrize(
        "offset_minutes,expected_offset",
        [
            (60, timedelta(minutes=60)),  # +01:00
            (-300, timedelta(minutes=-300)),  # -05:00
            (0, timedelta(0)),  # UTC
            (330, timedelta(minutes=330)),  # +05:30 (India)
            (-480, timedelta(minutes=-480)),  # -08:00 (Pacific)
        ],
        ids=["positive_1h", "negative_5h", "zero_utc", "positive_5h30", "negative_8h"],
    )
    def test_generate_tzinfo_from_tzoffset(self, offset_minutes, expected_offset):
        """Test generating tzinfo from various offsets."""
        tzinfo = _generate_tzinfo_from_tzoffset(offset_minutes)
        assert tzinfo is not None
        offset = tzinfo.utcoffset(datetime.now())
        assert offset == expected_offset

    @pytest.mark.parametrize(
        "months,expected",
        [
            (14, "1-2"),  # 1 year, 2 months
            (-14, "-1-2"),  # negative
            (0, "0-0"),  # zero
            (24, "2-0"),  # exact years
            (7, "0-7"),  # only months
            (12, "1-0"),  # exactly 1 year
            (100, "8-4"),  # large value
            (-1, "-0-1"),  # single negative month
        ],
        ids=[
            "1y2m",
            "neg_1y2m",
            "zero",
            "2y",
            "7m",
            "1y",
            "8y4m",
            "neg_1m",
        ],
    )
    def test_interval_year_month_to_string(self, months, expected):
        """Test interval conversion with various month values."""
        result = interval_year_month_to_string(months)
        assert result == expected


class TestArrowConverterContextInit:
    """Test ArrowConverterContext initialization."""

    @pytest.mark.parametrize(
        "session_params,expected_tz",
        [
            (None, None),
            ({}, None),
            ({PARAMETER_TIMEZONE: "America/New_York"}, "America/New_York"),
            ({PARAMETER_TIMEZONE: "UTC"}, "UTC"),
            ({PARAMETER_TIMEZONE: "Europe/London"}, "Europe/London"),
            ({"OTHER_PARAM": "value"}, None),
        ],
        ids=["none", "empty", "new_york", "utc", "london", "other_param"],
    )
    def test_init_with_parameters(self, session_params, expected_tz):
        """Test initialization with various session parameters."""
        context = ArrowConverterContext(session_parameters=session_params)
        assert context.timezone == expected_tz

    def test_timezone_setter(self):
        """Test timezone property setter."""
        context = ArrowConverterContext()
        context.timezone = "Europe/London"
        assert context.timezone == "Europe/London"


class TestTimestampTzToPython:
    """Test TIMESTAMP_TZ conversion methods."""

    @pytest.mark.parametrize(
        "epoch,microseconds,tz_offset,expected_tz_minutes",
        [
            (0, 0, 1440, 0),  # UTC (1440 - 1440 = 0)
            (0, 0, 1500, 60),  # +01:00 (1500 - 1440 = 60)
            (0, 0, 1140, -300),  # -05:00 (1140 - 1440 = -300)
            (0, 0, 1920, 480),  # +08:00 (1920 - 1440 = 480)
            (0, 0, 960, -480),  # -08:00 (960 - 1440 = -480)
        ],
        ids=["utc", "plus_1h", "minus_5h", "plus_8h", "minus_8h"],
    )
    def test_timestamp_tz_to_python_offsets(
        self, epoch, microseconds, tz_offset, expected_tz_minutes
    ):
        """Test TIMESTAMP_TZ conversion with various timezone offsets."""
        context = ArrowConverterContext()
        result = context.TIMESTAMP_TZ_to_python(epoch, microseconds, tz_offset)
        assert isinstance(result, datetime)
        assert result.tzinfo is not None
        offset = result.tzinfo.utcoffset(result)
        assert offset == timedelta(minutes=expected_tz_minutes)

    @pytest.mark.parametrize(
        "microseconds",
        [0, 1, 500000, 999999, 123456],
        ids=["zero", "one", "half_sec", "max", "arbitrary"],
    )
    def test_timestamp_tz_to_python_microseconds(self, microseconds):
        """Test TIMESTAMP_TZ conversion preserves microseconds."""
        context = ArrowConverterContext()
        result = context.TIMESTAMP_TZ_to_python(0, microseconds, 1440)
        assert result.microsecond == microseconds

    @pytest.mark.parametrize(
        "epoch,microseconds,tz_offset,expected_year",
        [
            # Negative epochs (before 1970) - the main use case for Windows function
            (-86400, 0, 1440, 1969),  # 1 day before epoch, UTC
            (-31536000, 0, 1500, 1969),  # 1969-01-01, +1h offset
            (-86400, 500000, 1140, 1969),  # with microseconds, -5h offset
            # Positive epochs should also work
            (0, 0, 1440, 1970),  # epoch, UTC
            (86400, 123456, 1920, 1970),  # 1 day after, +8h offset
        ],
        ids=["1969_utc", "1969_plus1h", "1969_minus5h", "epoch_utc", "1970_plus8h"],
    )
    def test_timestamp_tz_to_python_windows(
        self, epoch, microseconds, tz_offset, expected_year
    ):
        """Test Windows-specific TIMESTAMP_TZ for negative epochs (before 1970)."""
        context = ArrowConverterContext()
        result = context.TIMESTAMP_TZ_to_python_windows(epoch, microseconds, tz_offset)
        assert isinstance(result, datetime)
        assert result.tzinfo is not None
        assert result.year == expected_year
        assert result.microsecond == microseconds


class TestTimestampNtzToPython:
    """Test TIMESTAMP_NTZ conversion methods."""

    @pytest.mark.parametrize(
        "epoch,microseconds,expected_year,expected_month,expected_day",
        [
            (0, 0, 1970, 1, 1),  # epoch zero
            (1609459200, 0, 2021, 1, 1),  # 2021-01-01
            (1672531200, 0, 2023, 1, 1),  # 2023-01-01
            (86400, 0, 1970, 1, 2),  # one day after epoch
        ],
        ids=["epoch_zero", "2021", "2023", "one_day"],
    )
    def test_timestamp_ntz_to_python_dates(
        self, epoch, microseconds, expected_year, expected_month, expected_day
    ):
        """Test TIMESTAMP_NTZ conversion for various dates."""
        context = ArrowConverterContext()
        result = context.TIMESTAMP_NTZ_to_python(epoch, microseconds)
        assert isinstance(result, datetime)
        assert result.tzinfo is None
        assert result.year == expected_year
        assert result.month == expected_month
        assert result.day == expected_day

    @pytest.mark.parametrize(
        "microseconds",
        [0, 123456, 500000, 999999],
        ids=["zero", "arbitrary", "half_sec", "max"],
    )
    def test_timestamp_ntz_to_python_microseconds(self, microseconds):
        """Test TIMESTAMP_NTZ conversion preserves microseconds."""
        context = ArrowConverterContext()
        result = context.TIMESTAMP_NTZ_to_python(0, microseconds)
        assert result.microsecond == microseconds

    @pytest.mark.parametrize(
        "epoch,microseconds,expected_year,expected_month,expected_day",
        [
            # Negative epochs (before 1970) - the main use case for Windows function
            (-86400, 0, 1969, 12, 31),  # 1 day before epoch
            (-31536000, 0, 1969, 1, 1),  # 1969-01-01
            (-2208988800, 0, 1900, 1, 1),  # 1900-01-01
            # Positive epochs should also work
            (0, 0, 1970, 1, 1),  # epoch
            (86400, 500000, 1970, 1, 2),  # 1 day after epoch with microseconds
        ],
        ids=["1969_dec_31", "1969_jan_1", "1900", "epoch", "1970_jan_2"],
    )
    def test_timestamp_ntz_to_python_windows(
        self, epoch, microseconds, expected_year, expected_month, expected_day
    ):
        """Test Windows-specific TIMESTAMP_NTZ for negative epochs (before 1970)."""
        context = ArrowConverterContext()
        result = context.TIMESTAMP_NTZ_to_python_windows(epoch, microseconds)
        assert isinstance(result, datetime)
        assert result.tzinfo is None
        assert result.year == expected_year
        assert result.month == expected_month
        assert result.day == expected_day
        assert result.microsecond == microseconds


class TestTimestampLtzToPython:
    """Test TIMESTAMP_LTZ conversion methods."""

    @pytest.mark.parametrize(
        "timezone_str",
        ["America/New_York", "UTC", "Europe/London", "Asia/Tokyo"],
        ids=["new_york", "utc", "london", "tokyo"],
    )
    def test_timestamp_ltz_to_python_timezones(self, timezone_str):
        """Test TIMESTAMP_LTZ conversion with various session timezones."""
        context = ArrowConverterContext(
            session_parameters={PARAMETER_TIMEZONE: timezone_str}
        )
        result = context.TIMESTAMP_LTZ_to_python(1609459200, 0)
        assert isinstance(result, datetime)
        assert result.tzinfo is not None
        assert str(result.tzinfo) == timezone_str

    def test_timestamp_ltz_to_python_defaults_to_utc(self):
        """Test TIMESTAMP_LTZ defaults to UTC when no timezone is set."""
        context = ArrowConverterContext()  # No timezone parameter
        result = context.TIMESTAMP_LTZ_to_python(1609459200, 0)
        assert isinstance(result, datetime)
        assert result.tzinfo is not None
        assert str(result.tzinfo) == "UTC"

    @pytest.mark.parametrize(
        "microseconds",
        [0, 999999, 500000, 123456],
        ids=["zero", "max", "half", "arbitrary"],
    )
    def test_timestamp_ltz_to_python_microseconds(self, microseconds):
        """Test TIMESTAMP_LTZ conversion preserves microseconds."""
        context = ArrowConverterContext(session_parameters={PARAMETER_TIMEZONE: "UTC"})
        result = context.TIMESTAMP_LTZ_to_python(0, microseconds)
        assert result.microsecond == microseconds

    @pytest.mark.parametrize(
        "epoch,microseconds,timezone_str",
        [
            # Negative epochs (before 1970) - main use case for Windows function
            (-86400, 0, "UTC"),  # 1 day before epoch
            (-31536000, 0, "America/New_York"),  # 1969-01-01
            (-86400, 500000, "Europe/London"),  # with microseconds
            # Positive epochs should also work
            (0, 0, "UTC"),  # epoch
            (86400, 123456, "Asia/Tokyo"),  # 1 day after with microseconds
        ],
        ids=[
            "1969_dec_utc",
            "1969_jan_ny",
            "1969_dec_london",
            "epoch_utc",
            "1970_jan_tokyo",
        ],
    )
    def test_timestamp_ltz_to_python_windows(self, epoch, microseconds, timezone_str):
        """Test Windows-specific TIMESTAMP_LTZ for negative epochs (before 1970)."""
        context = ArrowConverterContext(
            session_parameters={PARAMETER_TIMEZONE: timezone_str}
        )
        result = context.TIMESTAMP_LTZ_to_python_windows(epoch, microseconds)
        assert isinstance(result, datetime)
        assert result.tzinfo is not None

    def test_timestamp_ltz_to_python_windows_defaults_to_utc(self):
        """Test Windows TIMESTAMP_LTZ defaults to UTC when no timezone is set."""
        context = ArrowConverterContext()  # No timezone parameter
        # Use negative epoch to test the actual Windows use case
        result = context.TIMESTAMP_LTZ_to_python_windows(-86400, 0)
        assert isinstance(result, datetime)
        assert result.tzinfo is not None


class TestDecimal128ToDecimal:
    """Test DECIMAL128 conversion."""

    @pytest.mark.parametrize(
        "int_value,scale,expected",
        [
            (123, 0, 123),  # no scale
            (12345, 2, decimal.Decimal("123.45")),  # scale 2
            (-12345, 2, decimal.Decimal("-123.45")),  # negative
            (123456789, 6, decimal.Decimal("123.456789")),  # large scale
            (1, 0, 1),  # minimal value
            (0, 0, 0),  # zero
            (0, 5, decimal.Decimal("0")),  # zero with scale
            (100, 2, decimal.Decimal("1.00")),  # trailing zeros
            (
                99999999999999999999999999999999999999,
                0,
                99999999999999999999999999999999999999,
            ),  # max 38-digit
        ],
        ids=[
            "no_scale",
            "scale_2",
            "negative",
            "large_scale",
            "minimal",
            "zero",
            "zero_with_scale",
            "trailing_zeros",
            "max_38_digit",
        ],
    )
    def test_decimal128_to_decimal(self, int_value, scale, expected):
        """Test DECIMAL128 conversion with various values and scales."""
        context = ArrowConverterContext()
        int128_bytes = int_value.to_bytes(16, byteorder=sys.byteorder, signed=True)
        result = context.DECIMAL128_to_decimal(int128_bytes, scale)
        assert result == expected


class TestDecfloatConversions:
    """Test DECFLOAT conversion methods."""

    @pytest.mark.parametrize(
        "exponent,significand_int,expected",
        [
            (-5, 314159, decimal.Decimal("3.14159")),  # positive
            (-5, -271828, decimal.Decimal("-2.71828")),  # negative significand
            (3, 123, decimal.Decimal("123000")),  # positive exponent
            (0, 42, decimal.Decimal("42")),  # zero exponent
            (-10, 1, decimal.Decimal("0.0000000001")),  # small value
            (10, 1, decimal.Decimal("10000000000")),  # large value
        ],
        ids=[
            "positive",
            "negative_sig",
            "pos_exp",
            "zero_exp",
            "small",
            "large",
        ],
    )
    def test_decfloat_to_decimal(self, exponent, significand_int, expected):
        """Test DECFLOAT to decimal conversion."""
        context = ArrowConverterContext()
        significand = significand_int.to_bytes(16, byteorder="big", signed=True)
        result = context.DECFLOAT_to_decimal(exponent, significand)
        assert result == expected


class TestNumpyConversions:
    """Test numpy conversion methods."""

    @pytest.fixture(autouse=True)
    def check_numpy(self):
        """Skip tests if numpy is not available."""
        pytest.importorskip("numpy")

    @pytest.mark.parametrize(
        "value",
        [3.14159, -2.71828, 0.0, 1e10, -1e-10],
        ids=["pi", "neg_e", "zero", "large", "small_neg"],
    )
    def test_real_to_numpy_float64(self, value):
        """Test REAL to numpy float64 conversion."""
        import numpy as np

        context = ArrowConverterContext()
        result = context.REAL_to_numpy_float64(value)
        assert isinstance(result, np.float64)
        assert result == np.float64(value)

    @pytest.mark.parametrize(
        "value",
        [123456789, -987654321, 0, 9223372036854775807, -9223372036854775808],
        ids=["positive", "negative", "zero", "max_int64", "min_int64"],
    )
    def test_fixed_to_numpy_int64(self, value):
        """Test FIXED to numpy int64 conversion."""
        import numpy as np

        context = ArrowConverterContext()
        result = context.FIXED_to_numpy_int64(value)
        assert isinstance(result, np.int64)
        assert result == np.int64(value)

    @pytest.mark.parametrize(
        "value,scale,expected",
        [
            (12345, 2, 123.45),
            (100, 2, 1.0),
            (1, 3, 0.001),
            (-5000, 2, -50.0),
        ],
        ids=["normal", "even", "small", "negative"],
    )
    def test_fixed_to_numpy_float64(self, value, scale, expected):
        """Test FIXED to numpy float64 with scale."""
        import numpy as np

        context = ArrowConverterContext()
        result = context.FIXED_to_numpy_float64(value, scale)
        assert isinstance(result, np.float64)
        assert result == np.float64(expected)

    @pytest.mark.parametrize(
        "days",
        [0, 18628, 1, 36525, -1],
        ids=["epoch", "2021", "day_1", "2070", "before_epoch"],
    )
    def test_date_to_numpy_datetime64(self, days):
        """Test DATE to numpy datetime64 conversion."""
        import numpy as np

        context = ArrowConverterContext()
        result = context.DATE_to_numpy_datetime64(days)
        assert isinstance(result, np.datetime64)

    @pytest.mark.parametrize(
        "value,scale",
        [(1000000000, 9), (1000000, 6), (1000, 3), (1609459200000000000, 9)],
        ids=["nano_scale9", "micro_scale6", "milli_scale3", "real_ts"],
    )
    def test_timestamp_ntz_one_field_to_numpy_datetime64(self, value, scale):
        """Test TIMESTAMP_NTZ one-field to numpy datetime64."""
        import numpy as np

        context = ArrowConverterContext()
        result = context.TIMESTAMP_NTZ_ONE_FIELD_to_numpy_datetime64(value, scale)
        assert isinstance(result, np.datetime64)

    @pytest.mark.parametrize(
        "epoch,fraction",
        [(1609459200, 0), (0, 0), (1609459200, 500000000)],
        ids=["2021", "epoch", "with_fraction"],
    )
    def test_timestamp_ntz_two_field_to_numpy_datetime64(self, epoch, fraction):
        """Test TIMESTAMP_NTZ two-field to numpy datetime64."""
        import numpy as np

        context = ArrowConverterContext()
        result = context.TIMESTAMP_NTZ_TWO_FIELD_to_numpy_datetime64(epoch, fraction)
        assert isinstance(result, np.datetime64)

    def test_decfloat_to_numpy_float64(self):
        """Test DECFLOAT to numpy float64 conversion."""
        import numpy as np

        context = ArrowConverterContext()
        significand = (314159).to_bytes(16, byteorder="big", signed=True)
        result = context.DECFLOAT_to_numpy_float64(-5, significand)
        assert isinstance(result, np.float64)


class TestIntervalConversions:
    """Test interval conversion methods."""

    @pytest.mark.parametrize(
        "months,expected",
        [(25, "2-1"), (0, "0-0"), (12, "1-0"), (7, "0-7")],
        ids=["2y1m", "zero", "1y", "7m"],
    )
    def test_interval_year_month_to_str(self, months, expected):
        """Test INTERVAL_YEAR_MONTH to string conversion."""
        context = ArrowConverterContext()
        result = context.INTERVAL_YEAR_MONTH_to_str(months)
        assert result == expected

    @pytest.mark.parametrize(
        "nanos,expected_seconds",
        [
            (1_000_000_000, 1.0),  # 1 second
            (0, 0.0),  # zero
            (500_000_000, 0.5),  # half second
            (60_000_000_000, 60.0),  # 1 minute
            (3_600_000_000_000, 3600.0),  # 1 hour
        ],
        ids=["1sec", "zero", "half_sec", "1min", "1hour"],
    )
    def test_interval_day_time_int_to_timedelta(self, nanos, expected_seconds):
        """Test INTERVAL_DAY_TIME int to timedelta."""
        context = ArrowConverterContext()
        result = context.INTERVAL_DAY_TIME_int_to_timedelta(nanos)
        assert isinstance(result, timedelta)
        assert result.total_seconds() == expected_seconds

    @pytest.mark.parametrize(
        "nanos,expected_microseconds",
        [
            (1_500_000, 1500),  # 1.5 milliseconds
            (1_000, 1),  # 1 microsecond
            (999_000, 999),  # just under 1ms
        ],
        ids=["1500us", "1us", "999us"],
    )
    def test_interval_day_time_int_to_timedelta_microseconds(
        self, nanos, expected_microseconds
    ):
        """Test INTERVAL_DAY_TIME int precision (microseconds)."""
        context = ArrowConverterContext()
        result = context.INTERVAL_DAY_TIME_int_to_timedelta(nanos)
        assert result.microseconds == expected_microseconds

    @pytest.mark.parametrize(
        "nanos,expected_seconds",
        [
            (1_000_000_000, 1.0),  # 1 second
            (86_400_000_000_000, 86400.0),  # 1 day
            (0, 0.0),  # zero
        ],
        ids=["1sec", "1day", "zero"],
    )
    def test_interval_day_time_decimal_to_timedelta(self, nanos, expected_seconds):
        """Test INTERVAL_DAY_TIME decimal to timedelta."""
        context = ArrowConverterContext()
        value = nanos.to_bytes(16, byteorder="little", signed=True)
        result = context.INTERVAL_DAY_TIME_decimal_to_timedelta(value)
        assert isinstance(result, timedelta)
        assert result.total_seconds() == expected_seconds


class TestIntervalNumpyConversions:
    """Test interval numpy conversion methods (requires numpy)."""

    @pytest.fixture(autouse=True)
    def check_numpy(self):
        """Skip tests if numpy not available."""
        pytest.importorskip("numpy")

    @pytest.mark.parametrize(
        "months", [12, 0, 24, 1, 100], ids=["1y", "0", "2y", "1m", "8y4m"]
    )
    def test_interval_year_month_to_numpy_timedelta(self, months):
        """Test INTERVAL_YEAR_MONTH to numpy timedelta."""
        import numpy as np

        context = ArrowConverterContext()
        result = context.INTERVAL_YEAR_MONTH_to_numpy_timedelta(months)
        assert isinstance(result, np.timedelta64)

    @pytest.mark.parametrize(
        "nanos",
        [1_000_000_000, 0, 86_400_000_000_000],
        ids=["1sec", "zero", "1day"],
    )
    def test_interval_day_time_int_to_numpy_timedelta(self, nanos):
        """Test INTERVAL_DAY_TIME int to numpy timedelta."""
        import numpy as np

        context = ArrowConverterContext()
        result = context.INTERVAL_DAY_TIME_int_to_numpy_timedelta(nanos)
        assert isinstance(result, np.timedelta64)

    @pytest.mark.parametrize(
        "nanos",
        [86400_000_000_000, 1_000_000_000, 0],
        ids=["1day", "1sec", "zero"],
    )
    def test_interval_day_time_decimal_to_numpy_timedelta(self, nanos):
        """Test INTERVAL_DAY_TIME decimal to numpy timedelta."""
        import numpy as np

        context = ArrowConverterContext()
        value = nanos.to_bytes(16, byteorder="little", signed=True)
        result = context.INTERVAL_DAY_TIME_decimal_to_numpy_timedelta(value)
        assert isinstance(result, np.timedelta64)


class TestZeroEpochConstant:
    """Test ZERO_EPOCH constant."""

    def test_zero_epoch_value(self):
        """Test ZERO_EPOCH is correct."""
        assert ZERO_EPOCH == datetime(1970, 1, 1, 0, 0, 0)
        assert ZERO_EPOCH.tzinfo is None
