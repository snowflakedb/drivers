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


def test_should_handle_maximum_precision_values_of_number_correctly(cursor):
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


def test_should_cast_int_and_its_synonyms_to_appropriate_type_and_preserve_values_when_selecting_literals(
    cursor,
):
    # Given Snowflake client is logged in
    assert cursor

    # When Query selecting literal values of INT, INTEGER, BIGINT, SMALLINT, TINYINT, BYTEINT types is executed
    sql = """
        SELECT 
            123456789::INT as int_col,
            -987654321::INTEGER as integer_col,
            9223372036854775807::BIGINT as bigint_col,
            32767::SMALLINT as smallint_col,
            127::TINYINT as tinyint_col,
            -128::BYTEINT as byteint_col
        """
    cursor.execute(sql)
    values = cursor.fetchone()

    # Then All returned values should be cast to integers
    assert all(
        isinstance(value, int) for value in values
    ), f"All values should be int, got types: {[type(v) for v in values]}"

    # And All returned values should be equal to the expected literals
    expected_values = (
        123456789,
        -987654321,
        9223372036854775807,
        32767,
        127,
        -128,
    )
    assert values == expected_values, f"Expected {expected_values}, got {values}"


def test_should_cast_int_and_its_synonyms_to_appropriate_type_and_preserve_values_when_selecting_from_table(
    cursor, tmp_schema
):
    # Given Snowflake client is logged in
    table_name = f"{tmp_schema}.test_int_select_from_table"
    assert cursor, "Cursor should be open"

    # And A table with columns of types INT, INTEGER, BIGINT, SMALLINT, TINYINT, BYTEINT is created
    create_sql = (
        f"CREATE OR REPLACE TEMPORARY TABLE {table_name}"
        " (int_col INT,"
        " integer_col INTEGER,"
        " bigint_col BIGINT,"
        " smallint_col SMALLINT,"
        " tinyint_col TINYINT,"
        " byteint_col BYTEINT)"
    )
    cursor.execute(create_sql)

    # And Data is inserted into the table
    insert_sql = (
        f"INSERT INTO {table_name} VALUES"
        " (123456789, -987654321, 9223372036854775807, 32767, 127, -128)"
    )
    cursor.execute(insert_sql)

    # When Query selecting data from the table is executed
    select_sql = f"SELECT * FROM {table_name}"
    cursor.execute(select_sql)
    values = cursor.fetchone()

    # Then All returned values should be cast to integers
    assert all(
        isinstance(value, int) for value in values
    ), f"All values should be int, got types: {[type(v) for v in values]}"

    # And All returned values should be equal to the inserted values
    expected_values = (
        123456789,
        -987654321,
        9223372036854775807,
        32767,
        127,
        -128,
    )
    assert values == expected_values, f"Expected {expected_values}, got {values}"


def test_should_handle_maximum_values_of_int_correctly(cursor):
    # Given Snowflake client is logged in
    assert cursor

    # When Query "SELECT 99999999999999999999999999999999999999::INT as max_value_col" is executed
    # And Query "SELECT -99999999999999999999999999999999999999::INT as min_value_col" is executed
    sql = """SELECT 99999999999999999999999999999999999999::INT as max_value_col,
        -99999999999999999999999999999999999999::INT as min_value_col"""
    cursor.execute(sql)
    values = cursor.fetchone()

    # Then All queries should return expected integer values
    assert all(
        isinstance(value, int) for value in values
    ), f"All values should be int, got types: {[type(v) for v in values]}"

    expected_values = (
        int("99999999999999999999999999999999999999"),
        int("-99999999999999999999999999999999999999"),
    )
    assert values == expected_values, f"Expected {expected_values}, got {values}"


def test_should_cast_float_and_its_synonyms_to_appropriate_type_and_preserve_values_when_selecting_literals(
    cursor,
):
    # Given Snowflake client is logged in
    assert cursor

    # When Query selecting literal values of FLOAT, FLOAT4, FLOAT8, DOUBLE, DOUBLE PRECISION, REAL types is executed
    sql = """
        SELECT 
            3.14159::FLOAT as float_col,
            -2.71828::FLOAT4 as float4_col,
            1.41421::FLOAT8 as float8_col,
            2.99792e8::DOUBLE as double_col,
            6.62607e-34::DOUBLE PRECISION as double_precision_col,
            -1.60218e-19::REAL as real_col
        """
    cursor.execute(sql)
    values = cursor.fetchone()

    # Then All returned values should be cast to floats
    assert all(
        isinstance(value, float) for value in values
    ), f"All values should be float, got types: {[type(v) for v in values]}"

    # And All returned values should be equal to the expected literals
    expected_values = (
        3.14159,
        -2.71828,
        1.41421,
        2.99792e8,
        6.62607e-34,
        -1.60218e-19,
    )
    for actual, expected in zip(values, expected_values):
        assert actual == expected, f"Expected {expected}, got {actual}"


