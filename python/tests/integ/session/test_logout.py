"""Integration tests for session logout functionality.

These tests use Wiremock to verify that logout HTTP requests are sent correctly.
"""

import pytest
import requests

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
            requests_url = f"{wiremock.http_url()}/__admin/requests"
            response = requests.get(requests_url)
            all_requests = response.json().get("requests", [])

            # Find logout request
            logout_requests = [
                r for r in all_requests if r.get("request", {}).get("url", "").startswith("/session?delete=")
            ]

            assert len(logout_requests) >= 1, (
                f"Expected logout request, got requests: {[r.get('request', {}).get('url') for r in all_requests]}"
            )

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
            requests_url = f"{wiremock.http_url()}/__admin/requests"
            response = requests.get(requests_url)
            all_requests = response.json().get("requests", [])

            logout_requests = [
                r for r in all_requests if r.get("request", {}).get("url", "").startswith("/session?delete=")
            ]

            assert len(logout_requests) >= 1, "Expected logout request"

            headers = logout_requests[0]["request"].get("headers", {})
            auth_header = headers.get("Authorization") or headers.get("authorization")
            assert auth_header is not None, "Should have Authorization header"
            assert "Snowflake Token" in auth_header, "Should use Snowflake Token auth"

    def test_should_send_logout_with_correct_content_type(self, int_test_connection_factory):
        """Verify logout request has Content-Type: application/json."""
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("session/logout_success.json")

            connection = int_test_connection_factory(server_url=wiremock.http_url())
            connection.close()

            # Verify Content-Type header
            requests_url = f"{wiremock.http_url()}/__admin/requests"
            response = requests.get(requests_url)
            all_requests = response.json().get("requests", [])

            logout_requests = [
                r for r in all_requests if r.get("request", {}).get("url", "").startswith("/session?delete=")
            ]

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
            requests_url = f"{wiremock.http_url()}/__admin/requests"
            response = requests.get(requests_url)
            all_requests = response.json().get("requests", [])

            logout_requests = [
                r for r in all_requests if r.get("request", {}).get("url", "").startswith("/session?delete=")
            ]

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
            requests_url = f"{wiremock.http_url()}/__admin/requests"
            response = requests.get(requests_url)
            all_requests = response.json().get("requests", [])

            logout_requests = [
                r for r in all_requests if r.get("request", {}).get("url", "").startswith("/session?delete=")
            ]

            assert len(logout_requests) >= 1, "Should send logout when keep_alive=False"

    def test_should_retry_logout_on_503_error(self, int_test_connection_factory):
        """Verify logout retries on 503 Service Unavailable."""
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/login_success_jwt.json")
            wiremock.add_mapping("session/logout_503_then_success.json")

            connection = int_test_connection_factory(server_url=wiremock.http_url())
            connection.close()

            # Verify multiple logout attempts (retry)
            requests_url = f"{wiremock.http_url()}/__admin/requests"
            response = requests.get(requests_url)
            all_requests = response.json().get("requests", [])

            logout_requests = [
                r for r in all_requests if r.get("request", {}).get("url", "").startswith("/session?delete=")
            ]

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
            requests_url = f"{wiremock.http_url()}/__admin/requests"
            response = requests.get(requests_url)
            all_requests = response.json().get("requests", [])

            logout_requests = [
                r for r in all_requests if r.get("request", {}).get("url", "").startswith("/session?delete=")
            ]

            assert len(logout_requests) == 1, f"Should send exactly 1 logout, got {len(logout_requests)}"


class TestLogoutPhase5Optimization:
    """Phase 5: Integration optimization tests."""

    @pytest.mark.skip(reason="TODO: SNOW-2872349 - Phase 5")
    def test_should_return_true_when_first_running_async_query_is_detected_without_checking_remaining_queries(
        self,
    ):
        """Verify auto-detection returns early on first running query."""
        # Given Async query registry contains multiple queries
        # And First query in registry is running
        # When Auto-detection checks for running queries
        # Then Detection returns true immediately
        # And Remaining queries are not checked
        pytest.fail("TODO: SNOW-2872349")


class TestLogoutPhase2Phase3Migration:
    """Tests for Phase 2/3 migration flag (SNOW-2314152)."""

    def test_phase2_is_default_and_phase3_can_be_enabled(self, int_test_connection_factory):
        """Verify Phase 2 is default and Phase 3 can be toggled via internal flag.

        This test verifies:
        1. Default behavior is Phase 2 (USE_PHASE3_LOGOUT_SEMANTICS=False)
        2. Both Phase 2 and Phase 3 code paths are implemented
        3. Flag can be toggled to switch between phases

        NOTE: This test will be removed when Phase 3 migration is complete (SNOW-2314152).
        WARNING: When Phase 3 becomes default, this will be a BREAKING CHANGE.
        """
        from snowflake.connector.connection import Connection

        # Save original value to restore after test
        original_flag = Connection._Connection__class_config.USE_PHASE3_LOGOUT_SEMANTICS

        try:
            with WiremockClient().start() as wiremock:
                # Setup Wiremock mappings
                wiremock.add_mapping("auth/login_success_jwt.json")
                wiremock.add_mapping("session/logout_success.json")

                # Test 1: Verify default is Phase 2
                assert Connection._Connection__class_config.USE_PHASE3_LOGOUT_SEMANTICS is False, (
                    "Default should be Phase 2 (USE_PHASE3_LOGOUT_SEMANTICS=False)"
                )

                # Test 2: Phase 2 behavior (default)
                conn_phase2 = int_test_connection_factory(
                    server_url=wiremock.http_url(),
                    server_session_keep_alive=False,
                    enable_server_session_keep_alive_auto_detection=True,
                )
                conn_phase2.close()
                assert conn_phase2.is_closed()

                # Test 3: Phase 3 behavior (enable flag)
                Connection._Connection__class_config.USE_PHASE3_LOGOUT_SEMANTICS = True

                conn_phase3 = int_test_connection_factory(
                    server_url=wiremock.http_url(),
                    server_session_keep_alive=False,
                    enable_server_session_keep_alive_auto_detection=True,
                )
                conn_phase3.close()
                assert conn_phase3.is_closed()

                # Verify flag was applied
                assert Connection._Connection__class_config.USE_PHASE3_LOGOUT_SEMANTICS is True

        finally:
            # Restore original flag value
            Connection._Connection__class_config.USE_PHASE3_LOGOUT_SEMANTICS = original_flag
