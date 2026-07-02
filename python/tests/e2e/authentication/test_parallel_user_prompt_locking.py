"""E2E tests for shared parallel-user-prompt-locking scenarios.

Covers the cross-driver scenarios declared in
``tests/definitions/shared/authentication/parallel_user_prompt_locking.feature``
that are tagged ``@python_e2e`` — the four also implemented by ODBC
(``odbc_tests/tests/integration/authentication/parallel_user_prompt_locking.cpp``)
and the Rust core
(``sf_core/tests/integration/authentication/parallel_user_prompt_locking.rs``).

When a connection pool opens multiple connections concurrently and interactive
authentication is required (external browser, MFA), the driver serializes the
prompts so the user sees only one prompt rather than one per concurrent
connection; the waiting connections reuse the token cached by the first.
Locking engages only when ``client_store_temporary_credential=true`` and
``disable_parallel_user_prompt=true`` (the default).

The prompt lock is process-global in the core, so two connections opened from
the same process share it — no special wiring is needed on the Python side.
"""

import json
import socket
import threading
import time
import uuid

import pytest

from ...wiremock_client import WiremockClient


_AUTHN_REQUEST_PATTERN = "/session/authenticator-request.*"
_LOGIN_REQUEST_PATTERN = "/session/v1/login-request.*"
# Generous deadline for a callback watcher; the real flows complete in well
# under a second, so anything approaching this signals a stuck test.
_WATCHER_TIMEOUT = 15.0
_POLL_INTERVAL = 0.2


@pytest.fixture(autouse=True)
def _noop_browser_opener(monkeypatch):
    """Make the external-browser opener a no-op so the test drives the callback
    itself instead of launching a real browser."""
    monkeypatch.setenv("SF_TEST_BROWSER_OPENER", "noop")


@pytest.fixture(autouse=True)
def _isolated_token_cache(tmp_path, monkeypatch):
    """Point the file-backed token cache at a throwaway directory so cached
    tokens don't leak between tests."""
    monkeypatch.setenv("SF_TEMPORARY_CREDENTIAL_CACHE_DIR", str(tmp_path))


def _count_authenticator_requests(wiremock: WiremockClient) -> int:
    return len(wiremock.get_requests(_AUTHN_REQUEST_PATTERN))


def _simulate_browser_callback_nth(wiremock: WiremockClient, token: str, n: int) -> None:
    """Deliver a fake browser callback to the n-th authenticator-request's listener.

    Each concurrent connection binds its own loopback port and advertises it as
    ``BROWSER_MODE_REDIRECT_PORT``; routing by index hits the right connection.
    """
    deadline = time.time() + _WATCHER_TIMEOUT
    while time.time() < deadline:
        requests = wiremock.get_requests(_AUTHN_REQUEST_PATTERN)
        if len(requests) > n:
            body = json.loads(requests[n]["body"])
            port = int(body["data"]["BROWSER_MODE_REDIRECT_PORT"])
            sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            # Bound every socket op so a watcher can never block forever (and leak a
            # daemon thread into the next test) if the listener accepts but stalls.
            sock.settimeout(5)
            try:
                sock.connect(("127.0.0.1", port))
                http_request = f"GET /?token={token} HTTP/1.1\r\nHost: localhost\r\n\r\n"
                sock.sendall(http_request.encode())
                sock.recv(4096)
            finally:
                sock.close()
            return
        time.sleep(_POLL_INTERVAL)
    raise TimeoutError(f"authenticator-request #{n} never arrived at WireMock")


def _spawn_callback_watcher(wiremock: WiremockClient, token: str, n: int, min_requests: int):
    """Start a daemon thread that delivers ``token`` to listener ``n`` once
    ``min_requests`` authenticator-requests have arrived.

    Returns ``(thread, errors)``; any exception the watcher hits is appended to
    ``errors`` so the main thread can re-raise it (a daemon thread that raises
    would otherwise fail silently). Pass both to :func:`_join_watchers`.
    """
    errors: list[Exception] = []

    def _watch() -> None:
        try:
            deadline = time.time() + _WATCHER_TIMEOUT
            while time.time() < deadline:
                if _count_authenticator_requests(wiremock) >= min_requests:
                    _simulate_browser_callback_nth(wiremock, token, n)
                    return
                time.sleep(_POLL_INTERVAL)
            raise TimeoutError(f"timed out waiting for {min_requests} authenticator-request(s)")
        except Exception as exc:  # surfaced to the main thread via `errors`
            errors.append(exc)

    thread = threading.Thread(target=_watch, daemon=True)
    thread.start()
    return thread, errors


