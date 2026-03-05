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

        # When Query "SELECT %s, %s, %s, %s, %s" is executed
        #   with positional parameters [42, 3.14, "hello", True, None]
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
        # Given Snowflake client is logged in with pyformat paramstyle
        # When Query "SELECT %s" is executed with parameter "it's a test"
        cursor.execute("SELECT %s", ("it's a test",))
        result = cursor.fetchone()
        # Then Result should contain the exact string "it's a test"
        assert result == ("it's a test",)

    def test_should_bind_string_with_double_quote(self, cursor):
        """Test string binding with double quote character."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # When Query "SELECT %s" is executed with parameter containing double quotes
        cursor.execute("SELECT %s", ('hello "world"',))
        result = cursor.fetchone()
        # Then Result should contain the exact string with double quotes
        assert result == ('hello "world"',)

    def test_should_bind_string_with_backslash(self, cursor):
        """Test string binding with backslash character."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # When Query "SELECT %s" is executed with parameter "path\to\file"
        cursor.execute("SELECT %s", ("path\\to\\file",))
        result = cursor.fetchone()
        # Then Result should contain the exact string with backslashes
        assert result == ("path\\to\\file",)

    def test_should_bind_string_with_newline(self, cursor):
        """Test string binding with newline character."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # When Query "SELECT %s" is executed with parameter containing newline
        cursor.execute("SELECT %s", ("line1\nline2",))
        result = cursor.fetchone()
        # Then Result should contain the exact string with newline
        assert result == ("line1\nline2",)

    def test_should_bind_string_with_tab(self, cursor):
        """Test string binding with tab character."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # When Query "SELECT %s" is executed with parameter containing tab
        cursor.execute("SELECT %s", ("col1\tcol2",))
        result = cursor.fetchone()
        # Then Result should contain the exact string with tab
        assert result == ("col1\tcol2",)

    def test_should_bind_string_with_carriage_return(self, cursor):
        """Test string binding with carriage return character."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # When Query "SELECT %s" is executed with parameter containing carriage return
        cursor.execute("SELECT %s", ("line1\rline2",))
        result = cursor.fetchone()
        # Then Result should contain the exact string with carriage return
        assert result == ("line1\rline2",)


@with_paramstyle("pyformat")
class TestPyformatNamedBinding:
    """Tests for pyformat %(name)s named binding (client-side interpolation)."""

    def test_should_bind_basic_types_with_named_pyformat(self, cursor):
        """Test basic type binding with %(name)s placeholders."""
        # Given Snowflake client is logged in with pyformat paramstyle

        # When Query "SELECT %(a)s, %(b)s, %(c)s" is executed
        #   with named parameters (a=100, b="test", c=True)
        sql = "SELECT %(a)s, %(b)s, %(c)s"
        params = dict(a=100, b="test", c=True)
        cursor.execute(sql, params)
        result = cursor.fetchone()

        # Then Result should contain values matching the bound parameters
        assert result == (100, "test", True)

    def test_should_bind_same_parameter_multiple_times(self, cursor):
        """Test that the same named parameter can be used multiple times."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # When Query "SELECT %(val)s, %(val)s, %(val)s" is executed
        #   with named parameter (val=42)
        sql = "SELECT %(val)s, %(val)s, %(val)s"
        params = dict(val=42)
        cursor.execute(sql, params)
        result = cursor.fetchone()
        # Then Result should contain [42, 42, 42]
        assert result == (42, 42, 42)

    def test_should_bind_with_mixed_order_named_params(self, cursor):
        """Test named parameters used in different order than dict."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # When Query "SELECT %(z)s, %(a)s, %(m)s" is executed
        #   with named parameters (a=1, m=2, z=3)
        sql = "SELECT %(z)s, %(a)s, %(m)s"
        params = dict(a=1, m=2, z=3)
        cursor.execute(sql, params)
        result = cursor.fetchone()
        # Then Result should contain [3, 1, 2]
        assert result == (3, 1, 2)


@with_paramstyle("pyformat")
class TestEscapeHandling:
    """Tests for proper escape handling in client-side binding."""

    def test_should_escape_special_characters(self, cursor, tmp_schema):
        """Test that special characters are properly escaped."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # And A temporary table with columns (name VARCHAR) exists
        table_name = f"{tmp_schema}.test_escape"
        cursor.execute(f"CREATE TABLE {table_name} (name VARCHAR)")

        # When Strings with special characters
        #   (newlines, backslashes, quotes, tabs) are inserted using %s binding
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

        for s in test_strings:
            cursor.execute(f"INSERT INTO {table_name} VALUES (%s)", (s,))

        # Then All strings should be retrievable with exact content preserved
        cursor.execute(f"SELECT name FROM {table_name}")
        results = {row[0] for row in cursor.fetchall()}

        for expected in test_strings:
            assert expected in results, f"String {expected!r} not found in results"

    def test_should_prevent_sql_injection_with_positional_binding(self, cursor, tmp_schema):
        """Test that SQL injection attempts are safely escaped."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # And A temporary table with columns (id NUMBER, name VARCHAR) exists
        # And Rows are inserted
        table_name = f"{tmp_schema}.test_injection"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR)")
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s, %s)", (1, "test1"))
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s, %s)", (2, "test2"))

        # When Query with "WHERE id = %s" is executed with SQL injection string "1 or 1=1"

        # The injection string '1 or 1=1' will be quoted as a string literal,
        # which then cannot be converted to NUMBER, causing a type error.
        with pytest.raises(Exception) as excinfo:
            cursor.execute(f"SELECT * FROM {table_name} WHERE id = %s", ("1 or 1=1",))

        # Then Error should be raised for numeric type conversion (injection is prevented)
        error_msg = str(excinfo.value).lower()
        assert "numeric" in error_msg or "number" in error_msg or "type" in error_msg, (
            f"Expected numeric type error, got: {excinfo.value}"
        )

    def test_should_prevent_sql_injection_with_named_binding(self, cursor, tmp_schema):
        """Test that SQL injection attempts are safely escaped with named params."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # And A temporary table with columns (id NUMBER, name VARCHAR) exists
        # And Rows are inserted
        table_name = f"{tmp_schema}.test_injection_named"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR)")
        cursor.execute(f"INSERT INTO {table_name} VALUES (%(id)s, %(name)s)", dict(id=1, name="test1"))
        cursor.execute(f"INSERT INTO {table_name} VALUES (%(id)s, %(name)s)", dict(id=2, name="test2"))

        # When Query with WHERE id = %(id)s is executed with SQL injection string "1 or 1=1"
        with pytest.raises(Exception) as excinfo:
            cursor.execute(f"SELECT * FROM {table_name} WHERE id = %(id)s", dict(id="1 or 1=1"))

        # Then Error should be raised for numeric type conversion (injection is prevented)
        error_msg = str(excinfo.value).lower()
        assert "numeric" in error_msg or "number" in error_msg or "type" in error_msg, (
            f"Expected numeric type error, got: {excinfo.value}"
        )

    def test_should_handle_complex_escape_sequence(self, cursor):
        """Test complex string with multiple escape sequences."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # When Query "SELECT %s" is executed with a complex string
        #   containing quotes, backslashes, and newlines
        complex_string = "',an\\\\escaped\"line\n"
        cursor.execute("SELECT %s", (complex_string,))
        result = cursor.fetchone()
        # Then Result should contain the exact complex string
        assert result == (complex_string,)


@with_paramstyle("pyformat")
class TestQuoteHandling:
    """Tests for proper quote handling in client-side binding."""

    def test_should_quote_null_as_null(self, cursor):
        """Test that None is converted to NULL."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # When Query "SELECT %s" is executed with parameter None
        cursor.execute("SELECT %s", (None,))
        result = cursor.fetchone()
        # Then Result should contain NULL
        assert result == (None,)

    def test_should_quote_boolean_true(self, cursor):
        """Test that True is properly handled."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # When Query "SELECT %s" is executed with parameter True
        cursor.execute("SELECT %s", (True,))
        result = cursor.fetchone()
        # Then Result should contain True
        assert result == (True,)

    def test_should_quote_boolean_false(self, cursor):
        """Test that False is properly handled."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # When Query "SELECT %s" is executed with parameter False
        cursor.execute("SELECT %s", (False,))
        result = cursor.fetchone()
        # Then Result should contain False
        assert result == (False,)

    def test_should_quote_integer(self, cursor):
        """Test that integers are properly handled."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # When Query "SELECT %s" is executed with parameter 12345
        cursor.execute("SELECT %s", (12345,))
        result = cursor.fetchone()
        # Then Result should contain 12345
        assert result == (12345,)

    def test_should_quote_negative_integer(self, cursor):
        """Test that negative integers are properly handled."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # When Query "SELECT %s" is executed with parameter -12345
        cursor.execute("SELECT %s", (-12345,))
        result = cursor.fetchone()
        # Then Result should contain -12345
        assert result == (-12345,)

    def test_should_quote_float(self, cursor):
        """Test that floats are properly handled."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # When Query "SELECT %s" is executed with parameter 3.14159
        cursor.execute("SELECT %s", (3.14159,))
        result = cursor.fetchone()
        # Then Result should contain approximately 3.14159
        assert abs(float(result[0]) - 3.14159) < 0.00001  # Snowflake may return Decimal

    def test_should_quote_empty_string(self, cursor):
        """Test that empty string is properly handled."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # When Query "SELECT %s" is executed with parameter ""
        cursor.execute("SELECT %s", ("",))
        result = cursor.fetchone()
        # Then Result should contain empty string
        assert result == ("",)

    def test_should_quote_binary_data(self, cursor):
        """Test that binary data is properly handled."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # When Query "SELECT %s::BINARY" is executed with binary parameter
        binary_data = b"\x00\x01\x02\xff"
        cursor.execute("SELECT %s::BINARY", (binary_data,))
        result = cursor.fetchone()
        # Then Result should contain the same binary data
        assert result[0] == binary_data


@with_paramstyle("pyformat")
class TestListBinding:
    """Tests for list binding in IN clauses."""

    def test_should_bind_list_for_in_clause(self, cursor, tmp_schema):
        """Test list parameter for IN clause."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # And A temporary table with columns (id NUMBER, name VARCHAR) exists
        # And Rows [1, "Alice"], [2, "Bob"], [3, "Charlie"] are inserted
        table_name = f"{tmp_schema}.test_list_in"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR)")
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s, %s)", (1, "Alice"))
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s, %s)", (2, "Bob"))
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s, %s)", (3, "Charlie"))

        # When Query with WHERE id IN (%s) is executed with list parameter [1, 3]
        cursor.execute(f"SELECT name FROM {table_name} WHERE id IN (%s) ORDER BY id", ([1, 3],))
        result = cursor.fetchall()

        # Then Result should contain rows for "Alice" and "Charlie"
        assert len(result) == 2
        assert result[0][0] == "Alice"
        assert result[1][0] == "Charlie"


