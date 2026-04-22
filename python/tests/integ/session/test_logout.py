"""Integration tests for session logout functionality.

These tests use Wiremock to verify that logout HTTP requests are sent correctly.
"""

import warnings

from unittest.mock import patch

import pytest

from tests.wiremock_client import WiremockClient


class TestLogoutWithWiremock:
    """Integration tests for logout using Wiremock to verify HTTP requests."""

    def test_should_send_logout_request_with_correct_method_and_endpoint(self, int_test_connection_factory):
        """Verify logout sends POST to /session?delete=true."""
        with WiremockClient().start() as wiremock:
            # Setup: auth + logout mappings
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("session/logout_success.json")

            # Given: Connected client
            connection = int_test_connection_factory(server_url=wiremock.http_url())

            # When: Connection is closed
            connection.close()

            # Then: Verify logout request was sent
            logout_requests = wiremock.get_logout_requests()

            assert len(logout_requests) >= 1, "Expected at least one logout request"

            logout_req = logout_requests[0]["request"]
            assert logout_req["method"] == "POST", "Logout should use POST method"
            assert "delete=true" in logout_req["url"], "Should have delete=true param"

    def test_should_send_logout_with_authorization_header(self, int_test_connection_factory):
        """Verify logout request includes Authorization header with session token."""
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("session/logout_success.json")

            connection = int_test_connection_factory(server_url=wiremock.http_url())
            connection.close()

            # Verify request headers
            logout_requests = wiremock.get_logout_requests()

            assert len(logout_requests) >= 1, "Expected logout request"

            headers = logout_requests[0]["request"].get("headers", {})
            auth_header = headers.get("Authorization") or headers.get("authorization")
            assert auth_header is not None, "Should have Authorization header"
            assert auth_header.startswith("Snowflake Token="), "Should use Snowflake Token auth"

    def test_should_send_logout_with_correct_content_type(self, int_test_connection_factory):
        """Verify logout request has Content-Type: application/json."""
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("session/logout_success.json")

            connection = int_test_connection_factory(server_url=wiremock.http_url())
            connection.close()

            # Verify Content-Type header
            logout_requests = wiremock.get_logout_requests()

            assert len(logout_requests) >= 1, "Expected logout request"

            headers = logout_requests[0]["request"].get("headers", {})
            content_type = headers.get("Content-Type") or headers.get("content-type")
            assert content_type is not None, "Should have Content-Type header"
            assert "application/json" in content_type, "Content-Type should be JSON"

    def test_should_not_send_logout_when_server_session_keep_alive_is_true(self, int_test_connection_factory):
        """Verify no logout request when server_session_keep_alive=True."""
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            # Note: logout mapping not added - request should not be made

            connection = int_test_connection_factory(server_url=wiremock.http_url(), server_session_keep_alive=True)
            connection.close()

            # Verify NO logout request was sent
            logout_requests = wiremock.get_logout_requests()

            assert len(logout_requests) == 0, (
                f"Should NOT send logout when keep_alive=True, but got {len(logout_requests)} requests"
            )

    def test_should_send_logout_when_server_session_keep_alive_is_false(self, int_test_connection_factory):
        """Verify logout IS sent when server_session_keep_alive=False."""
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("session/logout_success.json")

            connection = int_test_connection_factory(server_url=wiremock.http_url(), server_session_keep_alive=False)
            connection.close()

            # Verify logout request WAS sent
            logout_requests = wiremock.get_logout_requests()

            assert len(logout_requests) >= 1, "Should send logout when keep_alive=False"

    @pytest.mark.skip_reference  # Old connector (v4.3.0) doesn't retry logout on 503
    def test_should_retry_logout_on_503_error(self, int_test_connection_factory):
        """Verify logout retries on 503 Service Unavailable."""
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("session/logout_503_then_success.json")

            connection = int_test_connection_factory(server_url=wiremock.http_url())
            connection.close()

            # Verify multiple logout attempts (retry)
            logout_requests = wiremock.get_logout_requests()

            assert len(logout_requests) >= 2, f"Should retry on 503, got {len(logout_requests)} attempts"


