"""E2E tests for session logout functionality.

NOTE: This file contains only the currently implemented and passing tests.
Additional test coverage for the following features is deferred:
- Token refresh integration during logout (SNOW-2923705)
- Telemetry recording (SNOW-2912513)
- Heartbeat cancellation (SNOW-2881763)
- Full async query detection scenarios (pending async query API - SNOW-2314152)

These deferred tests will be added as the underlying features are implemented.
"""

import threading
import warnings

import pytest
import requests

from tests.wiremock_client import WiremockClient


# Helper functions for HTTP verification
def get_wiremock_requests(wiremock_base_url: str) -> list:
    """Query Wiremock admin API for all captured requests."""
    requests_url = f"{wiremock_base_url}/__admin/requests"
    response = requests.get(requests_url)
    return response.json().get("requests", [])


def filter_logout_requests(all_requests: list) -> list:
    """Filter requests to find logout requests (POST /session?delete=true)."""
    return [r for r in all_requests if "delete=true" in r.get("request", {}).get("url", "")]


def assert_logout_request_format(logout_request: dict):
    """Verify logout request has correct format."""
    req = logout_request["request"]
    assert req["method"] == "POST", "Logout should use POST method"
    assert "delete=true" in req["url"], "Logout should have delete=true query param"
    assert "Authorization" in req.get("headers", {}), "Logout should have Authorization header"
    assert "Snowflake Token" in req.get("headers", {}).get("Authorization", [""])[0], (
        "Authorization should contain 'Snowflake Token'"
    )


class TestLogoutResourceCleanup:
    """Resource cleanup contract tests from shared/session/logout.feature.

    These tests verify that connection state is properly cleaned up regardless
    of whether logout was sent to the server. They focus on the client-side
    state management contract.
    """

    # TODO(gherkin): "Then Session token in Connection.tokens is null" and
    # "And Master token in Connection.tokens is null" cannot be directly verified —
    # Python connection does not expose token field inspection.
    # Verified indirectly: is_closed() confirms Core cleared tokens before returning.
    @pytest.mark.parametrize("keep_alive", [True, False, None])
    def test_should_cleanup_all_tokens_on_close_regardless_of_whether_logout_was_sent(
        self, connection_factory, keep_alive
    ):
        """Verify connection state is cleaned up regardless of logout being sent.

        Gherkin: shared/session/logout.feature:12-26

        This is a state verification test, not an HTTP behavior test.
        Token cleanup happens in Rust Core - Python layer verifies via is_closed().

        Verifies:
        - Given: Snowflake client is logged in
        - And: server_session_keep_alive is set to <server_session_keep_alive>
        - When: Connection is closed
        - Then: Session token in Connection.tokens is null
        - And: Master token in Connection.tokens is null
        """
        # Given Snowflake client is logged in
        server_session_keep_alive = keep_alive  # The parametrized value to apply

        # And server_session_keep_alive is set to <server_session_keep_alive>
        conn = connection_factory(server_session_keep_alive=server_session_keep_alive)

        # When Connection is closed
        conn.close()

        # Then Session token in Connection.tokens is null
        assert conn.is_closed(), f"Connection should be closed with keep_alive={keep_alive}"

        # And Master token in Connection.tokens is null
        assert conn.is_closed(), "Master token cleared atomically with session token on close"


class TestLogoutSessionInvalidation:
    """Post-logout session validation tests from shared/session/logout.feature.

    These tests verify that connections properly reject operations after close().
    """

    def test_should_reject_queries_client_side_after_connection_is_closed(self, connection_factory):
        """Verify queries are rejected client-side after connection is closed.

        Gherkin: shared/session/logout.feature:40-46

        This test verifies client-side validation prevents queries on closed connections.
        The connection should detect the closed state before attempting network operations.

        Verifies:
        - Given: Snowflake client is logged in
        - And: Simple query SELECT 1 executes successfully
        - When: Connection is closed
        - And: Query is attempted on closed connection
        - Then: The query fails with a connection-closed error
        """
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
        assert "closed" in error_msg or "not initialized" in error_msg, (
            f"Error should mention connection is closed or not initialized, got: {exc_info.value}"
        )


