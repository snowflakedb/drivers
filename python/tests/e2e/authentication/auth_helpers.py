import socket
import subprocess
import threading
import time

from collections.abc import Callable

import requests

from requests.auth import HTTPBasicAuth

from snowflake.connector.errors import DatabaseError


# These Node.js scripts and the Chromium remote-debugging port are provided by the
# snowdrivers-test-external-browser-universal-driver Docker image (see tests/auth/). They
# do not exist outside that container, which is why these tests are gated behind the
# `requires_browser` marker.
PROVIDE_CREDENTIALS_SCRIPT = "/externalbrowser/provideBrowserCredentials.js"
CLEAN_BROWSER_SCRIPT = "/externalbrowser/cleanBrowserProcesses.js"
TOTP_GENERATOR_SCRIPT = "/externalbrowser/totpGenerator.js"
CHROMIUM_DEBUG_PORT = 9222

# How long connect_with_browser_automation() waits for the connect leg before giving up.
_CONNECT_JOIN_TIMEOUT_SECONDS = 90

# Tests that drive an interactive OAuth/browser connect through
# connect_with_browser_automation() should pass this as `authentication_timeout` in their
# connect_params. It must stay below _CONNECT_JOIN_TIMEOUT_SECONDS: the driver's own
# `authentication_timeout` (120s by default - see connection_config.py) bounds how long the
# connect leg keeps its OAuth loopback listener open waiting for the IdP redirect. If that
# default outlives our join timeout, a failed browser leg (e.g. rejected credentials) leaves
# the connect leg's daemon thread alive - and its OS-level loopback port bound - well past
# the point where this test has already failed and returned control to pytest. The next
# browser-OAuth test in the same worker then fails to bind that same port
# ("Address already in use"), for a reason that has nothing to do with its own test body.
RECOMMENDED_AUTHENTICATION_TIMEOUT_SECONDS = 75


def verify_simple_query_execution(connection):
    """Verify that a simple query can be executed successfully."""
    with connection.cursor() as cursor:
        cursor.execute("SELECT 1")
        result = cursor.fetchone()
        assert result is not None
        assert result[0] == 1


def verify_login_error(exception, keywords):
    """Verify that an exception is a DatabaseError from an authentication failure.

    Asserts that every keyword in *keywords* appears in the error message
    (case-insensitive).
    """
    assert exception is not None
    assert str(exception).strip() != "", "Login error message should not be empty"

    assert isinstance(exception.value, DatabaseError), f"Expected DatabaseError, got: {type(exception.value)}"

    error_msg = str(exception.value).lower()
    for kw in keywords:
        assert kw in error_msg, f"Expected error to contain {kw!r}, got: {exception.value}"


def retrieve_oauth_access_token(
    *,
    token_url: str,
    client_id: str,
    client_secret: str,
    user: str,
    password: str,
    role: str,
) -> str:
    """Mint a fresh OAuth access token via the IdP's Resource Owner Password grant."""
    response = requests.post(
        url=token_url,
        data={
            "username": user,
            "password": password,
            "grant_type": "password",
            "scope": f"session:role:{role.lower()}",
        },
        headers={"Content-Type": "application/x-www-form-urlencoded;charset=UTF-8"},
        auth=HTTPBasicAuth(client_id, client_secret),
    )
    response.raise_for_status()
    return response.json()["access_token"]


def clean_browser_processes():
    """Kill any lingering Chromium processes from previous test runs."""
    subprocess.run(["node", CLEAN_BROWSER_SCRIPT], timeout=15, capture_output=True)


def is_totp_retryable_error(exc: Exception) -> bool:
    """Return True when the error indicates an expired/invalid TOTP code."""
    msg = str(exc)
    return "TOTP Invalid" in msg or "invalid passcode" in msg.lower()


TOTP_STEP_SECONDS = 30

# Passcodes already sent to Snowflake in this pytest process. Snowflake rejects
# TOTP replay within a time window, so serial MFA tests must not reuse codes.
_USED_TOTP_CODES: set[str] = set()


