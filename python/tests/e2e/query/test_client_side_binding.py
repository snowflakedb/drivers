"""Client-side binding tests for pyformat and format paramstyles.

This module tests client-side parameter interpolation functionality including:
- pyformat style: %(name)s (named) and %s (positional)
- format style: %s (positional only)
- Escape handling (special characters, SQL injection prevention)
- Quote handling (strings, NULL, booleans, numbers, binary)
- Backwards compatibility with reference driver
"""

from __future__ import annotations

import pytest

from ...conftest import with_paramstyle


@with_paramstyle("pyformat")
class TestPyformatPositionalBinding:
    """Tests for pyformat %s positional binding (client-side interpolation)."""

    def test_should_bind_basic_types_with_positional_pyformat(self, cursor):
        """Test basic type binding with %s placeholders."""
        # Given Snowflake client is logged in with pyformat paramstyle

        # When Query "SELECT %s, %s, %s, %s, %s" is executed with positional parameters
        sql = "SELECT %s, %s, %s, %s, %s"
        params = (42, 3.14, "hello", True, None)
        cursor.execute(sql, params)
        result = cursor.fetchone()

        # Then Result should contain values matching the bound parameters
        assert result is not None
        assert result[0] == 42
        assert abs(float(result[1]) - 3.14) < 0.01  # Snowflake may return Decimal
        assert result[2] == "hello"
        assert result[3] is True
        assert result[4] is None

    def test_should_bind_string_with_single_quote(self, cursor):
        """Test string binding with single quote character."""
        cursor.execute("SELECT %s", ("it's a test",))
        result = cursor.fetchone()
        assert result == ("it's a test",)

    def test_should_bind_string_with_double_quote(self, cursor):
        """Test string binding with double quote character."""
        cursor.execute("SELECT %s", ('hello "world"',))
        result = cursor.fetchone()
        assert result == ('hello "world"',)

    def test_should_bind_string_with_backslash(self, cursor):
        """Test string binding with backslash character."""
        cursor.execute("SELECT %s", ("path\\to\\file",))
        result = cursor.fetchone()
        assert result == ("path\\to\\file",)

    def test_should_bind_string_with_newline(self, cursor):
        """Test string binding with newline character."""
        cursor.execute("SELECT %s", ("line1\nline2",))
        result = cursor.fetchone()
        assert result == ("line1\nline2",)

    def test_should_bind_string_with_tab(self, cursor):
        """Test string binding with tab character."""
        cursor.execute("SELECT %s", ("col1\tcol2",))
        result = cursor.fetchone()
        assert result == ("col1\tcol2",)

    def test_should_bind_string_with_carriage_return(self, cursor):
        """Test string binding with carriage return character."""
        cursor.execute("SELECT %s", ("line1\rline2",))
        result = cursor.fetchone()
        assert result == ("line1\rline2",)


@with_paramstyle("pyformat")
class TestPyformatNamedBinding:
    """Tests for pyformat %(name)s named binding (client-side interpolation)."""

    def test_should_bind_basic_types_with_named_pyformat(self, cursor):
        """Test basic type binding with %(name)s placeholders."""
        # Given Snowflake client is logged in with pyformat paramstyle

        # When Query with named parameters is executed
        sql = "SELECT %(a)s, %(b)s, %(c)s"
        params = {"a": 100, "b": "test", "c": True}
        cursor.execute(sql, params)
        result = cursor.fetchone()

        # Then Result should contain values matching the bound parameters
        assert result == (100, "test", True)

    def test_should_bind_same_parameter_multiple_times(self, cursor):
        """Test that the same named parameter can be used multiple times."""
        sql = "SELECT %(val)s, %(val)s, %(val)s"
        params = {"val": 42}
        cursor.execute(sql, params)
        result = cursor.fetchone()
        assert result == (42, 42, 42)

    def test_should_bind_with_mixed_order_named_params(self, cursor):
        """Test named parameters used in different order than dict."""
        sql = "SELECT %(z)s, %(a)s, %(m)s"
        params = {"a": 1, "m": 2, "z": 3}
        cursor.execute(sql, params)
        result = cursor.fetchone()
        assert result == (3, 1, 2)