@with_paramstyle("pyformat")
class TestTableOperationsWithPyformat:
    """Tests for table operations using pyformat binding."""

    def test_should_insert_with_positional_pyformat(self, cursor, tmp_schema):
        """Test INSERT with %s positional binding."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # And A temporary table with columns (id NUMBER, name VARCHAR) exists
        table_name = f"{tmp_schema}.test_insert_pyformat"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR)")

        # When Row is inserted using "INSERT INTO table VALUES (%s, %s)"
        #   with parameters (1, "Alice")
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s, %s)", (1, "Alice"))

        # Then Table should contain [1, "Alice"]
        cursor.execute(f"SELECT * FROM {table_name}")
        result = cursor.fetchone()
        assert result == (1, "Alice")

    def test_should_insert_with_named_pyformat(self, cursor, tmp_schema):
        """Test INSERT with %(name)s named binding."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # And A temporary table with columns (id NUMBER, name VARCHAR) exists
        table_name = f"{tmp_schema}.test_insert_named"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR)")

        # When Row is inserted using
        #   "INSERT INTO table VALUES (%(id)s, %(name)s)"
        #   with named parameters (id=1, name="Alice")
        cursor.execute(
            f"INSERT INTO {table_name} VALUES (%(id)s, %(name)s)",
            dict(id=1, name="Alice"),
        )

        # Then Table should contain [1, "Alice"]
        cursor.execute(f"SELECT * FROM {table_name}")
        result = cursor.fetchone()
        assert result == (1, "Alice")

    def test_should_update_with_pyformat(self, cursor, tmp_schema):
        """Test UPDATE with pyformat binding."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # And A temporary table with columns (id NUMBER, name VARCHAR) exists
        # And Row [1, "Alice"] is inserted
        table_name = f"{tmp_schema}.test_update_pyformat"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR)")
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s, %s)", (1, "Alice"))

        # When Query "UPDATE table SET name = %s WHERE id = %s"
        #   is executed with parameters ("Alice Updated", 1)
        cursor.execute(f"UPDATE {table_name} SET name = %s WHERE id = %s", ("Alice Updated", 1))

        # Then Table should contain [1, "Alice Updated"]
        cursor.execute(f"SELECT * FROM {table_name}")
        result = cursor.fetchone()
        assert result == (1, "Alice Updated")

    def test_should_delete_with_pyformat(self, cursor, tmp_schema):
        """Test DELETE with pyformat binding."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # And A temporary table with columns (id NUMBER, name VARCHAR) exists
        # And Rows [1, "Alice"] and [2, "Bob"] are inserted
        table_name = f"{tmp_schema}.test_delete_pyformat"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR)")
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s, %s)", (1, "Alice"))
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s, %s)", (2, "Bob"))

        # When Query "DELETE FROM table WHERE id = %s" is executed with parameter (1)
        cursor.execute(f"DELETE FROM {table_name} WHERE id = %s", (1,))

        # Then Table should contain only [2, "Bob"]
        cursor.execute(f"SELECT * FROM {table_name}")
        result = cursor.fetchall()
        assert len(result) == 1
        assert result[0] == (2, "Bob")

    def test_should_select_where_with_pyformat(self, cursor, tmp_schema):
        """Test SELECT WHERE with pyformat binding."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # And A temporary table with columns (id NUMBER, name VARCHAR, age NUMBER) exists
        # And Rows [1, "Alice", 30], [2, "Bob", 25], [3, "Charlie", 35] are inserted
        table_name = f"{tmp_schema}.test_select_pyformat"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR, age NUMBER)")
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s, %s, %s)", (1, "Alice", 30))
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s, %s, %s)", (2, "Bob", 25))
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s, %s, %s)", (3, "Charlie", 35))

        # When Query "SELECT name FROM table WHERE age > %s ORDER BY name"
        #   is executed with parameter (28)
        cursor.execute(f"SELECT name FROM {table_name} WHERE age > %s ORDER BY name", (28,))
        result = cursor.fetchall()

        # Then Result should contain rows for "Alice" and "Charlie"
        assert len(result) == 2
        assert result[0][0] == "Alice"
        assert result[1][0] == "Charlie"


@with_paramstyle("pyformat")
class TestExecutemanyWithPyformat:
    """Tests for executemany with pyformat binding."""

    def test_should_executemany_with_dict_params(self, cursor, tmp_schema):
        """Test executemany with dictionary parameters."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # And A temporary table with columns (id NUMBER, name VARCHAR) exists
        table_name = f"{tmp_schema}.test_executemany_dict"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR)")

        # When executemany is called with dict parameters
        #   [(id=1, name="Alice"), (id=2, name="Bob"), (id=3, name="Charlie")]
        rows = [dict(id=1, name="Alice"), dict(id=2, name="Bob"), dict(id=3, name="Charlie")]
        cursor.executemany(f"INSERT INTO {table_name} VALUES (%(id)s, %(name)s)", rows)

        # Then Table should contain 3 rows with correct values
        cursor.execute(f"SELECT * FROM {table_name} ORDER BY id")
        result = cursor.fetchall()
        assert result == [(1, "Alice"), (2, "Bob"), (3, "Charlie")]

    def test_should_executemany_with_tuple_params(self, cursor, tmp_schema):
        """Test executemany with tuple parameters."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # And A temporary table with columns (id NUMBER, name VARCHAR) exists
        table_name = f"{tmp_schema}.test_executemany_tuple"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR)")

        # When executemany is called with tuple parameters
        #   [(1, "Alice"), (2, "Bob"), (3, "Charlie")]
        rows = [(1, "Alice"), (2, "Bob"), (3, "Charlie")]
        cursor.executemany(f"INSERT INTO {table_name} VALUES (%s, %s)", rows)

        # Then Table should contain 3 rows with correct values
        cursor.execute(f"SELECT * FROM {table_name} ORDER BY id")
        result = cursor.fetchall()
        assert result == [(1, "Alice"), (2, "Bob"), (3, "Charlie")]


@with_paramstyle("format")
class TestFormatBinding:
    """Tests for format paramstyle (%s only, no named parameters)."""

    def test_should_bind_with_format_style(self, cursor):
        """Test basic binding with format paramstyle."""
        # Given Snowflake client is logged in with format paramstyle
        # When Query "SELECT %s, %s, %s" is executed with parameters (1, "test", True)
        sql = "SELECT %s, %s, %s"
        params = (1, "test", True)
        cursor.execute(sql, params)
        result = cursor.fetchone()
        # Then Result should contain [1, "test", True]
        assert result == (1, "test", True)

    def test_should_escape_special_chars_with_format(self, cursor):
        """Test escape handling with format paramstyle."""
        # Given Snowflake client is logged in with format paramstyle
        # When Query "SELECT %s" is executed with parameter "it's a 'test'"
        cursor.execute("SELECT %s", ("it's a 'test'",))
        result = cursor.fetchone()
        # Then Result should contain the exact string "it's a 'test'"
        assert result == ("it's a 'test'",)


@with_paramstyle("pyformat")
class TestClientSideBindingErrors:
    """Tests for error handling in client-side binding."""

    def test_should_raise_error_for_too_many_positional_parameters(self, cursor):
        """Test that too many positional parameters raises TypeError."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # When Query "SELECT %s, %s" is executed with 3 parameters
        # Then TypeError should be raised indicating not all arguments converted
        with pytest.raises(TypeError, match="not all arguments converted"):
            cursor.execute("SELECT %s, %s", (1, 2, 3))

    def test_should_raise_error_for_not_enough_positional_parameters(self, cursor):
        """Test that not enough positional parameters raises TypeError."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # When Query "SELECT %s, %s, %s" is executed with 2 parameters
        # Then TypeError should be raised indicating not enough arguments
        with pytest.raises(TypeError, match="not enough arguments"):
            cursor.execute("SELECT %s, %s, %s", (1, 2))

    def test_should_raise_error_for_missing_named_parameter(self, cursor):
        """Test that missing named parameter raises KeyError."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # When Query "SELECT %(name)s, %(age)s" is executed
        #   with only (name="Alice")
        # Then KeyError should be raised for missing parameter
        with pytest.raises(KeyError):
            cursor.execute("SELECT %(name)s, %(age)s", dict(name="Alice"))

    def test_should_ignore_extra_named_parameters(self, cursor):
        """Test that extra named parameters are silently ignored (Python behavior)."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # When Query "SELECT %(name)s" is executed
        #   with (name="Alice", extra="ignored")
        cursor.execute("SELECT %(name)s", dict(name="Alice", extra="ignored"))
        result = cursor.fetchone()
        # Then Result should contain "Alice" and no error is raised
        assert result == ("Alice",)


class TestInvalidParamstyle:
    """Tests for invalid paramstyle handling."""

    def test_should_raise_error_for_invalid_paramstyle(self, connection_factory):
        """Test that invalid paramstyle raises ProgrammingError."""
        from snowflake.connector import ProgrammingError

        # Given Connection is created with paramstyle "invalid"
        # Then ProgrammingError should be raised indicating invalid paramstyle
        with pytest.raises(ProgrammingError, match="Invalid paramstyle"):
            connection_factory(paramstyle="invalid")


@with_paramstyle("pyformat")
class TestDecimalBinding:
    """Tests for Decimal type binding."""

    def test_should_bind_decimal_value(self, cursor):
        """Test that Decimal values are properly handled."""
        from decimal import Decimal

        # Given Snowflake client is logged in with pyformat paramstyle
        # When Query "SELECT %s" is executed with Decimal("123.456")
        cursor.execute("SELECT %s", (Decimal("123.456"),))
        result = cursor.fetchone()
        # Then Result should contain approximately 123.456
        assert abs(float(result[0]) - 123.456) < 0.001

    def test_should_bind_decimal_with_high_precision(self, cursor):
        """Test Decimal with high precision."""
        from decimal import Decimal

        # Given Snowflake client is logged in with pyformat paramstyle
        # When Query "SELECT %s::NUMBER(38,18)" is executed with high-precision Decimal
        value = Decimal("12345678901234567890.123456789012345678")
        cursor.execute("SELECT %s::NUMBER(38,18)", (value,))
        result = cursor.fetchone()
        # Then Result should preserve full precision
        assert str(result[0]) == str(value)


@with_paramstyle("pyformat")
class TestLiteralPercentInQuery:
    """Tests for queries containing literal percent signs."""

    def test_should_handle_like_with_percent_wildcard(self, cursor, tmp_schema):
        """Test LIKE queries with %% escaped percent signs."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # And A temporary table with columns (name VARCHAR) exists
        # And Rows ["Alice"], ["Bob"], ["Charlie"] are inserted
        table_name = f"{tmp_schema}.test_like_percent"
        cursor.execute(f"CREATE TABLE {table_name} (name VARCHAR)")
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s)", ("Alice",))
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s)", ("Bob",))
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s)", ("Charlie",))

        # When Query with LIKE '%%li%%' pattern is executed (escaped percent signs)
        cursor.execute(f"SELECT name FROM {table_name} WHERE name LIKE '%%li%%' ORDER BY name")
        result = cursor.fetchall()

        # Then Result should contain "Alice" and "Charlie"
        assert len(result) == 2
        assert result[0][0] == "Alice"
        assert result[1][0] == "Charlie"

    def test_should_handle_like_with_param_and_percent(self, cursor, tmp_schema):
        """Test LIKE with both parameter and literal percent."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # And A temporary table with columns (name VARCHAR) exists
        # And Rows ["Alice"], ["Bob"] are inserted
        table_name = f"{tmp_schema}.test_like_mixed"
        cursor.execute(f"CREATE TABLE {table_name} (name VARCHAR)")
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s)", ("Alice",))
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s)", ("Bob",))

        # When Query with LIKE %s is executed with parameter "%li%"
        cursor.execute(
            f"SELECT name FROM {table_name} WHERE name LIKE %s",
            ("%li%",),  # The percent is part of the parameter value
        )
        result = cursor.fetchall()
        # Then Result should contain "Alice"
        assert len(result) == 1
        assert result[0][0] == "Alice"


@with_paramstyle("pyformat")
class TestListBindingWithSpecialChars:
    """Tests for list binding with special characters (escaping)."""

    def test_should_bind_list_with_special_chars_in_strings(self, cursor, tmp_schema):
        """Test list binding where strings contain special characters."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # And A temporary table with columns (name VARCHAR) exists
        # And Rows with special characters are inserted
        table_name = f"{tmp_schema}.test_list_special"
        cursor.execute(f"CREATE TABLE {table_name} (name VARCHAR)")
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s)", ("it's Alice",))
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s)", ("Bob\\n",))
        cursor.execute(f"INSERT INTO {table_name} VALUES (%s)", ("Charlie",))

        # When Query with WHERE name IN (%s) is executed with list containing special characters
        cursor.execute(
            f"SELECT name FROM {table_name} WHERE name IN (%s) ORDER BY name",
            (["it's Alice", "Bob\\n"],),
        )
        result = cursor.fetchall()

        # Then Result should contain the matching rows with preserved special characters
        assert len(result) == 2
        assert result[0][0] == "Bob\\n"
        assert result[1][0] == "it's Alice"


