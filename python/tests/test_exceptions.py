"""
Tests for PEP 249 exception classes.
"""

from unittest.mock import MagicMock

from snowflake.connector._internal.api_client.client_api import _proto_to_public_error
from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
    DriverError as ProtoDriverError,
)
from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
    DriverException as ProtoDriverException,
)
from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
    LoginError as ProtoLoginError,
)
from snowflake.connector._internal.status_codes import (
    STATUS_CODE_AUTHENTICATION_ERROR,
    STATUS_CODE_INTERNAL_ERROR,
    STATUS_CODE_INVALID_ARGUMENT,
    STATUS_CODE_INVALID_DATA,
    STATUS_CODE_INVALID_PARAMETER_VALUE,
    STATUS_CODE_LOGIN_ERROR,
    STATUS_CODE_MISSING_PARAMETER,
    STATUS_CODE_NOT_FOUND,
    STATUS_CODE_NOT_IMPLEMENTED,
    STATUS_CODE_TIMEOUT,
    STATUS_TO_EXCEPTION,
)
from snowflake.connector.errors import (
    DatabaseError,
    DataError,
    Error,
    IntegrityError,
    InterfaceError,
    InternalError,
    NotSupportedError,
    OperationalError,
    ProgrammingError,
    Warning,
)


class TestExceptionHierarchy:
    """Test the exception hierarchy as defined in PEP 249."""

    def test_warning_inheritance(self):
        assert issubclass(Warning, Warning)

    def test_error_inheritance(self):
        assert issubclass(Error, Exception)

    def test_interface_error_inheritance(self):
        assert issubclass(InterfaceError, Error)

    def test_database_error_inheritance(self):
        assert issubclass(DatabaseError, Error)

    def test_data_error_inheritance(self):
        assert issubclass(DataError, DatabaseError)

    def test_operational_error_inheritance(self):
        assert issubclass(OperationalError, DatabaseError)

    def test_integrity_error_inheritance(self):
        assert issubclass(IntegrityError, DatabaseError)

    def test_internal_error_inheritance(self):
        assert issubclass(InternalError, DatabaseError)

    def test_programming_error_inheritance(self):
        assert issubclass(ProgrammingError, DatabaseError)

    def test_not_supported_error_inheritance(self):
        assert issubclass(NotSupportedError, DatabaseError)


class TestExceptionInstantiation:
    """Test Error attributes (msg, errno, sqlstate, sfqid, query)."""

    def test_error_default_attributes(self):
        error = Error("something went wrong")
        assert error.raw_msg == "something went wrong"
        assert error.errno == -1
        assert error.sqlstate is None
        assert error.sfqid is None
        assert error.query is None

    def test_error_full_attributes(self):
        error = Error("oops", errno=42, sqlstate="HY000", sfqid="abc-123", query="SELECT 1")
        assert error.raw_msg == "oops"
        assert error.errno == 42
        assert error.sqlstate == "HY000"
        assert error.sfqid == "abc-123"
        assert error.query == "SELECT 1"

    def test_error_with_errno(self):
        error = Error("fail", errno=1003)
        assert "001003" in error.msg
        assert error.errno == 1003

    def test_interface_error_with_attributes(self):
        err = InterfaceError("closed", errno=252006)
        assert err.errno == 252006
        assert "closed" in str(err)

    def test_operational_error(self):
        err = OperationalError("timeout", errno=4)
        assert err.errno == 4

    def test_subclass_inherits_attributes(self):
        err = ProgrammingError("bad sql", errno=1003, sqlstate="42000")
        assert isinstance(err, DatabaseError)
        assert isinstance(err, Error)
        assert err.errno == 1003
        assert err.sqlstate == "42000"

    def test_not_supported_error(self):
        err = NotSupportedError("not supported")
        assert isinstance(err, DatabaseError)

    def test_error_with_query(self):
        err = Error("failed", query="SELECT 1")
        assert err.query == "SELECT 1"

    def test_plain_message(self):
        err = Error("hello")
        assert err.raw_msg == "hello"
        assert "hello" in str(err)

    def test_unknown_error_when_no_message(self):
        err = Error("")
        assert err.msg == ""


