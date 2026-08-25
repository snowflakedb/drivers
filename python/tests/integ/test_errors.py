"""
Integration tests for the Snowflake error contract.

Authentication login-failure errno/sqlstate coverage uses Wiremock so the
server GS code under test is deterministic (mirroring snowflake-connector-python
SNOW-3775156). Live Snowflake auth-failure smoke belongs in e2e.

Query/object error classes below still hit a real Snowflake account.

These tests are designed to pass against both the new (universal) driver and
the old (reference) snowflake-connector-python driver.
"""

import uuid

import pytest

from snowflake.connector.errors import DatabaseError, Error, ProgrammingError
from tests.compatibility import is_new_driver, is_old_driver


# SQLSTATE values asserted as literals so both UD (_internal.sqlstate) and
# reference (sqlstate) agree without dual import paths.
SQLSTATE_CONNECTION_WAS_NOT_ESTABLISHED = "08001"
SQLSTATE_AUTHORIZATION_FAILURE = "28000"


def _surfaces_server_login_error_codes() -> bool:
    """True when the installed driver surfaces GS login codes (SNOW-3775156 / SNOW-3888007).

    PyPI reference 4.7.1 still hardcodes errno 250001; post-fix reference
    exposes ``CREDENTIAL_REJECTION_GS_CODES`` on ``network``.
    """
    if is_new_driver():
        return True
    try:
        from snowflake.connector.network import CREDENTIAL_REJECTION_GS_CODES  # noqa: F401

        return True
    except ImportError:
        return False


def _require_server_login_error_codes() -> None:
    if not _surfaces_server_login_error_codes():
        pytest.skip(
            "Reference driver predates SNOW-3775156 (no CREDENTIAL_REJECTION_GS_CODES); "
            "still hardcodes errno 250001 for login failures"
        )


class TestAuthenticationErrors:
    """Login-failure errno/sqlstate contract via Wiremock (deterministic GS codes).

    Credential-rejection codes (390100, 390144, …) must surface as the server's
    own errno with SQLSTATE 28000. Other login codes still surface the server
    errno but keep SQLSTATE 08001. See SNOW-3775156 / SNOW-3888007.
    """

    def test_should_surface_credential_rejection_errno_and_sqlstate_28000(self, int_test_connection_factory, wiremock):
        """GS 390100 (AUTHORIZATION_FAILURE) → errno 390100, sqlstate 28000."""
        _require_server_login_error_codes()
        wiremock.add_mapping("auth/login_failure_credential_rejection.json")

        with pytest.raises(DatabaseError) as excinfo:
            # Unique LOGIN_NAME keeps this stub from matching other suites' logins.
            int_test_connection_factory(
                server_url=wiremock.http_url(),
                user="wiremock_login_failure_390100",
            )
        error = excinfo.value

        assert error.errno == 390100
        assert error.sqlstate == SQLSTATE_AUTHORIZATION_FAILURE
        assert "incorrect username or password" in error.msg.lower()

    def test_should_surface_jwt_invalid_errno_and_sqlstate_28000(self, int_test_connection_factory, wiremock):
        """GS 390144 (JWT_TOKEN_INVALID) → errno 390144, sqlstate 28000."""
        _require_server_login_error_codes()
        wiremock.add_mapping("auth/login_failure_jwt_token_invalid.json")

        with pytest.raises(DatabaseError) as excinfo:
            int_test_connection_factory(
                server_url=wiremock.http_url(),
                user="wiremock_login_failure_390144",
            )
        error = excinfo.value

        assert error.errno == 390144
        assert error.sqlstate == SQLSTATE_AUTHORIZATION_FAILURE
        assert "jwt" in error.msg.lower()

    def test_should_surface_generic_login_errno_with_sqlstate_08001(self, int_test_connection_factory, wiremock):
        """Non-credential-rejection GS 390401 → errno 390401, sqlstate 08001."""
        _require_server_login_error_codes()
        wiremock.add_mapping("auth/login_failure_generic.json")

        with pytest.raises(DatabaseError) as excinfo:
            int_test_connection_factory(
                server_url=wiremock.http_url(),
                user="wiremock_login_failure_390401",
            )
        error = excinfo.value

        assert error.errno == 390401
        assert error.sqlstate == SQLSTATE_CONNECTION_WAS_NOT_ESTABLISHED

    def test_should_reject_unsupported_authenticator_value(self, connection_factory):
        """Unsupported authenticator fails client-side before any server round-trip."""
        with pytest.raises(ProgrammingError) as excinfo:
            connection_factory(authenticator="INVALID_AUTH_METHOD", password="dummy")
        error = excinfo.value
        assert "authenticator" in error.msg.lower()
        assert error.errno == 251007


