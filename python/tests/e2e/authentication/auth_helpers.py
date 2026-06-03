import socket
import subprocess
import threading
import time

from collections.abc import Callable

from snowflake.connector.errors import DatabaseError


# These Node.js scripts and the Chromium remote-debugging port are provided by the
# snowdrivers-test-external-browser-universal-driver Docker image (see ci/auth/). They
# do not exist outside that container, which is why these tests are gated behind the
# `requires_browser` marker.
PROVIDE_CREDENTIALS_SCRIPT = "/externalbrowser/provideBrowserCredentials.js"
CLEAN_BROWSER_SCRIPT = "/externalbrowser/cleanBrowserProcesses.js"
CHROMIUM_DEBUG_PORT = 9222


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


def clean_browser_processes():
    """Kill any lingering Chromium processes from previous test runs."""
    subprocess.run(["node", CLEAN_BROWSER_SCRIPT], timeout=15, capture_output=True)


def provide_browser_credentials(scenario: str, login: str, password: str):
    """Run the Node.js browser automation script that fills IdP credentials."""
    result = subprocess.run(
        ["node", PROVIDE_CREDENTIALS_SCRIPT, scenario, login, password],
        timeout=60,
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
):
    """Run connect_fn and browser automation concurrently.

    Returns the connection on success, raises on failure.
    """
    errors = []
    result_holder = []

    def _connect():
        try:
            conn = connect_fn()
            result_holder.append(conn)
        except Exception as e:
            errors.append(e)

    def _browser():
        try:
            if not _wait_for_chromium():
                raise RuntimeError(f"Chromium did not start on port {CHROMIUM_DEBUG_PORT} within timeout")
            provide_browser_credentials(scenario, login, password)
        except Exception as e:
            errors.append(e)

    # daemon=True so a hung thread can never block interpreter shutdown / leak across
    # tests; the explicit joins below are the real synchronization point.
    t_connect = threading.Thread(target=_connect, daemon=True)
    t_browser = threading.Thread(target=_browser, daemon=True)

    t_connect.start()
    t_browser.start()

    t_browser.join(timeout=90)
    t_connect.join(timeout=120)

    # Distinguish a hung thread (join timed out) from a clean failure so the cause
    # isn't swallowed into a generic "connection not established".
    if t_browser.is_alive():
        raise TimeoutError("Browser automation thread did not finish within 90s")
    if t_connect.is_alive():
        raise TimeoutError("Connect thread did not finish within 120s")

    assert not errors, f"Errors during browser authentication: {errors}"
    assert len(result_holder) == 1, "Connection was not established"

    return result_holder[0]
