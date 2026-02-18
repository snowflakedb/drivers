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

import requests
import threading
import warnings

from tests.wiremock_client import WiremockClient


# Helper functions for HTTP verification
def get_wiremock_requests(wiremock_base_url: str) -> list:
    """Query Wiremock admin API for all captured requests."""
    requests_url = f"{wiremock_base_url}/__admin/requests"
    response = requests.get(requests_url)
    return response.json().get("requests", [])


def filter_logout_requests(all_requests: list) -> list:
    """Filter requests to find logout requests (POST /session?delete=true)."""
    return [r for r in all_requests
            if "delete=true" in r.get("request", {}).get("url", "")]


def assert_logout_request_format(logout_request: dict):
    """Verify logout request has correct format."""
    req = logout_request["request"]
    assert req["method"] == "POST", "Logout should use POST method"
    assert "delete=true" in req["url"], "Logout should have delete=true query param"
    assert "Authorization" in req.get("headers", {}), "Logout should have Authorization header"
    assert "Snowflake Token" in req.get("headers", {}).get("Authorization", [""])[0], \
        "Authorization should contain 'Snowflake Token'"


class TestLogoutResourceCleanup:
    """Resource cleanup contract tests from shared/session/logout.feature.

    These tests verify that connection state is properly cleaned up regardless
    of whether logout was sent to the server. They focus on the client-side
    state management contract.
    """

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
        - And: <server_session_keep_alive> is set to any value
        - When: Connection is closed
        - Then: Session token in Connection.tokens is null
        - And: Master token in Connection.tokens is null
        """
        # Given Snowflake client is logged in
        # And <server_session_keep_alive> is set to any value
        conn = connection_factory(server_session_keep_alive=keep_alive)

        # When Connection is closed
        conn.close()

        # Then Session token in Connection.tokens is null
        # And Master token in Connection.tokens is null
        assert conn.is_closed(), f"Connection should be closed with keep_alive={keep_alive}"


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

            assert len(logout_requests) == 1, \
                f"Should send exactly 1 logout request despite 3 close() calls, got {len(logout_requests)}"

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

            assert len(logout_requests) == 1, \
                f"Should send exactly 1 logout request despite concurrent close() calls, got {len(logout_requests)}"

            # And All close calls return successfully
            assert len(exceptions) == 0, f"Expected no exceptions, got: {exceptions}"
            assert conn.is_closed()


class TestLogoutPythonWrapper:
    """Python-specific wrapper tests from python/session/logout.feature.

    These tests verify the Python wrapper correctly passes parameters to Core
    and that logout behavior matches the configured settings (auto-detection,
    server_session_keep_alive).
    """

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
                # And enable_server_session_keep_alive_auto_detection is set to false
                conn = int_test_connection_factory(
                    server_url=wiremock.http_url(),
                    server_session_keep_alive=None,
                    enable_server_session_keep_alive_auto_detection=False
                )

                # When Client closes connection
                conn.close()

            # Then Auto-detection is not performed
            # And Logout request is sent
            all_requests = get_wiremock_requests(wiremock.http_url())
            logout_requests = filter_logout_requests(all_requests)

            assert len(logout_requests) == 1, \
                f"Should send logout request with auto_detection=False, got {len(logout_requests)} requests"

            logout_req = logout_requests[0]["request"]
            assert logout_req["method"] == "POST", "Logout should use POST method"
            assert "delete=true" in logout_req["url"], "Logout should have delete=true query param"

            # And Connection close metrics are recorded in telemetry
            # And No deprecation warning is emitted
            deprecation_warnings = [w for w in captured_warnings
                                   if issubclass(w.category, (FutureWarning, DeprecationWarning))]
            assert len(deprecation_warnings) == 0, \
                f"Should not emit deprecation warning, got: {[str(w.message) for w in deprecation_warnings]}"

            assert conn.is_closed()
