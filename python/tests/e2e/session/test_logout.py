"""E2E tests for session logout functionality.

NOTE: This file contains only the currently implemented and passing tests.
Additional test coverage for the following features is deferred:
- Token refresh integration during logout (SNOW-2923705)
- Telemetry recording (SNOW-2912513)
- Heartbeat cancellation (SNOW-2881763)
- Full async query detection scenarios (pending async query API - SNOW-2314152)

These deferred tests will be added as the underlying features are implemented.
"""

import logging
import subprocess
import sys
import textwrap
import threading
import time
import warnings

import pytest

from tests.private_key_helper import get_test_private_key_path
from tests.wiremock_client import WiremockClient


def assert_logout_request_format(logout_request: dict) -> None:
    """Verify logout request has correct format."""
    req = logout_request["request"]
    assert req["method"] == "POST", "Logout should use POST method"
    assert "delete=true" in req["url"], "Logout should have delete=true query param"
    assert "Authorization" in req.get("headers", {}), "Logout should have Authorization header"
    auth_header = req.get("headers", {}).get("Authorization", "")
    assert auth_header[:16] == "Snowflake Token=", "Authorization header should start with 'Snowflake Token='"


@pytest.mark.skip_reference(reason="conn.rest is None on reference connector (different token access pattern)")
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


