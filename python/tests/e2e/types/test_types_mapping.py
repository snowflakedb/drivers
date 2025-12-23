from decimal import Decimal


def test_should_cast_number_to_integer_when_scale_is_0(
    cursor,
):
    # Given Snowflake client is logged in
    assert cursor

    # When Query selecting values of NUMBER, DECIMAL, DEC, NUMERIC with scale 0 is executed
    sql = """
        SELECT 
            123456789::NUMBER(10,0) as number_col,
            123456::DECIMAL(10,0) as decimal_col,
            123::DEC(20,0) as dec_col,
            -5::NUMERIC(20,0) as numeric_col
        """
    cursor.execute(sql)
    values = cursor.fetchone()

    # Then All returned values should be cast to integers
    assert all(isinstance(value, int) for value in values)

    # And All returned values should be equal to the expected literals
    expected_values = (int("123456789"), int("123456"), int("123"), int("-5"))
    assert values == expected_values, f"Expected {expected_values}, got {values}"


def test_should_cast_number_to_decimal_when_scale_is_nonzero(
    cursor,
):
    # Given Snowflake client is logged in
    assert cursor

    # When Query selecting values of NUMBER, DECIMAL, DEC, NUMERIC with scale > 0 is executed
    sql = """
        SELECT 
            123.456::NUMBER(10,3) as number_col,
            -4::DECIMAL(10,6) as decimal_col,
            123456789::DEC(20,7) as dec_col,
            -12.3456789::NUMERIC(20,12) as numeric_col
        """
    cursor.execute(sql)
    values = cursor.fetchone()

    # Then All returned values should be cast to Decimal
    assert all(isinstance(value, Decimal) for value in values)

    # And All returned values should be equal to the expected literals
    expected_values = (
        Decimal("123.456"),
        Decimal("-4"),
        Decimal("123456789"),
        Decimal("-12.3456789"),
    )
    assert values == expected_values, f"Expected {expected_values}, got {values}"


def test_should_cast_int_and_its_synonyms_to_integer(
    cursor,
):
    # Given Snowflake client is logged in
    assert cursor

    # When Query selecting values of INT, INTEGER, BIGINT, SMALLINT, TINYINT, BYTEINT is executed
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


def test_should_cast_float_and_its_synonyms_to_float(
    cursor,
):
    # Given Snowflake client is logged in
    assert cursor

    # When Query selecting values of FLOAT, FLOAT4, FLOAT8, DOUBLE, DOUBLE PRECISION, REAL is executed
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


def test_should_cast_float_subnormal_value_1e_324_to_zero(
    cursor,
):
    # Given Snowflake client is logged in
    assert cursor

    # When Query selecting subnormal float value 1e-324 is executed
    sql = "SELECT 1e-324::FLOAT as subnormal_col"
    cursor.execute(sql)
    value = cursor.fetchone()[0]

    # Then The returned value should be cast to float
    assert isinstance(value, float), f"Value should be float, got {type(value)}"

    # And The returned value should be equal to 0.0
    assert value == 0.0, f"Expected 0.0, got {value}"


def test_should_cast_decfloat_to_decimal(
    cursor,
):
    # Given Snowflake client is logged in
    assert cursor

    # When Query selecting values of DECFLOAT is executed
    sql = """
        SELECT 
            3.141592653589793238462643383::DECFLOAT as decfloat_col1,
            -2.718281828459045235360287471::DECFLOAT as decfloat_col2,
            0::DECFLOAT as decfloat_col3
        """
    cursor.execute(sql)
    values = cursor.fetchone()

    # Then All returned values should be cast to Decimal
    assert all(
        isinstance(value, Decimal) for value in values
    ), f"All values should be Decimal, got types: {[type(v) for v in values]}"

    # And All returned values should be equal to the expected literals
    expected_values = (
        Decimal("3.141592653589793238462643383"),
        Decimal("-2.718281828459045235360287471"),
        Decimal("0"),
    )
    assert values == expected_values, f"Expected {expected_values}, got {values}"
