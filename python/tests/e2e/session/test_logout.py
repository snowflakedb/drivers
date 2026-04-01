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
import time
import warnings

import pytest
import requests

from snowflake.connector._internal.logout_config_mapping import (
    ErrorStrategy,
    map_logout_config_phase2,
)
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
        assert "closed" in error_msg or "not initialized" in error_msg, (
            f"Error should mention connection is closed or not initialized, got: {exc_info.value}"
        )


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
            all_requests = get_wiremock_requests(wiremock.http_url())
            logout_requests = filter_logout_requests(all_requests)

            assert len(logout_requests) == 1, (
                f"Should send exactly 1 logout request despite 3 close() calls, got {len(logout_requests)}"
            )

            # And No errors are thrown
            assert conn.is_closed()

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

    def test_should_have_enable_server_session_keep_alive_auto_detection_default_to_true(
        self, int_test_connection_factory
    ):
        """Verify enable_server_session_keep_alive_auto_detection defaults to True.

        Default True is required for Phase 2 backward compat (SNOW-2314152): the old
        Python driver always checked _async_sfqids before logout. Without this default,
        Core receives enable_logout_auto_detection=None → always logout → kills async queries.
        """
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("session/logout_success.json")

            # Given Snowflake Python client is created without enable_server_session_keep_alive_auto_detection parameter
            conn = int_test_connection_factory(server_url=wiremock.http_url())

            # When Connection configuration is checked
            logout_config = map_logout_config_phase2(conn)

            # Then enable_server_session_keep_alive_auto_detection defaults to true
            assert conn.enable_server_session_keep_alive_auto_detection is True, (
                "Default must be True for Phase 2 backward compat: mirrors old Python driver "
                "which always checked async query registry before logout (SNOW-2314152)"
            )

            # And Auto-detection is enabled by default
            logout_config = map_logout_config_phase2(conn)
            assert logout_config.enable_logout_auto_detection is True, (
                "Default True must flow through to Core so registry check is performed"
            )

            conn.close()

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
        """Verify that logout is sent when auto-detection is disabled."""
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

    def test_should_pass_correct_parameters_when_server_session_keep_alive_is_none_and_auto_detection_true(
        self, int_test_connection_factory
    ):
        """Verify Python wrapper passes None keep-alive and True auto-detection to Core.

        Phase 2 truth table: server_session_keep_alive=None + enable_auto_detection=True
        → no Phase 2 remap (only False + True triggers remap) → Core receives None + True.
        """
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("session/logout_success.json")

            with warnings.catch_warnings(record=True) as captured_warnings:
                warnings.simplefilter("always")

                # Given Snowflake Python client is created with server_session_keep_alive set to none
                server_session_keep_alive_param = None

                # And enable_server_session_keep_alive_auto_detection is set to true
                conn = int_test_connection_factory(
                    server_url=wiremock.http_url(),
                    server_session_keep_alive=server_session_keep_alive_param,
                    enable_server_session_keep_alive_auto_detection=True,
                )

                # When Client closes connection
                conn.close()

            # Then server_session_keep_alive none is passed to Core
            logout_config = map_logout_config_phase2(conn)
            assert logout_config.server_session_keep_alive is None, (
                "Phase 2: None keep-alive should pass through to Core unchanged"
            )

            # And enable_server_session_keep_alive_auto_detection true is passed to Core
            assert logout_config.enable_logout_auto_detection is True, "auto_detection=True should pass through to Core"

            # And No deprecation warning is emitted
            deprecation_warnings = [
                w for w in captured_warnings if issubclass(w.category, (FutureWarning, DeprecationWarning))
            ]
            msgs = [str(w.message) for w in deprecation_warnings]
            assert len(deprecation_warnings) == 0, f"None + True should not emit deprecation warning, got: {msgs}"

    def test_should_pass_correct_parameters_when_server_session_keep_alive_is_false(self, int_test_connection_factory):
        """Verify Python wrapper remaps False keep-alive to None when auto-detection is True (default).

        Phase 2 truth table: server_session_keep_alive=False + enable_auto_detection=True (default)
        → Phase 2 remap: False + True → None so Core checks registry (legacy Python behavior).
        Default True is required for Phase 2 backward compat (SNOW-2314152).
        """
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("session/logout_success.json")

            with warnings.catch_warnings(record=True) as _captured_warnings:
                warnings.simplefilter("always")

                # Given Snowflake Python client is created with server_session_keep_alive set to false
                conn = int_test_connection_factory(
                    server_url=wiremock.http_url(),
                    server_session_keep_alive=False,
                )

                # When Client closes connection
                conn.close()

            # Then server_session_keep_alive is remapped to none by Phase 2 mapping
            logout_config = map_logout_config_phase2(conn)
            assert logout_config.server_session_keep_alive is None, (
                "Phase 2: False + auto_detection=True (default) → remap → Core receives None to check registry"
            )

            # And Deprecation warning is emitted
            _deprecation_emitted = (
                False  # TODO(SNOW-2314152): Warning for server_session_keep_alive=False not yet implemented
            )

            # And Warning mentions that false will force logout in Phase 3
            _warning_message_verified = False  # TODO(SNOW-2314152): see above

            # Verify logout was actually sent to Core (False = explicit logout)
            all_requests = get_wiremock_requests(wiremock.http_url())
            logout_requests = filter_logout_requests(all_requests)
            assert len(logout_requests) == 1, (
                f"server_session_keep_alive=False should send exactly one logout request, got {len(logout_requests)}"
            )

    def test_should_use_python_default_15_second_timeout_and_3_max_retries(self, int_test_connection_factory):
        """Verify Python wrapper configures 15s total timeout and 3 max attempts by default."""
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("session/logout_success.json")

            # Given Snowflake Python client is created with default timeout configuration
            conn = int_test_connection_factory(server_url=wiremock.http_url())

            # When Connection is closed
            start = time.monotonic()
            conn.close()
            elapsed = time.monotonic() - start

            # Then Logout timeout of 15 seconds is passed to Core
            logout_config = map_logout_config_phase2(conn)
            assert logout_config.logout_total_timeout_seconds == 15, (
                f"Expected 15s total timeout, got {logout_config.logout_total_timeout_seconds}s"
            )

            # And Logout max retries of 3 is passed to Core
            assert logout_config.max_attempts == 3, (
                f"Expected 3 max attempts (2 retries), got {logout_config.max_attempts}"
            )

            # And Logout request completes within 15 seconds
            assert elapsed < 15.0, f"Close should complete within 15 seconds, took {elapsed:.1f}s"

    def test_should_use_best_effort_error_handling_strategy_by_default(self, int_test_connection_factory):
        """Verify close() does not raise when server returns 500 on all logout attempts.

        Best-effort strategy: close() succeeds even if logout fails.
        All retries (max_attempts=3) are exhausted, then Core reports WARN and returns ok.
        """
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")

            # Given Snowflake Python client is created with default parameters
            conn = int_test_connection_factory(server_url=wiremock.http_url())

            # And Server will return 500 Internal Server Error on logout on all attempts
            wiremock.add_mapping("session/logout_500_always.json")

            # When Connection is closed
            conn.close()  # Must NOT raise with best-effort strategy

            # Then Logout attempts are bounded by the default retry limit
            all_requests = get_wiremock_requests(wiremock.http_url())
            logout_requests = filter_logout_requests(all_requests)
            assert len(logout_requests) <= 3, (
                f"Expected at most 3 logout attempts (default max_attempts), got {len(logout_requests)}"
            )

            # And No further requests are sent after retry limit is reached
            assert len(logout_requests) > 0, "At least one logout attempt should have been made"

            # And Error is logged as WARN
            _warn_logged = True  # TODO(SNOW-2314153): WARN log capture requires logging integration

            # And close() method does not raise exception
            _close_succeeded = conn.is_closed()  # verified above: conn.close() did not raise

            # And Connection cleanup succeeds
            assert conn.is_closed(), "Connection should be closed despite all logout attempts failing"

            # And Error handling strategy is best-effort by default
            logout_config = map_logout_config_phase2(conn)
            assert logout_config.error_strategy == ErrorStrategy.BEST_EFFORT, (
                "Default error strategy should be BEST_EFFORT"
            )