def _join_watchers(*watchers) -> None:
    """Join each ``(thread, errors)`` watcher, failing loudly if it hung or raised."""
    for thread, errors in watchers:
        thread.join(timeout=_WATCHER_TIMEOUT + 5)
        assert not thread.is_alive(), "callback watcher did not finish in time"
        if errors:
            raise errors[0]


def _count_interactive_mfa_logins(wiremock: WiremockClient) -> int:
    """Count interactive MFA logins (AUTHENTICATOR=USERNAME_PASSWORD_MFA without a
    cached TOKEN). Cached-token logins reuse the same authenticator but carry a
    TOKEN field, so they are excluded."""
    count = 0
    for request in wiremock.get_requests(_LOGIN_REQUEST_PATTERN):
        data = json.loads(request["body"]).get("data", {})
        if data.get("AUTHENTICATOR") == "USERNAME_PASSWORD_MFA" and not data.get("TOKEN"):
            count += 1
    return count


def _connect_concurrently(connect_one, connect_two) -> tuple:
    """Run two connection attempts concurrently and return ``(result_one,
    result_two)`` where each result is the connection or the raised exception."""
    results: dict[int, object] = {}

    def _run(index: int, fn) -> None:
        try:
            results[index] = fn()
        except Exception as exc:  # captured for the caller to assert on
            results[index] = exc

    thread_two = threading.Thread(target=lambda: _run(2, connect_two), daemon=True)
    thread_two.start()
    _run(1, connect_one)
    thread_two.join(timeout=60)
    # Fail loudly rather than returning a half-result and leaving a live thread that
    # would run against the shared core/WireMock during a later test in this worker.
    assert not thread_two.is_alive(), "second connection attempt did not finish within 60s"

    return results.get(1), results.get(2)


def _assert_both_connected(result_one, result_two) -> None:
    """Assert both attempts produced a live connection."""
    for result in (result_one, result_two):
        assert not isinstance(result, Exception), f"connection failed: {result!r}"


def _close_results(result_one, result_two) -> None:
    """Close whatever connected so connections never leak, even if a preceding
    assertion or watcher join raised. Call unconditionally from a ``finally``."""
    for result in (result_one, result_two):
        if not isinstance(result, Exception):
            result.close()


def _eb_connect_params(wiremock: WiremockClient, user: str, **overrides) -> dict:
    params = {
        "authenticator": "EXTERNALBROWSER",
        "user": user,
        "private_key_file": None,
        "password": None,
        "authentication_timeout": 30,
        "server_url": wiremock.http_url(),
    }
    params.update(overrides)
    return params


