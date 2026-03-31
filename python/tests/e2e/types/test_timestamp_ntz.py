"""TIMESTAMP_NTZ type tests for Universal Driver.

TIMESTAMP_NTZ (No Time Zone) stores a wall-clock datetime without any timezone.
Values are preserved exactly as entered; session timezone has no effect.
Python type: datetime with tzinfo=None (naive).

All SQL string literals omit timezone offset because NTZ preserves the
wall-clock value regardless of session timezone, making assertions
deterministic without UTC normalization.
"""

from __future__ import annotations

from datetime import datetime, timedelta, timezone

import pytest

from ...conftest import with_paramstyle
from .utils import assert_datetime_type, assert_sequential_values, batch_insert


# =============================================================================
# EXPECTED DATETIME VALUES (naive, no tzinfo)
# =============================================================================
TS_2024_JAN = datetime(2024, 1, 15, 10, 30, 0)
TS_2024_JUN = datetime(2024, 6, 20, 14, 45, 30)
TS_EPOCH = datetime(1970, 1, 1, 0, 0, 0)
TS_WITH_MICROSECONDS = datetime(2024, 1, 15, 10, 30, 0, 123456)

# =============================================================================
# SQL STRING REPRESENTATIONS (no timezone offset)
# =============================================================================
TS_2024_JAN_STR = "2024-01-15 10:30:00"
TS_2024_JUN_STR = "2024-06-20 14:45:30"
TS_EPOCH_STR = "1970-01-01 00:00:00"
TS_WITH_MICROSECONDS_STR = "2024-01-15 10:30:00.123456"

# =============================================================================
# LARGE RESULT SET
# =============================================================================
LARGE_RESULT_SET_SIZE = 50_000
SEQUENTIAL_BASE = datetime(2024, 1, 1, 0, 0, 0)


def sequential_timestamp(i):
    """Transform index to expected sequential naive timestamp."""
    return SEQUENTIAL_BASE + timedelta(seconds=i)


class TestTimestampNtzTypeCasting:
    """Tests for TIMESTAMP_NTZ type casting to appropriate type."""

    def test_should_cast_timestamp_ntz_values_to_appropriate_type(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP_NTZ" is executed
        result = execute_query(f"SELECT '{TS_2024_JAN_STR}'::TIMESTAMP_NTZ", single_row=True)

        # Then All values should be returned as appropriate type
        assert_datetime_type(result, require_tzinfo=False)
        # And Values should not have timezone info
        assert result[0].tzinfo is None


class TestTimestampNtzLiteral:
    """Tests for TIMESTAMP_NTZ type using SELECT with literals (no tables)."""

    @pytest.mark.parametrize(
        "query_values,expected_values",
        [
            ([TS_2024_JAN_STR, TS_2024_JUN_STR], (TS_2024_JAN, TS_2024_JUN)),
            ([TS_EPOCH_STR], (TS_EPOCH,)),
            ([TS_WITH_MICROSECONDS_STR], (TS_WITH_MICROSECONDS,)),
        ],
    )
    def test_should_select_timestamp_ntz_values(self, execute_query, query_values, expected_values):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT <query_values>" is executed
        select_cols = ", ".join(f"'{v}'::TIMESTAMP_NTZ" for v in query_values)
        result = execute_query(f"SELECT {select_cols}", single_row=True)

        # Then Result should contain timestamps <expected_values>
        assert tuple(result) == expected_values
        # And Values should not have timezone info
        assert_datetime_type(result, require_tzinfo=False)
        assert all(v.tzinfo is None for v in result)

    def test_should_handle_null_values_for_timestamp_ntz(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP_NTZ, NULL::TIMESTAMP_NTZ" is executed
        result = execute_query(
            f"SELECT '{TS_2024_JAN_STR}'::TIMESTAMP_NTZ, NULL::TIMESTAMP_NTZ",
            single_row=True,
        )

        # Then Result should contain [2024-01-15 10:30:00, NULL]
        assert_datetime_type(result, can_be_none=True, require_tzinfo=False)
        assert tuple(result) == (TS_2024_JAN, None)

    def test_should_download_large_result_set_with_multiple_chunks_for_timestamp_ntz(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1,
        #   '2024-01-01 00:00:00'::TIMESTAMP_NTZ) as ts
        #   FROM TABLE(GENERATOR(ROWCOUNT => 50000)) ORDER BY ts" is executed
        sql = (
            f"SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, "
            f"'2024-01-01 00:00:00'::TIMESTAMP_NTZ) as ts "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE})) "
            f"ORDER BY 1"
        )
        rows = execute_query(sql)

        # Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00
        values = [row[0] for row in rows]
        assert_datetime_type(values, require_tzinfo=False)
        assert all(v.tzinfo is None for v in values)
        assert_sequential_values(values, LARGE_RESULT_SET_SIZE, transform=sequential_timestamp)


