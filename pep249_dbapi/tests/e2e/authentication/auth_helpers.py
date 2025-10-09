from ...compatibility import OLD_DRIVER_ONLY, NEW_DRIVER_ONLY
from pep249_dbapi.protobuf_gen.proto_exception import ProtoApplicationException
from pep249_dbapi.protobuf_gen.database_driver_v1_pb2 import DriverException
import snowflake.connector.errors


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

    if NEW_DRIVER_ONLY("BD#4"):
        assert isinstance(exception.value.api_error_pb, DriverException), f"Expected DriverException, got: {type(exception.value)}"
        assert exception.value.error.WhichOneof("error_type") == "login_error", "Expected login error"
        assert exception.value.error.login_error.code != 0, "Login error code should not be zero"
        assert exception.value.error.login_error.message.strip() != "", "Login error message should not be empty"

    if OLD_DRIVER_ONLY("BD#4"):
        # Reference driver uses DatabaseError from snowflake.connector.errors
        assert isinstance(exception.value, snowflake.connector.errors.DatabaseError), \
            f"Expected DatabaseError, got: {type(exception.value)}"
        # Verify it's specifically an authentication/JWT error
        error_msg = str(exception.value).lower()
        assert "jwt" in error_msg or "token" in error_msg or "invalid" in error_msg, \
            f"Expected authentication-related error, got: {exception.value}"