class TestStatusCodeMapping:
    """Test that proto status codes map to the correct PEP 249 exception class."""

    def test_authentication_error_maps_to_database_error(self):
        assert STATUS_TO_EXCEPTION[STATUS_CODE_AUTHENTICATION_ERROR] is DatabaseError

    def test_internal_error_maps_to_programming(self):
        assert STATUS_TO_EXCEPTION[STATUS_CODE_INTERNAL_ERROR] is ProgrammingError

    def test_login_error_maps_to_database_error(self):
        assert STATUS_TO_EXCEPTION[STATUS_CODE_LOGIN_ERROR] is DatabaseError

    def test_timeout_maps_to_operational(self):
        assert STATUS_TO_EXCEPTION[STATUS_CODE_TIMEOUT] is OperationalError

    def test_not_implemented_maps_to_not_supported(self):
        assert STATUS_TO_EXCEPTION[STATUS_CODE_NOT_IMPLEMENTED] is NotSupportedError

    def test_not_found_maps_to_programming(self):
        assert STATUS_TO_EXCEPTION[STATUS_CODE_NOT_FOUND] is ProgrammingError

    def test_invalid_argument_maps_to_programming(self):
        assert STATUS_TO_EXCEPTION[STATUS_CODE_INVALID_ARGUMENT] is ProgrammingError

    def test_missing_parameter_maps_to_programming(self):
        assert STATUS_TO_EXCEPTION[STATUS_CODE_MISSING_PARAMETER] is ProgrammingError

    def test_invalid_parameter_value_maps_to_programming(self):
        assert STATUS_TO_EXCEPTION[STATUS_CODE_INVALID_PARAMETER_VALUE] is ProgrammingError

    def test_invalid_data_maps_to_data_error(self):
        assert STATUS_TO_EXCEPTION[STATUS_CODE_INVALID_DATA] is DataError


class TestExtractErrorDetail:
    """Test _extract_error_detail helper."""

    def test_no_error_field(self):
        from snowflake.connector._internal.api_client.client_api import _extract_error_detail

        driver_exc = MagicMock()
        driver_exc.error = None
        assert _extract_error_detail(driver_exc) is None

    def test_missing_parameter(self):
        from snowflake.connector._internal.api_client.client_api import _extract_error_detail
        from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
            MissingParameter as ProtoMissingParameter,
        )

        driver_exc = MagicMock()
        driver_exc.error.WhichOneof.return_value = "missing_parameter"
        driver_exc.error.missing_parameter = ProtoMissingParameter(parameter="account")
        result = _extract_error_detail(driver_exc)
        assert "account" in result

    def test_invalid_parameter_value(self):
        from snowflake.connector._internal.api_client.client_api import _extract_error_detail
        from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
            InvalidParameterValue as ProtoInvalidParameterValue,
        )

        driver_exc = MagicMock()
        driver_exc.error.WhichOneof.return_value = "invalid_parameter_value"
        driver_exc.error.invalid_parameter_value = ProtoInvalidParameterValue(
            parameter="authenticator", value="BAD", explanation="not supported"
        )
        result = _extract_error_detail(driver_exc)
        assert "authenticator" in result
        assert "BAD" in result


