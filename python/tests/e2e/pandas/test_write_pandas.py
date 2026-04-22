"""write_pandas tests (Python-specific).

This module tests the write_pandas function that writes Pandas DataFrames
to Snowflake tables via the Parquet stage upload pipeline.
"""

from __future__ import annotations

import math
import warnings

from datetime import date, datetime, timezone
from uuid import uuid4

import pandas as pd
import pytest

from snowflake.connector.cursor import DictCursor
from snowflake.connector.errors import ProgrammingError
from snowflake.connector.pandas_tools import write_pandas
from tests.e2e.types.utils import assert_connection_is_open


SAMPLE_DATA = [
    ("Alice", 100),
    ("Bob", 200),
    ("Charlie", 300),
    ("Diana", 400),
    ("Eve", 500),
]
SAMPLE_DF = pd.DataFrame(SAMPLE_DATA, columns=["NAME", "SCORE"])


def _table(prefix: str) -> str:
    return f"{prefix}_{uuid4().hex[:8]}".upper()


class TestWritePandas:
    """Tests for write_pandas function."""

    def test_should_write_a_dataframe_to_a_pre_created_table_and_read_it_back(
        self, execute_query, connection, cursor, tmp_schema
    ):
        table_name = _table("WP_BASIC")
        fq_table = f"{tmp_schema}.{table_name}"

        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And A temporary table with columns name STRING and score INT exists
        cursor.execute(f"CREATE OR REPLACE TEMPORARY TABLE {fq_table} (NAME STRING, SCORE INT)")

        # When write_pandas is called with the sample DataFrame
        success, nchunks, nrows, _ = write_pandas(
            connection,
            SAMPLE_DF,
            table_name,
            schema=tmp_schema,
            quote_identifiers=False,
        )

        # Then write_pandas should return success with correct chunk and row counts
        assert success
        assert nchunks == 1
        assert nrows == len(SAMPLE_DATA)

        # And SELECT from the table should return all original rows
        result = cursor.execute(f"SELECT * FROM {fq_table}").fetchall()
        assert set(result) == set(SAMPLE_DATA)

    def test_should_auto_create_a_table_from_dataframe_schema(self, execute_query, connection, cursor, tmp_schema):
        table_name = _table("WP_AUTOCREATE")
        fq_table = f"{tmp_schema}.{table_name}"

        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When write_pandas is called with auto_create_table=True and table_type="temp"
        success, nchunks, nrows, _ = write_pandas(
            connection,
            SAMPLE_DF,
            table_name,
            schema=tmp_schema,
            quote_identifiers=False,
            auto_create_table=True,
            table_type="temp",
        )

        # Then write_pandas should return success with correct chunk and row counts
        assert success
        assert nchunks == 1
        assert nrows == len(SAMPLE_DATA)

        # And SELECT from the table should return all original rows
        result = cursor.execute(f"SELECT * FROM {fq_table}").fetchall()
        assert set(result) == set(SAMPLE_DATA)

    def test_should_overwrite_existing_data_with_new_data(self, execute_query, connection, cursor, tmp_schema):
        table_name = _table("WP_OVERWRITE")
        fq_table = f"{tmp_schema}.{table_name}"
        initial_data = [("Frank", 10), ("Grace", 20), ("Hank", 30)]
        initial_df = pd.DataFrame(initial_data, columns=["NAME", "SCORE"])
        new_data = [("Ivy", 99)]
        new_df = pd.DataFrame(new_data, columns=["NAME", "SCORE"])

        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And A temporary table with columns name STRING and score INT exists
        cursor.execute(f"CREATE OR REPLACE TEMPORARY TABLE {fq_table} (NAME STRING, SCORE INT)")

        # And The table contains initial data
        write_pandas(connection, initial_df, table_name, schema=tmp_schema, quote_identifiers=False)

        # When write_pandas is called with new data and overwrite=True
        success, nchunks, nrows, _ = write_pandas(
            connection,
            new_df,
            table_name,
            schema=tmp_schema,
            quote_identifiers=False,
            overwrite=True,
        )

        # Then write_pandas should return success with correct chunk and row counts
        assert success
        assert nchunks == 1
        assert nrows == 1

        # And The table should contain only the new data
        result = cursor.execute(f"SELECT * FROM {fq_table}").fetchall()
        assert result == new_data

    def test_should_write_dataframe_in_multiple_chunks(self, execute_query, connection, cursor, tmp_schema):
        table_name = _table("WP_CHUNKED")
        fq_table = f"{tmp_schema}.{table_name}"
        chunk_size = 2
        expected_chunks = math.ceil(len(SAMPLE_DATA) / chunk_size)

        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And A temporary table with columns name STRING and score INT exists
        cursor.execute(f"CREATE OR REPLACE TEMPORARY TABLE {fq_table} (NAME STRING, SCORE INT)")

        # When write_pandas is called with chunk_size=2
        success, nchunks, nrows, _ = write_pandas(
            connection,
            SAMPLE_DF,
            table_name,
            schema=tmp_schema,
            quote_identifiers=False,
            chunk_size=chunk_size,
        )

        # Then write_pandas should return 3 chunks for a 5-row DataFrame
        assert success
        assert nchunks == expected_chunks
        assert nrows == len(SAMPLE_DATA)

        # And All original rows should be present in the table
        result = cursor.execute(f"SELECT * FROM {fq_table}").fetchall()
        assert set(result) == set(SAMPLE_DATA)

    def test_should_round_trip_multiple_data_types_through_write_pandas(self, execute_query, connection, tmp_schema):
        table_name = _table("WP_TYPES")
        fq_table = f"{tmp_schema}.{table_name}"
        ts_tz = datetime(2026, 4, 1, 9, 30, 29, tzinfo=timezone.utc)
        ts_ntz = datetime(2026, 4, 2, 14, 15, 59)
        types_df = pd.DataFrame(
            {
                "COL_INT": [1, 2],
                "COL_FLOAT": [1.25, 2.75],
                "COL_STR": ["hello", "world"],
                "COL_BOOL": [True, False],
                "COL_DATE": [date(2026, 4, 1), date(2026, 4, 2)],
                "COL_BINARY": [b"\xde\xad", b"\xbe\xef"],
                "COL_TS_TZ": [ts_tz, ts_tz],
                "COL_TS_NTZ": [ts_ntz, ts_ntz],
            }
        )

        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When write_pandas is called with a multi-type DataFrame using auto_create_table=True and use_logical_type=True
        success, nchunks, nrows, _ = write_pandas(
            connection,
            types_df,
            table_name,
            schema=tmp_schema,
            quote_identifiers=False,
            auto_create_table=True,
            table_type="temp",
            use_logical_type=True,
        )

        # Then write_pandas should return success with correct chunk and row counts
        assert success
        assert nchunks == 1
        assert nrows == 2

        # And All values should match the original data including timestamps
        with connection.cursor(DictCursor) as cur:
            rows = cur.execute(f"SELECT * FROM {fq_table} ORDER BY COL_INT").fetchall()
        assert len(rows) == 2

        row0, row1 = rows[0], rows[1]

        assert row0["COL_INT"] == 1
        assert row1["COL_INT"] == 2
        assert row0["COL_FLOAT"] == pytest.approx(1.25)
        assert row1["COL_FLOAT"] == pytest.approx(2.75)
        assert row0["COL_STR"] == "hello"
        assert row1["COL_STR"] == "world"
        assert row0["COL_BOOL"] is True
        assert row1["COL_BOOL"] is False
        assert row0["COL_DATE"] == date(2026, 4, 1)
        assert row1["COL_DATE"] == date(2026, 4, 2)
        assert row0["COL_BINARY"] == b"\xde\xad"
        assert row1["COL_BINARY"] == b"\xbe\xef"
        assert row0["COL_TS_TZ"] == ts_tz
        assert row0["COL_TS_NTZ"] == ts_ntz
        assert row1["COL_TS_TZ"] == ts_tz
        assert row1["COL_TS_NTZ"] == ts_ntz


