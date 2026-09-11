"""TIMESTAMP_TZ type tests for Universal Driver -- numpy consumer.

Mirrors scenarios in ``tests/definitions/shared/types/timestamp_tz.feature``
using ``numpy=True`` cursor fetch (``fetchone`` / ``fetchall``).

Row fetch yields plain ``datetime`` instances, so **each row keeps its own offset**, as in
``python/tests/e2e/types/test_timestamp_tz.py``. The offset arrives as a fixed-offset
``tzinfo`` rather than the named zone the value was written with, because TIMESTAMP_TZ
stores an offset and not a region; assertions therefore compare ``utcoffset()`` and UTC
instants rather than zone names. The one-timezone-per-column collapse that the pandas suite
works around does not apply here.

NULL scalars are ``None``.
"""

from __future__ import annotations

from datetime import UTC, datetime, timedelta, timezone

import pytest

from tests.conftest import with_paramstyle
from tests.e2e.numpy.utils import (
    assert_dtypes,
    execute_and_fetch,
    execute_and_fetch_multiple_batches,
    get_column,
    get_row,
    is_datetime64_tz,
)
from tests.e2e.types.utils import assert_sequential_values, batch_insert


# Constants aligned with python/tests/e2e/types/test_timestamp_tz.py
SESSION_TZ_NAME = "America/New_York"

TZ_PLUS_5 = timezone(timedelta(hours=5))
TZ_MINUS_8 = timezone(timedelta(hours=-8))

TS_2024_JAN = datetime(2024, 1, 15, 10, 30, 0, tzinfo=TZ_PLUS_5)
TS_2024_JUN = datetime(2024, 6, 20, 14, 45, 30, tzinfo=TZ_MINUS_8)
TS_EPOCH = datetime(1970, 1, 1, 0, 0, 0, tzinfo=UTC)
TS_WITH_MICROSECONDS = datetime(2024, 1, 15, 10, 30, 0, 123456, tzinfo=TZ_PLUS_5)

TS_2024_JAN_STR = "2024-01-15 10:30:00 +05:00"
TS_2024_JUN_STR = "2024-06-20 14:45:30 -08:00"
TS_EPOCH_STR = "1970-01-01 00:00:00 +00:00"
TS_WITH_MICROSECONDS_STR = "2024-01-15 10:30:00.123456 +05:00"

LARGE_RESULT_SET_SIZE = 50_000
SEQUENTIAL_BASE = datetime(2024, 1, 1, 0, 0, 0, tzinfo=UTC)


def to_utc(values):
    """Convert timestamps to UTC; treat ``None`` / ``NaT`` as missing."""
    out = []
    for v in values:
        if v is None:
            out.append(None)
        else:
            out.append(v.astimezone(UTC))
    return out


def sequential_timestamp(i):
    """Expected sequential UTC instant for the large-result-set generators."""
    return SEQUENTIAL_BASE + timedelta(seconds=i)


def compare_ts_utc(actual, expected):
    """Compare timestamps by UTC instant (offsets may vary in representation)."""
    return actual.astimezone(UTC) == expected


@pytest.fixture(autouse=True)
def _set_session_timezone(cursor):
    """Set session timezone to a non-UTC zone for all tests in this module."""
    cursor.execute(f"ALTER SESSION SET TIMEZONE = '{SESSION_TZ_NAME}'")


LITERAL_SELECT_TEST_CASES = [
    ("basic", [TS_2024_JAN_STR, TS_2024_JUN_STR], [TS_2024_JAN, TS_2024_JUN]),
    ("epoch", [TS_EPOCH_STR], [TS_EPOCH]),
    ("microseconds", [TS_WITH_MICROSECONDS_STR], [TS_WITH_MICROSECONDS]),
]

EDGE_DATE_TEST_CASES = [
    ("year 9999", "9999-12-31 23:59:59 +00:00", datetime(9999, 12, 31, 23, 59, 59, tzinfo=UTC)),
    ("year 1900", "1900-01-01 00:00:00 +00:00", datetime(1900, 1, 1, 0, 0, 0, tzinfo=UTC)),
    ("pre-epoch", "1960-06-15 12:00:00 +05:00", datetime(1960, 6, 15, 12, 0, 0, tzinfo=TZ_PLUS_5)),
]

TABLE_SELECT_TEST_CASES = [
    ("basic", [TS_2024_JAN_STR, TS_2024_JUN_STR], [TS_2024_JAN, TS_2024_JUN]),
    ("epoch", [TS_EPOCH_STR, TS_2024_JAN_STR], [TS_EPOCH, TS_2024_JAN]),
    ("microseconds", [TS_2024_JAN_STR, TS_WITH_MICROSECONDS_STR], [TS_2024_JAN, TS_WITH_MICROSECONDS]),
    ("null", [None, TS_2024_JAN_STR], [TS_2024_JAN, None]),
]