class TestConvertProtoError:
    """Test _proto_to_public_error end-to-end conversion."""

    def test_application_exception_with_status_code(self):
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        driver_exc = MagicMock()
        driver_exc.message = "Query failed"
        driver_exc.status_code = STATUS_CODE_INVALID_ARGUMENT
        driver_exc.report = ""
        driver_exc.error = None
        driver_exc.HasField.return_value = False
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        assert isinstance(result, ProgrammingError)
        assert "Query failed" in str(result)

    def test_application_exception_authentication(self):
        from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
            AuthenticationError as ProtoAuthenticationError,
        )
        from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
            DriverError as ProtoDriverError,
        )
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        driver_exc = MagicMock()
        driver_exc.message = "Authentication failed"
        driver_exc.status_code = STATUS_CODE_AUTHENTICATION_ERROR
        driver_exc.report = ""
        driver_exc.error = ProtoDriverError(
            auth_error=ProtoAuthenticationError(detail="Token expired"),
        )
        driver_exc.HasField.return_value = False
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        assert isinstance(result, DatabaseError)
        assert "Authentication failed" in str(result)
        assert "Token expired" in str(result)
        assert result.errno == 250001

    def test_application_exception_not_implemented(self):
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        driver_exc = MagicMock()
        driver_exc.message = "Feature X not implemented"
        driver_exc.status_code = STATUS_CODE_NOT_IMPLEMENTED
        driver_exc.report = ""
        driver_exc.error = None
        driver_exc.HasField.return_value = False
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        assert isinstance(result, NotSupportedError)

    def test_transport_exception_becomes_operational(self):
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoTransportException,
        )

        proto_exc = ProtoTransportException("connection lost")
        result = _proto_to_public_error(proto_exc)
        assert isinstance(result, OperationalError)

    def test_unknown_exception_type_becomes_database_error(self):
        result = _proto_to_public_error(Exception("something unexpected"))
        assert isinstance(result, DatabaseError)
        assert "something unexpected" in str(result)

    def test_application_exception_preserves_errno(self):
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        driver_exc = MagicMock()
        driver_exc.message = "Internal"
        driver_exc.status_code = STATUS_CODE_INTERNAL_ERROR
        driver_exc.report = ""
        driver_exc.error = None
        driver_exc.HasField.return_value = False
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        assert isinstance(result, ProgrammingError)
        # INTERNAL_ERROR has no old-driver errno mapping, so proto status code
        # is used as fallback.
        assert result.errno == STATUS_CODE_INTERNAL_ERROR

    def test_application_exception_login_error_uses_old_errno(self):
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        driver_exc = MagicMock()
        driver_exc.message = "Login failed"
        driver_exc.status_code = STATUS_CODE_LOGIN_ERROR
        driver_exc.report = ""
        driver_exc.error = ProtoDriverError(
            login_error=ProtoLoginError(
                message="Incorrect username or password",
                code=390100,
            ),
        )
        driver_exc.HasField.return_value = False
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        assert isinstance(result, DatabaseError)
        # Login errors use ER_FAILED_TO_CONNECT_TO_DB (250001) to match old driver.
        assert result.errno == 250001
        assert result.sqlstate == "08001"

    def test_application_exception_uses_vendor_code_and_sqlstate(self):
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        driver_exc = ProtoDriverException(
            message="SQL compilation error: syntax error",
            status_code=STATUS_CODE_INTERNAL_ERROR,
            vendor_code=1003,
            sql_state="42000",
        )
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        assert isinstance(result, ProgrammingError)
        # vendor_code from proto takes priority over status-code-based mapping
        assert result.errno == 1003
        assert result.sqlstate == "42000"

    def test_application_exception_root_cause_appended(self):
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        driver_exc = ProtoDriverException(
            message="Query failed",
            status_code=STATUS_CODE_INTERNAL_ERROR,
            root_cause="division by zero",
        )
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        assert isinstance(result, ProgrammingError)
        assert "Query failed" in str(result)
        assert "division by zero" in str(result)

    def test_application_exception_root_cause_not_duplicated(self):
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        driver_exc = ProtoDriverException(
            message="division by zero",
            status_code=STATUS_CODE_INTERNAL_ERROR,
            root_cause="division by zero",
        )
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        # root_cause should not be appended when it already appears in message
        msg_str = str(result)
        assert msg_str.count("division by zero") == 1

    def test_application_exception_report_not_included(self):
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        driver_exc = MagicMock()
        driver_exc.message = "Query failed"
        driver_exc.status_code = STATUS_CODE_INVALID_ARGUMENT
        driver_exc.report = "Diagnostic report:\n  line 1: unexpected token"
        driver_exc.error = None
        driver_exc.HasField.return_value = False
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        assert "Query failed" in str(result)
        assert "Diagnostic report" not in str(result)


class TestErrorAttributes:
    """Test that errors carry expected PEP 249 attributes."""

    def test_error_has_raw_msg(self):
        err = Error("test message", errno=42)
        assert err.raw_msg == "test message"
        assert "test message" in err.msg

    def test_error_formatting_with_sqlstate(self):
        err = Error("fail", errno=1003, sqlstate="42000")
        assert "001003" in err.msg
        assert "(42000)" in err.msg
        assert "fail" in err.msg
