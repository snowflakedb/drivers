import json
import os
import socket
import threading
import time

import pytest

from ...wiremock_client import WiremockClient


os.environ["SF_TEST_BROWSER_OPENER"] = "noop"

# TODO(SNOW-2881750): Add e2e tests that exercise the full external browser flow against a real
# browser using headless Chrome in Docker. These integ tests only simulate the callback via
# raw sockets; real browser tests would validate the full redirect/SSO UX.


def _simulate_browser_callback(wiremock: WiremockClient, token: str, timeout: float = 10.0) -> None:
    """Poll WireMock for the authenticator-request, extract the redirect port, send a fake token."""
    deadline = time.time() + timeout
    while time.time() < deadline:
        requests = wiremock.get_requests("/session/authenticator-request.*")
        if requests:
            body = json.loads(requests[0]["body"])
            port = int(body["data"]["BROWSER_MODE_REDIRECT_PORT"])
            sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            try:
                sock.connect(("127.0.0.1", port))
                http_request = f"GET /?token={token} HTTP/1.1\r\nHost: localhost\r\n\r\n"
                sock.sendall(http_request.encode())
                sock.recv(4096)
            finally:
                sock.close()
            return
        time.sleep(0.2)
    raise TimeoutError("authenticator-request never arrived at WireMock")


@pytest.mark.skip_reference(
    reason="Reference driver (v4.3.0) does not support SF_TEST_BROWSER_OPENER and attempts "
    "real stdin/browser interaction which cannot work in CI"
)
class TestExternalBrowserAuthentication:
    def test_should_login_with_external_browser_using_simulated_callback(self, int_test_connection_factory):
        # Given Wiremock returns valid ssoUrl and proofKey for authenticator-request
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/external_browser_authenticator_request.json")

            # And Login endpoint returns success
            wiremock.add_mapping("auth/login_success_external_browser.json")

            # When Trying to Connect with simulated browser callback delivering a token
            token = "browser_sso_token_12345"
            callback_thread = threading.Thread(
                target=_simulate_browser_callback,
                args=(wiremock, token),
                daemon=True,
            )
            callback_thread.start()

            connection = int_test_connection_factory(
                authenticator="EXTERNALBROWSER",
                private_key_file=None,
                password=None,
                server_url=wiremock.http_url(),
            )

            # Then Login is successful
            callback_thread.join(timeout=15)
            connection.close()

            # And Login request contains EXTERNALBROWSER authenticator, token, proof key, and login name
            login_requests = wiremock.get_requests("/session/v1/login-request.*")
            assert len(login_requests) >= 1
            body = json.loads(login_requests[0]["body"])
            assert body["data"]["AUTHENTICATOR"] == "EXTERNALBROWSER"
            assert body["data"]["TOKEN"] == token
            assert body["data"]["PROOF_KEY"] == "mock_proof_key_abc123"
            assert body["data"]["LOGIN_NAME"] == "test_user"

    def test_should_fail_when_authenticator_request_returns_forbidden(self, int_test_connection_factory):
        # Given Wiremock returns HTTP 403 for authenticator-request
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/external_browser_authenticator_request_forbidden.json")

            # When Trying to Connect
            with pytest.raises(Exception) as exc_info:
                int_test_connection_factory(
                    authenticator="EXTERNALBROWSER",
                    private_key_file=None,
                    password=None,
                    server_url=wiremock.http_url(),
                )

            # Then Connection fails with authenticator error
            error_msg = str(exc_info.value)
            assert "403" in error_msg or "Forbidden" in error_msg or "authenticator" in error_msg.lower()

    def test_should_fail_when_authenticator_request_returns_logical_failure(self, int_test_connection_factory):
        # Given Wiremock returns success false for authenticator-request
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/external_browser_authenticator_request_logical_failure.json")

            # When Trying to Connect
            with pytest.raises(Exception) as exc_info:
                int_test_connection_factory(
                    authenticator="EXTERNALBROWSER",
                    private_key_file=None,
                    password=None,
                    server_url=wiremock.http_url(),
                )

            # Then Connection fails with authenticator error
            error_msg = str(exc_info.value)
            assert "not enabled" in error_msg or "authenticator" in error_msg.lower()

    def test_should_fail_with_timeout_when_no_browser_callback_arrives(self, int_test_connection_factory):
        # Given Wiremock returns valid ssoUrl and proofKey for authenticator-request
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/external_browser_authenticator_request.json")

            # And Authentication timeout is set to 2 seconds
            authentication_timeout = 2

            # When Trying to Connect without any browser callback
            with pytest.raises(Exception) as exc_info:
                int_test_connection_factory(
                    authenticator="EXTERNALBROWSER",
                    private_key_file=None,
                    password=None,
                    server_url=wiremock.http_url(),
                    authentication_timeout=authentication_timeout,
                )

            # Then Connection fails with timeout or browser error
            error_msg = str(exc_info.value)
            assert "timeout" in error_msg.lower() or "browser" in error_msg.lower()

    def test_should_fail_when_login_request_is_rejected_after_browser_callback(self, int_test_connection_factory):
        # Given Wiremock returns valid ssoUrl and proofKey for authenticator-request
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/external_browser_authenticator_request.json")

            # And Login endpoint returns failure
            wiremock.add_mapping("auth/login_failure_external_browser.json")

            # When Trying to Connect with simulated browser callback delivering a token
            token = "browser_sso_token_rejected"
            callback_thread = threading.Thread(
                target=_simulate_browser_callback,
                args=(wiremock, token),
                daemon=True,
            )
            callback_thread.start()

            with pytest.raises(Exception) as exc_info:
                int_test_connection_factory(
                    authenticator="EXTERNALBROWSER",
                    private_key_file=None,
                    password=None,
                    server_url=wiremock.http_url(),
                )

            # Then Connection fails with login error
            callback_thread.join(timeout=15)
            error_msg = str(exc_info.value)
            assert "Invalid credentials" in error_msg or "login" in error_msg.lower()
