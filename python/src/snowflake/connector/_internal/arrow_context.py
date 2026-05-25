"""Arrow converter context for type conversions.

This module is based on the original snowflake-connector-python arrow_context.py
and provides Python helper functions for Arrow conversions in the C++ converters.
"""

from __future__ import annotations

import decimal

from datetime import datetime, timedelta, timezone, tzinfo
from logging import getLogger
from sys import byteorder
from typing import TYPE_CHECKING

import pytz

from .extras import numpy as np
from .extras import tzlocal, MissingOptionalDependency

if TYPE_CHECKING:
    from numpy import datetime64, float64, int64, timedelta64

    from ..connection import Connection


ZERO_EPOCH = datetime.fromtimestamp(0, timezone.utc).replace(tzinfo=None)
PARAMETER_TIMEZONE = "TIMEZONE"
logger = getLogger(__name__)


def _generate_tzinfo_from_tzoffset(tzoffset_minutes: int) -> tzinfo:
    """Generate tzinfo object from tzoffset."""
    return pytz.FixedOffset(tzoffset_minutes)


def interval_year_month_to_string(interval: int, scale: int) -> str:
    """Convert a year-month interval to a string.

    Args:
        interval: The year-month interval value in months.
        scale: The scale of the interval which represents subtype as follows:
            0: INTERVAL YEAR TO MONTH
            1: INTERVAL YEAR
            2: INTERVAL MONTH

    Returns:
        The string representation of the interval.
    """
    sign = "+" if interval >= 0 else "-"
    interval = abs(interval)
    if scale == 2:  # INTERVAL MONTH
        return f"{sign}{interval}"
    years = interval // 12
    if scale == 1:  # INTERVAL YEAR
        return f"{sign}{years}"
    # INTERVAL YEAR TO MONTH
    months = interval % 12
    return f"{sign}{years}-{months:02}"