class TestLogoutIdempotency:
    """Tests for logout idempotency."""

    def test_should_only_send_one_logout_when_close_called_multiple_times(self, int_test_connection_factory):
        """Verify only one logout request is sent for multiple close() calls."""
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("session/logout_success.json")

            connection = int_test_connection_factory(server_url=wiremock.http_url())

            # Call close multiple times
            connection.close()
            connection.close()
            connection.close()

            # Verify only ONE logout request
            logout_requests = wiremock.get_logout_requests()

            assert len(logout_requests) == 1, f"Should send exactly 1 logout, got {len(logout_requests)}"


class TestLogoutConfigPassing:
    """Verify Python wrapper correctly passes logout config to Core."""

    @pytest.mark.skip_reference(reason="core_mock fixture imports _internal protobuf types")
    def test_should_have_enable_server_session_keep_alive_auto_detection_default_to_true(self, core_mock):
        """Verify enable_server_session_keep_alive_auto_detection defaults to True.

        Default True is required for backward compat (SNOW-2314152): the old Python
        driver always checked _async_sfqids before logout. Without this default,
        Core receives enable_server_session_keep_alive_auto_detection=None → always logout → kills async queries.
        A FutureWarning is emitted because this default will change in a future version.
        """
        # Given Snowflake Python client is created without enable_server_session_keep_alive_auto_detection parameter
        from snowflake.connector.connection import Connection

        with pytest.warns(FutureWarning) as warning_list:
            Connection(user="test", account="test")

        # When Connection configuration is checked
        options = core_mock.get_options_sent()

        # Then enable_server_session_keep_alive_auto_detection defaults to true
        assert options.get("enable_server_session_keep_alive_auto_detection") is True, (
            "Default True must flow through to Core so registry check is performed"
        )

        # And Auto-detection is enabled by default
        assert "server_session_keep_alive" not in options, (
            "Default server_session_keep_alive=None should not be sent to Core"
        )

        # And Deprecation warning is emitted about auto_detection default changing
        assert any(
            "enable_server_session_keep_alive_auto_detection was not set" in str(w.message) for w in warning_list
        ), f"Expected FutureWarning about auto_detection default, got: {[str(w.message) for w in warning_list]}"

    @pytest.mark.skip_reference(reason="core_mock fixture imports _internal protobuf types")
    def test_should_not_emit_auto_detection_deprecation_warning_when_explicitly_set_to_true(self, core_mock):
        """Verify no FutureWarning when user explicitly passes auto_detection=True.

        Explicit True means the user has made a conscious choice — no warning needed.
        """
        from snowflake.connector.connection import Connection

        # Given Snowflake Python client is created with enable_server_session_keep_alive_auto_detection set to true
        with warnings.catch_warnings(record=True) as captured_warnings:
            warnings.simplefilter("always")
            Connection(
                user="test",
                account="test",
                enable_server_session_keep_alive_auto_detection=True,
            )

        # When Connection configuration is checked
        options = core_mock.get_options_sent()

        # Then enable_server_session_keep_alive_auto_detection is true
        assert options.get("enable_server_session_keep_alive_auto_detection") is True

        # And No deprecation warning is emitted about auto_detection default
        auto_detection_warnings = [
            w for w in captured_warnings if issubclass(w.category, FutureWarning) and "auto_detection" in str(w.message)
        ]
        assert len(auto_detection_warnings) == 0, (
            f"No FutureWarning expected when auto_detection is explicitly True, "
            f"got: {[str(w.message) for w in auto_detection_warnings]}"
        )


class TestAutoCleanupConfig:
    """Verify auto_cleanup defaults and atexit handler registration."""

    @pytest.mark.skip_reference(reason="core_mock fixture imports _internal protobuf types")
    def test_should_have_auto_cleanup_enabled_by_default(self, core_mock):
        """Verify auto_cleanup defaults to True and atexit handler is registered."""
        from snowflake.connector.connection import Connection

        with patch("snowflake.connector.connection.atexit") as mock_atexit:
            # Given Snowflake Python client is created with default parameters
            conn = Connection(user="test", account="test")

            # When Connection is initialized
            auto_cleanup_value = conn.auto_cleanup

            # Then auto_cleanup defaults to true
            assert auto_cleanup_value is True, (
                f"auto_cleanup should default to True for backward compat, got {auto_cleanup_value}"
            )

            # And atexit handler is registered at connection init
            mock_atexit.register.assert_called_once_with(conn._close_at_process_exit)