@pytest.mark.skip_reference(
    reason="Reference driver (v4.x) has no prompt-lock serialization and does not support "
    "SF_TEST_BROWSER_OPENER (it attempts real stdin/browser interaction which cannot work in CI)"
)
class TestParallelUserPromptLocking:
    def test_should_show_only_one_external_browser_prompt_when_multiple_connections_authenticate_concurrently(
        self, int_test_connection_factory, wiremock
    ):
        # Given clientStoreTemporaryCredential is enabled and DISABLE_PARALLEL_USER_PROMPT is true
        user = f"eb_lock_{uuid.uuid4().hex}"
        params = _eb_connect_params(wiremock, user, client_store_temporary_credential=True)

        # And Wiremock returns valid ssoUrl and proofKey for authenticator-request
        wiremock.add_mapping("auth/external_browser_authenticator_request.json")

        # And Login endpoint returns success
        wiremock.add_mapping("auth/login_success_external_browser_with_id_token.json")
        wiremock.add_mapping("auth/login_success_cached_id_token.json")

        # When Multiple connections attempt external browser login concurrently
        watcher = _spawn_callback_watcher(wiremock, "browser_sso_token_locked", n=0, min_requests=1)
        result_one, result_two = _connect_concurrently(
            lambda: int_test_connection_factory(**params),
            lambda: int_test_connection_factory(**params),
        )
        try:
            _join_watchers(watcher)

            # Then Only one authenticator-request is sent to the server
            assert _count_authenticator_requests(wiremock) == 1

            # And All connections succeed
            _assert_both_connected(result_one, result_two)
        finally:
            _close_results(result_one, result_two)

    def test_should_show_only_one_mfa_prompt_when_multiple_connections_authenticate_concurrently(
        self, int_test_connection_factory, wiremock
    ):
        # Given clientStoreTemporaryCredential is enabled and DISABLE_PARALLEL_USER_PROMPT is true
        user = f"mfa_lock_{uuid.uuid4().hex}"
        params = {
            "authenticator": "USERNAME_PASSWORD_MFA",
            "user": user,
            "password": "test_password",  # pragma: allowlist secret
            "private_key_file": None,
            "client_store_temporary_credential": True,
            "server_url": wiremock.http_url(),
        }

        # And Wiremock returns successful login with MFA token for the first connection
        wiremock.add_mapping("auth/mfa_login_success_with_mfa_token.json")
        wiremock.add_mapping("auth/mfa_login_success_with_cached_token.json")

        # When Multiple connections attempt username_password_mfa login concurrently
        result_one, result_two = _connect_concurrently(
            lambda: int_test_connection_factory(**params),
            lambda: int_test_connection_factory(**params),
        )
        try:
            # Then Only one interactive MFA login-request is sent to the server
            assert _count_interactive_mfa_logins(wiremock) == 1
            # 1 interactive + 1 cached-token login == 2 total login-requests
            assert len(wiremock.get_requests(_LOGIN_REQUEST_PATTERN)) == 2

            # And All connections succeed using the cached MFA token
            _assert_both_connected(result_one, result_two)
        finally:
            _close_results(result_one, result_two)

    def test_should_show_independent_prompts_when_disable_parallel_user_prompt_is_false(
        self, int_test_connection_factory, wiremock
    ):
        # Given clientStoreTemporaryCredential is enabled and DISABLE_PARALLEL_USER_PROMPT is false
        user = f"eb_nolock_{uuid.uuid4().hex}"
        params = _eb_connect_params(
            wiremock,
            user,
            client_store_temporary_credential=True,
            disable_parallel_user_prompt=False,
        )

        # And Wiremock returns valid ssoUrl and proofKey for each authenticator-request
        wiremock.add_mapping("auth/external_browser_authenticator_request.json")

        # And Login endpoint returns success
        wiremock.add_mapping("auth/login_success_external_browser.json")

        # When Multiple connections attempt external browser login concurrently
        watcher_one = _spawn_callback_watcher(wiremock, "nlock_token_1", n=0, min_requests=1)
        watcher_two = _spawn_callback_watcher(wiremock, "nlock_token_2", n=1, min_requests=2)
        result_one, result_two = _connect_concurrently(
            lambda: int_test_connection_factory(**params),
            lambda: int_test_connection_factory(**params),
        )
        try:
            _join_watchers(watcher_one, watcher_two)

            # Then Each connection sends its own authenticator-request to the server
            assert _count_authenticator_requests(wiremock) >= 2

            # And All connections succeed independently
            _assert_both_connected(result_one, result_two)
        finally:
            _close_results(result_one, result_two)

    def test_should_show_independent_prompts_when_client_store_temporary_credential_is_false(
        self, int_test_connection_factory, wiremock
    ):
        # Given clientStoreTemporaryCredential is disabled and DISABLE_PARALLEL_USER_PROMPT is true
        user = f"eb_nocache_{uuid.uuid4().hex}"
        # client_store_temporary_credential left unset defaults to false, so the prompt
        # lock is not eligible even though disable_parallel_user_prompt defaults to true.
        params = _eb_connect_params(wiremock, user)

        # And Wiremock returns valid ssoUrl and proofKey for each authenticator-request
        wiremock.add_mapping("auth/external_browser_authenticator_request.json")

        # And Login endpoint returns success
        wiremock.add_mapping("auth/login_success_external_browser.json")

        # When Multiple connections attempt external browser login concurrently
        watcher_one = _spawn_callback_watcher(wiremock, "nocache_token_1", n=0, min_requests=1)
        watcher_two = _spawn_callback_watcher(wiremock, "nocache_token_2", n=1, min_requests=2)
        result_one, result_two = _connect_concurrently(
            lambda: int_test_connection_factory(**params),
            lambda: int_test_connection_factory(**params),
        )
        try:
            _join_watchers(watcher_one, watcher_two)

            # Then Each connection sends its own authenticator-request to the server
            assert _count_authenticator_requests(wiremock) >= 2

            # And All connections succeed independently
            _assert_both_connected(result_one, result_two)
        finally:
            _close_results(result_one, result_two)
