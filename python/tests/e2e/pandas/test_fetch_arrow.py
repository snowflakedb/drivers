"""Arrow fetch method tests (Python-specific).

This module tests the cursor methods that return results as Arrow tables:
- fetch_arrow_all
- fetch_arrow_batches
"""

from __future__ import annotations

import pyarrow as pa
import pytest

from tests.e2e.types.utils import assert_connection_is_open, assert_float_equal


LARGE_RESULT_SET_ROW_COUNT = 100_000


@pytest.mark.skip_universal("SNOW-3243341 - not implemented yet")
class TestFetchArrowAll:
    """Tests for fetch_arrow_all cursor method."""

    def test_should_fetch_typed_rows_with_nulls_as_pyarrow_table(self, execute_query, cursor, tmp_schema):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And A temporary table with columns (id NUMBER, name VARCHAR, score FLOAT, active BOOLEAN) exists
        table_name = f"{tmp_schema}.test_arrow_all_types"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR, score FLOAT, active BOOLEAN)")

        # And Rows [1, "Alice", 9.5, TRUE], [2, NULL, NULL, FALSE] are inserted
        cursor.execute(f"INSERT INTO {table_name} VALUES (1, 'Alice', 9.5, TRUE)")
        cursor.execute(f"INSERT INTO {table_name} VALUES (2, NULL, NULL, FALSE)")

        # When Query "SELECT * FROM {table} ORDER BY id" is executed
        cursor.execute(f"SELECT * FROM {table_name} ORDER BY id")

        # And fetch_arrow_all is called
        result: pa.Table | None = cursor.fetch_arrow_all()

        # Then The result should be a pyarrow.Table with 2 rows
        assert isinstance(result, pa.Table)
        assert result.num_rows == 2

        # And Row 1 should contain [1, "Alice", 9.5, True]
        assert result.column("ID")[0].as_py() == 1
        assert result.column("NAME")[0].as_py() == "Alice"
        assert_float_equal(result.column("SCORE")[0].as_py(), 9.5)
        assert result.column("ACTIVE")[0].as_py() is True

        # And Row 2 should contain [2, NULL, NULL, False]
        assert result.column("ID")[1].as_py() == 2
        assert result.column("NAME")[1].as_py() is None
        assert result.column("SCORE")[1].as_py() is None
        assert result.column("ACTIVE")[1].as_py() is False

    def test_should_return_none_from_fetch_arrow_all_for_empty_result_set(self, execute_query, cursor):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query "SELECT 1 AS id WHERE 1=0" is executed
        cursor.execute("SELECT 1 AS id WHERE 1=0")

        # And fetch_arrow_all is called
        result: pa.Table | None = cursor.fetch_arrow_all()

        # Then The result should be None
        assert result is None


@pytest.mark.skip_universal("SNOW-3243341 - not implemented yet")
class TestFetchArrowBatches:
    """Tests for fetch_arrow_batches cursor method."""

    def test_should_yield_multiple_arrow_batches_for_large_result_set(self, execute_query, cursor):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When Query "SELECT seq8() as id FROM TABLE(GENERATOR(ROWCOUNT => 100000)) v" is executed
        cursor.execute(f"SELECT seq8() AS id FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_ROW_COUNT})) v")

        # And fetch_arrow_batches is called
        batch_count = 0
        total_rows = 0
        for batch in cursor.fetch_arrow_batches():
            # And Each element should be a pyarrow.Table
            assert isinstance(batch, pa.Table)
            batch_count += 1
            total_rows += batch.num_rows

        # Then More than one batch should be yielded
        assert batch_count > 1

        # And The total row count across all batches should be 100000
        assert total_rows == LARGE_RESULT_SET_ROW_COUNT