def get_totp_codes(seed: str) -> list[str]:
    """Generate TOTP passcodes via the headless browser container helper."""
    result = subprocess.run(
        ["node", TOTP_GENERATOR_SCRIPT, seed],
        timeout=40,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        stderr = result.stderr.strip()
        raise RuntimeError(f"totpGenerator.js failed (rc={result.returncode}): {stderr}")

    codes = result.stdout.strip().split()
    if not codes:
        raise RuntimeError("totpGenerator.js produced no TOTP codes")
    return codes


def _fresh_totp_codes(seed: str) -> list[str]:
    return [code for code in get_totp_codes(seed) if code not in _USED_TOTP_CODES]


def _sleep_to_next_totp_window() -> None:
    """Block until the next 30s TOTP window (plus a 1s buffer)."""
    wait = TOTP_STEP_SECONDS - (time.time() % TOTP_STEP_SECONDS)
    if wait > 0:
        wait += 1.0
        print(f"[mfa-helper] Waiting {wait:.0f}s for next TOTP window")
        time.sleep(wait)


def acquire_totp_passcode(seed: str, *, max_windows: int = 3) -> str:
    """Return one unused TOTP passcode, advancing to the next window if needed."""
    for window_idx in range(max_windows):
        fresh = _fresh_totp_codes(seed)
        if fresh:
            passcode = fresh[0]
            _USED_TOTP_CODES.add(passcode)
            return passcode
        if window_idx < max_windows - 1:
            print(f"[mfa-helper] No unused codes in window {window_idx + 1}, advancing")
            _sleep_to_next_totp_window()
    raise RuntimeError(f"No unused TOTP passcodes available after {max_windows} windows")


def connect_with_totp_retry(
    connection_factory: Callable,
    totp_seed: str,
    *,
    passcode_in_password: bool = False,
    max_windows: int = 3,
    **connect_kwargs,
):
    """Connect using USERNAME_PASSWORD_MFA with TOTP dedup across tests.

    Snowflake rejects reused TOTP codes within a time window. Codes already
    consumed in this pytest process are skipped; when exhausted, waits for the
    next 30s window before regenerating (totpGenerator yields 2-3 codes per window).
    """
    last_error = None
    base_password = connect_kwargs.get("password")

    for window_idx in range(max_windows):
        fresh_codes = _fresh_totp_codes(totp_seed)
        if not fresh_codes:
            if window_idx >= max_windows - 1:
                break
            print(f"[mfa-helper] No unused codes in window {window_idx + 1}, advancing")
            _sleep_to_next_totp_window()
            continue

        for code_idx, passcode in enumerate(fresh_codes):
            _USED_TOTP_CODES.add(passcode)
            kwargs = dict(connect_kwargs)
            if passcode_in_password:
                kwargs["password"] = base_password + passcode
                kwargs["passcode_in_password"] = True
            else:
                kwargs["passcode"] = passcode

            try:
                return connection_factory(**kwargs)
            except Exception as e:
                last_error = e
                if is_totp_retryable_error(e):
                    print(
                        f"[mfa-helper] TOTP code {code_idx + 1}/{len(fresh_codes)} "
                        f"in window {window_idx + 1} failed, retrying"
                    )
                    continue
                raise

        if window_idx < max_windows - 1:
            _sleep_to_next_totp_window()

    raise AssertionError(
        f"Failed to connect after {max_windows} TOTP windows. Last error: {last_error}"
    ) from last_error


def provide_browser_credentials(scenario: str, login: str, password: str, totp_seed: str | None = None):
    """Run the Node.js browser automation script that fills IdP credentials.

    ``totp_seed`` is forwarded so the script can fill Snowflake's authenticator-app
    MFA verification step, if presented; omit it for accounts that don't require MFA.
    """
    cmd = ["node", PROVIDE_CREDENTIALS_SCRIPT, scenario, login, password]
    if totp_seed:
        cmd.append(totp_seed)
    result = subprocess.run(
        cmd,
        timeout=90,
        text=True,
    )
    if result.returncode != 0:
        raise RuntimeError(f"provideBrowserCredentials.js failed (rc={result.returncode})")


def _wait_for_chromium(timeout: float = 60.0, poll_interval: float = 1.0):
    """Block until Chromium's remote debugging port is accepting connections."""
    deadline = time.time() + timeout
    start = time.time()
    while time.time() < deadline:
        try:
            with socket.create_connection(("localhost", CHROMIUM_DEBUG_PORT), timeout=1):
                elapsed = time.time() - start
                print(f"[browser-helper] Chromium port {CHROMIUM_DEBUG_PORT} ready after {elapsed:.1f}s")
                return True
        except OSError:
            time.sleep(poll_interval)
    print(f"[browser-helper] Chromium port {CHROMIUM_DEBUG_PORT} NOT ready after {timeout}s")
    return False


def connect_with_browser_automation(
    connect_fn: Callable,
    scenario: str,
    login: str,
    password: str,
    totp_seed: str | None = None,
):
    """Run connect_fn and browser automation concurrently.

    The connect leg is authoritative: returns the connection on success, or
    re-raises the driver's exception unchanged so negative tests can assert on it.
    A browser-leg failure is only surfaced when the connect leg has no result.
    ``totp_seed`` is forwarded to the browser leg for accounts that require MFA.
    """
    connect_result = {}
    browser_error_holder = []

    def _connect():
        try:
            connect_result["connection"] = connect_fn()
        except Exception as e:
            connect_result["error"] = e

    def _browser():
        try:
            if not _wait_for_chromium():
                raise RuntimeError(f"Chromium did not start on port {CHROMIUM_DEBUG_PORT} within timeout")
            provide_browser_credentials(scenario, login, password, totp_seed)
        except Exception as e:
            browser_error_holder.append(e)

    # daemon=True so a hung thread can never block interpreter shutdown / leak across
    # tests; the explicit joins below are the real synchronization point.
    t_connect = threading.Thread(target=_connect, daemon=True)
    t_browser = threading.Thread(target=_browser, daemon=True)

    t_connect.start()
    t_browser.start()

    t_browser.join(timeout=90)
    t_connect.join(timeout=_CONNECT_JOIN_TIMEOUT_SECONDS)

    if t_connect.is_alive():
        raise TimeoutError(f"Connect thread did not finish within {_CONNECT_JOIN_TIMEOUT_SECONDS}s")

    if "error" in connect_result:
        raise connect_result["error"]
    if "connection" in connect_result:
        return connect_result["connection"]

    # Connect leg produced nothing: the browser leg (or its absence) explains why.
    if t_browser.is_alive():
        raise TimeoutError("Browser automation thread did not finish within 90s")
    if browser_error_holder:
        raise RuntimeError(f"Browser automation failed: {browser_error_holder}")
    raise AssertionError("Connection was not established")
