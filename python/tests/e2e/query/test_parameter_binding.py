"""Parameter binding tests for Universal Driver (Python-specific).

This module tests JSON parameter binding functionality including:
- Basic type support (int, float, str, bool, None, bytes, datetime, Decimal)
- Positional parameters (? and :1 style)
- Multirow binding (multi-row inserts)
- Edge cases (NULL values, empty parameters, special characters)
- Backwards compatibility with old connector format
"""

from __future__ import annotations

from datetime import datetime
from decimal import Decimal

import pytest

from snowflake.connector import ProgrammingError


class TestBasicTypeBinding:
    """Tests for binding basic Python types to Snowflake."""

    def test_should_bind_basic_types_with_positional_parameters(self, cursor):
        # Given Snowflake client is logged in

        # When Query "SELECT ?, ?, ?, ?, ?" is executed with positional parameters [42, 3.14, "hello", True, None]
        sql = "SELECT ?, ?, ?, ?, ?"
        params = (42, 3.14, "hello", True, None)
        cursor.execute(sql, params)
        result = cursor.fetchone()

        # Then Result should contain values matching the bound parameters
        assert result is not None
        assert result[0] == 42
        assert abs(result[1] - 3.14) < 0.01  # Float comparison with tolerance
        assert result[2] == "hello"
        assert result[3] is True
        assert result[4] is None

    def test_should_bind_positional_parameters_with_numeric_placeholders(self, cursor):
        # Given Snowflake client is logged in

        # When Query "SELECT :1, :2, :3" is executed with positional parameters [100, "test", True]
        sql = "SELECT :1, :2, :3"
        params = (100, "test", True)
        cursor.execute(sql, params)
        result = cursor.fetchone()

        # Then Result should contain values in order [100, "test", True]
        assert result == (100, "test", True)



class TestTableOperations:
    """Tests for parameter binding with table operations."""

    def test_should_insert_single_row_with_parameter_binding(self, cursor, tmp_schema):
        # Given Snowflake client is logged in
        # And A temporary table with columns (id NUMBER, name VARCHAR, active BOOLEAN) exists
        table_name = f"{tmp_schema}.test_binding_insert"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR, active BOOLEAN)")

        # When Row with values [1, "Alice", True] is inserted using parameter binding
        cursor.execute(f"INSERT INTO {table_name} VALUES (?, ?, ?)", (1, "Alice", True))

        # And Query "SELECT * FROM table" is executed
        cursor.execute(f"SELECT * FROM {table_name}")
        result = cursor.fetchall()

        # Then Result should contain the inserted row
        assert len(result) == 1
        assert result[0] == (1, "Alice", True)

    def test_should_insert_multiple_rows_sequentially_with_parameter_binding(self, cursor, tmp_schema):
        # Given Snowflake client is logged in
        # And A temporary table with columns (id NUMBER, name VARCHAR) exists
        table_name = f"{tmp_schema}.test_binding_multiple"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR)")

        # When Multiple rows are inserted using parameter binding
        rows = [(1, "Alice"), (2, "Bob"), (3, "Charlie")]
        for row in rows:
            cursor.execute(f"INSERT INTO {table_name} VALUES (?, ?)", row)

        # Then Query "SELECT * FROM table ORDER BY id" should return 3 rows with correct values
        cursor.execute(f"SELECT * FROM {table_name} ORDER BY id")
        result = cursor.fetchall()

        assert len(result) == 3
        assert result == rows

    def test_should_update_row_with_parameter_binding(self, cursor, tmp_schema):
        # Setup: Create table and insert initial data
        table_name = f"{tmp_schema}.test_binding_update"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR)")
        cursor.execute(f"INSERT INTO {table_name} VALUES (?, ?)", (1, "Alice"))

        # When: Update using parameter binding
        cursor.execute(f"UPDATE {table_name} SET name = ? WHERE id = ?", ("Alice Updated", 1))

        # Then: Verify update
        cursor.execute(f"SELECT * FROM {table_name}")
        result = cursor.fetchone()
        assert result == (1, "Alice Updated")

    def test_should_delete_row_with_parameter_binding(self, cursor, tmp_schema):
        # Setup: Create table and insert data
        table_name = f"{tmp_schema}.test_binding_delete"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR)")
        cursor.execute(f"INSERT INTO {table_name} VALUES (?, ?)", (1, "Alice"))
        cursor.execute(f"INSERT INTO {table_name} VALUES (?, ?)", (2, "Bob"))

        # When: Delete using parameter binding
        cursor.execute(f"DELETE FROM {table_name} WHERE id = ?", (1,))

        # Then: Verify deletion
        cursor.execute(f"SELECT * FROM {table_name}")
        result = cursor.fetchall()
        assert len(result) == 1
        assert result[0] == (2, "Bob")

    def test_should_select_with_where_clause_parameter_binding(self, cursor, tmp_schema):
        # Setup: Create table and insert data
        table_name = f"{tmp_schema}.test_binding_select_where"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR, age NUMBER)")
        cursor.execute(f"INSERT INTO {table_name} VALUES (?, ?, ?)", (1, "Alice", 30))
        cursor.execute(f"INSERT INTO {table_name} VALUES (?, ?, ?)", (2, "Bob", 25))
        cursor.execute(f"INSERT INTO {table_name} VALUES (?, ?, ?)", (3, "Charlie", 35))

        # When: Select with WHERE clause using parameter binding
        cursor.execute(f"SELECT * FROM {table_name} WHERE age > ?", (28,))
        result = cursor.fetchall()

        # Then: Verify correct rows selected
        assert len(result) == 2
        names = {row[1] for row in result}
        assert names == {"Alice", "Charlie"}


