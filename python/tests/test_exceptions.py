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
    HttpError,
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

    def test_revocation_check_error_inheritance(self):
        assert issubclass(RevocationCheckError, OperationalError)

    def test_missing_dependency_error_inheritance(self):
        assert issubclass(MissingDependencyError, Error)

    def test_config_source_error_inheritance(self):
        # ConfigSourceError should inherit from Error directly, not ConfigManagerError
        assert issubclass(ConfigSourceError, Error)
        assert not issubclass(ConfigSourceError, ConfigManagerError)

    def test_missing_config_option_error_inheritance(self):
        assert issubclass(MissingConfigOptionError, ConfigSourceError)

    def test_config_manager_error_inheritance(self):
        assert issubclass(ConfigManagerError, Error)

    def test_http_error_inheritance(self):
        assert issubclass(HttpError, Error)

    def test_internal_server_error_inheritance(self):
        assert issubclass(InternalServerError, Error)

    def test_service_unavailable_error_inheritance(self):
        assert issubclass(ServiceUnavailableError, Error)

    def test_gateway_timeout_error_inheritance(self):
        assert issubclass(GatewayTimeoutError, Error)

    def test_forbidden_error_inheritance(self):
        assert issubclass(ForbiddenError, Error)

    def test_request_timeout_error_inheritance(self):
        assert issubclass(RequestTimeoutError, Error)

    def test_bad_request_inheritance(self):
        assert issubclass(BadRequest, Error)

    def test_bad_gateway_error_inheritance(self):
        assert issubclass(BadGatewayError, Error)

    def test_method_not_allowed_inheritance(self):
        assert issubclass(MethodNotAllowed, Error)

    def test_too_many_requests_inheritance(self):
        assert issubclass(TooManyRequests, Error)

    def test_refresh_token_error_inheritance(self):
        assert issubclass(RefreshTokenError, Error)

    def test_other_http_retryable_error_inheritance(self):
        assert issubclass(OtherHTTPRetryableError, Error)

    def test_bind_upload_error_inheritance(self):
        assert issubclass(BindUploadError, Error)

    def test_request_exceed_max_retry_error_inheritance(self):
        assert issubclass(RequestExceedMaxRetryError, Error)

    def test_token_expired_error_inheritance(self):
        assert issubclass(TokenExpiredError, Error)

    def test_presigned_url_expired_error_inheritance(self):
        assert issubclass(PresignedUrlExpiredError, Error)


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

    def test_missing_dependency_error_instantiation(self):
        err = MissingDependencyError("pandas")
        assert isinstance(err, Error)
        assert "pandas" in str(err)
        assert "Missing optional dependency" in str(err)

    def test_config_source_error_instantiation(self):
        err = ConfigSourceError("invalid config value")
        assert isinstance(err, Error)
        assert "invalid config value" in str(err)
        assert err.errno == -1

    def test_missing_config_option_error_instantiation(self):
        err = MissingConfigOptionError("missing required option")
        assert isinstance(err, ConfigSourceError)
        assert "missing required option" in str(err)

    def test_config_manager_error_instantiation(self):
        err = ConfigManagerError("manager error")
        assert isinstance(err, Error)
        assert "manager error" in str(err)

    def test_revocation_check_error_instantiation(self):
        err = RevocationCheckError("certificate revocation check failed")
        assert isinstance(err, OperationalError)
        assert "certificate revocation check failed" in str(err)

    def test_http_error_instantiation(self):
        err = HttpError("HTTP error occurred")
        assert isinstance(err, Error)
        assert "HTTP error occurred" in str(err)

    def test_internal_server_error_instantiation(self):
        err = InternalServerError("500 server error")
        assert isinstance(err, Error)
        assert "500 server error" in str(err)

    def test_service_unavailable_error_instantiation(self):
        err = ServiceUnavailableError("503 service unavailable")
        assert isinstance(err, Error)
        assert "503 service unavailable" in str(err)

    def test_gateway_timeout_error_instantiation(self):
        err = GatewayTimeoutError("504 gateway timeout")
        assert isinstance(err, Error)
        assert "504 gateway timeout" in str(err)

    def test_forbidden_error_instantiation(self):
        err = ForbiddenError("403 forbidden")
        assert isinstance(err, Error)
        assert "403 forbidden" in str(err)

    def test_request_timeout_error_instantiation(self):
        err = RequestTimeoutError("408 request timeout")
        assert isinstance(err, Error)
        assert "408 request timeout" in str(err)

    def test_bad_request_instantiation(self):
        err = BadRequest("400 bad request")
        assert isinstance(err, Error)
        assert "400 bad request" in str(err)

    def test_bad_gateway_error_instantiation(self):
        err = BadGatewayError("502 bad gateway")
        assert isinstance(err, Error)
        assert "502 bad gateway" in str(err)

    def test_method_not_allowed_instantiation(self):
        err = MethodNotAllowed("405 method not allowed")
        assert isinstance(err, Error)
        assert "405 method not allowed" in str(err)

    def test_too_many_requests_instantiation(self):
        err = TooManyRequests("429 too many requests")
        assert isinstance(err, Error)
        assert "429 too many requests" in str(err)

    def test_refresh_token_error_instantiation(self):
        err = RefreshTokenError("token refresh required")
        assert isinstance(err, Error)
        assert "token refresh required" in str(err)

    def test_other_http_retryable_error_instantiation(self):
        err = OtherHTTPRetryableError("retryable HTTP error")
        assert isinstance(err, Error)
        assert "retryable HTTP error" in str(err)

    def test_bind_upload_error_instantiation(self):
        err = BindUploadError("bind upload failed")
        assert isinstance(err, Error)
        assert "bind upload failed" in str(err)

    def test_request_exceed_max_retry_error_instantiation(self):
        err = RequestExceedMaxRetryError("max retries exceeded")
        assert isinstance(err, Error)
        assert "max retries exceeded" in str(err)

    def test_token_expired_error_instantiation(self):
        err = TokenExpiredError("authentication token expired")
        assert isinstance(err, Error)
        assert "authentication token expired" in str(err)

    def test_presigned_url_expired_error_instantiation(self):
        err = PresignedUrlExpiredError("presigned URL expired")
        assert isinstance(err, Error)
        assert "presigned URL expired" in str(err)


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