class TestTimestampNtzTable:
    """Tests for TIMESTAMP_NTZ type using table operations."""

    @pytest.mark.parametrize(
        "values_name,insert_values,expected_values,can_be_none",
        [
            ("basic", [TS_2024_JAN_STR, TS_2024_JUN_STR], [TS_2024_JAN, TS_2024_JUN], False),
            ("epoch", [TS_EPOCH_STR, TS_2024_JAN_STR], [TS_EPOCH, TS_2024_JAN], False),
            ("microseconds", [TS_2024_JAN_STR, TS_WITH_MICROSECONDS_STR], [TS_2024_JAN, TS_WITH_MICROSECONDS], False),
            ("null", [None, TS_2024_JAN_STR], [TS_2024_JAN, None], True),
        ],
    )
    def test_should_select_values_from_table_for_timestamp_ntz(
        self, execute_query, tmp_schema, values_name, insert_values, expected_values, can_be_none
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with TIMESTAMP_NTZ column exists with values <insert_values>
        table_name = f"{tmp_schema}.timestamp_ntz_table_{values_name}"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col TIMESTAMP_NTZ)")
        batch_insert(execute_query, table_name, insert_values, quote_strings=True)

        # When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY col NULLS LAST")
        result = [row[0] for row in rows]

        # Then Result should contain timestamps <expected_values>
        assert result == expected_values
        # And Values should not have timezone info
        assert_datetime_type(result, can_be_none=can_be_none, require_tzinfo=False)
        assert all(v.tzinfo is None for v in result if v is not None)

    def test_should_download_large_result_set_with_multiple_chunks_from_table_for_timestamp_ntz(
        self, execute_query, tmp_schema
    ):
        # Given Snowflake client is logged in
        pass

        # And Table with TIMESTAMP_NTZ column exists with 50000 sequential timestamp values
        table_name = f"{tmp_schema}.large_timestamp_ntz_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col TIMESTAMP_NTZ)")
        execute_query(
            f"INSERT INTO {table_name} "
            f"SELECT DATEADD(second, ROW_NUMBER() OVER (ORDER BY seq8()) - 1, "
            f"'2024-01-01 00:00:00'::TIMESTAMP_NTZ) "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE}))"
        )

        # When Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY col NULLS LAST")

        # Then Result should contain 50000 sequentially increasing timestamps from 2024-01-01 00:00:00
        values = [row[0] for row in rows]
        assert_datetime_type(values, require_tzinfo=False)
        assert all(v.tzinfo is None for v in values)
        assert_sequential_values(values, LARGE_RESULT_SET_SIZE, transform=sequential_timestamp)