@with_paramstyle("pyformat")
class TestEscapeHandling:
    """Tests for proper escape handling in client-side binding."""

    def test_should_escape_special_characters(self, cursor, tmp_schema):
        """Test that special characters are properly escaped."""
        table_name = f"{tmp_schema}.test_escape"
        cursor.execute(f"CREATE TABLE {table_name} (name VARCHAR)")

        # Test strings with various special characters
        test_strings = [
            "abc\ndef",  # Newline
            "abc\\ndef",  # Escaped backslash + literal n
            "abc\\\ndef",  # Escaped backslash + newline
            "abc\\\\ndef",  # Double escaped backslash + literal n
            'abc"def',  # Double quote
            'abc""def',  # Double double-quote
            "abc'def",  # Single quote
            "abc''def",  # Double single-quote
            "abc\tdef",  # Tab
            "abc\\tdef",  # Escaped backslash + literal t
            "\\x",  # Backslash + x
        ]

        # Insert all test strings
        for s in test_strings:
            cursor.execute(f"INSERT INTO {table_name} VALUES (%s)", (s,))

        # Verify all strings were inserted correctly
        cursor.execute(f"SELECT name FROM {table_name}")
        results = {row[0] for row in cursor.fetchall()}

        for expected in test_strings:
            assert expected in results, f"String {expected!r} not found in results"

    def test_should_prevent_sql_injection_with_positional_binding(self, cursor, tmp_schema):
        """Test that SQL injection attempts are safely escaped."""
        table_name = f"{tmp_schema}.test_injection"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR)")
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s, %s)", (1, "test1"))
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s, %s)", (2, "test2"))

        # SQL injection attempt - should NOT return all rows
        # The injection string should be treated as a literal string value
        # This will either return empty (no match), raise type error (string vs number),
        # or return 1 row at most - but NOT return all rows due to injection
        # The injection string '1 or 1=1' will be quoted as a string literal,
        # which then cannot be converted to NUMBER, causing a type error.
        # This demonstrates the SQL injection is prevented.
        with pytest.raises(Exception) as excinfo:
            cursor.execute(f"SELECT * FROM {table_name} WHERE id = %s", ("1 or 1=1",))

        # Verify the error is about numeric conversion (not about SQL syntax)
        error_msg = str(excinfo.value).lower()
        assert "numeric" in error_msg or "number" in error_msg or "type" in error_msg, (
            f"Expected numeric type error, got: {excinfo.value}"
        )

    def test_should_prevent_sql_injection_with_named_binding(self, cursor, tmp_schema):
        """Test that SQL injection attempts are safely escaped with named params."""
        table_name = f"{tmp_schema}.test_injection_named"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR)")
        cursor.execute(f"INSERT INTO {table_name} VALUES (%(id)s, %(name)s)", {"id": 1, "name": "test1"})
        cursor.execute(f"INSERT INTO {table_name} VALUES (%(id)s, %(name)s)", {"id": 2, "name": "test2"})

        # SQL injection attempt - will be quoted as a string literal
        with pytest.raises(Exception) as excinfo:
            cursor.execute(f"SELECT * FROM {table_name} WHERE id = %(id)s", {"id": "1 or 1=1"})

        # Verify the error is about numeric conversion (not about SQL syntax)
        error_msg = str(excinfo.value).lower()
        assert "numeric" in error_msg or "number" in error_msg or "type" in error_msg, (
            f"Expected numeric type error, got: {excinfo.value}"
        )

    def test_should_handle_complex_escape_sequence(self, cursor):
        """Test complex string with multiple escape sequences."""
        complex_string = "',an\\\\escaped\"line\n"
        cursor.execute("SELECT %s", (complex_string,))
        result = cursor.fetchone()
        assert result == (complex_string,)


@with_paramstyle("pyformat")
class TestQuoteHandling:
    """Tests for proper quote handling in client-side binding."""

    def test_should_quote_null_as_null(self, cursor):
        """Test that None is converted to NULL."""
        cursor.execute("SELECT %s", (None,))
        result = cursor.fetchone()
        assert result == (None,)

    def test_should_quote_boolean_true(self, cursor):
        """Test that True is properly handled."""
        cursor.execute("SELECT %s", (True,))
        result = cursor.fetchone()
        assert result == (True,)

    def test_should_quote_boolean_false(self, cursor):
        """Test that False is properly handled."""
        cursor.execute("SELECT %s", (False,))
        result = cursor.fetchone()
        assert result == (False,)

    def test_should_quote_integer(self, cursor):
        """Test that integers are properly handled."""
        cursor.execute("SELECT %s", (12345,))
        result = cursor.fetchone()
        assert result == (12345,)

    def test_should_quote_negative_integer(self, cursor):
        """Test that negative integers are properly handled."""
        cursor.execute("SELECT %s", (-12345,))
        result = cursor.fetchone()
        assert result == (-12345,)

    def test_should_quote_float(self, cursor):
        """Test that floats are properly handled."""
        cursor.execute("SELECT %s", (3.14159,))
        result = cursor.fetchone()
        assert abs(float(result[0]) - 3.14159) < 0.00001  # Snowflake may return Decimal

    def test_should_quote_empty_string(self, cursor):
        """Test that empty string is properly handled."""
        cursor.execute("SELECT %s", ("",))
        result = cursor.fetchone()
        assert result == ("",)

    def test_should_quote_binary_data(self, cursor):
        """Test that binary data is properly handled."""
        binary_data = b"\x00\x01\x02\xff"
        cursor.execute("SELECT %s::BINARY", (binary_data,))
        result = cursor.fetchone()
        assert result[0] == binary_data


@with_paramstyle("pyformat")
class TestListBinding:
    """Tests for list binding in IN clauses."""

    def test_should_bind_list_for_in_clause(self, cursor, tmp_schema):
        """Test list parameter for IN clause."""
        table_name = f"{tmp_schema}.test_list_in"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR)")
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s, %s)", (1, "Alice"))
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s, %s)", (2, "Bob"))
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s, %s)", (3, "Charlie"))

        # Query with list parameter for IN clause
        cursor.execute(f"SELECT name FROM {table_name} WHERE id IN (%s) ORDER BY id", ([1, 3],))
        result = cursor.fetchall()

        assert len(result) == 2
        assert result[0][0] == "Alice"
        assert result[1][0] == "Charlie"