class TestEdgeCases:
    """Tests for edge cases in parameter binding."""

    def test_should_handle_null_values_in_parameter_binding(self, cursor):
        # Given Snowflake client is logged in

        # When Query "SELECT ?, ?, ?" is executed with parameters [None, 42, None]
        cursor.execute("SELECT ?, ?, ?", (None, 42, None))
        result = cursor.fetchone()

        # Then Result should contain [NULL, 42, NULL]
        assert result == (None, 42, None)


    def test_should_handle_special_characters_in_string_binding(self, cursor):
        # Given Snowflake client is logged in

        # When Query "SELECT ?::VARCHAR" is executed with parameter containing special characters
        special_strings = [
            "'; DROP TABLE test; --",  # SQL injection attempt
            "<script>alert('xss')</script>",  # XSS attempt
            "Line1\nLine2\nLine3",  # Multiple newlines
            "Tab\t\tSeparated\t\tValues",  # Multiple tabs
            "Quote'Within\"String",  # Mixed quotes
            "\\n\\t\\r\\\\",  # Escaped sequences as literal
        ]

        for special_str in special_strings:
            cursor.execute("SELECT ?::VARCHAR", (special_str,))
            result = cursor.fetchone()
            # Then Result should contain the exact special character string
            assert result == (special_str,), f"Failed for: {special_str!r}"

    def test_should_handle_unicode_characters_in_parameter_binding(self, cursor):
        # Given Snowflake client is logged in

        # When Query "SELECT ?::VARCHAR, ?::VARCHAR" is executed with parameters ["日本語", "⛄"]
        cursor.execute("SELECT ?::VARCHAR, ?::VARCHAR", ("日本語", "⛄"))
        result = cursor.fetchone()

        # Then Result should contain Unicode strings ["日本語", "⛄"]
        assert result == ("日本語", "⛄")


    def test_should_bind_zero_values(self, cursor):
        # Given Snowflake client is logged in

        # When Query "SELECT ?, ?, ?" is executed with parameters [0, 0.0, ""]
        cursor.execute("SELECT ?, ?::FLOAT, ?::VARCHAR", (0, 0.0, ""))
        result = cursor.fetchone()

        # Then Result should contain zero and empty values
        assert result == (0, 0.0, "")

    def test_should_handle_mixed_type_casting_with_parameter_binding(self, cursor):
        # Given Snowflake client is logged in

        # When Query "SELECT ?::NUMBER, ?::VARCHAR, ?::BOOLEAN" is executed with parameters [42, "hello", True]
        cursor.execute("SELECT ?::NUMBER, ?::VARCHAR, ?::BOOLEAN", (42, "hello", True))
        result = cursor.fetchone()

        # Then Result should match the type-casted parameters
        assert result == (42, "hello", True)


class TestArrayBinding:
    """Tests for multirow binding (executemany functionality)."""

    def test_should_insert_multiple_rows_using_multirow_binding(self, cursor, tmp_schema):
        """Test multirow binding with basic INSERT."""
        table_name = f"{tmp_schema}.test_executemany"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR)")

        rows = [(1, "Alice"), (2, "Bob"), (3, "Charlie")]
        cursor.executemany(f"INSERT INTO {table_name} VALUES (?, ?)", rows)

        cursor.execute(f"SELECT * FROM {table_name} ORDER BY id")
        result = cursor.fetchall()
        assert result == rows

    def test_should_handle_empty_sequence_in_multirow_binding(self, cursor):
        """Test multirow binding with empty sequence is no-op."""
        cursor.executemany("INSERT INTO table VALUES (?)", [])
        # Should not raise error

    def test_should_validate_parameter_length_in_multirow_binding(self, cursor):
        """Test multirow binding raises error for inconsistent lengths."""
        with pytest.raises(ProgrammingError) as excinfo:
            cursor.executemany("INSERT INTO table VALUES (?, ?)", [(1, "a"), (2, "b", "extra")])
        assert "Parameter sequence" in str(excinfo.value)

    def test_should_handle_null_values_in_multirow_binding(self, cursor, tmp_schema):
        """Test multirow binding handles NULL values."""
        table_name = f"{tmp_schema}.test_nulls"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, value VARCHAR)")

        cursor.executemany(f"INSERT INTO {table_name} VALUES (?, ?)", [(1, None), (2, "value"), (3, None)])

        cursor.execute(f"SELECT * FROM {table_name} ORDER BY id")
        result = cursor.fetchall()
        assert result == [(1, None), (2, "value"), (3, None)]

    def test_should_handle_large_batch_in_multirow_binding(self, cursor, tmp_schema):
        """Test multirow binding with 1000 rows."""
        table_name = f"{tmp_schema}.test_large"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER)")

        rows = [(i,) for i in range(1000)]
        cursor.executemany(f"INSERT INTO {table_name} VALUES (?)", rows)

        cursor.execute(f"SELECT COUNT(*) FROM {table_name}")
        assert cursor.fetchone() == (1000,)


