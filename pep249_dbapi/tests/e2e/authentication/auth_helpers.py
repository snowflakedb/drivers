def verify_simple_query_execution(connection):
    """Verify that a simple query can be executed successfully."""
    with connection.cursor() as cursor:
        cursor.execute("SELECT 1")
        result = cursor.fetchone()
        assert result is not None
        assert result[0] == 1


def verify_login_error(exception):
    """Verify that an exception contains a valid login error with code and message."""
    assert exception is not None
    assert str(exception).strip() != "", "Login error message should not be empty"
    if hasattr(exception, 'error') and hasattr(exception.error, 'loginError') and exception.error.loginError is not None:
        assert exception.error.loginError.code != 0, "Login error code should not be zero"
        assert exception.error.loginError.message.strip() != "", "Login error message should not be empty"


