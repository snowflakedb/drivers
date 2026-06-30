import pytest

from snowflake.connector.errors import DatabaseError, Error, ProgrammingError
from tests.compatibility import IS_UNIVERSAL_DRIVER

from .auth_helpers import verify_simple_query_execution


def _extract_session_tokens(connection) -> dict:
    """Return session_token and master_token from an open connection.

    Must be called while the connection is still open — both values are
    cleared when the session is closed.
    """
    session_token = connection.rest.token
    master_token = connection.rest.master_token
    assert session_token, "session_token must not be empty on a live connection"
    assert master_token, "master_token must not be empty on a live connection"
    return {"session_token": session_token, "master_token": master_token}


class TestSessionTokenAuthentication:
    def test_should_authenticate_using_session_tokens_from_active_connection(self, connection_factory):
        # Given an active connection whose tokens are still valid
        with connection_factory() as base_conn:
            tokens = _extract_session_tokens(base_conn)

            # When opening a second connection using those tokens
            with connection_factory(**tokens) as session_conn:
                # Then queries succeed on the session-token connection
                verify_simple_query_execution(session_conn)

    def test_session_token_connection_receives_real_session_id_from_server(self, connection_factory):
        # Given an active connection with a known session ID
        with connection_factory() as base_conn:
            with base_conn.cursor() as cur:
                cur.execute("SELECT CURRENT_SESSION()")
                original_session_id = cur.fetchone()[0]

            tokens = _extract_session_tokens(base_conn)

            # When opening a second connection using those tokens
            with connection_factory(**tokens) as session_conn:
                with session_conn.cursor() as cur:
                    cur.execute("SELECT CURRENT_SESSION()")
                    session_token_session_id = cur.fetchone()[0]

                # Then the session ID matches the originating session (real server value, not a local hash)
                assert session_token_session_id == original_session_id, (
                    f"Expected session_id {original_session_id!r} but got {session_token_session_id!r}"
                )

    def test_session_token_connection_can_execute_multiple_queries(self, connection_factory):
        # Given a session-token connection opened from a live base connection
        with connection_factory() as base_conn:
            tokens = _extract_session_tokens(base_conn)

            # When executing multiple queries on the session-token connection
            with connection_factory(**tokens) as session_conn:
                with session_conn.cursor() as cur:
                    cur.execute("SELECT 1")
                    # Then each query returns the expected result
                    assert cur.fetchone()[0] == 1

                    cur.execute("SELECT 'hello'")
                    assert cur.fetchone()[0] == "hello"

                    cur.execute("SELECT CURRENT_USER()")
                    user = cur.fetchone()[0]
                    assert user, "CURRENT_USER() must return a non-empty string"

    def test_should_fail_authentication_with_invalid_session_token(self, connection_factory):
        # BD#35: error type and timing differ between drivers.
        # UD validates tokens eagerly via RENEW → DatabaseError raised from connect().
        # Old driver accepts the tokens and discovers they are invalid only at the
        # post-login heartbeat → HttpError (a subclass of Error, not DatabaseError).
        if IS_UNIVERSAL_DRIVER:
            expected_error = DatabaseError
        else:
            expected_error = Error

        # When connecting with obviously invalid tokens
        # Then the connection raises an appropriate error
        with pytest.raises(expected_error):
            connection_factory(
                session_token="invalid_session_token_xyz",
                master_token="invalid_master_token_xyz",
            )

    def test_should_require_master_token_alongside_session_token(self, connection_factory):
        # BD#35: UD requires master_token at config time because the RENEW call uses
        # it as the Authorization bearer; omitting it raises ProgrammingError before
        # any network call. Old driver does not enforce this requirement.
        # Given a session token extracted from an active connection
        with connection_factory() as base_conn:
            session_token = base_conn.rest.token

        # When connecting with session_token but no master_token
        if IS_UNIVERSAL_DRIVER:
            # Then UD raises ProgrammingError at configuration time (before any network call)
            with pytest.raises(ProgrammingError):
                connection_factory(session_token=session_token)
        else:
            pytest.skip("old driver does not require master_token alongside session_token (BD#35)")
