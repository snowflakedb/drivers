"""Pandas fetch method tests (Python-specific).

This module tests the cursor methods that return results as Pandas DataFrames:
- fetch_pandas_all
- fetch_pandas_batches
"""

from __future__ import annotations

import pandas as pd
import pytest

from tests.e2e.types.utils import assert_connection_is_open, assert_float_equal


LARGE_RESULT_SET_ROW_COUNT = 100_000


@pytest.mark.skip_universal("SNOW-3243341 - not implemented yet")
class TestFetchPandasAll:
    """Tests for fetch_pandas_all cursor method."""

    def test_should_fetch_typed_rows_with_nulls_as_pandas_dataframe(self, execute_query, cursor, tmp_schema):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And A temporary table with columns (id NUMBER, name VARCHAR, score FLOAT, active BOOLEAN) exists
        table_name = f"{tmp_schema}.test_pandas_all_types"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR, score FLOAT, active BOOLEAN)")

        # And Rows [1, "Alice", 9.5, TRUE], [2, NULL, NULL, FALSE] are inserted
        cursor.execute(f"INSERT INTO {table_name} VALUES (1, 'Alice', 9.5, TRUE)")
        cursor.execute(f"INSERT INTO {table_name} VALUES (2, NULL, NULL, FALSE)")

        # When Query "SELECT * FROM {table} ORDER BY id" is executed
        cursor.execute(f"SELECT * FROM {table_name} ORDER BY id")

        # And fetch_pandas_all is called
        result: pd.DataFrame = cursor.fetch_pandas_all()

        # Then The result should be a pandas.DataFrame with 2 rows
        assert isinstance(result, pd.DataFrame)
        assert len(result) == 2

        # And Row 1 should contain [1, "Alice", 9.5, True]
        row1 = result.iloc[0]
        assert row1["ID"] == 1
        assert row1["NAME"] == "Alice"
        assert_float_equal(row1["SCORE"], 9.5)
        assert row1["ACTIVE"] is True or row1["ACTIVE"] == True  # noqa: E712 — pandas bool

        # And Row 2 should contain [2, None/NaN, None/NaN, False]
        row2 = result.iloc[1]
        assert row2["ID"] == 2
        assert pd.isna(row2["NAME"])
        assert pd.isna(row2["SCORE"])
        assert row2["ACTIVE"] is False or row2["ACTIVE"] == False  # noqa: E712

    def test_should_return_empty_pandas_dataframe_for_empty_result_set(self, execute_query, cursor):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query "SELECT 1 AS id WHERE 1=0" is executed
        cursor.execute("SELECT 1 AS id WHERE 1=0")

        # And fetch_pandas_all is called
        result: pd.DataFrame = cursor.fetch_pandas_all()

        # Then The result should be a pandas.DataFrame with 0 rows
        assert isinstance(result, pd.DataFrame)
        assert len(result) == 0


@pytest.mark.skip_universal("SNOW-3243341 - not implemented yet")
class TestFetchPandasBatches:
    """Tests for fetch_pandas_batches cursor method."""

    def test_should_yield_multiple_pandas_dataframes_for_large_result_set(self, execute_query, cursor):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 100000)) v" is executed
        cursor.execute(f"SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_ROW_COUNT})) v")

        # And fetch_pandas_batches is called
        batch_count = 0
        total_rows = 0
        for batch in cursor.fetch_pandas_batches():
            # And Each element should be a pandas.DataFrame
            assert isinstance(batch, pd.DataFrame)
            batch_count += 1
            total_rows += len(batch)

        # Then More than one DataFrame should be yielded
        assert batch_count > 1

        # And The total row count across all DataFrames should be 100000
        assert total_rows == LARGE_RESULT_SET_ROW_COUNT