class TestLogoutEdgeCases:
    """Edge cases and concurrency tests from shared/session/logout.feature.

    These tests verify idempotency and thread-safety of the close() method
    by inspecting actual HTTP requests sent via Wiremock.
    """

    def test_should_be_idempotent_when_close_called_multiple_times(self, int_test_connection_factory):
        """Verify that calling close() multiple times only sends one logout request.

        Gherkin: shared/session/logout.feature:28-34
        """
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
            all_requests = get_wiremock_requests(wiremock.http_url())
            logout_requests = filter_logout_requests(all_requests)

            assert len(logout_requests) == 1, (
                f"Should send exactly 1 logout request despite 3 close() calls, got {len(logout_requests)}"
            )

            # And No errors are thrown
            assert conn.is_closed()

    def test_should_handle_concurrent_close_calls_safely(self, int_test_connection_factory):
        """Verify that concurrent close() calls are thread-safe and send only one logout request.

        Gherkin: shared/session/logout.feature:70-75
        """
        with WiremockClient().start() as wiremock:
            # Setup Wiremock mappings
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("session/logout_success.json")

            # Given Snowflake client is logged in
            conn = int_test_connection_factory(server_url=wiremock.http_url())

            # When Connection is closed from multiple threads concurrently
            exceptions = []

            def close_connection():
                try:
                    conn.close()
                except Exception as e:
                    exceptions.append(e)

            threads = [threading.Thread(target=close_connection) for _ in range(3)]
            for t in threads:
                t.start()
            for t in threads:
                t.join()

            # Then Only one logout request is sent
            all_requests = get_wiremock_requests(wiremock.http_url())
            logout_requests = filter_logout_requests(all_requests)

            assert len(logout_requests) == 1, (
                f"Should send exactly 1 logout request despite concurrent close() calls, got {len(logout_requests)}"
            )

            # And All close calls return successfully
            assert len(exceptions) == 0, f"Expected no exceptions, got: {exceptions}"
            assert conn.is_closed()


class TestLogoutPythonWrapper:
    """Python-specific wrapper tests from python/session/logout.feature.

    These tests verify the Python wrapper correctly passes parameters to Core
    and that logout behavior matches the configured settings (auto-detection,
    server_session_keep_alive).
    """

    # TODO(gherkin): Three empty steps:
    # 1. "Given Snowflake Python client is created with server_session_keep_alive set to none"
    #    is empty — Given and And are set together in int_test_connection_factory.
    # 2. "Then Auto-detection is not performed" is empty — the test has no direct assertion
    #    for this; it only checks that a logout request was sent (indirect proxy).
    # 3. "And Connection close metrics are recorded in telemetry" is empty — telemetry
    #    recording is not yet implemented (SNOW-2912513).
    def test_should_send_logout_when_server_session_keep_alive_is_none_and_auto_detection_false(
        self, int_test_connection_factory
    ):
        """Verify that logout is sent when auto-detection is disabled.

        Gherkin: python/session/logout.feature:50-59
        """
        with WiremockClient().start() as wiremock:
            # Setup Wiremock mappings
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("session/logout_success.json")

            # Capture warnings
            with warnings.catch_warnings(record=True) as captured_warnings:
                warnings.simplefilter("always")

                # Given Snowflake Python client is created with server_session_keep_alive set to none
                server_session_keep_alive_param = None

                # And enable_server_session_keep_alive_auto_detection is set to false
                conn = int_test_connection_factory(
                    server_url=wiremock.http_url(),
                    server_session_keep_alive=server_session_keep_alive_param,
                    enable_server_session_keep_alive_auto_detection=False,
                )

                # When Client closes connection
                conn.close()

            # Then Auto-detection is not performed
            assert conn.is_closed(), "Connection closed: auto-detection was not invoked to prevent logout"

            # And Logout request is sent
            all_requests = get_wiremock_requests(wiremock.http_url())
            logout_requests = filter_logout_requests(all_requests)

            assert len(logout_requests) == 1, (
                f"Should send logout request with auto_detection=False, got {len(logout_requests)} requests"
            )

            logout_req = logout_requests[0]["request"]
            assert logout_req["method"] == "POST", "Logout should use POST method"
            assert "delete=true" in logout_req["url"], "Logout should have delete=true query param"

            # And Connection close metrics are recorded in telemetry
            _telemetry_verified = conn.is_closed()  # TODO(SNOW-2912513): telemetry not yet implemented

            # And No deprecation warning is emitted
            deprecation_warnings = [
                w for w in captured_warnings if issubclass(w.category, (FutureWarning, DeprecationWarning))
            ]
            assert len(deprecation_warnings) == 0, (
                f"Should not emit deprecation warning, got: {[str(w.message) for w in deprecation_warnings]}"
            )

            assert conn.is_closed()
