"""Python-only EXTERNALBROWSER integration tests for ``open_external_browser_callback``.

The legacy connector opened the SSO URL through ``webbrowser``, which callers
could replace in-process. ``open_external_browser_callback`` is the wrapper's
replacement for that seam and has no counterpart in the shared
``external_browser.feature`` file.
"""

from __future__ import annotations

import json
import socket
import threading
import time

import pytest

from snowflake.connector.errors import Error

from ...wiremock_client import WiremockClient


def _deliver_token_to_loopback(wiremock: WiremockClient, token: str) -> None:
    requests = wiremock.get_requests("/session/authenticator-request.*")
    port = int(json.loads(requests[0]["body"])["data"]["BROWSER_MODE_REDIRECT_PORT"])
    with socket.create_connection(("127.0.0.1", port)) as sock:
        sock.sendall(f"GET /?token={token} HTTP/1.1\r\nHost: localhost\r\n\r\n".encode())
        sock.recv(4096)


@pytest.mark.skip_reference(reason="open_external_browser_callback is a universal driver parameter")
class TestOpenExternalBrowserCallback:
    def test_should_login_when_callback_completes_sso_on_connecting_thread(self, int_test_connection_factory):
        # Given Wiremock returns valid ssoUrl and proofKey for authenticator-request
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/external_browser_authenticator_request.json")

            # And Login endpoint returns success
            wiremock.add_mapping("auth/login_success_external_browser.json")

            # And a callback that completes the SSO redirect itself
            token = "browser_sso_token_callback"
            calls: list[tuple[str, int]] = []

            def open_external_browser_callback(url: str) -> None:
                calls.append((url, threading.get_ident()))
                _deliver_token_to_loopback(wiremock, token)

            # When Trying to Connect with the callback
            connection = int_test_connection_factory(
                authenticator="EXTERNALBROWSER",
                private_key_file=None,
                password=None,
                server_url=wiremock.http_url(),
                open_external_browser_callback=open_external_browser_callback,
            )
            connection.close()

            # Then the callback received the SSO URL once, on the connecting thread
            assert calls == [("https://idp.example.com/sso", threading.get_ident())]

            # And Login request carries the token delivered by the callback
            login_requests = wiremock.get_requests("/session/v1/login-request.*")
            body = json.loads(login_requests[0]["body"])
            assert body["data"]["AUTHENTICATOR"] == "EXTERNALBROWSER"
            assert body["data"]["TOKEN"] == token

    def test_should_fail_without_waiting_for_timeout_when_callback_raises(self, int_test_connection_factory):
        # Given Wiremock returns valid ssoUrl and proofKey for authenticator-request
        with WiremockClient().start() as wiremock:
            wiremock.add_mapping("auth/external_browser_authenticator_request.json")

            def open_external_browser_callback(url: str) -> None:
                raise RuntimeError("idp login form not found")

            # When Trying to Connect with a callback that raises
            started = time.monotonic()
            with pytest.raises(Error) as exc_info:
                int_test_connection_factory(
                    authenticator="EXTERNALBROWSER",
                    private_key_file=None,
                    password=None,
                    server_url=wiremock.http_url(),
                    authentication_timeout=60,
                    open_external_browser_callback=open_external_browser_callback,
                )

            # Then Connection fails with the callback's error before the authentication timeout
            assert "idp login form not found" in str(exc_info.value)
            assert time.monotonic() - started < 30