TZ_P0530 = timezone(timedelta(hours=5, minutes=30))
TZ_M0800 = timezone(timedelta(hours=-8))
TZ_P0430 = timezone(timedelta(hours=4, minutes=30))
TZ_M0230 = timezone(timedelta(hours=-2, minutes=-30))

# UTC instants for literals in test_should_preserve_timezone_offset_for_timestamp_tz (column order).
PRESERVE_OFFSETS_EXPECTED_UTCS = tuple(
    datetime(2024, 1, 15, 10, 30, tzinfo=tz).astimezone(UTC) for tz in (TZ_P0530, TZ_M0800, UTC, TZ_P0430, TZ_M0230)
)


class TestFetchNumpyTimestampTzTypeCasting:
    """Type-casting coverage for TIMESTAMP_TZ via numpy fetch."""

    def test_should_cast_timestamp_tz_values_to_appropriate_type(self, cursor):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # When Query "SELECT '2024-01-15 10:30:00 +05:00'::TIMESTAMP_TZ" is executed
        df = execute_and_fetch(cursor, f"SELECT '{TS_2024_JAN_STR}'::TIMESTAMP_TZ")

        # Then All values should be returned as appropriate type
        assert_dtypes(df, [is_datetime64_tz])
        val = get_row(df, 0)[0]
        assert isinstance(val, datetime)
        # And Values should have timezone info
        assert val.tzinfo is not None
        assert val.astimezone(UTC) == TS_2024_JAN.astimezone(UTC)


class TestFetchNumpyTimestampTzLiteral:
    """SELECT-with-literal coverage via numpy fetch."""

    @pytest.mark.parametrize(
        "values_name,query_values,expected_values",
        LITERAL_SELECT_TEST_CASES,
        ids=[c[0] for c in LITERAL_SELECT_TEST_CASES],
    )
    def test_should_select_timestamp_tz_values(self, cursor, values_name, query_values, expected_values):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # When Query "SELECT <query_values>" is executed
        select_cols = ", ".join(f"'{v}'::TIMESTAMP_TZ" for v in query_values)
        df = execute_and_fetch(cursor, f"SELECT {select_cols}")

        # Then Result should contain timestamps <expected_values>
        row = get_row(df, 0)
        assert_dtypes(df, [is_datetime64_tz for _ in expected_values])
        assert tuple(to_utc(row)) == tuple(to_utc(expected_values))
        # And Values should have timezone info
        assert all(v.tzinfo is not None for v in row)

    def test_should_preserve_timezone_offset_for_timestamp_tz(self, cursor):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # When Query "SELECT
        #   '2024-01-15 10:30:00 +05:30'::TIMESTAMP_TZ,
        #   '2024-01-15 10:30:00 -08:00'::TIMESTAMP_TZ,
        #   '2024-01-15 10:30:00 +00:00'::TIMESTAMP_TZ,
        #   '2024-01-15 10:30:00 +04:30'::TIMESTAMP_TZ,
        #   '2024-01-15 10:30:00 -02:30'::TIMESTAMP_TZ" is executed
        df = execute_and_fetch(
            cursor,
            "SELECT "
            "'2024-01-15 10:30:00 +05:30'::TIMESTAMP_TZ, "
            "'2024-01-15 10:30:00 -08:00'::TIMESTAMP_TZ, "
            "'2024-01-15 10:30:00 +00:00'::TIMESTAMP_TZ, "
            "'2024-01-15 10:30:00 +04:30'::TIMESTAMP_TZ, "
            "'2024-01-15 10:30:00 -02:30'::TIMESTAMP_TZ",
        )

        # Then Result should preserve offsets [+05:30, -08:00, +00:00, +04:30, -02:30]
        assert_dtypes(df, [is_datetime64_tz for _ in range(5)])
        row = get_row(df, 0)
        for cell, exp_utc in zip(row, PRESERVE_OFFSETS_EXPECTED_UTCS, strict=True):
            assert cell.astimezone(UTC) == exp_utc
            assert cell.tzinfo is not None

    @pytest.mark.parametrize(
        "values_name,query_str,expected",
        EDGE_DATE_TEST_CASES,
        ids=[c[0] for c in EDGE_DATE_TEST_CASES],
    )
    def test_should_select_edge_date_timestamp_tz_values(self, cursor, values_name, query_str, expected):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # When Query "SELECT <query_values>" is executed
        df = execute_and_fetch(cursor, f"SELECT '{query_str}'::TIMESTAMP_TZ")

        # Then Result should contain timestamps <expected_values>
        assert_dtypes(df, [is_datetime64_tz])
        val = get_row(df, 0)[0]
        assert val.astimezone(UTC) == expected.astimezone(UTC)
        # And Values should have timezone info
        assert val.tzinfo is not None

    def test_should_handle_null_values_for_timestamp_tz(self, cursor):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # When Query "SELECT '2024-01-15 10:30:00 +05:00'::TIMESTAMP_TZ, NULL::TIMESTAMP_TZ" is executed
        df = execute_and_fetch(
            cursor,
            f"SELECT '{TS_2024_JAN_STR}'::TIMESTAMP_TZ, NULL::TIMESTAMP_TZ",
        )

        # Then Result should contain [2024-01-15 10:30:00 +05:00, NULL]
        assert_dtypes(df, [is_datetime64_tz, is_datetime64_tz])
        row = get_row(df, 0)
        assert to_utc(row) == [TS_2024_JAN.astimezone(UTC), None]
        assert row[0].tzinfo is not None
        assert row[1] is None

    def test_should_download_large_result_set_with_multiple_chunks_for_timestamp_tz(self, cursor):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # When Query "SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1,
        #   '2024-01-01 00:00:00 +00:00'::TIMESTAMP_TZ) as ts
        #   FROM TABLE(GENERATOR(ROWCOUNT => 50000)) ORDER BY ts" is executed
        combined = execute_and_fetch_multiple_batches(
            cursor,
            "SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, "
            "'2024-01-01 00:00:00 +00:00'::TIMESTAMP_TZ) as ts "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) ORDER BY ts",
        )

        # Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00 +00:00
        col = get_column(combined, 0)
        assert_sequential_values(
            col,
            LARGE_RESULT_SET_SIZE,
            transform=sequential_timestamp,
            compare=compare_ts_utc,
        )


