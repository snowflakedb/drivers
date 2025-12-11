from decimal import Decimal


def test_should_cast_number_and_its_synonyms_to_appropriate_type_and_preserve_values_when_selecting_literals(
    cursor,
):
    # Given Snowflake client is logged in
    assert cursor

    # When Query selecting literal values of NUMBER, DECIMAL, DEC, NUMERIC types is executed
    sql = """
        SELECT 
            123456789::NUMBER(10,0) as number_col,
            123456.789::DECIMAL(10,3) as decimal_col,
            123::DEC(20,6) as dec_col,
            0.123456789::NUMERIC(20,9) as numeric_col
        """
    cursor.execute(sql)
    values = cursor.fetchone()

    # Then All returned values should be of appropriate type
    expected_types = (int, Decimal, Decimal, Decimal)
    for i, (value, expected_type) in enumerate(zip(values, expected_types)):
        assert isinstance(
            value, expected_type
        ), f"{i+1}th value should be {expected_type}, got {type(value)}"

    # And All returned values should be equal to the expected literals
    expected_values = (
        int("123456789"),
        Decimal("123456.789"),
        Decimal("123.0"),
        Decimal("0.123456789"),
    )
    assert values == expected_values, f"Expected {expected_values}, got {values}"


def test_should_cast_number_and_its_synonyms_to_appropriate_type_and_preserve_values_when_selecting_from_table(
    cursor, tmp_schema
):
    # Given Snowflake client is logged in
    table_name = f"{tmp_schema}.test_number_select_from_table"
    assert cursor, "Cursor should be open"
    # And A table with columns of types NUMBER, DECIMAL, DEC, NUMERIC is created

    # SQL weirdly split into lines as test validator had a stroke reading multiline strings
    create_sql = (
        f"CREATE OR REPLACE TEMPORARY TABLE {table_name}"
        " (number_col NUMBER(10,0),"
        " decimal_col DECIMAL(10,3),"
        " dec_col DEC(20,6),"
        " numeric_col NUMERIC(20,9))"
    )
    cursor.execute(create_sql)
    # And Data is inserted into the table
    insert_sql = (
        f"INSERT INTO {table_name} VALUES (123456789, 123456.789, 123.0, 0.123456789)"
    )
    cursor.execute(insert_sql)

    # When Query selecting data from the table is executed
    select_sql = f"SELECT * FROM {table_name}"
    cursor.execute(select_sql)
    values = cursor.fetchone()

    # Then All returned values should be of appropriate type
    expected_types = (int, Decimal, Decimal, Decimal)
    for value, expected_type in zip(values, expected_types):
        assert isinstance(
            value, expected_type
        ), f"Value should be Decimal, got {type(value)}"
    # And All returned values should be equal to the inserted values
    expected_values = (
        int("123456789"),
        Decimal("123456.789"),
        Decimal("123.0"),
        Decimal("0.123456789"),
    )
    assert values == expected_values, f"Expected {expected_values}, got {values}"


def test_should_handle_maximum_precision_values_correctly(cursor):
    # Given Snowflake client is logged in
    assert cursor

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
        int("99999999999999999999999999999999999999"),
        int("-99999999999999999999999999999999999999"),
    )
    assert values == expected_values, f"Expected {expected_values}, got {values}"


def test_type_mappings_for_numeric_types_are_tested():
    # Given wrapper implements numeric types

    # Then type mapping for NUMBER should be tested

    # And type mapping for INT should be tested

    # And type mapping for FLOAT should be tested

    # And type mapping for DECFLOAT should be tested

    assert True
