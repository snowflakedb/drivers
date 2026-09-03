import os
import socket
import subprocess
import threading
import time

from collections.abc import Callable
from contextlib import contextmanager
from pathlib import Path

import pytest
import requests

from requests.auth import HTTPBasicAuth

from snowflake.connector.errors import DatabaseError


try:
    import fcntl
except ImportError:  # Windows local runs; auth-browser CI is Linux
    fcntl = None


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


def is_mfa_lockout_error(exc: Exception) -> bool:
    msg = str(exc)
    return "394512" in msg or "too many failed mfa" in msg.lower()


TOTP_STEP_SECONDS = 30
# Matches totpGenerator.js MIN_VALIDITY_SECONDS. Image :4 does not wait
# internally; callers must skip a soon-to-expire current window themselves.
MIN_TOTP_VALIDITY_SECONDS = 8

# Passcodes already sent to Snowflake in this pytest process. Snowflake rejects
# TOTP replay within a time window, so serial MFA tests must not reuse codes.
_USED_TOTP_CODES: set[str] = set()
# Circuit breaker for the shared MFA Jenkins user.
# - 394512: mark + skip this test (infra lockout). Later tests skip.
# - Retry-budget exhaustion after >=1 Snowflake submit: mark + fail this
#   test (keep CI red). Later tests skip so they do not spend 3 more attempts.
# - Zero submits: fail without marking (Snowflake was not hit).
_MFA_CONNECT_EXHAUSTED = False
_CACHED_TOTP_WINDOW: int | None = None
_CACHED_TOTP_SEED: str | None = None
_CACHED_TOTP_CODE: str | None = None


