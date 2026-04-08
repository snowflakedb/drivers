"""write_pandas tests (Python-specific).

This module tests the write_pandas function that writes Pandas DataFrames
to Snowflake tables via the Parquet stage upload pipeline.
"""

from __future__ import annotations

import math

from datetime import date, datetime, timezone
from uuid import uuid4

import pandas as pd
import pytest

from snowflake.connector.cursor import DictCursor
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


@pytest.mark.skip_universal(reason="write_pandas not yet implemented in universal driver")
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