@with_paramstyle("format")
class TestFormatParamstyleErrors:
    """Tests for error handling with format paramstyle."""

    @pytest.mark.skip_reference(reason="Universal driver has stricter format paramstyle validation")
    def test_should_reject_dict_params_with_format_paramstyle(self, cursor):
        """Test that dict parameters raise ProgrammingError with format paramstyle."""
        from snowflake.connector import ProgrammingError

        # Given Snowflake client is logged in with format paramstyle
        # When Query "SELECT %(name)s" is executed with dict parameters
        # Then ProgrammingError should be raised indicating dict parameters not supported
        with pytest.raises(ProgrammingError, match="Dict parameters not supported"):
            cursor.execute("SELECT %(name)s", dict(name="Alice"))


@with_paramstyle("pyformat")
class TestExecutemanyRowcount:
    """Tests for executemany rowcount accumulation."""

    def test_should_accumulate_rowcount_in_executemany(self, cursor, tmp_schema):
        """Test that executemany accumulates rowcount across iterations."""
        # Given Snowflake client is logged in with pyformat paramstyle
        # And A temporary table with columns (id NUMBER, name VARCHAR) exists
        table_name = f"{tmp_schema}.test_executemany_rowcount"
        cursor.execute(f"CREATE TABLE {table_name} (id NUMBER, name VARCHAR)")

        # When executemany is called to insert 3 rows using %s binding
        rows = [(1, "Alice"), (2, "Bob"), (3, "Charlie")]
        cursor.executemany(f"INSERT INTO {table_name} VALUES (%s, %s)", rows)

        # Then cursor.rowcount should be 3
        assert cursor.rowcount == 3