@with_paramstyle("pyformat")
class TestTableOperationsWithPyformat:
    """Tests for table operations using pyformat binding."""

    def test_should_insert_with_positional_pyformat(self, cursor, tmp_schema):
        """Test INSERT with %s positional binding."""
        table_name = f"{tmp_schema}.test_insert_pyformat"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR)")

        cursor.execute(f"INSERT INTO {table_name} VALUES (%s, %s)", (1, "Alice"))

        cursor.execute(f"SELECT * FROM {table_name}")
        result = cursor.fetchone()
        assert result == (1, "Alice")

    def test_should_insert_with_named_pyformat(self, cursor, tmp_schema):
        """Test INSERT with %(name)s named binding."""
        table_name = f"{tmp_schema}.test_insert_named"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR)")

        cursor.execute(
            f"INSERT INTO {table_name} VALUES (%(id)s, %(name)s)",
            {"id": 1, "name": "Alice"},
        )

        cursor.execute(f"SELECT * FROM {table_name}")
        result = cursor.fetchone()
        assert result == (1, "Alice")

    def test_should_update_with_pyformat(self, cursor, tmp_schema):
        """Test UPDATE with pyformat binding."""
        table_name = f"{tmp_schema}.test_update_pyformat"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR)")
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s, %s)", (1, "Alice"))

        cursor.execute(f"UPDATE {table_name} SET name = %s WHERE id = %s", ("Alice Updated", 1))

        cursor.execute(f"SELECT * FROM {table_name}")
        result = cursor.fetchone()
        assert result == (1, "Alice Updated")

    def test_should_delete_with_pyformat(self, cursor, tmp_schema):
        """Test DELETE with pyformat binding."""
        table_name = f"{tmp_schema}.test_delete_pyformat"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR)")
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s, %s)", (1, "Alice"))
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s, %s)", (2, "Bob"))

        cursor.execute(f"DELETE FROM {table_name} WHERE id = %s", (1,))

        cursor.execute(f"SELECT * FROM {table_name}")
        result = cursor.fetchall()
        assert len(result) == 1
        assert result[0] == (2, "Bob")

    def test_should_select_where_with_pyformat(self, cursor, tmp_schema):
        """Test SELECT WHERE with pyformat binding."""
        table_name = f"{tmp_schema}.test_select_pyformat"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR, age NUMBER)")
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s, %s, %s)", (1, "Alice", 30))
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s, %s, %s)", (2, "Bob", 25))
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s, %s, %s)", (3, "Charlie", 35))

        cursor.execute(f"SELECT name FROM {table_name} WHERE age > %s ORDER BY name", (28,))
        result = cursor.fetchall()

        assert len(result) == 2
        assert result[0][0] == "Alice"
        assert result[1][0] == "Charlie"


@with_paramstyle("pyformat")
class TestExecutemanyWithPyformat:
    """Tests for executemany with pyformat binding."""

    def test_should_executemany_with_dict_params(self, cursor, tmp_schema):
        """Test executemany with dictionary parameters."""
        table_name = f"{tmp_schema}.test_executemany_dict"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR)")

        rows = [{"id": 1, "name": "Alice"}, {"id": 2, "name": "Bob"}, {"id": 3, "name": "Charlie"}]
        cursor.executemany(f"INSERT INTO {table_name} VALUES (%(id)s, %(name)s)", rows)

        cursor.execute(f"SELECT * FROM {table_name} ORDER BY id")
        result = cursor.fetchall()
        assert result == [(1, "Alice"), (2, "Bob"), (3, "Charlie")]

    def test_should_executemany_with_tuple_params(self, cursor, tmp_schema):
        """Test executemany with tuple parameters."""
        table_name = f"{tmp_schema}.test_executemany_tuple"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR)")

        rows = [(1, "Alice"), (2, "Bob"), (3, "Charlie")]
        cursor.executemany(f"INSERT INTO {table_name} VALUES (%s, %s)", rows)

        cursor.execute(f"SELECT * FROM {table_name} ORDER BY id")
        result = cursor.fetchall()
        assert result == [(1, "Alice"), (2, "Bob"), (3, "Charlie")]


@with_paramstyle("format")
class TestFormatBinding:
    """Tests for format paramstyle (%s only, no named parameters)."""

    def test_should_bind_with_format_style(self, cursor):
        """Test basic binding with format paramstyle."""
        sql = "SELECT %s, %s, %s"
        params = (1, "test", True)
        cursor.execute(sql, params)
        result = cursor.fetchone()
        assert result == (1, "test", True)

    def test_should_escape_special_chars_with_format(self, cursor):
        """Test escape handling with format paramstyle."""
        cursor.execute("SELECT %s", ("it's a 'test'",))
        result = cursor.fetchone()
        assert result == ("it's a 'test'",)
