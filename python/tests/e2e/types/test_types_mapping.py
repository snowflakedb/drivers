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
