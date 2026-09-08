"""Out-of-range nanosecond timestamps on Arrow / pandas fetch.

``datetime64[ns]`` spans roughly 1677-09-21 to 2262-04-11, so a Snowflake
``TIMESTAMP(9)`` at year 0001 or 9999 overflows int64 nanoseconds. When the
fraction also carries sub-microsecond digits the value cannot be downscaled to
microseconds, and the fetch reports a catchable error; a C++ exception escaping
the converter instead would terminate the interpreter.

``force_microsecond_precision=True`` drops digits 7-9 and fetches the same
value successfully.

The assertions match on the message rather than the exception class because
the reference driver wraps this failure in a different class.
"""

from __future__ import annotations

import pandas as pd
import pyarrow as pa
import pytest

from tests.e2e.types.utils import assert_connection_is_open


OUT_OF_RANGE_NS_QUERIES = [
    pytest.param(
        "SELECT '9999-12-31 23:59:59.123456789'::TIMESTAMP_NTZ(9) AS ts",
        id="year 9999 ntz",
    ),
    pytest.param(
        "SELECT '0001-01-01 00:00:00.123456789'::TIMESTAMP_NTZ(9) AS ts",
        id="year 0001 ntz",
    ),
    pytest.param(
        "SELECT '9999-12-31 23:59:59.123456789'::TIMESTAMP_LTZ(9) AS ts",
        id="year 9999 ltz",
    ),
    pytest.param(
        "SELECT '9999-12-31 23:59:59.123456789 +00:00'::TIMESTAMP_TZ(9) AS ts",
        id="year 9999 tz",
    ),
]

FORCED_MICROSECOND_QUERY = "SELECT '9999-12-31 23:59:59.123456789'::TIMESTAMP_NTZ(9) AS ts"


class TestOutOfRangeNanosecondTimestampFetch:
    @pytest.mark.parametrize("sql", OUT_OF_RANGE_NS_QUERIES)
    def test_should_raise_error_when_fetching_out_of_range_nanosecond_timestamp_as_pandas(
        self, execute_query, cursor, sql
    ):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And Session TIMEZONE is UTC
        cursor.execute("ALTER SESSION SET TIMEZONE = 'UTC'")

        # When Query "<sql>" is executed
        cursor.execute(sql)

        # And fetch_pandas_all is called
        with pytest.raises(Exception, match="overflows int64 range") as exc_info:
            cursor.fetch_pandas_all()

        # Then The error names the supported nanosecond range
        assert "1677-09-21" in str(exc_info.value)

    @pytest.mark.parametrize("sql", OUT_OF_RANGE_NS_QUERIES)
    def test_should_raise_error_when_fetching_out_of_range_nanosecond_timestamp_as_arrow(
        self, execute_query, cursor, sql
    ):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And Session TIMEZONE is UTC
        cursor.execute("ALTER SESSION SET TIMEZONE = 'UTC'")

        # When Query "<sql>" is executed
        cursor.execute(sql)

        # And fetch_arrow_all is called
        with pytest.raises(Exception, match="overflows int64 range") as exc_info:
            cursor.fetch_arrow_all()

        # Then The error names the supported nanosecond range
        assert "1677-09-21" in str(exc_info.value)

    def test_should_fetch_out_of_range_nanosecond_timestamp_as_arrow_when_microsecond_precision_is_forced(
        self, execute_query, cursor
    ):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And Session TIMEZONE is UTC
        cursor.execute("ALTER SESSION SET TIMEZONE = 'UTC'")

        # When Query "SELECT '9999-12-31 23:59:59.123456789'::TIMESTAMP_NTZ(9) AS ts" is executed
        cursor.execute(FORCED_MICROSECOND_QUERY)

        # And fetch_arrow_all is called with force_microsecond_precision=True
        table = cursor.fetch_arrow_all(force_microsecond_precision=True)

        # Then Column TS is a microsecond timestamp holding 9999-12-31 23:59:59.123456
        ts_type = table.schema.field("TS").type
        assert pa.types.is_timestamp(ts_type)
        assert ts_type.unit == "us"
        value = table.column("TS")[0].as_py()
        assert (value.year, value.month, value.day) == (9999, 12, 31)
        assert value.microsecond == 123456

    def test_should_fetch_out_of_range_nanosecond_timestamp_as_pandas_when_microsecond_precision_is_forced(
        self, execute_query, cursor
    ):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And Session TIMEZONE is UTC
        cursor.execute("ALTER SESSION SET TIMEZONE = 'UTC'")

        # When Query "SELECT '9999-12-31 23:59:59.123456789'::TIMESTAMP_NTZ(9) AS ts" is executed
        cursor.execute(FORCED_MICROSECOND_QUERY)

        # And fetch_pandas_all is called with force_microsecond_precision=True
        df = cursor.fetch_pandas_all(force_microsecond_precision=True)

        # Then Column TS holds 9999-12-31 23:59:59.123456
        value = df["TS"].iloc[0]
        assert isinstance(value, pd.Timestamp)
        assert (value.year, value.month, value.day) == (9999, 12, 31)
        assert value.microsecond == 123456
