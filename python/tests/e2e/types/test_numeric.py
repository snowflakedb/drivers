from decimal import Decimal


def test_should_cast_number_and_its_synonyms_to_appropriate_type_and_preserve_values_when_selecting_literals(
    cursor,
):
    # Given Snowflake client is logged in
    assert not cursor.connection.is_closed(), "Connection should be open"

    # When Query selecting literal values of NUMBER, DECIMAL, DEC, NUMERIC types is executed
    sql = """
        SELECT 
            123.456789::NUMBER(10,6) as number_col,
            123.456789::DECIMAL(10,6) as decimal_col,
            123.456789::DEC(10,6) as dec_col,
            123.456789::NUMERIC(10,6) as numeric_col
        """
    cursor.execute(sql)
    values = cursor.fetchone()

    # Then All returned values should be of appropriate type
    for value in values:
        assert isinstance(value, Decimal), f"Value should be Decimal, got {type(value)}"

    # And All returned values should be equal to the expected literals
    expected_value = Decimal("123.456789")
    assert values == (
        expected_value,
        expected_value,
        expected_value,
        expected_value,
    ), f"Expected {expected_value}, got {values}"


def test_should_cast_number_and_its_synonyms_to_appropriate_type_and_preserve_values_when_selecting_from_table(
    cursor,
):
    """Test SELECT FROM TABLE with NUMBER, DECIMAL, DEC, NUMERIC types."""
    # Given Snowflake client is logged in
    table_name = "test_number_select_from_table"
    assert not cursor.connection.is_closed(), "Connection should be open"
    try:
        # And A table with columns of types NUMBER, DECIMAL, DEC, NUMERIC is created

        # SQL weirdly split into lines as test validator had a stroke reading multiline strings
        create_sql = (
            f"CREATE OR REPLACE TEMPORARY TABLE {table_name}"
            " (number_col NUMBER(38,0),"
            " decimal_col DECIMAL(38,0),"
            " dec_col DEC(38,0),"
            " numeric_col NUMERIC(38,0))"
        )
        cursor.execute(create_sql)
        # And Data is inserted into the table
        insert_sql = f"INSERT INTO {table_name} VALUES (123.456789, 123.456789, 123.456789, 123.456789)"
        cursor.execute(insert_sql)

        # When Query selecting data from the table is executed
        select_sql = f"SELECT * FROM {table_name}"
        cursor.execute(select_sql)
        values = cursor.fetchone()

        # Then All returned values should be of appropriate type
        for value in values:
            assert isinstance(
                value, Decimal
            ), f"Value should be Decimal, got {type(value)}"
        # And All returned values should be equal to the inserted values
        expected_values = (
            Decimal("123.456789"),
            Decimal("123.456789"),
            Decimal("123.456789"),
        )
        assert values == expected_values, f"Expected {expected_values}, got {values}"
    finally:
        try:
            cursor.execute(f"DROP TABLE IF EXISTS {table_name}")
        except Exception:
            pass


def test_should_handle_maximum_precision_values_correctly(cursor):
    # Given Snowflake client is logged in
    assert not cursor.connection.is_closed(), "Connection should be open"

    # When Query "SELECT 1.2345678901234567890123456789012345678::NUMBER(38,37) as max_precision_col" is executed
    # And Query "SELECT 99999999999999999999999999999999999999::NUMBER(38,0) as max_value_col" is executed
    # And Query "SELECT -99999999999999999999999999999999999999::NUMBER(38,0) as min_value_col" is executed
    sql = """SELECT 1.2345678901234567890123456789012345678::NUMBER(38,37) as max_precision_col,
        99999999999999999999999999999999999999::NUMBER(38,0) as max_value_col,
        -99999999999999999999999999999999999999::NUMBER(38,0) as min_value_col"""
    cursor.execute(sql)
    values = cursor.fetchone()

    # Then All queries should return expected values
    expected_values = (
        Decimal("1.2345678901234567890123456789012345678"),
        Decimal("99999999999999999999999999999999999999"),
        Decimal("-99999999999999999999999999999999999999"),
    )
    assert values == expected_values, f"Expected {expected_values}, got {values}"
