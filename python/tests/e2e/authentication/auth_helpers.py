from snowflake.connector.errors import DatabaseError


def verify_simple_query_execution(connection):
    """Verify that a simple query can be executed successfully."""
    with connection.cursor() as cursor:
        cursor.execute("SELECT 1")
        result = cursor.fetchone()
        assert result is not None
        assert result[0] == 1


def verify_login_error(exception, keywords):
    """Verify that an exception is a DatabaseError from an authentication failure.

    Asserts that every keyword in *keywords* appears in the error message
    (case-insensitive).
    """
    assert exception is not None
    assert str(exception).strip() != "", "Login error message should not be empty"

    assert isinstance(exception.value, DatabaseError), f"Expected DatabaseError, got: {type(exception.value)}"

    error_msg = str(exception.value).lower()
    for kw in keywords:
        assert kw in error_msg, f"Expected error to contain {kw!r}, got: {exception.value}"