class TestFetchNumpyTimestampTzTable:
    """Table-based scenarios via numpy fetch."""

    @pytest.mark.parametrize(
        "values_name,insert_values,expected_values",
        TABLE_SELECT_TEST_CASES,
        ids=[c[0] for c in TABLE_SELECT_TEST_CASES],
    )
    def test_should_select_values_from_table_for_timestamp_tz(
        self,
        execute_query,
        cursor,
        tmp_schema,
        values_name,
        insert_values,
        expected_values,
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # And Table with TIMESTAMP_TZ column exists with values <insert_values>
        table_name = f"{tmp_schema}.np_tstz_table_{values_name}"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col TIMESTAMP_TZ)")
        batch_insert(execute_query, table_name, insert_values, quote_strings=True)

        # When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col NULLS LAST")

        # Then Result should contain timestamps <expected_values>
        col = get_column(df, 0)
        assert_dtypes(df, [is_datetime64_tz])
        assert to_utc(col) == to_utc(expected_values)
        # And Values should have timezone info
        for actual, expected in zip(col, expected_values, strict=True):
            if expected is None:
                assert actual is None
                continue
            assert actual.utcoffset() == expected.utcoffset()

    def test_should_download_large_result_set_with_multiple_chunks_from_table_for_timestamp_tz(
        self, execute_query, cursor, tmp_schema
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # And Table with TIMESTAMP_TZ column exists with 50000 sequential timestamp values
        table_name = f"{tmp_schema}.np_tstz_large_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col TIMESTAMP_TZ)")
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, "
            f"'2024-01-01 00:00:00 +00:00'::TIMESTAMP_TZ) "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE}))"
        )

        # When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
        combined = execute_and_fetch_multiple_batches(cursor, f"SELECT * FROM {table_name} ORDER BY col NULLS LAST")

        # Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00 +00:00
        col = get_column(combined, 0)
        assert_sequential_values(
            col,
            LARGE_RESULT_SET_SIZE,
            transform=sequential_timestamp,
            compare=compare_ts_utc,
        )


