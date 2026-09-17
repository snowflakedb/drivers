"""Extreme INTERVAL DAY TO SECOND values on Arrow / pandas fetch.

Snowflake stores day-time intervals as a nanosecond count. Magnitudes past
about 106_751 days no longer fit in int64 nanoseconds, so the wire type is
Decimal128. ``convertBatch`` always emits Arrow ``duration[ns]`` by taking the
low 64 bits of that count (``ArrowDecimalGetIntUnsafe``), so pandas returns a
wrapped ``timedelta64[ns]`` instead of erroring or downscaling.

INTERVAL support requires ENABLE_INTERVAL_TYPE to be active on the account.
"""

from __future__ import annotations

import pandas as pd
import pyarrow as pa
import pytest

from tests.e2e.pandas.utils import execute_and_fetch, is_timedelta
from tests.e2e.types.utils import assert_connection_is_open


NANOS_PER_SECOND = 1_000_000_000
NANOS_PER_DAY = 86_400 * NANOS_PER_SECOND
EXTREME_DAY_TIME_NS = (
    999_999_999 * NANOS_PER_DAY + 23 * 3_600 * NANOS_PER_SECOND + 59 * 60 * NANOS_PER_SECOND + 59 * NANOS_PER_SECOND
)
EXTREME_CASES = [
    ("SELECT '999999999 23:59:59'::INTERVAL DAY TO SECOND AS iv", EXTREME_DAY_TIME_NS),
    ("SELECT '-999999999 23:59:59'::INTERVAL DAY TO SECOND AS iv", -EXTREME_DAY_TIME_NS),
]


def _as_int64(value: int) -> int:
    return int((value + 2**63) % 2**64 - 2**63)


class TestIntervalDayTimePandasOverflow:
    @pytest.mark.parametrize("sql,nanos", EXTREME_CASES, ids=["spec max", "spec min"])
    def test_should_silently_overflow_extreme_interval_day_time_when_fetched_as_pandas(
        self, execute_query, cursor, sql, nanos
    ):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query selecting a spec-extreme INTERVAL DAY TO SECOND is executed
        df = execute_and_fetch(cursor, sql)

        # Then fetch succeeds and the timedelta's int64 nanosecond payload is the
        # wrapped low 64 bits, not the original nanosecond count
        assert is_timedelta(df.dtypes.iloc[0])
        value = df.iloc[0, 0]
        assert isinstance(value, pd.Timedelta)
        wrapped = _as_int64(nanos)
        assert int(value.value) == wrapped
        assert wrapped != nanos

    @pytest.mark.parametrize("sql,nanos", EXTREME_CASES, ids=["spec max", "spec min"])
    def test_should_silently_overflow_extreme_interval_day_time_when_fetched_as_arrow(
        self, execute_query, cursor, sql, nanos
    ):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query selecting a spec-extreme INTERVAL DAY TO SECOND is executed
        cursor.execute(sql)
        table = cursor.fetch_arrow_all()

        # Then fetch succeeds with duration[ns] holding the wrapped int64 count
        duration_type = table.schema.field(0).type
        assert pa.types.is_duration(duration_type)
        assert duration_type.unit == "ns"
        wrapped = _as_int64(nanos)
        got = table.column(0)[0].cast(pa.int64()).as_py()
        assert got == wrapped
        assert wrapped != nanos
