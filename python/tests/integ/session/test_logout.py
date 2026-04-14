"""Integration tests for session logout functionality.

These tests use Wiremock to verify that logout HTTP requests are sent correctly.
"""

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
