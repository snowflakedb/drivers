"""Stored Procedure Lifecycle E2E tests for Universal Driver.

This module tests stored procedure lifecycle functionality including:
- Creating SQL stored procedures
- Calling stored procedures with parameters
- Dropping stored procedures
- Verifying procedures via SHOW PROCEDURES
"""

from __future__ import annotations

from tests.e2e.types.utils import assert_connection_is_open


class TestStoredProcedureLifecycle:
    """Tests for stored procedure creation, calling, and dropping."""

    def test_should_create_call_and_drop_a_sql_stored_procedure(self, execute_query, cursor, tmp_schema):
        """Test creating, calling, and dropping a SQL stored procedure."""
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)
        proc_name = f"{tmp_schema}.e2e_test_proc"

        # When SHOW PROCEDURES LIKE 'e2e_test_proc' is executed
        cursor.execute(f"SHOW PROCEDURES LIKE 'e2e_test_proc' IN SCHEMA {tmp_schema}")

        # Then The result should be empty
        assert len(cursor.fetchall()) == 0

        # When A SQL stored procedure "e2e_test_proc" is created that returns 'Hello, ' || name
        cursor.execute(
            f"CREATE OR REPLACE PROCEDURE {proc_name}(name VARCHAR) "
            "RETURNS VARCHAR LANGUAGE SQL AS $$ BEGIN RETURN 'Hello, ' || name; END; $$"
        )

        # Then SHOW PROCEDURES LIKE 'e2e_test_proc' should return 1 row
        cursor.execute(f"SHOW PROCEDURES LIKE 'e2e_test_proc' IN SCHEMA {tmp_schema}")
        assert len(cursor.fetchall()) == 1

        # When CALL e2e_test_proc('World') is executed
        cursor.execute(f"CALL {proc_name}('World')")
        result = cursor.fetchone()

        # Then The result should be "Hello, World"
        assert result[0] == "Hello, World"

        # When The procedure is dropped
        cursor.execute(f"DROP PROCEDURE IF EXISTS {proc_name}(VARCHAR)")

        # Then SHOW PROCEDURES LIKE 'e2e_test_proc' should return 0 rows
        cursor.execute(f"SHOW PROCEDURES LIKE 'e2e_test_proc' IN SCHEMA {tmp_schema}")
        assert len(cursor.fetchall()) == 0