@with_paramstyle("qmark")
class TestTimestampNtzBinding:
    """Tests for TIMESTAMP_NTZ type using parameter binding.

    The driver binds datetimes as TIMESTAMP_NTZ (see PYTHON_TO_SNOWFLAKE_TYPE).
    Naive datetimes are stored as-is. Tz-aware datetimes are converted to UTC
    and then tzinfo is stripped — only the UTC wall-clock value is stored.
    Exact returned values are deterministic regardless of session timezone.
    """

    def test_should_select_timestamp_ntz_using_parameter_binding(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::TIMESTAMP_NTZ, ?::TIMESTAMP_NTZ" is executed with bound timestamp values
        result = execute_query(
            "SELECT ?::TIMESTAMP_NTZ, ?::TIMESTAMP_NTZ",
            (TS_2024_JAN, TS_2024_JUN),
            single_row=True,
        )

        # Then Result should contain [2024-01-15 10:30:00, 2024-06-20 14:45:30]
        assert tuple(result) == (TS_2024_JAN, TS_2024_JUN)
        # And Values should not have timezone info
        assert_datetime_type(result, require_tzinfo=False)
        assert all(v.tzinfo is None for v in result)

    def test_should_return_null_when_selecting_timestamp_ntz_using_parameter_binding_with_null_value(
        self, execute_query
    ):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::TIMESTAMP_NTZ" is executed with bound NULL value
        result = execute_query("SELECT ?::TIMESTAMP_NTZ", (None,), single_row=True)

        # Then Result should contain [NULL]
        assert result == (None,)

    def test_should_insert_timestamp_ntz_using_parameter_binding(self, execute_query, executemany_insert, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # And Table with TIMESTAMP_NTZ column exists
        table_name = f"{tmp_schema}.timestamp_ntz_bind_table"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col TIMESTAMP_NTZ)")

        # When Timestamp values are bulk-inserted using multirow binding
        test_values = [
            (TS_2024_JAN,),
            (TS_2024_JUN,),
            (None,),
        ]
        executemany_insert(table_name, f"INSERT INTO {table_name} VALUES (?)", test_values)

        # And Query "SELECT * FROM <table> ORDER BY col NULLS LAST" is executed
        rows = execute_query(f"SELECT * FROM {table_name} ORDER BY col NULLS LAST")
        result = [row[0] for row in rows]

        # Then SELECT should return the inserted values in ascending order
        assert_datetime_type([r for r in result if r is not None], require_tzinfo=False)
        assert result == [TS_2024_JAN, TS_2024_JUN, None]

    @pytest.mark.parametrize(
        "aware_input,expected",
        [
            (datetime(2024, 1, 15, 10, 30, 0, tzinfo=timezone.utc), datetime(2024, 1, 15, 10, 30, 0)),
            (datetime(2024, 1, 15, 12, 30, 0, tzinfo=timezone(timedelta(hours=2))), datetime(2024, 1, 15, 10, 30, 0)),
            (datetime(2024, 1, 15, 10, 30, 0, tzinfo=timezone(timedelta(hours=-5))), datetime(2024, 1, 15, 15, 30, 0)),
        ],
    )
    def test_should_store_utc_equivalent_when_binding_timezone_aware_datetime_to_timestamp_ntz(
        self, execute_query, aware_input, expected
    ):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT ?::TIMESTAMP_NTZ" is executed with bound aware datetime <input>
        result = execute_query("SELECT ?::TIMESTAMP_NTZ", (aware_input,), single_row=True)

        # Then Result should contain [<expected>]
        assert result == (expected,)
        # And Values should not have timezone info
        assert result[0].tzinfo is None


class TestTimestampNtzAliases:
    """Tests for TIMESTAMP and DATETIME aliases that map to TIMESTAMP_NTZ.

    Snowflake's TIMESTAMP_TYPE_MAPPING session parameter controls what
    TIMESTAMP and DATETIME resolve to. When set to TIMESTAMP_NTZ (the
    Snowflake default), both aliases return naive datetimes. When set to
    a different type (e.g. TIMESTAMP_LTZ), they return aware datetimes.
    """

    @pytest.mark.parametrize("type_name", ["TIMESTAMP", "DATETIME"])
    def test_should_return_naive_datetime_for_type_name_alias_when_session_mapping_is_timestamp_ntz(
        self, execute_query, type_name
    ):
        # Given Snowflake client is logged in
        pass

        try:
            # And Session TIMESTAMP_TYPE_MAPPING is set to TIMESTAMP_NTZ
            execute_query("ALTER SESSION SET TIMESTAMP_TYPE_MAPPING = 'TIMESTAMP_NTZ'")

            # When Query "SELECT '2024-01-15 10:30:00'::<type_name>" is executed
            result = execute_query(f"SELECT '{TS_2024_JAN_STR}'::{type_name}", single_row=True)

            # Then All values should be returned as appropriate type
            assert_datetime_type(result, require_tzinfo=False)
            # And Values should not have timezone info
            assert result[0].tzinfo is None
        finally:
            execute_query("ALTER SESSION UNSET TIMESTAMP_TYPE_MAPPING")

    def test_should_return_aware_datetime_for_timestamp_alias_when_session_mapping_is_timestamp_ltz(
        self, execute_query
    ):
        # Given Snowflake client is logged in
        pass

        try:
            # And Session TIMESTAMP_TYPE_MAPPING is set to TIMESTAMP_LTZ
            execute_query("ALTER SESSION SET TIMESTAMP_TYPE_MAPPING = 'TIMESTAMP_LTZ'")

            # When Query "SELECT '2024-01-15 10:30:00'::TIMESTAMP" is executed
            result = execute_query(f"SELECT '{TS_2024_JAN_STR}'::TIMESTAMP", single_row=True)

            # Then All values should be returned as appropriate type
            assert_datetime_type(result, require_tzinfo=True)
            # And Values should have timezone info
            assert result[0].tzinfo is not None
        finally:
            execute_query("ALTER SESSION UNSET TIMESTAMP_TYPE_MAPPING")


class TestTimestampNtzPrecision:
    """Python-specific precision behaviour for TIMESTAMP_NTZ.

    Python datetime is capped at microsecond precision (6 decimal places).
    Sub-microsecond digits received from Snowflake are silently truncated — not rounded.
    The .999999999 case is the critical proof: rounding would increment the second,
    truncation does not.
    """

    @pytest.mark.parametrize(
        "input_str,expected",
        [
            ("2024-01-15 10:30:00.123456789", datetime(2024, 1, 15, 10, 30, 0, 123456)),
            ("2024-01-15 10:30:00.999999999", datetime(2024, 1, 15, 10, 30, 0, 999999)),
        ],
    )
    def test_should_truncate_nanosecond_precision_to_microseconds_for_timestamp_ntz(
        self, execute_query, input_str, expected
    ):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT '<input>'::TIMESTAMP_NTZ" is executed
        result = execute_query(f"SELECT '{input_str}'::TIMESTAMP_NTZ", single_row=True)

        # Then Result should contain [<expected>]
        assert result[0] == expected
        # And Values should not have timezone info
        assert_datetime_type(result, require_tzinfo=False)
        assert result[0].tzinfo is None
