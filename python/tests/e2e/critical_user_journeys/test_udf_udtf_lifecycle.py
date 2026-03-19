"""UDF and UDTF Lifecycle E2E tests for Universal Driver.

This module tests user-defined function lifecycle including:
- Creating and calling SQL UDFs
- Creating and calling SQL UDTFs
- Dropping UDFs/UDTFs and verifying via SHOW
"""

from __future__ import annotations

from tests.e2e.types.utils import assert_connection_is_open


class TestUdfUdtfLifecycle:
    """Tests for UDF and UDTF creation, calling, and dropping."""

    def test_should_create_call_and_drop_a_sql_udf(self, execute_query, cursor, tmp_schema):
        """Test creating, calling, and dropping a SQL UDF."""
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)
        udf_name = f"{tmp_schema}.e2e_test_udf"

        # When A SQL UDF "e2e_test_udf" is created that returns x * 2
        cursor.execute(f"CREATE OR REPLACE FUNCTION {udf_name}(x NUMBER) RETURNS NUMBER AS $$ x * 2 $$")

        # Then SELECT e2e_test_udf(21) should return 42
        cursor.execute(f"SELECT {udf_name}(21)")
        assert cursor.fetchone()[0] == 42

        # When The UDF is dropped
        cursor.execute(f"DROP FUNCTION IF EXISTS {udf_name}(NUMBER)")

        # Then SHOW FUNCTIONS LIKE 'e2e_test_udf' should return 0 rows
        cursor.execute(f"SHOW FUNCTIONS LIKE 'e2e_test_udf' IN SCHEMA {tmp_schema}")
        assert len(cursor.fetchall()) == 0

    def test_should_create_call_and_drop_a_sql_udtf(self, execute_query, cursor, tmp_schema):
        """Test creating, calling, and dropping a SQL UDTF."""
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)
        udtf_name = f"{tmp_schema}.e2e_test_udtf"

        # When A SQL UDTF "e2e_test_udtf" is created that generates n rows
        cursor.execute(
            f"CREATE OR REPLACE FUNCTION {udtf_name}(n NUMBER) "
            "RETURNS TABLE(row_num NUMBER) AS "
            "$$ SELECT ROW_NUMBER() OVER (ORDER BY SEQ8()) as row_num "
            "FROM TABLE(GENERATOR(ROWCOUNT => n)) $$"
        )

        # Then SELECT * FROM TABLE(e2e_test_udtf(5)) should return 5 rows
        cursor.execute(f"SELECT * FROM TABLE({udtf_name}(5))")
        results = cursor.fetchall()
        assert len(results) == 5
        assert sorted([row[0] for row in results]) == [1, 2, 3, 4, 5]

        # When The UDTF is dropped
        cursor.execute(f"DROP FUNCTION IF EXISTS {udtf_name}(NUMBER)")

        # Then SHOW FUNCTIONS LIKE 'e2e_test_udtf' should return 0 rows
        cursor.execute(f"SHOW FUNCTIONS LIKE 'e2e_test_udtf' IN SCHEMA {tmp_schema}")
        assert len(cursor.fetchall()) == 0