def _totp_window_id(now: float | None = None) -> int:
    return int((time.time() if now is None else now) // TOTP_STEP_SECONDS)


def _seconds_until_next_window() -> float:
    remaining = TOTP_STEP_SECONDS - (time.time() % TOTP_STEP_SECONDS)
    return remaining if remaining > 0 else 0.0


def _wait_if_near_totp_boundary() -> None:
    remaining = _seconds_until_next_window()
    if remaining < MIN_TOTP_VALIDITY_SECONDS:
        time.sleep(remaining + 1.0)


def _parse_current_totp_code(stdout: str) -> str:
    tokens = [token for token in stdout.split() if token.isdigit() and len(token) == 6]
    if len(tokens) == 1:
        return tokens[0]
    if len(tokens) in (2, 3):
        # Image :4: past/current/future or current/future. Second-to-last is current.
        return tokens[-2]
    raise RuntimeError(f"totpGenerator.js produced {len(tokens)} 6-digit tokens; expected 1 or 2–3")


def get_current_totp_code(seed: str) -> str:
    """Generate the currently valid TOTP code via the browser helper."""
    global _CACHED_TOTP_WINDOW, _CACHED_TOTP_SEED, _CACHED_TOTP_CODE
    _wait_if_near_totp_boundary()
    window = _totp_window_id()
    if _CACHED_TOTP_CODE is not None and _CACHED_TOTP_WINDOW == window and _CACHED_TOTP_SEED == seed:
        return _CACHED_TOTP_CODE
    result = subprocess.run(
        ["node", TOTP_GENERATOR_SCRIPT, seed],
        timeout=40,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        stderr = result.stderr.strip()
        raise RuntimeError(f"totpGenerator.js failed (rc={result.returncode}): {stderr}")

    code = _parse_current_totp_code(result.stdout)
    # Window after generate: Node may have waited into the next step.
    _CACHED_TOTP_WINDOW = _totp_window_id()
    _CACHED_TOTP_SEED = seed
    _CACHED_TOTP_CODE = code
    return code


def _mfa_build_tag() -> str:
    return os.environ.get("BUILD_TAG") or "local"


def _mfa_state_dir() -> Path:
    root = os.environ.get("WORKSPACE_ROOT") or os.environ.get("WORKSPACE") or os.environ.get("TMPDIR", "/tmp")
    return Path(root) / ".ud-mfa-totp-state" / _mfa_build_tag()


def _used_codes_path() -> Path:
    return _mfa_state_dir() / "ud-mfa-used-totp-codes"


def _exhausted_flag_path() -> Path:
    return _mfa_state_dir() / "ud-mfa-connect-exhausted"


def _shared_mfa_exhausted() -> bool:
    return _MFA_CONNECT_EXHAUSTED or _exhausted_flag_path().exists()


def _mark_shared_mfa_exhausted() -> None:
    global _MFA_CONNECT_EXHAUSTED
    _MFA_CONNECT_EXHAUSTED = True
    try:
        _mfa_state_dir().mkdir(parents=True, exist_ok=True)
        _exhausted_flag_path().write_text("1\n")
    except OSError:
        pass


@contextmanager
def _used_codes_lock():
    path = _used_codes_path()
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a+") as handle:
        if fcntl is not None:
            fcntl.flock(handle.fileno(), fcntl.LOCK_EX)
        try:
            yield handle
        finally:
            if fcntl is not None:
                fcntl.flock(handle.fileno(), fcntl.LOCK_UN)


def _claim_totp_code(code: str) -> bool:
    if code in _USED_TOTP_CODES:
        return False
    try:
        with _used_codes_lock() as handle:
            handle.seek(0)
            if code in {line.strip() for line in handle.read().splitlines()}:
                _USED_TOTP_CODES.add(code)
                return False
            handle.write(code + "\n")
            handle.flush()
    except OSError:
        return False
    _USED_TOTP_CODES.add(code)
    return True


def _fresh_totp_code(seed: str) -> str | None:
    code = get_current_totp_code(seed)
    return code if _claim_totp_code(code) else None


def _sleep_to_next_totp_window() -> None:
    """Block until the next 30s TOTP window (plus a 1s buffer)."""
    wait = _seconds_until_next_window()
    if wait > 0:
        wait += 1.0
        print(f"[mfa-helper] Waiting {wait:.0f}s for next TOTP window")
        time.sleep(wait)


def _sleep_if_still_in_window(window_id: int) -> None:
    if _totp_window_id() == window_id:
        _sleep_to_next_totp_window()


def acquire_totp_passcode(seed: str, *, max_windows: int = 3) -> str:
    """Return one unused TOTP passcode, advancing to the next window if needed."""
    advances = 0
    while advances < max_windows:
        passcode = _fresh_totp_code(seed)
        if passcode is not None:
            return passcode
        print("[mfa-helper] No unused codes in this window, advancing")
        _sleep_to_next_totp_window()
        advances += 1
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

    Snowflake rejects reused TOTP codes within a time window. A code already
    consumed in this pytest process is skipped (does not consume the submit
    budget); after a retryable rejection, wait only if still in that window.
    """
    if _shared_mfa_exhausted():
        pytest.skip("Shared MFA account already exhausted TOTP retries in this run")

    last_error = None
    base_password = connect_kwargs.get("password")
    submits = 0
    advances = 0

    while submits < max_windows:
        passcode = _fresh_totp_code(totp_seed)
        if passcode is None:
            if advances >= max_windows:
                break
            print("[mfa-helper] No unused codes in this window, advancing")
            _sleep_to_next_totp_window()
            advances += 1
            continue

        window_id = _totp_window_id()
        submits += 1
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
            if is_mfa_lockout_error(e):
                _mark_shared_mfa_exhausted()
                pytest.skip("Shared MFA account locked (394512); skipping this and later MFA tests")
            if not is_totp_retryable_error(e):
                raise
            print(f"[mfa-helper] TOTP submit {submits} failed; retrying if a fresh window is available")
            if submits < max_windows:
                _sleep_if_still_in_window(window_id)

    if submits == 0:
        raise AssertionError(f"No unused TOTP passcodes after {max_windows} windows")
    _mark_shared_mfa_exhausted()
    raise AssertionError(f"Failed to connect after {submits} TOTP submits. Last error: {last_error}") from last_error


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