class TestBackwardCompatibility:
    """Tests for backward compatibility with old connector parameter format."""

    def test_should_handle_both_tuple_and_list_parameter_formats(self, cursor):
        # Test that both tuple and list work as parameter containers
        sql = "SELECT ?, ?"

        # Tuple format
        cursor.execute(sql, (1, "test"))
        result_tuple = cursor.fetchone()

        # List format
        cursor.execute(sql, [1, "test"])
        result_list = cursor.fetchone()

        assert result_tuple == result_list == (1, "test")


class TestComplexScenarios:
    """Tests for complex parameter binding scenarios."""

    def test_should_bind_many_parameters(self, cursor):
        # Test with many parameters (e.g., 20 parameters)
        num_params = 20
        sql = "SELECT " + ", ".join(["?"] * num_params)
        params = tuple(range(num_params))

        cursor.execute(sql, params)
        result = cursor.fetchone()

        assert result == params

    def test_should_bind_parameters_in_complex_query_with_aggregation(self, cursor, tmp_schema):
        # Test parameter binding in a complex query with aggregation.
        table_name = f"{tmp_schema}.test_complex_query"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, category VARCHAR, value NUMBER)")

        # Insert test data
        cursor.execute(f"INSERT INTO {table_name} VALUES (?, ?, ?)", (1, "A", 100))
        cursor.execute(f"INSERT INTO {table_name} VALUES (?, ?, ?)", (2, "A", 200))
        cursor.execute(f"INSERT INTO {table_name} VALUES (?, ?, ?)", (3, "B", 150))
        cursor.execute(f"INSERT INTO {table_name} VALUES (?, ?, ?)", (4, "B", 250))

        # Complex query with parameter binding
        sql = f"""
            SELECT category, SUM(value) as total
            FROM {table_name}
            WHERE value > ? AND category = ?
            GROUP BY category
            HAVING SUM(value) > ?
        """
        cursor.execute(sql, (50, "A", 100))
        result = cursor.fetchall()

        assert len(result) == 1
        assert result[0][0] == "A"
        assert result[0][1] == 300

    def test_should_bind_parameters_with_or_clause_for_multiple_value_matching(self, cursor, tmp_schema):
        # Test binding multiple parameters with OR clause for value matching.
        table_name = f"{tmp_schema}.test_in_clause"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR)")

        # Insert test data
        for i, name in enumerate(["Alice", "Bob", "Charlie", "David", "Eve"], 1):
            cursor.execute(f"INSERT INTO {table_name} VALUES (?, ?)", (i, name))

        # Using OR for multiple values (alternative to IN clause)
        cursor.execute(
            f"SELECT * FROM {table_name} WHERE id = ? OR id = ? OR id = ? ORDER BY id",
            (1, 3, 5),
        )
        result = cursor.fetchall()

        assert len(result) == 3
        assert [r[1] for r in result] == ["Alice", "Charlie", "Eve"]

    def test_should_bind_parameters_in_subquery(self, cursor, tmp_schema):
        # Test parameter binding in queries with subqueries
        table_name = f"{tmp_schema}.test_subquery"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, value NUMBER)")

        # Insert test data
        for i in range(1, 11):
            cursor.execute(f"INSERT INTO {table_name} VALUES (?, ?)", (i, i * 10))

        # Query with subquery using parameters
        sql = f"""
            SELECT id, value
            FROM {table_name}
            WHERE value > (SELECT AVG(value) FROM {table_name} WHERE id <= ?)
            AND id > ?
        """
        cursor.execute(sql, (5, 3))
        result = cursor.fetchall()

        # Should return rows where id > 3 and value > avg of first 5 rows
        assert len(result) > 0
        assert all(row[0] > 3 for row in result)