class TestBackCompatExceptionImports:
    """Test that all exception classes can be imported for backward compatibility."""

    def test_all_pep249_exceptions_importable(self):
        """Verify PEP 249 exception classes can be imported."""
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

        assert issubclass(Error, Exception)
        assert issubclass(Warning, Warning)
        assert issubclass(DatabaseError, Error)
        assert issubclass(InterfaceError, Error)
        assert issubclass(DataError, DatabaseError)
        assert issubclass(OperationalError, DatabaseError)
        assert issubclass(IntegrityError, DatabaseError)
        assert issubclass(InternalError, DatabaseError)
        assert issubclass(ProgrammingError, DatabaseError)
        assert issubclass(NotSupportedError, DatabaseError)

    def test_all_config_exceptions_importable(self):
        """Verify config-related exception classes can be imported."""
        from snowflake.connector.errors import (
            ConfigManagerError,
            ConfigSourceError,
            MissingConfigOptionError,
        )

        assert issubclass(ConfigSourceError, Error)
        assert issubclass(MissingConfigOptionError, ConfigSourceError)
        assert issubclass(ConfigManagerError, Error)

    def test_all_http_exceptions_importable(self):
        """Verify HTTP/retry exception classes can be imported."""
        from snowflake.connector.errors import (
            BadGatewayError,
            BadRequest,
            ForbiddenError,
            GatewayTimeoutError,
            HttpError,
            InternalServerError,
            MethodNotAllowed,
            OtherHTTPRetryableError,
            RefreshTokenError,
            RequestTimeoutError,
            ServiceUnavailableError,
            TooManyRequests,
        )

        # All should inherit from Error
        assert issubclass(HttpError, Error)
        assert issubclass(InternalServerError, Error)
        assert issubclass(ServiceUnavailableError, Error)
        assert issubclass(GatewayTimeoutError, Error)
        assert issubclass(ForbiddenError, Error)
        assert issubclass(RequestTimeoutError, Error)
        assert issubclass(BadRequest, Error)
        assert issubclass(BadGatewayError, Error)
        assert issubclass(MethodNotAllowed, Error)
        assert issubclass(TooManyRequests, Error)
        assert issubclass(RefreshTokenError, Error)
        assert issubclass(OtherHTTPRetryableError, Error)

    def test_all_storage_exceptions_importable(self):
        """Verify storage/binding exception classes can be imported."""
        from snowflake.connector.errors import (
            BindUploadError,
            PresignedUrlExpiredError,
            RequestExceedMaxRetryError,
            TokenExpiredError,
        )

        assert issubclass(BindUploadError, Error)
        assert issubclass(RequestExceedMaxRetryError, Error)
        assert issubclass(TokenExpiredError, Error)
        assert issubclass(PresignedUrlExpiredError, Error)

    def test_revocation_check_error_importable(self):
        """Verify RevocationCheckError can be imported."""
        from snowflake.connector.errors import RevocationCheckError

        assert issubclass(RevocationCheckError, OperationalError)

    def test_missing_dependency_error_importable(self):
        """Verify MissingDependencyError can be imported."""
        from snowflake.connector.errors import MissingDependencyError

        assert issubclass(MissingDependencyError, Error)