class TestLogoutPythonWrapper:
    """Python-specific wrapper tests from python/session/logout.feature.

    These tests verify the Python wrapper correctly passes parameters to Core
    and that logout behavior matches the configured settings (auto-detection,
    server_session_keep_alive).
    """

    # core_mock tests moved to tests/integ/session/test_logout.py::TestLogoutConfigPassing

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
            logout_requests = wiremock.get_logout_requests()

            assert len(logout_requests) == 1, (
                f"Should send logout request with auto_detection=False, got {len(logout_requests)} requests"
            )

            assert_logout_request_format(logout_requests[0])

            # And Connection close metrics are recorded in telemetry
            pass  # TODO(SNOW-2912513): telemetry not yet implemented — step unverified

            # And No deprecation warning is emitted
            deprecation_warnings = [
                w for w in captured_warnings if issubclass(w.category, (FutureWarning, DeprecationWarning))
            ]
            assert len(deprecation_warnings) == 0, (
                f"Should not emit deprecation warning, got: {[str(w.message) for w in deprecation_warnings]}"
            )

            assert conn.is_closed()

    @pytest.mark.skip_reference(reason="core_proxy fixture imports _internal")
    def test_should_pass_correct_parameters_when_server_session_keep_alive_is_none_and_auto_detection_true(
        self, int_test_connection_factory, core_proxy
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
        options = core_proxy.get_options_sent()
        assert "server_session_keep_alive" not in options, "None keep-alive should not be sent to Core"

        # And enable_server_session_keep_alive_auto_detection true is passed to Core
        assert options.get("enable_server_session_keep_alive_auto_detection") is True, (
            "auto_detection=True should pass through to Core"
        )

        # And No deprecation warning is emitted
        deprecation_warnings = [
            w for w in captured_warnings if issubclass(w.category, (FutureWarning, DeprecationWarning))
        ]
        msgs = [str(w.message) for w in deprecation_warnings]
        assert len(deprecation_warnings) == 0, f"None + True should not emit deprecation warning, got: {msgs}"

    @pytest.mark.skip_reference(reason="core_proxy fixture imports _internal")
    def test_should_remap_server_session_keep_alive_false_to_none_when_auto_detection_defaults_to_true(
        self, int_test_connection_factory, core_proxy
    ):
        """Verify Python wrapper remaps False keep-alive to None when auto-detection is True (default).

        Phase 2 truth table: server_session_keep_alive=False + enable_auto_detection=True (default)
        → Phase 2 remap: False + True → None so Core checks registry (legacy Python behavior).
        Default True is required for Phase 2 backward compat (SNOW-2314152).
        """
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("session/logout_success.json")

            # Given Snowflake Python client is created with server_session_keep_alive set to false
            with warnings.catch_warnings(record=True) as captured_warnings:
                warnings.simplefilter("always")
                conn = int_test_connection_factory(
                    server_url=wiremock.http_url(),
                    server_session_keep_alive=False,
                )

            # When Client closes connection
            conn.close()

            # Then server_session_keep_alive false is remapped to none
            options = core_proxy.get_options_sent()
            assert "server_session_keep_alive" not in options, (
                "False + auto_detection=True (default) → remap → None → not sent to Core"
            )

            future_warnings = [w for w in captured_warnings if issubclass(w.category, FutureWarning)]

            # And Deprecation warning is emitted
            assert any("server_session_keep_alive=False" in str(w.message) for w in future_warnings), (
                f"Expected FutureWarning about server_session_keep_alive=False, "
                f"got: {[str(w.message) for w in captured_warnings]}"
            )

            # And Warning mentions that false will force logout in Phase 3
            assert any("always logout" in str(w.message) for w in future_warnings), (
                f"Warning should mention Phase 3 'always logout' behavior, "
                f"got: {[str(w.message) for w in future_warnings]}"
            )

            # Verify logout was actually sent to Core (False = explicit logout)
            logout_requests = wiremock.get_logout_requests()
            assert len(logout_requests) == 1, (
                f"server_session_keep_alive=False should send exactly one logout request, got {len(logout_requests)}"
            )

    @pytest.mark.skip_reference(reason="core_proxy fixture imports _internal")
    def test_should_pass_server_session_keep_alive_false_to_core_when_auto_detection_explicitly_disabled(
        self, int_test_connection_factory, core_proxy
    ):
        """Verify Python wrapper passes False to Core when auto_detection is explicitly disabled.

        False + auto_detection=False (explicit) → no Phase 2 remap → Core receives False (force logout).
        No deprecation warning: user opted out of auto-detection consciously.
        """
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("session/logout_success.json")

            with warnings.catch_warnings(record=True) as captured_warnings:
                warnings.simplefilter("always")

                # Given Snowflake Python client is created with server_session_keep_alive set to false
                server_session_keep_alive_param = False

                # And enable_server_session_keep_alive_auto_detection is set to false
                conn = int_test_connection_factory(
                    server_url=wiremock.http_url(),
                    server_session_keep_alive=server_session_keep_alive_param,
                    enable_server_session_keep_alive_auto_detection=False,
                )

            # When Client closes connection
            conn.close()

        # Then server_session_keep_alive false is passed to Core
        options = core_proxy.get_options_sent()
        assert options.get("server_session_keep_alive") is False, (
            "False + auto_detection=False → no remap → Core receives False"
        )

        # And No deprecation warning is emitted
        deprecation_warnings = [
            w for w in captured_warnings if issubclass(w.category, (FutureWarning, DeprecationWarning))
        ]
        assert len(deprecation_warnings) == 0, (
            f"False + auto_detection=False should not emit deprecation warning, "
            f"got: {[str(w.message) for w in deprecation_warnings]}"
        )

    @pytest.mark.skip_reference(reason="core_proxy fixture imports _internal")
    @pytest.mark.parametrize(
        "auto_detection",
        [True, False],
        ids=["auto_detection_true", "auto_detection_false"],
    )
    def test_should_skip_logout_when_server_session_keep_alive_is_true_regardless_of_auto_detection(
        self, int_test_connection_factory, core_proxy, auto_detection: bool
    ) -> None:
        """Verify no logout is sent when server_session_keep_alive=True, regardless of auto_detection.

        Phase 2 truth table: True + any auto_detection → no logout, no deprecation.
        Core receives server_session_keep_alive=True and skips the logout request.
        """
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")

            with warnings.catch_warnings(record=True) as captured_warnings:
                warnings.simplefilter("always")

                # Given Snowflake Python client is created with server_session_keep_alive set to true
                server_session_keep_alive_param = True

                # And enable_server_session_keep_alive_auto_detection is set to <auto_detection>
                conn = int_test_connection_factory(
                    server_url=wiremock.http_url(),
                    server_session_keep_alive=server_session_keep_alive_param,
                    enable_server_session_keep_alive_auto_detection=auto_detection,
                )

                # When Connection is closed
                conn.close()

            # Then No logout request is sent
            logout_requests = wiremock.get_logout_requests()
            assert len(logout_requests) == 0, (
                f"Expected no logout when server_session_keep_alive=True "
                f"(auto_detection={auto_detection}), got {len(logout_requests)} requests"
            )

            # And server_session_keep_alive true is passed to Core
            options = core_proxy.get_options_sent()
            assert options.get("server_session_keep_alive") is True, (
                f"Expected server_session_keep_alive=True passed to Core, "
                f"got {options.get('server_session_keep_alive')!r}"
            )

            # And No deprecation warning is emitted
            deprecation_warnings = [
                w for w in captured_warnings if issubclass(w.category, (FutureWarning, DeprecationWarning))
            ]
            assert len(deprecation_warnings) == 0, (
                f"server_session_keep_alive=True should not emit deprecation warning, "
                f"got: {[str(w.message) for w in deprecation_warnings]}"
            )

    @pytest.mark.skip_reference(reason="core_proxy fixture imports _internal")
    def test_should_use_python_default_15_second_timeout_and_3_max_retries(
        self, int_test_connection_factory, core_proxy
    ):
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
            options = core_proxy.get_options_sent()
            assert options.get("logout_total_timeout_seconds") == 15, (
                f"Expected {15}s total timeout, got {options.get('logout_total_timeout_seconds')}"
            )

            # And Logout max retries of 3 is passed to Core
            assert options.get("logout_max_attempts") == 3, (
                f"Expected {3} max attempts, got {options.get('logout_max_attempts')}"
            )

            # And Logout request completes within 15 seconds
            assert elapsed < 15.0, f"Close should complete within 15 seconds, took {elapsed:.1f}s"

    @pytest.mark.skip_reference(reason="core_proxy fixture imports _internal")
    def test_should_use_best_effort_error_handling_strategy_by_default(
        self, int_test_connection_factory, core_proxy, tmp_path
    ):
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

            # Capture Core logs to a temp file. We use a file instead of pytest caplog
            # because the Rust→Python FFI log bridge (sf_core_init_logger callback in
            # c_api.py) writes to the snowflake.connector._core logger which has
            # propagate=False and NullHandler by default — caplog can't intercept it
            # without reconfiguring propagation. A FileHandler on the logger directly
            # captures the FFI-bridged logs.
            log_file = tmp_path / "core.log"
            core_logger = logging.getLogger("snowflake.connector._core")
            handler = logging.FileHandler(str(log_file))
            handler.setLevel(logging.WARNING)
            handler.setFormatter(logging.Formatter("%(levelname)s %(message)s"))
            core_logger.addHandler(handler)
            original_level = core_logger.level
            core_logger.setLevel(logging.WARNING)

            try:
                # When Connection is closed
                conn.close()  # Must NOT raise with best-effort strategy
            finally:
                core_logger.removeHandler(handler)
                core_logger.setLevel(original_level)
                handler.close()

            # Then Logout attempts are bounded by the default retry limit
            logout_requests = wiremock.get_logout_requests()
            assert len(logout_requests) <= 3, (
                f"Expected at most 3 logout attempts (default max_attempts), got {len(logout_requests)}"
            )

            # And No further requests are sent after retry limit is reached
            assert len(logout_requests) == 3, (
                f"Expected exactly {3} attempts (limit hit, no more sent). Got {len(logout_requests)}"
            )

            # And Error is logged as WARN
            log_content = log_file.read_text()
            assert "WARNING" in log_content and "Logout failed" in log_content, (
                f"Expected WARNING log with 'Logout failed' from Core.\nCaptured:\n{log_content}"
            )

            # And close() method does not raise exception
            pass  # proven by conn.close() completing without exception in the try block above

            # And Connection cleanup succeeds
            assert conn.is_closed()

            # And Error handling strategy is best-effort by default
            options = core_proxy.get_options_sent()
            assert options.get("logout_error_strategy") == "best_effort", "Default error strategy should be BEST_EFFORT"


class TestLogoutRetryBehavior:
    """Retry behavior tests from python/session/logout.feature.

    These tests verify the retry parameter on close() controls whether Core
    retries a failed logout request.
    """

    @pytest.mark.skip_reference(reason="Old connector (v4.3.0) does not retry logout on 503")
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
            logout_requests = wiremock.get_logout_requests()
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
            logout_requests = wiremock.get_logout_requests()
            assert len(logout_requests) == 1, (
                f"retry=False should prevent retries despite 503, got {len(logout_requests)} requests"
            )

            # And Only one logout request was sent to server
            assert_logout_request_format(logout_requests[0])

            # And Error is handled according to best-effort strategy
            assert conn.is_closed(), (
                "Connection should be closed: best-effort strategy suppresses error from single failed attempt"
            )


@pytest.mark.skip_reference(
    reason="subprocess imports Connection (not SnowflakeConnection), _close_at_process_exit missing"
)
@pytest.mark.skipif(
    sys.version_info[:2] == (3, 9),
    reason="SNOW-3416420: Rust tokio runtime teardown crashes during Py_Finalize on py3.9",
)
class TestAutoCleanup:
    """Auto-cleanup deprecation tests from python/session/logout.feature.

    Phase 2 (SNOW-2314152): atexit hooks preserved for backward compatibility,
    gated behind auto_cleanup param (default: enabled), with deprecation warning
    when auto-cleanup actually runs.
    """

    # test_should_have_auto_cleanup_enabled_by_default moved to integ (uses core_mock)

    def test_should_unregister_atexit_handler_when_close_called_explicitly(self, int_test_connection_factory):
        """Verify close() unregisters atexit handler so process exit doesn't trigger second close."""
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("session/logout_success.json")

            wiremock_url = wiremock.http_url()
            private_key_path = get_test_private_key_path()

            # Given Snowflake Python client is created with auto_cleanup enabled
            subprocess_code = textwrap.dedent(f"""\
                import atexit
                from snowflake.connector.connection import Connection
                conn = Connection(
                    user="test_user",
                    account="test_account",
                    database="test_database",
                    schema="test_schema",
                    warehouse="test_warehouse",
                    role="test_role",
                    server_url="{wiremock_url}",
                    authenticator="SNOWFLAKE_JWT",
                    private_key_file=r"{private_key_path}",
                    auto_cleanup=True,
                    enable_server_session_keep_alive_auto_detection=False,
                )
                # And atexit handler is registered at connection init
                assert conn.auto_cleanup is True, "auto_cleanup must be True for atexit.register to fire"
                print("ATEXIT_REGISTERED")
                # When close() is called explicitly
                orig = atexit.unregister
                def _spy(f): orig(f); print("ATEXIT_UNREGISTERED")
                atexit.unregister = _spy
                conn.close()
                print("CLOSE_CALLED")
            """)

            # And atexit handler is registered at connection init
            result = subprocess.run(
                [sys.executable, "-c", subprocess_code],
                capture_output=True,
                text=True,
                timeout=120,
            )
            assert result.returncode == 0, f"Subprocess failed:\nstderr: {result.stderr}"
            assert "ATEXIT_REGISTERED" in result.stdout, "Subprocess must confirm atexit registration"

            # When close() is called explicitly
            assert "CLOSE_CALLED" in result.stdout, "Subprocess must confirm close() was called"

            # Then atexit handler is unregistered
            assert "ATEXIT_UNREGISTERED" in result.stdout, "conn.close() must call atexit.unregister()"
            logout_requests = wiremock.get_logout_requests()

            # And Subsequent process exit will not trigger second close
            assert len(logout_requests) == 1, (
                f"Expected exactly 1 logout (from close()), not 2 (close + atexit). "
                f"Got {len(logout_requests)}: unregister failed or atexit fired despite close()."
            )

    # This test spawns 2 subprocesses × 120s timeout each (240s worst case).
    # CI timeout must be >= 300s.
    def test_should_call_close_with_retry_false_from_atexit_handler(self, int_test_connection_factory):
        """Verify atexit handler calls close(retry=False), no retries, exceptions suppressed."""
        private_key_path = get_test_private_key_path()

        def _build_subprocess_code(wiremock_url: str) -> str:
            return textwrap.dedent(f"""\
                from snowflake.connector.connection import Connection
                conn = Connection(
                    user="test_user",
                    account="test_account",
                    database="test_database",
                    schema="test_schema",
                    warehouse="test_warehouse",
                    role="test_role",
                    server_url="{wiremock_url}",
                    authenticator="SNOWFLAKE_JWT",
                    private_key_file=r"{private_key_path}",
                    auto_cleanup=True,
                    enable_server_session_keep_alive_auto_detection=False,
                )
            """)

        # Phase A: process exits with leaked connection; first logout attempt returns 503, second
        # attempt returns 200. With retry=False, the 503 is never retried (len==1). This
        # distinguishes retry=False (len==1) from retry=True (len==2: 503 + retry 200).
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("session/logout_503_then_success.json")

            # Given Snowflake Python client is created with auto_cleanup enabled
            subprocess_code = _build_subprocess_code(wiremock.http_url())

            # And Connection was not closed explicitly
            assert "conn.close()" not in subprocess_code

            # When Process exits
            result = subprocess.run(
                [sys.executable, "-c", subprocess_code],
                capture_output=True,
                text=True,
                timeout=120,
            )
            assert result.returncode == 0, f"Subprocess failed:\nstderr: {result.stderr}"

            # Then atexit handler calls close(retry=False)
            logout_requests = wiremock.get_logout_requests()

            # 503 is retried under retry=True (→ len==2) but not under retry=False (→ len==1).
            # And No retries are attempted during atexit close
            assert len(logout_requests) == 1, (
                f"retry=False: 503 must not be retried (retry=True would push count to 2), "
                f"got {len(logout_requests)} requests"
            )

            # And Session is logged out if conditions allow
            assert_logout_request_format(logout_requests[0])

        # Phase B: server error — atexit handler must not crash the process
        with WiremockClient().start() as wiremock2:
            wiremock2.add_mapping("auth/login_success_jwt.json")
            wiremock2.add_mapping("session/logout_500_always.json")

            # And All exceptions during atexit close are suppressed
            subprocess_code_b = _build_subprocess_code(wiremock2.http_url())
            result_b = subprocess.run(
                [sys.executable, "-c", subprocess_code_b],
                capture_output=True,
                text=True,
                timeout=120,
            )
            assert result_b.returncode == 0, (
                f"Process must exit cleanly despite 500 on logout.\nstderr: {result_b.stderr}"
            )
            assert len(wiremock2.get_logout_requests()) >= 1, (
                "Phase B must reach the logout endpoint to prove exception suppression"
            )

    def test_should_emit_deprecation_warning_only_once_when_multiple_auto_cleanup_handlers_run_during_process_exit(
        self, int_test_connection_factory
    ):
        """Verify warning deduplication: 10 leaked connections emit only 1 FutureWarning."""
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            # logout_503_then_success: first request → 503, then 200.
            # Proves retry=False: the 503 is not retried (total stays 10, not 11+).
            wiremock.add_mapping("session/logout_503_then_success.json")

            wiremock_url = wiremock.http_url()
            private_key_path = get_test_private_key_path()

            # Given A separate Python subprocess is spawned
            subprocess_code = textwrap.dedent(f"""\
                import sys
                from snowflake.connector.connection import Connection
                connections = []
                for i in range(10):
                    conn = Connection(
                        user="test_user",
                        account="test_account",
                        database="test_database",
                        schema="test_schema",
                        warehouse="test_warehouse",
                        role="test_role",
                        server_url="{wiremock_url}",
                        authenticator="SNOWFLAKE_JWT",
                        private_key_file=r"{private_key_path}",
                        auto_cleanup=True,
                        enable_server_session_keep_alive_auto_detection=False,
                    )
                    connections.append(conn)
            """)

            # And 10 Snowflake clients are created with auto_cleanup enabled
            assert subprocess_code.count("auto_cleanup=True") == 1

            # And None of the connections are explicitly closed
            assert "conn.close()" not in subprocess_code

            # When The subprocess exits
            result = subprocess.run(
                [sys.executable, "-c", subprocess_code],
                capture_output=True,
                text=True,
                timeout=120,
            )
            # Then Auto-cleanup is triggered for all 10 leaked connections
            logout_requests = wiremock.get_logout_requests()
            assert len(logout_requests) == 10, (
                f"Expected 10 logout requests (one per leaked connection), got {len(logout_requests)}"
            )

            # retry=False → 503 is not retried → total stays at 10 (step above) and the 503
            # is visible in the journal. retry=True → 503 would trigger a retry → 11+ total.
            # Both (a) total==10 and (b) one got 503 together prove retry=False was used.
            # And Each auto-cleanup close is invoked with retry false
            responses_503 = [r for r in logout_requests if r.get("response", {}).get("status") == 503]
            assert len(responses_503) == 1, (
                f"Expected exactly 1 connection to receive 503 (retry=False: not retried). "
                f"Got {len(responses_503)} 503 responses out of {len(logout_requests)} total. "
                f"If retry=True, the 503 retry would push total to 11+."
            )

            # And Deprecation warning is emitted only once per process
            warning_text = "Auto-cleanup at exit will be disabled"
            warning_count = result.stderr.count(warning_text)
            assert warning_count == 1, (
                f"FutureWarning should be emitted exactly once per process (deduplication), "
                f"got {warning_count} occurrences.\nstderr:\n{result.stderr}"
            )

    def test_should_not_register_atexit_handler_when_auto_cleanup_explicitly_disabled(
        self, int_test_connection_factory
    ):
        """Verify auto_cleanup=False prevents atexit registration entirely."""
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")

            wiremock_url = wiremock.http_url()
            private_key_path = get_test_private_key_path()

            # Given Snowflake Python client is created with auto_cleanup set to false
            subprocess_code = textwrap.dedent(f"""\
                import warnings
                warnings.filterwarnings("ignore", category=FutureWarning)
                from snowflake.connector.connection import Connection
                conn = Connection(
                    user="test_user",
                    account="test_account",
                    database="test_database",
                    schema="test_schema",
                    warehouse="test_warehouse",
                    role="test_role",
                    server_url="{wiremock_url}",
                    authenticator="SNOWFLAKE_JWT",
                    private_key_file=r"{private_key_path}",
                    auto_cleanup=False,
                    enable_server_session_keep_alive_auto_detection=False,
                )
            """)

            # And Connection is not explicitly closed
            assert "conn.close()" not in subprocess_code

            # When Process exits
            result = subprocess.run(
                [sys.executable, "-c", subprocess_code],
                capture_output=True,
                text=True,
                timeout=120,
            )
            assert result.returncode == 0, f"Subprocess failed:\nstderr: {result.stderr}"

            # Then No atexit handler was registered
            logout_requests = wiremock.get_logout_requests()

            # And No automatic close is performed
            assert len(logout_requests) == 0, (
                "auto_cleanup=False must not register an atexit handler: no logout expected on process exit"
            )

    def test_should_emit_telemetry_and_warn_when_connection_leaked_at_process_exit(
        self, int_test_connection_factory
    ) -> None:
        """Leak detection: WARN + telemetry when connection not closed before exit.

        Telemetry assertions are pending SNOW-2912513.
        WARN is currently the FutureWarning emitted by _close_at_process_exit();
        a proper logger.warning() call will be added under SNOW-2912513.
        """
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("session/logout_success.json")

            with warnings.catch_warnings(record=True) as captured_warnings:
                warnings.simplefilter("always")

                # Given Snowflake Python client is logged in
                conn = int_test_connection_factory(server_url=wiremock.http_url())

                # And Connection is not explicitly closed
                assert not conn.is_closed()

                # When Process exit is detected
                conn._close_at_process_exit()

            # Current: FutureWarning via warnings.warn(); logger.warning() pending SNOW-2912513.
            # Then Leak detection emits WARN log
            leak_warnings = [
                w
                for w in captured_warnings
                if issubclass(w.category, FutureWarning) and "not explicitly closed" in str(w.message)
            ]
            assert len(leak_warnings) == 1, f"Expected 1 leak detection FutureWarning, got {len(leak_warnings)}"

            # And Telemetry event is sent with leak information
            pass  # TODO(SNOW-2912513): telemetry not yet implemented
            # And Connection details are included for debugging
            pass  # TODO(SNOW-2912513): connection details are part of telemetry above
