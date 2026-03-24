"""Tests for COPY INTO (Bulk Loading).

Bulk data loading via PUT + COPY INTO.
Used by SQLAlchemy, snowflake-cli, Snowpark, and Snowfort.
"""

from __future__ import annotations

import pytest


class TestCopyInto:
    """Tests for COPY INTO bulk loading operations."""

    def test_should_bulk_load_csv_data_via_put_and_copy_into(self, cursor, tmp_schema, tmp_path):
        """Test bulk loading CSV data via PUT and COPY INTO."""
        # Given Snowflake client is logged in
        stage_name = f"{tmp_schema}.copy_test_stage"
        table_name = f"{tmp_schema}.copy_test"

        # And A temporary stage "copy_test_stage" exists
        cursor.execute(f"CREATE STAGE {stage_name}")

        # And A temporary table "copy_test" with columns (id INT, name VARCHAR, val FLOAT) exists
        cursor.execute(f"CREATE TABLE {table_name} (id INT, name VARCHAR, val FLOAT)")

        # And A local CSV file with 3 rows of test data exists
        csv_path = tmp_path / "data.csv"
        csv_path.write_text("1,Alice,3.14\n2,Bob,2.72\n3,Charlie,1.41\n")

        # When The CSV file is PUT to the stage
        cursor.execute(f"PUT 'file://{csv_path}' @{stage_name} AUTO_COMPRESS=FALSE OVERWRITE=TRUE")

        # Then LS @copy_test_stage should show 1 file
        cursor.execute(f"LS @{stage_name}")
        ls_results = cursor.fetchall()
        assert len(ls_results) == 1

        # When COPY INTO copy_test FROM @copy_test_stage is executed
        cursor.execute(
            f"COPY INTO {table_name} FROM @{stage_name} FILE_FORMAT=(TYPE='CSV' FIELD_OPTIONALLY_ENCLOSED_BY='\"')"
        )

        # Then SELECT * FROM copy_test should return 3 rows with correct values
        cursor.execute(f"SELECT * FROM {table_name} ORDER BY id")
        rows = cursor.fetchall()

        assert len(rows) == 3
        assert rows[0][0] == 1
        assert rows[0][1] == "Alice"
        assert rows[0][2] == pytest.approx(3.14)
        assert rows[1][0] == 2
        assert rows[1][1] == "Bob"
        assert rows[1][2] == pytest.approx(2.72)
        assert rows[2][0] == 3
        assert rows[2][1] == "Charlie"
        assert rows[2][2] == pytest.approx(1.41)
