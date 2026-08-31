"""
Tests for PEP 249 exception classes.
"""

import warnings

from unittest.mock import MagicMock

import pytest

from snowflake.connector._internal.api_client.client_api import _proto_to_public_error
from snowflake.connector._internal.error_kinds import (
    ERROR_KIND_AUTHENTICATION_ERROR,
    ERROR_KIND_INTERNAL_ERROR,
    ERROR_KIND_INVALID_ARGUMENT,
    ERROR_KIND_INVALID_PARAMETER_VALUE,
    ERROR_KIND_LOGIN_ERROR,
    ERROR_KIND_MISSING_PARAMETER,
    ERROR_KIND_NOT_IMPLEMENTED,
    ERROR_KIND_QUERY_FAILED,
    ERROR_KIND_STAGE_BINDING,
    ERROR_KIND_TIMEOUT,
    KIND_TO_ERRNO,
    KIND_TO_EXCEPTION,
)
from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
    DriverException as ProtoDriverException,
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
    ReauthenticationRequest,
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

    def test_reauthentication_request_inheritance(self):
        assert issubclass(ReauthenticationRequest, ProgrammingError)
        assert issubclass(ReauthenticationRequest, DatabaseError)
        assert issubclass(ReauthenticationRequest, Error)
        # Deliberately NOT OperationalError yet — base changes in a future
        # major release (SNOW-3965765). This assertion is meant to flip when
        # that happens, not silently pass either way.
        assert not issubclass(ReauthenticationRequest, OperationalError)


class TestReauthenticationRequest:
    """Tests for ``ReauthenticationRequest`` — one class, raised uniformly on
    every reauth path, handled the same as every other exception in
    ``errors.py``."""

    def test_is_catchable_as_programming_error(self):
        with pytest.raises(ProgrammingError):
            raise ReauthenticationRequest("master token expired", errno=390114)

    def test_is_catchable_as_database_error(self):
        with pytest.raises(DatabaseError):
            raise ReauthenticationRequest("master token expired", errno=390114)

    def test_is_catchable_as_error(self):
        with pytest.raises(Error):
            raise ReauthenticationRequest("master token expired", errno=390114)

    def test_is_not_catchable_as_operational_error(self):
        # `except OperationalError` must NOT intercept this — if it did, the
        # AssertionError below would replace the expected ReauthenticationRequest.
        with pytest.raises(ReauthenticationRequest):
            try:
                raise ReauthenticationRequest("master token expired", errno=390114)
            except OperationalError as exc:
                raise AssertionError("OperationalError must not catch ReauthenticationRequest yet") from exc

    def test_no_second_name_exists(self):
        # A modern-sounding alias would undercut the FutureWarning's "catch the
        # exact class" advice.
        import snowflake.connector.errors as errors_module

        assert not hasattr(errors_module, "ReauthenticationRequiredError")

    def test_raise_emits_future_warning_naming_escape_hatches(self):
        import snowflake.connector.errors as errors_module

        errors_module._FUTURE_BASE_CHANGE_WARNED = False
        try:
            with warnings.catch_warnings(record=True) as caught:
                warnings.simplefilter("always")
                ReauthenticationRequest("master token expired", errno=390114)
            future_warnings = [w for w in caught if issubclass(w.category, FutureWarning)]
            assert len(future_warnings) == 1
            message = str(future_warnings[0].message)
            assert "ReauthenticationRequest" in message
            assert "DatabaseError" in message
        finally:
            errors_module._FUTURE_BASE_CHANGE_WARNED = False

    def test_future_warning_fires_at_most_once_per_process(self):
        import snowflake.connector.errors as errors_module

        errors_module._FUTURE_BASE_CHANGE_WARNED = False
        try:
            with warnings.catch_warnings(record=True) as caught:
                warnings.simplefilter("always")
                ReauthenticationRequest("first", errno=390114)
                ReauthenticationRequest("second", errno=390195)
            future_warnings = [w for w in caught if issubclass(w.category, FutureWarning)]
            assert len(future_warnings) == 1
        finally:
            errors_module._FUTURE_BASE_CHANGE_WARNED = False

    def test_future_warning_never_masks_the_real_exception_under_filterwarnings_error(self):
        import snowflake.connector.errors as errors_module

        errors_module._FUTURE_BASE_CHANGE_WARNED = False
        try:
            with warnings.catch_warnings():
                warnings.simplefilter("error")
                with pytest.raises(ReauthenticationRequest):
                    raise ReauthenticationRequest("master token expired", errno=390114)
        finally:
            errors_module._FUTURE_BASE_CHANGE_WARNED = False