class TestQuerySyntaxErrors:
    """Test that SQL syntax errors raise DatabaseError with descriptive messages."""

    def test_invalid_sql_syntax(self, cursor):
        """Test that malformed SQL raises DatabaseError mentioning the syntax error."""
        with pytest.raises(DatabaseError) as excinfo:
            cursor.execute("SELEC 1")
        error = excinfo.value
        assert error.errno == 1003
        assert "sql compilation error" in error.msg.lower()
        assert "syntax error" in error.msg.lower()

    def test_unclosed_string_literal(self, cursor):
        """Test that an unclosed string literal raises DatabaseError with parse error."""
        with pytest.raises(DatabaseError) as excinfo:
            cursor.execute("SELECT 'unclosed")
        error = excinfo.value
        assert error.errno == 1003
        assert "sql compilation error" in error.msg.lower()
        assert "parse error" in error.msg.lower()

    def test_invalid_identifier(self, cursor):
        """Test that referencing a non-existent column raises DatabaseError."""
        with pytest.raises(DatabaseError) as excinfo:
            cursor.execute("SELECT nonexistent_column")
        error = excinfo.value
        assert error.errno == 904
        assert "sql compilation error" in error.msg.lower()
        assert "invalid identifier" in error.msg.lower()


class TestObjectNotFoundErrors:
    """Test that references to non-existent objects raise DatabaseError."""

    def test_select_from_nonexistent_table(self, cursor):
        """Test that selecting from a non-existent table raises DatabaseError."""
        table_name = f"nonexistent_table_{uuid.uuid4().hex[:8]}"
        with pytest.raises(DatabaseError) as excinfo:
            cursor.execute(f"SELECT * FROM {table_name}")
        error = excinfo.value
        assert error.errno == 2003
        assert "does not exist or not authorized" in error.msg.lower()

    def test_drop_nonexistent_table(self, cursor):
        """Test that dropping a non-existent table (without IF EXISTS) raises DatabaseError."""
        table_name = f"nonexistent_table_{uuid.uuid4().hex[:8]}"
        with pytest.raises(DatabaseError) as excinfo:
            cursor.execute(f"DROP TABLE {table_name}")
        error = excinfo.value
        assert error.errno == 2003
        assert "does not exist or not authorized" in error.msg.lower()

    def test_use_nonexistent_database(self, cursor):
        """Test that USE on a non-existent database raises DatabaseError."""
        db_name = f"nonexistent_db_{uuid.uuid4().hex[:8]}"
        with pytest.raises(DatabaseError) as excinfo:
            cursor.execute(f"USE DATABASE {db_name}")
        error = excinfo.value
        assert error.errno == 2043