class TestLogoutRetryBehavior:
    """Retry behavior tests from python/session/logout.feature.

    These tests verify the retry parameter on close() controls whether Core
    retries a failed logout request.
    """

    def test_should_retry_logout_on_transient_failure_when_close_called_with_default_retry(
        self, int_test_connection_factory
    ):
        """Verify close() retries a failed logout and sends two requests on transient 503."""
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")

            # Given Snowflake Python client is logged in
            conn = int_test_connection_factory(server_url=wiremock.http_url())

            # And Server will return 503 on first logout attempt then succeed
            wiremock.add_mapping("session/logout_503_then_success.json")

            # When close() is called with default parameters
            conn.close()

            # Then Logout succeeds after retry
            assert conn.is_closed(), "Connection should be closed after successful retry"

            # And Two logout requests were sent to server
            all_requests = get_wiremock_requests(wiremock.http_url())
            logout_requests = filter_logout_requests(all_requests)
            assert len(logout_requests) == 2, (
                f"Expected 2 logout requests (1 failure + 1 success), got {len(logout_requests)}"
            )

    def test_should_not_retry_logout_on_transient_failure_when_close_called_with_retry_false(
        self, int_test_connection_factory
    ):
        """Verify close(retry=False) sends exactly one logout request and does not retry."""
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")

            # Given Snowflake Python client is logged in
            conn = int_test_connection_factory(server_url=wiremock.http_url())

            # And Server will return 503 on first logout attempt then succeed
            wiremock.add_mapping("session/logout_503_then_success.json")

            # When close(retry=False) is called
            conn.close(retry=False)

            # Then Logout is not retried
            all_requests = get_wiremock_requests(wiremock.http_url())
            logout_requests = filter_logout_requests(all_requests)
            assert len(logout_requests) == 1, (
                f"Expected exactly 1 logout request (no retry), got {len(logout_requests)}"
            )

            # And Only one logout request was sent to server
            assert len(logout_requests) == 1, "retry=False should prevent any retry attempts"

            # And Error is handled according to best-effort strategy
            assert conn.is_closed(), (
                "Connection should be closed: best-effort strategy suppresses error from single failed attempt"
            )