@with_paramstyle("qmark")
class TestFetchNumpyTimestampTzBinding:
    """Parameter-binding scenarios via numpy fetch.

    Same contract as ``python/tests/e2e/types/test_timestamp_tz.py``: binding +
    ``?::TIMESTAMP_TZ`` does not guarantee the same UTC instants as the Python
    constants; we verify dtypes and timezone-aware cells only.
    """

    def test_should_select_timestamp_tz_using_parameter_binding(self, cursor):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # When Query "SELECT ?::TIMESTAMP_TZ, ?::TIMESTAMP_TZ" is executed with bound timestamp values
        df = execute_and_fetch(
            cursor,
            "SELECT ?::TIMESTAMP_TZ, ?::TIMESTAMP_TZ",
            params=(TS_2024_JAN, TS_2024_JUN),
        )

        # Then Result should contain the bound timestamps
        row = get_row(df, 0)
        assert_dtypes(df, [is_datetime64_tz, is_datetime64_tz])

        # And Values should have timezone info
        assert row[0].tzinfo is not None
        assert row[1].tzinfo is not None

    def test_should_select_null_timestamp_tz_using_parameter_binding(self, cursor):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # When Query "SELECT ?::TIMESTAMP_TZ" is executed with bound NULL value
        df = execute_and_fetch(cursor, "SELECT ?::TIMESTAMP_TZ", params=(None,))

        # Then Result should contain [NULL]
        assert_dtypes(df, [is_datetime64_tz])
        assert get_row(df, 0)[0] is None

    def test_should_insert_timestamp_tz_using_parameter_binding(
        self, execute_query, executemany_insert, cursor, tmp_schema
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # And Table with TIMESTAMP_TZ column exists
        table_name = f"{tmp_schema}.np_tstz_bind"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col TIMESTAMP_TZ)")

        # When Timestamp values are bulk-inserted using multirow binding
        test_values = [
            (TS_2024_JAN,),
            (TS_2024_JUN,),
            (None,),
        ]
        executemany_insert(table_name, f"INSERT INTO {table_name} VALUES (?)", test_values)

        # And Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col NULLS LAST")

        # Then SELECT should return the same values in any order
        col = get_column(df, 0)
        assert_dtypes(df, [is_datetime64_tz])
        assert col[0].tzinfo is not None
        assert col[1].tzinfo is not None
        assert col[2] is None


class TestFetchNumpyTimestampTzPrecision:
    """TIMESTAMP_TZ numpy fetch yields tz-aware ``datetime``, not ``numpy.datetime64``.

    The Gherkin scenario still names microsecond truncation. Row fetch cannot surface
    digits 7–9, so this asserts the UTC instant and ``microsecond`` from
    ``cursor.fetchall()`` with ``numpy=True``.
    """

    @pytest.mark.parametrize(
        "input_str,expected_microsecond",
        [
            ("2024-01-15 10:30:00.123456789 +05:00", 123456),
            ("2024-01-15 10:30:00.999999999 +05:00", 999999),
        ],
    )
    def test_should_truncate_nanosecond_precision_to_microseconds_for_timestamp_tz(
        self, cursor, input_str, expected_microsecond
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # When Query "SELECT '<input>'::TIMESTAMP_TZ" is executed
        df = execute_and_fetch(cursor, f"SELECT '{input_str}'::TIMESTAMP_TZ")

        # Then Result should contain [<expected>]
        assert_dtypes(df, [is_datetime64_tz])
        val = get_row(df, 0)[0]
        expected_ts = datetime.fromisoformat(input_str.replace(" ", "T", 1))
        assert val.astimezone(UTC) == expected_ts.astimezone(UTC)
        assert val.microsecond == expected_microsecond
        # And Values should have timezone info
        assert val.tzinfo is not None


class TestFetchNumpyTimestampTzNegativeEpoch:
    """Sub-second TIMESTAMP_TZ instants just before the Unix epoch, via numpy fetch.

    The server pre-floors the seconds-since-epoch decimal, so Arrow surfaces
    1969-12-31 23:59:59.999999999 with its borrowed second already applied.
    TIMESTAMP_TZ row fetch yields tz-aware ``datetime``, so assertions compare the
    UTC instant (Python ``datetime`` is microsecond-capped). The two scales cover
    both TZ builders: scale 9 (struct epoch+fraction+tz) and scale 3 (combined Int64 + tz).
    """

    @pytest.mark.parametrize(
        "query,expected",
        [
            ("SELECT '1969-12-31 23:59:59.999999999 +00:00'::TIMESTAMP_TZ(9)", "1969-12-31 23:59:59.999999999+00:00"),
            ("SELECT '1969-12-31 23:59:58.5 +00:00'::TIMESTAMP_TZ(3)", "1969-12-31 23:59:58.5+00:00"),
        ],
    )
    def test_should_select_sub_second_timestamp_tz_values_before_epoch(self, cursor, query, expected):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # When Sub-second timestamp_tz values before the epoch are selected
        df = execute_and_fetch(cursor, query)

        # Then Result should contain the expected sub-second values before the epoch
        assert_dtypes(df, [is_datetime64_tz])
        val = get_row(df, 0)[0]
        assert val.tzinfo is not None
        assert val.astimezone(UTC) == datetime.fromisoformat(expected).astimezone(UTC)