class TestExceptionInstantiation:
    """Test Error attributes (msg, errno, sqlstate, sfqid, query)."""

    def test_error_default_attributes(self):
        error = Error("something went wrong")
        assert error.raw_msg == "something went wrong"
        assert error.errno == -1
        assert error.sqlstate is None
        assert error.sfqid is None
        assert error.query is None
        assert error.request_id is None

    def test_error_full_attributes(self):
        error = Error(
            "oops",
            errno=42,
            sqlstate="HY000",
            sfqid="abc-123",
            query="SELECT 1",
            request_id="550e8400-e29b-41d4-a716-446655440000",
        )
        assert error.raw_msg == "oops"
        assert error.errno == 42
        assert error.sqlstate == "HY000"
        assert error.sfqid == "abc-123"
        assert error.query == "SELECT 1"
        assert error.request_id == "550e8400-e29b-41d4-a716-446655440000"

    def test_request_id_distinct_from_sfqid(self):
        error = Error(sfqid="01abc-query-id", request_id="550e8400-e29b-41d4-a716-446655440000")
        # sfqid is the server-assigned query id; request_id is the client UUID.
        # They live in different id spaces and must not be conflated.
        assert error.sfqid != error.request_id

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


class TestErrorKindMapping:
    """Test that proto error kinds map to the correct PEP 249 exception class."""

    def test_authentication_error_maps_to_database_error(self):
        assert KIND_TO_EXCEPTION[ERROR_KIND_AUTHENTICATION_ERROR] is DatabaseError

    def test_internal_error_maps_to_internal_error(self):
        assert KIND_TO_EXCEPTION[ERROR_KIND_INTERNAL_ERROR] is InternalError

    def test_login_error_maps_to_database_error(self):
        assert KIND_TO_EXCEPTION[ERROR_KIND_LOGIN_ERROR] is DatabaseError

    def test_not_implemented_maps_to_not_supported(self):
        assert KIND_TO_EXCEPTION[ERROR_KIND_NOT_IMPLEMENTED] is NotSupportedError

    def test_invalid_argument_maps_to_programming(self):
        assert KIND_TO_EXCEPTION[ERROR_KIND_INVALID_ARGUMENT] is ProgrammingError

    def test_missing_parameter_maps_to_programming(self):
        assert KIND_TO_EXCEPTION[ERROR_KIND_MISSING_PARAMETER] is ProgrammingError

    def test_invalid_parameter_value_maps_to_programming(self):
        assert KIND_TO_EXCEPTION[ERROR_KIND_INVALID_PARAMETER_VALUE] is ProgrammingError

    def test_query_failed_maps_to_programming(self):
        assert KIND_TO_EXCEPTION[ERROR_KIND_QUERY_FAILED] is ProgrammingError

    def test_timeout_maps_to_operational(self):
        assert KIND_TO_EXCEPTION[ERROR_KIND_TIMEOUT] is OperationalError

    def test_stage_binding_maps_to_operational(self):
        assert KIND_TO_EXCEPTION[ERROR_KIND_STAGE_BINDING] is OperationalError