class ArrowConverterContext:
    """Python helper functions for arrow conversions.

    Windows timestamp functions are necessary because Windows cannot handle -ve timestamps.
    Putting the OS check into the non-windows function would probably take up more CPU cycles then
    just deciding this at compile time.
    """

    def __init__(
        self,
        timezone: str | None = None,
    ) -> None:
        self._timezone = timezone

    @classmethod
    def create(cls, connection: Connection) -> ArrowConverterContext:
        """Create a context by fetching the session timezone."""
        tz = connection._get_session_parameter(PARAMETER_TIMEZONE)
        return cls(timezone=tz)

    @property
    def timezone(self) -> str | None:
        return self._timezone

    @timezone.setter
    def timezone(self, tz: str | None) -> None:
        self._timezone = tz

    def _get_session_tz(self) -> tzinfo:
        """Get the session timezone or use the local computer's timezone."""
        try:
            tz = "UTC" if not self.timezone else str(self.timezone)
            return pytz.timezone(tz)
        except pytz.exceptions.UnknownTimeZoneError:
            logger.warning("converting to tzinfo failed")
            if not isinstance(tzlocal, MissingOptionalDependency):
                return tzlocal.get_localzone()  # type: ignore[no-any-return]
            else:
                return timezone.utc

    def TIMESTAMP_TZ_to_python(self, epoch: int, microseconds: int, tz: int) -> datetime:
        tzinfo = _generate_tzinfo_from_tzoffset(tz - 1440)
        return datetime.fromtimestamp(epoch, tz=tzinfo) + timedelta(microseconds=microseconds)

    def TIMESTAMP_TZ_to_python_windows(self, epoch: int, microseconds: int, tz: int) -> datetime:
        tzinfo = _generate_tzinfo_from_tzoffset(tz - 1440)
        t = ZERO_EPOCH + timedelta(seconds=epoch, microseconds=microseconds)
        if pytz.utc != tzinfo:
            offset = tzinfo.utcoffset(t)
            if offset is not None:
                t += offset
        return t.replace(tzinfo=tzinfo)

    def TIMESTAMP_NTZ_to_python(self, epoch: int, microseconds: int) -> datetime:
        return datetime.fromtimestamp(epoch, timezone.utc).replace(tzinfo=None) + timedelta(microseconds=microseconds)

    def TIMESTAMP_NTZ_to_python_windows(self, epoch: int, microseconds: int) -> datetime:
        return ZERO_EPOCH + timedelta(seconds=epoch, microseconds=microseconds)

    def TIMESTAMP_LTZ_to_python(self, epoch: int, microseconds: int) -> datetime:
        tzinfo = self._get_session_tz()
        return datetime.fromtimestamp(epoch, tz=tzinfo) + timedelta(microseconds=microseconds)

    def TIMESTAMP_LTZ_to_python_windows(self, epoch: int, microseconds: int) -> datetime:
        tzinfo = self._get_session_tz()
        ts = ZERO_EPOCH + timedelta(seconds=epoch, microseconds=microseconds)
        return pytz.utc.localize(ts, is_dst=False).astimezone(tzinfo)

    def REAL_to_numpy_float64(self, py_double: float) -> float64:
        return np.float64(py_double)  # type: ignore[no-any-return]

    def FIXED_to_numpy_int64(self, py_long: int) -> int64:
        return np.int64(py_long)  # type: ignore[no-any-return]

    def FIXED_to_numpy_float64(self, py_long: int, scale: int) -> float64:
        return np.float64(decimal.Decimal(py_long).scaleb(-scale))  # type: ignore[no-any-return]

    def DATE_to_numpy_datetime64(self, py_days: int) -> datetime64:
        return np.datetime64(py_days, "D")  # type: ignore[no-any-return]

    def TIMESTAMP_NTZ_ONE_FIELD_to_numpy_datetime64(self, value: int, scale: int) -> datetime64:
        nanoseconds = int(decimal.Decimal(value).scaleb(9 - scale))
        return np.datetime64(nanoseconds, "ns")  # type: ignore[no-any-return]

    def TIMESTAMP_NTZ_TWO_FIELD_to_numpy_datetime64(self, epoch: int, fraction: int) -> datetime64:
        nanoseconds = int(decimal.Decimal(epoch).scaleb(9) + decimal.Decimal(fraction))
        return np.datetime64(nanoseconds, "ns")  # type: ignore[no-any-return]

    def DECIMAL128_to_decimal_or_int(self, int128_bytes: bytes, scale: int) -> decimal.Decimal | int:
        """When scale=0 (integer), returns Python int. When scale>0 (decimal), returns Python Decimal.

        Large scale integers (int128) fall back here as they're not supported by native C++ conversion.
        """
        int128 = int.from_bytes(int128_bytes, byteorder=byteorder, signed=True)
        if scale == 0:
            return int128
        digits = [int(digit) for digit in str(int128) if digit != "-"]
        sign = int128 < 0
        return decimal.Decimal((sign, digits, -scale))

    def DECFLOAT_to_decimal(self, exponent: int, significand: bytes) -> decimal.Decimal:
        # significand is two's complement big endian.
        significand_int = int.from_bytes(significand, byteorder="big", signed=True)
        return decimal.Decimal(significand_int).scaleb(exponent)

    def DECFLOAT_to_numpy_float64(self, exponent: int, significand: bytes) -> float64:
        return np.float64(self.DECFLOAT_to_decimal(exponent, significand))  # type: ignore[no-any-return]

    def INTERVAL_YEAR_MONTH_to_str(self, months: int, scale: int) -> str:
        return interval_year_month_to_string(months, scale)

    def INTERVAL_YEAR_MONTH_to_numpy_timedelta(self, months: int, scale: int) -> timedelta64:
        if scale == 1:
            return np.timedelta64(months // 12, "Y")  # type: ignore[no-any-return]
        return np.timedelta64(months, "M")  # type: ignore[no-any-return]

    def INTERVAL_DAY_TIME_int_to_numpy_timedelta(self, nanos: int) -> timedelta64:
        return np.timedelta64(nanos, "ns")  # type: ignore[no-any-return]

    def INTERVAL_DAY_TIME_int_to_timedelta(self, nanos: int) -> timedelta:
        # Python timedelta only supports microsecond precision. We receive value in
        # nanoseconds.
        return timedelta(microseconds=nanos // 1000)

    def INTERVAL_DAY_TIME_decimal_to_numpy_timedelta(self, value: bytes) -> timedelta64:
        # Snowflake supports up to 9 digits leading field precision for the day-time
        # interval. That when represented in nanoseconds can not be stored in a 64-bit
        # integer. So we send these as Decimal128 from server to client.
        # Arrow uses little-endian by default.
        # https://arrow.apache.org/docs/format/Columnar.html#byte-order-endianness
        nanos = int.from_bytes(value, byteorder="little", signed=True)
        # Numpy timedelta only supports up to 64-bit integers, so we need to change the
        # unit to milliseconds to avoid overflow.
        # Max value received from server
        #   = 10**9 * NANOS_PER_DAY - 1
        #   = 86399999999999999999999 nanoseconds
        #   = 86399999999999999 milliseconds
        # math.log2(86399999999999999) = 56.3 < 64
        return np.timedelta64(nanos // 1_000_000, "ms")  # type: ignore[no-any-return]

    def INTERVAL_DAY_TIME_decimal_to_timedelta(self, value: bytes) -> timedelta:
        # Snowflake supports up to 9 digits leading field precision for the day-time
        # interval. That when represented in nanoseconds can not be stored in a 64-bit
        # integer. So we send these as Decimal128 from server to client.
        # Arrow uses little-endian by default.
        # https://arrow.apache.org/docs/format/Columnar.html#byte-order-endianness
        nanos = int.from_bytes(value, byteorder="little", signed=True)
        # Python timedelta only supports microsecond precision. We receive value in
        # nanoseconds.
        return timedelta(microseconds=nanos // 1000)
