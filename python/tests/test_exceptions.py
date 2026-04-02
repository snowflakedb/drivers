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
    STATUS_TO_EXCEPTION,
)
from snowflake.connector._internal.errorcode import ER_HTTP_GENERAL_ERROR
from snowflake.connector.errors import (
    BadGatewayError,
    BadRequest,
    BindUploadError,
    ConfigManagerError,
    ConfigSourceError,
    DatabaseError,
    DataError,
    Error,
    ForbiddenError,
    GatewayTimeoutError,
    IntegrityError,
    InterfaceError,
    InternalError,
    InternalServerError,
    MethodNotAllowed,
    MissingConfigOptionError,
    MissingDependencyError,
    NotSupportedError,
    OperationalError,
    OtherHTTPRetryableError,
    PresignedUrlExpiredError,
    ProgrammingError,
    RefreshTokenError,
    RequestExceedMaxRetryError,
    RequestTimeoutError,
    RevocationCheckError,
    ServiceUnavailableError,
    TokenExpiredError,
    TooManyRequests,
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


class TestConfigHierarchyFix:
    """ConfigSourceError must be a sibling of ConfigManagerError, not a subclass."""

    def test_config_source_error_inherits_error(self):
        assert issubclass(ConfigSourceError, Error)

    def test_config_source_error_not_config_manager_subclass(self):
        assert not issubclass(ConfigSourceError, ConfigManagerError)

    def test_config_manager_error_inherits_error(self):
        assert issubclass(ConfigManagerError, Error)

    def test_missing_config_option_error_inherits_config_source(self):
        assert issubclass(MissingConfigOptionError, ConfigSourceError)

    def test_missing_config_option_error_not_config_manager_subclass(self):
        assert not issubclass(MissingConfigOptionError, ConfigManagerError)

    def test_config_source_error_not_caught_by_config_manager_handler(self):
        raised = ConfigSourceError("bad source")
        caught = False
        try:
            raise raised
        except ConfigManagerError:
            caught = True
        except ConfigSourceError:
            pass
        assert not caught, "ConfigSourceError must not be catchable as ConfigManagerError"


class TestNewExceptionHierarchy:
    """Verify inheritance for the 15 newly-added exception classes."""

    def test_bad_request_inherits_error(self):
        assert issubclass(BadRequest, Error)
        assert not issubclass(BadRequest, DatabaseError)

    def test_forbidden_error_inherits_error(self):
        assert issubclass(ForbiddenError, Error)

    def test_method_not_allowed_inherits_error(self):
        assert issubclass(MethodNotAllowed, Error)

    def test_request_timeout_error_inherits_error(self):
        assert issubclass(RequestTimeoutError, Error)

    def test_too_many_requests_inherits_error(self):
        assert issubclass(TooManyRequests, Error)

    def test_internal_server_error_inherits_error(self):
        assert issubclass(InternalServerError, Error)
        assert not issubclass(InternalServerError, DatabaseError)

    def test_bad_gateway_error_inherits_error(self):
        assert issubclass(BadGatewayError, Error)

    def test_service_unavailable_error_inherits_error(self):
        assert issubclass(ServiceUnavailableError, Error)

    def test_gateway_timeout_error_inherits_error(self):
        assert issubclass(GatewayTimeoutError, Error)

    def test_other_http_retryable_error_inherits_error(self):
        assert issubclass(OtherHTTPRetryableError, Error)

    def test_refresh_token_error_inherits_error(self):
        assert issubclass(RefreshTokenError, Error)
        assert not issubclass(RefreshTokenError, DatabaseError)

    def test_token_expired_error_inherits_error(self):
        assert issubclass(TokenExpiredError, Error)
        assert not issubclass(TokenExpiredError, DatabaseError)

    def test_revocation_check_error_inherits_operational_error(self):
        assert issubclass(RevocationCheckError, OperationalError)
        assert issubclass(RevocationCheckError, DatabaseError)

    def test_bind_upload_error_inherits_error(self):
        assert issubclass(BindUploadError, Error)
        assert not issubclass(BindUploadError, DatabaseError)

    def test_request_exceed_max_retry_error_inherits_error(self):
        assert issubclass(RequestExceedMaxRetryError, Error)

    def test_presigned_url_expired_error_inherits_error(self):
        assert issubclass(PresignedUrlExpiredError, Error)

    def test_missing_dependency_error_inherits_error(self):
        assert issubclass(MissingDependencyError, Error)
        assert not issubclass(MissingDependencyError, DatabaseError)


class TestHttpExceptionConstructors:
    """Verify default messages and errno offsets for HTTP exception classes."""

    def test_bad_request_default_message(self):
        err = BadRequest()
        assert "400" in str(err)
        assert err.errno == ER_HTTP_GENERAL_ERROR

    def test_forbidden_error_default_message(self):
        err = ForbiddenError()
        assert "403" in str(err)
        assert err.errno == ER_HTTP_GENERAL_ERROR

    def test_method_not_allowed_default_message(self):
        err = MethodNotAllowed()
        assert "405" in str(err)
        assert err.errno == ER_HTTP_GENERAL_ERROR

    def test_request_timeout_default_message(self):
        err = RequestTimeoutError()
        assert "408" in str(err)
        assert err.errno == ER_HTTP_GENERAL_ERROR

    def test_too_many_requests_default_message(self):
        err = TooManyRequests()
        assert "429" in str(err)
        assert err.errno == ER_HTTP_GENERAL_ERROR

    def test_internal_server_error_default_message(self):
        err = InternalServerError()
        assert "500" in str(err)
        assert err.errno == ER_HTTP_GENERAL_ERROR

    def test_bad_gateway_default_message(self):
        err = BadGatewayError()
        assert "502" in str(err)
        assert err.errno == ER_HTTP_GENERAL_ERROR

    def test_service_unavailable_default_message(self):
        err = ServiceUnavailableError()
        assert "503" in str(err)
        assert err.errno == ER_HTTP_GENERAL_ERROR

    def test_gateway_timeout_default_message(self):
        err = GatewayTimeoutError()
        assert "504" in str(err)
        assert err.errno == ER_HTTP_GENERAL_ERROR

    def test_http_custom_message_preserved(self):
        err = GatewayTimeoutError(msg="custom message")
        assert "custom message" in str(err)
        assert err.errno == ER_HTTP_GENERAL_ERROR

    def test_http_errno_offset(self):
        err = InternalServerError(errno=5)
        assert err.errno == ER_HTTP_GENERAL_ERROR + 5

    def test_other_http_retryable_with_code(self):
        err = OtherHTTPRetryableError(code=520)
        assert "520" in str(err)
        assert err.errno == ER_HTTP_GENERAL_ERROR

    def test_other_http_retryable_default(self):
        err = OtherHTTPRetryableError()
        assert "n/a" in str(err)
        assert err.errno == ER_HTTP_GENERAL_ERROR

    def test_http_exception_catchable_as_error(self):
        for cls in (BadRequest, ForbiddenError, MethodNotAllowed, RequestTimeoutError,
                    TooManyRequests, InternalServerError, BadGatewayError,
                    ServiceUnavailableError, GatewayTimeoutError, OtherHTTPRetryableError):
            err = cls()
            assert isinstance(err, Error), f"{cls.__name__} must be an instance of Error"


class TestRefreshTokenErrorConstructor:

    def test_default_message(self):
        err = RefreshTokenError()
        assert "Token Refresh Required" in str(err)

    def test_custom_message(self):
        err = RefreshTokenError(msg="custom reason")
        assert "custom reason" in str(err)

    def test_errno_passthrough(self):
        err = RefreshTokenError(errno=42)
        assert err.errno == 42


class TestMissingDependencyError:
    def test_message_contains_dependency_name(self):
        err = MissingDependencyError("pandas")
        assert "pandas" in str(err)

    def test_is_error_instance(self):
        assert isinstance(MissingDependencyError("x"), Error)
