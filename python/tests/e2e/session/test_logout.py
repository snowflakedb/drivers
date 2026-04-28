"""E2E tests for shared session logout scenarios (shared/session/logout.feature).

Tests here correspond to cross-driver scenarios declared in the shared feature file.
Python-specific logout tests (wrapper config, atexit, retry) live in
test_logout_python.py to keep the orphan validator happy — shared-matching
test files must only contain methods that map to shared scenarios.
"""

import threading

import pytest

from tests.wiremock_client import WiremockClient


class TestLogoutTokenCleanup:
    """Token cleanup tests from shared/session/logout.feature.

    Verifies that session and master tokens are null after close,
    regardless of whether a logout HTTP request was actually sent.
    """

    @pytest.mark.parametrize(
        "server_session_keep_alive",
        [False, True, None],
        ids=["keep_alive=False", "keep_alive=True", "keep_alive=None"],
    )
    @pytest.mark.skip_reference(reason="conn.rest is None on reference connector (different token access pattern)")
    def test_should_cleanup_all_tokens_on_close_regardless_of_whether_logout_was_sent(
        self, int_test_connection_factory, server_session_keep_alive
    ):
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("session/logout_success.json")

            # Given Snowflake client is logged in
            kwargs = {"server_url": wiremock.http_url()}

            # And server_session_keep_alive is set to <server_session_keep_alive>
            if server_session_keep_alive is not None:
                kwargs["server_session_keep_alive"] = server_session_keep_alive
            conn = int_test_connection_factory(**kwargs)
            assert conn.rest.token, "session_token must be non-null before close"
            assert conn.rest.master_token, "master_token must be non-null before close"

            # When Connection is closed
            conn.close()

            # Then Session token in Connection.tokens is null
            assert not conn.rest.token, (  # Core returns "" not None — falsy check
                f"session_token must be null after close (keep_alive={server_session_keep_alive}), "
                f"got {conn.rest.token!r}"
            )

            # And Master token in Connection.tokens is null
            assert not conn.rest.master_token, (
                f"master_token must be null after close (keep_alive={server_session_keep_alive}), "
                f"got {conn.rest.master_token!r}"
            )


class TestLogoutSessionInvalidation:
    """Post-logout session validation tests from shared/session/logout.feature.

    These tests verify that connections properly reject operations after close().
    """

    def test_should_reject_queries_client_side_after_connection_is_closed(self, connection_factory):
        """Verify queries are rejected client-side after connection is closed."""
        # Given Snowflake client is logged in
        conn = connection_factory()

        # And Simple query SELECT 1 executes successfully
        cursor = conn.cursor()
        cursor.execute("SELECT 1")
        result_before = cursor.fetchall()
        assert len(result_before) == 1, "SELECT 1 should return 1 row before close"

        # When Connection is closed
        conn.close()

        # And Query is attempted on closed connection
        with pytest.raises(Exception) as exc_info:
            cursor.execute("SELECT 1")

        # Then The query fails with a connection-closed error
        error_msg = str(exc_info.value).lower()
        assert "closed" in error_msg, f"Error must mention connection is closed, got: {exc_info.value}"


class TestLogoutEdgeCases:
    """Edge cases and concurrency tests from shared/session/logout.feature.

    These tests verify idempotency and thread-safety of the close() method
    by inspecting actual HTTP requests sent via Wiremock.
    """

    def test_should_be_idempotent_when_close_called_multiple_times(self, int_test_connection_factory):
        """Verify that calling close() multiple times only sends one logout request."""
        with WiremockClient().start() as wiremock:
            # Setup Wiremock mappings
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("session/logout_success.json")

            # Given Snowflake client is logged in
            conn = int_test_connection_factory(server_url=wiremock.http_url())

            # When Connection is closed
            conn.close()
            # And Connection is closed again
            conn.close()
            # And Connection is closed a third time
            conn.close()

            # Then Only one logout request is sent
            logout_requests = wiremock.get_logout_requests()

            assert len(logout_requests) == 1, (
                f"Should send exactly 1 logout request despite 3 close() calls, got {len(logout_requests)}"
            )

            # And No errors are thrown
            assert conn.is_closed()

    @pytest.mark.skip_reference(reason="Old connector has no close idempotency — 5 threads send 5 logouts")
    def test_should_handle_concurrent_close_calls_safely(self, int_test_connection_factory):
        """Verify that concurrent close() calls are thread-safe and send only one logout request."""
        with WiremockClient().start() as wiremock:
            # Setup Wiremock mappings
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("session/logout_success.json")

            # Given Snowflake client is logged in
            conn = int_test_connection_factory(server_url=wiremock.http_url())

            # When Connection is closed from multiple threads concurrently
            exceptions = []
            barrier = threading.Barrier(5)

            def close_connection():
                try:
                    barrier.wait()
                    conn.close()
                except Exception as e:
                    exceptions.append(e)

            threads = [threading.Thread(target=close_connection) for _ in range(5)]
            for t in threads:
                t.start()
            for t in threads:
                t.join()

            # Then Only one logout request is sent
            logout_requests = wiremock.get_logout_requests()

            assert len(logout_requests) == 1, (
                f"Should send exactly 1 logout request despite concurrent close() calls, got {len(logout_requests)}"
            )

            # And All close calls return successfully
            assert len(exceptions) == 0, f"Expected no exceptions, got: {exceptions}"
            assert conn.is_closed()