class TestConvertProtoError:
    """Test _proto_to_public_error end-to-end conversion."""

    def test_application_exception_with_kind(self):
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        driver_exc = MagicMock()
        driver_exc.message = "Query failed"
        driver_exc.kind = ERROR_KIND_INVALID_ARGUMENT
        driver_exc.report = ""
        driver_exc.HasField.return_value = False
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        assert isinstance(result, ProgrammingError)
        assert "Query failed" in str(result)

    def test_application_exception_authentication(self):
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        driver_exc = ProtoDriverException(
            message="Authentication failed",
            kind=ERROR_KIND_AUTHENTICATION_ERROR,
            root_cause="Token expired",
        )
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        assert isinstance(result, DatabaseError)
        assert "Authentication failed" in str(result)
        assert "Token expired" in str(result)
        assert result.errno == 250001
        assert result.sqlstate == "08001"

    @pytest.mark.parametrize("code", [390113, 390114, 390115, 390195, 390303, 390318])
    def test_application_exception_reauth_shaped_auth_error_constructs_reauthentication_request(self, code):
        # Mid-session (390113/114/115) and login-time (390195/303/318) reauth
        # triggers both route through presence of the same
        # DriverException.reauthentication_required discriminant.
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        driver_exc = ProtoDriverException(
            message="Master token can never be renewed",
            kind=ERROR_KIND_AUTHENTICATION_ERROR,
            vendor_code=code,
            reauthentication_required=True,
        )
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        assert isinstance(result, ReauthenticationRequest)
        assert isinstance(result, ProgrammingError)
        assert isinstance(result, DatabaseError)
        assert isinstance(result, Error)
        assert result.errno == code
        assert result.sqlstate == "08001"

    def test_application_exception_reauth_shaped_auth_error_without_code_still_constructs_reauthentication_request(
        self,
    ):
        # Client-predicted expiry: no server round-trip occurred, so no
        # vendor_code is set even though `reauthentication_required` is
        # `True`. The class decision must come from `reauthentication_required`
        # alone; requiring a code as well would silently fall back to a
        # generic DatabaseError here.
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        driver_exc = ProtoDriverException(
            message="Master token can never be renewed",
            kind=ERROR_KIND_AUTHENTICATION_ERROR,
            reauthentication_required=True,
        )
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        assert isinstance(result, ReauthenticationRequest)
        assert result.errno == KIND_TO_ERRNO[ERROR_KIND_AUTHENTICATION_ERROR]
        assert result.sqlstate == "08001"

    def test_application_exception_reauth_message_carries_gs_code_exactly_once(self):
        # Regression test: the assembled user-facing message must contain the
        # GS code exactly once, not 3x (message + root_cause + detail all
        # carrying the same "(GS code N)" text via _append_detail's dedup).
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        detail = "Master token can never be renewed - full re-authentication required (GS code 390114)."
        driver_exc = ProtoDriverException(
            message=detail,
            kind=ERROR_KIND_AUTHENTICATION_ERROR,
            vendor_code=390114,
            reauthentication_required=True,
        )
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        # raw_msg is the assembled message body, before Error.__str__ prepends
        # "errno (sqlstate): " — that prefix legitimately repeats the same
        # numeral as the errno, which is not the bug under test here.
        assert result.raw_msg.count("390114") == 1

    def test_application_exception_auth_error_without_reauthentication_required_stays_generic(self):
        # AuthenticationError without `reauthentication_required` set (unset
        # per proto3 message-field default) must NOT be misread as
        # reauth-required — this is the ordinary TlsClientCreation/
        # SessionRefresh/etc. AuthenticationError shape.
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        driver_exc = ProtoDriverException(
            message="TLS handshake failed",
            kind=ERROR_KIND_AUTHENTICATION_ERROR,
        )
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        assert not isinstance(result, ReauthenticationRequest)
        assert isinstance(result, DatabaseError)

    def test_application_exception_login_error_without_reauth_stays_plain_login_error(self):
        # Negative control: an ordinary login failure (bad password) must not
        # be misclassified as ReauthenticationRequest.
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        driver_exc = ProtoDriverException(
            message="Incorrect username or password",
            kind=ERROR_KIND_LOGIN_ERROR,
            vendor_code=390100,
        )
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        assert not isinstance(result, ReauthenticationRequest)
        assert isinstance(result, DatabaseError)

    def test_application_exception_not_implemented(self):
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        driver_exc = MagicMock()
        driver_exc.message = "Feature X not implemented"
        driver_exc.kind = ERROR_KIND_NOT_IMPLEMENTED
        driver_exc.report = ""
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
        driver_exc.kind = ERROR_KIND_INTERNAL_ERROR
        driver_exc.report = ""
        driver_exc.HasField.return_value = False
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        assert isinstance(result, InternalError)
        # INTERNAL_ERROR has no old-driver errno mapping, so proto ErrorKind is used as fallback.
        assert result.errno == ERROR_KIND_INTERNAL_ERROR

    def test_application_exception_login_error_uses_vendor_code_when_present(self):
        """sf_core surfaces the server's raw GS code for login failures (e.g. 390100
        for bad credentials) via vendor_code/sql_state, which takes priority over the
        KIND_TO_ERRNO fallback — matching legacy Python connector >=4.7.2 (SNOW-3775156)."""
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        driver_exc = ProtoDriverException(
            message="Failed to connect to DB",
            kind=ERROR_KIND_LOGIN_ERROR,
            vendor_code=390100,
            sql_state="28000",
        )
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        assert isinstance(result, DatabaseError)
        assert result.errno == 390100
        assert result.sqlstate == "28000"

    def test_application_exception_uses_vendor_code_and_sqlstate(self):
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        driver_exc = ProtoDriverException(
            message="SQL compilation error: syntax error",
            kind=ERROR_KIND_QUERY_FAILED,
            vendor_code=1003,
            sql_state="42000",
        )
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        assert isinstance(result, ProgrammingError)
        # vendor_code from proto takes priority over status-code-based mapping
        assert result.errno == 1003
        assert result.sqlstate == "42000"

    def test_application_exception_populates_query_id_and_request_id(self):
        """query_id maps to sfqid and request_id is surfaced on the exception."""
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        driver_exc = ProtoDriverException(
            message="SQL compilation error: syntax error",
            kind=ERROR_KIND_QUERY_FAILED,
            vendor_code=1003,
            sql_state="42000",
            query_id="01abc-def-12345",
            request_id="550e8400-e29b-41d4-a716-446655440000",
        )
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        assert result.sfqid == "01abc-def-12345"
        assert result.request_id == "550e8400-e29b-41d4-a716-446655440000"

    def test_application_exception_omits_ids_when_absent(self):
        """When the proto carries neither id, both stay None on the exception."""
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        driver_exc = ProtoDriverException(
            message="SQL compilation error",
            kind=ERROR_KIND_QUERY_FAILED,
            vendor_code=1003,
            sql_state="42000",
        )
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        assert result.sfqid is None
        assert result.request_id is None

    def test_application_exception_message_has_no_wrapper_prefixes(self):
        """Regression test: match old snowflake-connector-python error format.

        Old driver produces '002003 (42S02): SQL compilation error: ...'
        — no 'Query execution failed:' or 'Query failed:' wrapper.
        """
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        driver_exc = ProtoDriverException(
            message="SQL compilation error: Object 'FOO' does not exist.",
            kind=ERROR_KIND_QUERY_FAILED,
            vendor_code=2003,
            sql_state="42S02",
        )
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        msg = str(result)
        assert "Query execution failed" not in msg
        assert "Query failed:" not in msg
        assert msg == "002003 (42S02): SQL compilation error: Object 'FOO' does not exist."

    def test_application_exception_root_cause_appended(self):
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        driver_exc = ProtoDriverException(
            message="Query failed",
            kind=ERROR_KIND_QUERY_FAILED,
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
            kind=ERROR_KIND_QUERY_FAILED,
            root_cause="division by zero",
        )
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        # root_cause should not be appended when it already appears in message
        msg_str = str(result)
        assert msg_str.count("division by zero") == 1

    def test_application_exception_vendor_code_100072_maps_to_integrity_error(self):
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        driver_exc = ProtoDriverException(
            message="NULL result in a non-nullable column",
            kind=ERROR_KIND_QUERY_FAILED,
            vendor_code=100072,
            sql_state="23000",
        )
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        assert isinstance(result, IntegrityError)
        assert result.errno == 100072
        assert result.sqlstate == "23000"

    def test_application_exception_timeout_maps_to_operational_with_hyt00(self):
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        driver_exc = ProtoDriverException(
            message="Query timed out after 30s",
            kind=ERROR_KIND_TIMEOUT,
            sql_state="HYT00",
        )
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        assert isinstance(result, OperationalError)
        assert result.sqlstate == "HYT00"

    def test_application_exception_timeout_derives_hyt00_when_sql_state_unset(self):
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        driver_exc = ProtoDriverException(
            message="Query timed out after 30s",
            kind=ERROR_KIND_TIMEOUT,
        )
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        assert isinstance(result, OperationalError)
        assert result.sqlstate == "HYT00"

    def test_application_exception_query_failed_maps_to_programming(self):
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        driver_exc = ProtoDriverException(
            message="SQL compilation error: syntax error line 1",
            kind=ERROR_KIND_QUERY_FAILED,
            vendor_code=1003,
            sql_state="42000",
        )
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        assert isinstance(result, ProgrammingError)
        assert result.errno == 1003
        assert result.sqlstate == "42000"

    def test_application_exception_report_not_included(self):
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        driver_exc = MagicMock()
        driver_exc.message = "Query failed"
        driver_exc.kind = ERROR_KIND_INVALID_ARGUMENT
        driver_exc.report = "Diagnostic report:\n  line 1: unexpected token"
        driver_exc.HasField.return_value = False
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        assert "Query failed" in str(result)
        assert "Diagnostic report" not in str(result)

    def test_application_exception_invalid_parameter_value_carries_validation_code(self):
        # End-to-end: a validate_settings-originated InvalidParameterValue error
        # (e.g. the WIF cross-param guards) surfaces `parameter` and
        # `validation_code` as structured exception attributes, so callers can
        # discriminate without matching message text.
        from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (
            VALIDATION_CODE_CONFLICTING_WIF_PARAMETERS,
        )
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        driver_exc = ProtoDriverException(
            message="workload_identity_provider was set but authenticator was not set to WORKLOAD_IDENTITY",
            kind=ERROR_KIND_INVALID_PARAMETER_VALUE,
            parameter="workload_identity_provider",
            validation_code=VALIDATION_CODE_CONFLICTING_WIF_PARAMETERS,
        )
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        assert isinstance(result, ProgrammingError)
        assert result.parameter == "workload_identity_provider"
        assert result.validation_code == VALIDATION_CODE_CONFLICTING_WIF_PARAMETERS

    def test_application_exception_invalid_parameter_value_without_code_leaves_validation_code_none(self):
        # An InvalidParameterValue error that did not originate from
        # validate_settings (e.g. an unknown authenticator) carries no
        # ValidationCode on the wire, so `validation_code` stays None.
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        driver_exc = ProtoDriverException(
            message="Invalid authenticator",
            kind=ERROR_KIND_INVALID_PARAMETER_VALUE,
            parameter="authenticator",
            parameter_value="BAD",
        )
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        assert result.parameter == "authenticator"
        assert result.validation_code is None

    def test_application_exception_missing_parameter_surfaces_parameter(self):
        from snowflake.connector._internal.protobuf_gen.proto_exception import (
            ProtoApplicationException,
        )

        driver_exc = ProtoDriverException(
            message="Missing required parameter: account",
            kind=ERROR_KIND_MISSING_PARAMETER,
            parameter="account",
        )
        proto_exc = ProtoApplicationException(driver_exc)

        result = _proto_to_public_error(proto_exc)
        assert result.parameter == "account"
        assert result.validation_code is None


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

    def test_error_defaults_parameter_and_validation_code_to_none(self):
        err = Error("fail", errno=1003)
        assert err.parameter is None
        assert err.validation_code is None

    def test_error_stores_parameter_and_validation_code(self):
        err = Error("fail", errno=1003, parameter="workload_identity_provider", validation_code=6)
        assert err.parameter == "workload_identity_provider"
        assert err.validation_code == 6

    def test_error_kwargs_other_than_parameter_and_code_are_still_silently_absorbed(self):
        # Backward compatibility: arbitrary unrecognized kwargs must still be
        # swallowed without error (existing callers may pass old-driver-only keys).
        err = Error("fail", errno=1003, some_unrelated_kwarg="value")
        assert not hasattr(err, "some_unrelated_kwarg")