class TestWritePandasValidation:
    """Tests for write_pandas input validation and warnings.

    Validation fires before any Snowflake interaction, so these tests
    use a real connection but never actually write data.
    """

    def test_should_raise_programming_error_when_database_is_set_without_schema(self, execute_query, connection):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When write_pandas is called with database but no schema
        kwargs = {"database": "mydb"}

        # Then ProgrammingError should be raised
        with pytest.raises(ProgrammingError):
            write_pandas(connection, SAMPLE_DF, "t", **kwargs)

    def test_should_raise_programming_error_for_invalid_compression(self, execute_query, connection):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When write_pandas is called with an unsupported compression value
        kwargs = {"compression": "bzip2"}

        # Then ProgrammingError should be raised
        with pytest.raises(ProgrammingError):
            write_pandas(connection, SAMPLE_DF, "t", **kwargs)

    def test_should_raise_value_error_for_invalid_table_type(self, execute_query, connection):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When write_pandas is called with an invalid table_type
        kwargs = {"table_type": "bogus"}

        # Then ValueError should be raised
        with pytest.raises(ValueError):
            write_pandas(connection, SAMPLE_DF, "t", **kwargs)

    def test_should_emit_user_warning_for_tz_aware_columns_without_use_logical_type(self, execute_query, connection):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And A DataFrame with a tz-aware datetime column
        tz_df = pd.DataFrame({"ts": [datetime(2024, 1, 1, tzinfo=timezone.utc)]})

        # When write_pandas is called without use_logical_type=True
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            try:
                write_pandas(connection, tz_df, "t")
            except Exception:
                pass

        # Then UserWarning about timezone should be emitted
        assert any(issubclass(w.category, UserWarning) and "timezone" in str(w.message).lower() for w in caught), (
            f"Expected UserWarning about timezone, got: {[str(w.message) for w in caught]}"
        )

    def test_should_emit_user_warning_for_non_standard_dataframe_index(self, execute_query, connection):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And A DataFrame with a string index
        string_idx_df = pd.DataFrame({"val": [10, 20]}, index=["a", "b"])

        # When write_pandas is called with the non-standard index DataFrame
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            try:
                write_pandas(connection, string_idx_df, "t")
            except Exception:
                pass

        # Then UserWarning about non-standard index should be emitted
        assert any(issubclass(w.category, UserWarning) and "index" in str(w.message).lower() for w in caught), (
            f"Expected UserWarning about index, got: {[str(w.message) for w in caught]}"
        )

    def test_should_handle_invalid_iceberg_config_keys(self, execute_query, connection, tmp_schema):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When write_pandas is called with iceberg_config containing invalid keys
        kwargs = {"iceberg_config": {"invalid_key": "value"}}

        # Then ProgrammingError should be raised
        with pytest.raises(ProgrammingError, match="INVALID_KEY"):
            write_pandas(
                connection,
                SAMPLE_DF,
                _table("WP_OVERWRITE"),
                schema=tmp_schema,
                quote_identifiers=False,
                auto_create_table=True,
                **kwargs,
            )