class TestErrorAttributes:
    """Test that errors raised from real queries carry expected PEP 249 attributes."""

    def test_error_inherits_from_database_error(self, cursor):
        """Test that a query error is catchable as DatabaseError and Error."""
        with pytest.raises(DatabaseError):
            cursor.execute("SELEC 1")

    def test_error_has_errno(self, cursor):
        """Test that errors from the server carry the Snowflake server error code."""
        with pytest.raises(Error) as excinfo:
            cursor.execute("SELEC 1")
        error = excinfo.value
        assert error.errno == 1003

    def test_error_has_raw_msg(self, cursor):
        """Test that errors from the server carry raw_msg."""
        with pytest.raises(Error) as excinfo:
            cursor.execute("SELEC 1")
        assert excinfo.value.raw_msg is not None
        assert "sql compilation error" in excinfo.value.raw_msg.lower()

    def test_error_has_sqlstate(self, cursor):
        """Test that errors from the server carry sqlstate."""
        with pytest.raises(Error) as excinfo:
            cursor.execute("SELEC 1")
        assert excinfo.value.sqlstate == "42000"

    def test_error_does_not_leak_internal_cause(self, cursor):
        """Test that server errors do not expose internal proto exceptions via __cause__."""
        if is_old_driver():
            pytest.skip("__cause__ suppression is a new-driver concern")
        with pytest.raises(Error) as excinfo:
            cursor.execute("SELEC 1")
        assert excinfo.value.__cause__ is None

    @pytest.mark.skipif(is_old_driver(), reason="request_id on errors is a new-driver feature")
    def test_error_exposes_request_id(self, cursor):
        """Test that a failed query surfaces the client-generated request_id as a UUID."""
        with pytest.raises(Error) as excinfo:
            cursor.execute("SELEC 1")
        error = excinfo.value
        assert error.request_id, "expected a request_id on the query error"
        # request_id is a client-generated UUID v4.
        uuid.UUID(error.request_id)
        # It also lands on the cursor and is distinct from the server query id.
        assert cursor.request_id == error.request_id
        assert error.request_id != error.sfqid

    @pytest.mark.skipif(is_old_driver(), reason="new-driver sfqid-on-error contract")
    def test_error_exposes_sfqid(self, cursor):
        """Test that a failed query against a real table surfaces the server query id (sfqid)."""
        table_name = f"nonexistent_table_{uuid.uuid4().hex[:8]}"
        with pytest.raises(Error) as excinfo:
            cursor.execute("SELECT * FROM IDENTIFIER(?)", params=(table_name,), _force_qmark_paramstyle=True)
        error = excinfo.value
        # The server assigns a query id even for failed compilation.
        assert error.sfqid, "expected an sfqid on the query error"
        assert cursor.sfqid == error.sfqid


class TestErrorMessageFormat:
    """Reference tests asserting the exact on-the-wire error message format.

    The formatted message surfaced to users must match the legacy
    snowflake-connector-python driver exactly:
        ``{errno:06d} ({sqlstate}): {server_message}``
    with no wrapper prefixes like ``"Query execution failed:"`` or ``"Query failed:"``.
    """

    def test_query_error_message_has_no_wrapper_prefixes(self, cursor):
        with pytest.raises(DatabaseError) as excinfo:
            cursor.execute("SELEC 1")
        msg = str(excinfo.value)
        assert "Query execution failed" not in msg
        assert "Query failed:" not in msg

    @pytest.mark.skipif(is_old_driver(), reason="request_id/sfqid in error message is a new-driver feature")
    def test_error_message_contains_request_id_and_sfqid(self, cursor):
        """Test that the formatted error message includes both request_id and sfqid for support."""
        table_name = f"nonexistent_table_{uuid.uuid4().hex[:8]}"
        with pytest.raises(Error) as excinfo:
            cursor.execute("SELECT * FROM IDENTIFIER(?)", params=(table_name,), _force_qmark_paramstyle=True)
        error = excinfo.value
        msg = str(error)
        assert error.request_id, "expected a request_id on the query error"
        assert error.sfqid, "expected an sfqid on the query error"
        assert error.request_id in msg, f"expected request_id in error message, got: {msg!r}"
        assert error.sfqid in msg, f"expected sfqid in error message, got: {msg!r}"

    def test_query_error_message_format_matches_old_driver(self, cursor):
        """End-to-end: the formatted message has the exact shape the old driver produces."""
        table_name = f"nonexistent_table_{uuid.uuid4().hex[:8]}"
        with pytest.raises(DatabaseError) as excinfo:
            cursor.execute(f"SELECT * FROM {table_name}")
        error = excinfo.value
        msg = str(error)
        # Must start with zero-padded errno and sqlstate, e.g. "002003 (42S02): "
        prefix = f"{error.errno:06d} ({error.sqlstate}): "
        assert msg.startswith(prefix), f"Expected '<errno> (<sqlstate>): ...' prefix, got: {msg!r}"
        body = msg[len(prefix) :]
        # At INFO/DEBUG log level the old driver injects "{sfqid}: " before the
        # server message. Strip it if present so we can assert on the server text.
        if error.sfqid and body.startswith(f"{error.sfqid}: "):
            body = body[len(f"{error.sfqid}: ") :]
        # The body must start with the server's error class, not a wrapper.
        assert body.lower().startswith("sql compilation error"), (
            f"Expected body to start with 'SQL compilation error', got: {body!r}"
        )
