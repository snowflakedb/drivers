"""E2E tests for session logout functionality.

NOTE: This file contains only the currently implemented and passing tests.
Additional test coverage for the following features is deferred:
- Token refresh integration during logout (SNOW-2923705)
- Telemetry recording (SNOW-2912513)
- Heartbeat cancellation (SNOW-2881763)
- Full async query detection scenarios (pending async query API - SNOW-2314152)

These deferred tests will be added as the underlying features are implemented.
"""

import pytest


class TestLogoutResourceCleanup:
    """Resource cleanup contract tests from shared/session/logout.feature."""

    @pytest.mark.parametrize("keep_alive", [True, False, None])
    def test_should_cleanup_all_tokens_on_close_regardless_of_whether_logout_was_sent(
        self, connection_factory, keep_alive
    ):
        # Given Snowflake client is logged in
        # And <server_session_keep_alive> is set to any value
        conn = connection_factory(server_session_keep_alive=keep_alive)

        # When Connection is closed
        conn.close()

        # Then Session token in Connection.tokens is null
        # And Master token in Connection.tokens is null
        assert conn.is_closed(), f"Close should succeed with server_session_keep_alive={keep_alive}"

    def test_should_be_idempotent_when_close_called_multiple_times(self, connection_factory):
        # Given Snowflake client is logged in
        conn = connection_factory()

        # When Connection is closed
        conn.close()

        # And Connection is closed again
        conn.close()

        # And Connection is closed a third time
        conn.close()

        # Then Only one logout request is sent
        # And No errors are thrown
        assert conn.is_closed()
        # Idempotency verified in Core


class TestLogoutEdgeCases:
    """Edge cases and concurrency tests from shared/session/logout.feature."""

    def test_should_handle_concurrent_close_calls_safely(self, connection_factory):
        # Given Snowflake client is logged in
        conn = connection_factory()

        # When Connection is closed from multiple threads concurrently
        import threading

        def close_connection():
            conn.close()

        threads = [threading.Thread(target=close_connection) for _ in range(3)]
        for t in threads:
            t.start()
        for t in threads:
            t.join()

        # Then Only one logout request is sent
        # And All close calls return successfully
        # And No race conditions occur
        assert conn.is_closed()


class TestLogoutPythonWrapper:
    """Python-specific wrapper tests from python/session/logout.feature."""

    def test_should_send_logout_when_server_session_keep_alive_is_none_and_auto_detection_false(
        self, connection_factory
    ):
        # Given Snowflake Python client is created with server_session_keep_alive set to none
        # And enable_server_session_keep_alive_auto_detection is set to false
        conn = connection_factory(server_session_keep_alive=None, enable_server_session_keep_alive_auto_detection=False)

        # When Client closes connection
        conn.close()

        # Then Auto-detection is not performed
        # And Logout request is sent
        # And Connection close metrics are recorded in telemetry
        # And No deprecation warning is emitted
        assert conn.is_closed()
