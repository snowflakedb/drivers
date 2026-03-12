from snowflake.connector.errors import DatabaseError


def verify_simple_query_execution(connection):
    """Verify that a simple query can be executed successfully."""
    with connection.cursor() as cursor:
        cursor.execute("SELECT 1")
        result = cursor.fetchone()
        assert result is not None
        assert result[0] == 1


def verify_login_error(exception):
    """Verify that an exception contains a valid login error with code and message."""
    # Debug information about the exception can be seen when tests fail
    assert exception is not None
    assert str(exception).strip() != "", "Login error message should not be empty"

    assert isinstance(exception.value, DatabaseError), f"Expected DatabaseError, got: {type(exception.value)}"
    # Verify it's specifically an authentication/JWT error
    error_msg = str(exception.value).lower()
    assert "jwt" in error_msg or "token" in error_msg or "invalid" in error_msg, (
        f"Expected authentication-related error, got: {exception.value}"
    )