def test_should_cast_float_and_its_synonyms_to_appropriate_type_and_preserve_values_when_selecting_from_table(
    cursor, tmp_schema
):
    # Given Snowflake client is logged in
    table_name = f"{tmp_schema}.test_float_select_from_table"
    assert cursor, "Cursor should be open"

    # And A table with columns of types FLOAT, FLOAT4, FLOAT8, DOUBLE, DOUBLE PRECISION, REAL is created
    create_sql = (
        f"CREATE OR REPLACE TEMPORARY TABLE {table_name}"
        " (float_col FLOAT,"
        " float4_col FLOAT4,"
        " float8_col FLOAT8,"
        " double_col DOUBLE,"
        " double_precision_col DOUBLE PRECISION,"
        " real_col REAL)"
    )
    cursor.execute(create_sql)

    # And Data is inserted into the table
    insert_sql = (
        f"INSERT INTO {table_name} VALUES"
        " (3.14159, -2.71828, 1.41421, 2.99792e8, 6.62607e-34, -1.60218e-19)"
    )
    cursor.execute(insert_sql)

    # When Query selecting data from the table is executed
    select_sql = f"SELECT * FROM {table_name}"
    cursor.execute(select_sql)
    values = cursor.fetchone()

    # Then All returned values should be cast to floats
    assert all(
        isinstance(value, float) for value in values
    ), f"All values should be float, got types: {[type(v) for v in values]}"

    # And All returned values should be equal to the inserted values
    expected_values = (
        3.14159,
        -2.71828,
        1.41421,
        2.99792e8,
        6.62607e-34,
        -1.60218e-19,
    )
    for actual, expected in zip(values, expected_values):
        assert actual == expected, f"Expected {expected}, got {actual}"


def test_should_handle_extreme_values_of_float_correctly(cursor):
    import math

    # Given Snowflake client is logged in
    assert cursor

    # When Query selecting max safe integer values 9007199254740991 and -9007199254740991 is executed
    # And Query selecting extreme exponent values 1e308, 1e-308, and 1e-324 is executed
    # And Query selecting special float values NaN, Inf, and -Inf is executed
    sql = """
        SELECT
            9007199254740991::FLOAT as max_safe_int,
            (-9007199254740991)::FLOAT as min_safe_int,
            1e308::FLOAT as max_exponent,
            1e-308::FLOAT as min_normal_exponent,
            1e-324::FLOAT as min_subnormal,
            'NaN'::FLOAT as nan_val,
            'Inf'::FLOAT as pos_inf,
            '-Inf'::FLOAT as neg_inf
        """
    cursor.execute(sql)
    values = cursor.fetchone()

    # Then All queries should return expected float values
    assert all(
        isinstance(value, float) for value in values
    ), f"All values should be float, got types: {[type(v) for v in values]}"

    # Check max safe integers
    assert (
        values[0] == 9007199254740991.0
    ), f"Expected 9007199254740991.0, got {values[0]}"
    assert (
        values[1] == -9007199254740991.0
    ), f"Expected -9007199254740991.0, got {values[1]}"

    # Check extreme exponent values
    assert values[2] == 1e308, f"Expected 1e308, got {values[2]}"
    assert values[3] == 1e-308, f"Expected 1e-308, got {values[3]}"
    assert values[4] == 1e-324, f"Expected 1e-324, got {values[4]}"

    # Check special float values
    assert math.isnan(values[5]), f"Expected NaN, got {values[5]}"
    assert math.isinf(values[6]) and values[6] > 0, f"Expected +Inf, got {values[6]}"
    assert math.isinf(values[7]) and values[7] < 0, f"Expected -Inf, got {values[7]}"


def test_type_mappings_for_numeric_types_are_tested():
    # Given wrapper implements numeric types

    # Then type mapping for NUMBER should be tested

    # And type mapping for INT should be tested

    # And type mapping for FLOAT should be tested

    # And handling FLOAT subnormal value 1e-324 should be tested

    # And type mapping for DECFLOAT should be tested

    assert True
