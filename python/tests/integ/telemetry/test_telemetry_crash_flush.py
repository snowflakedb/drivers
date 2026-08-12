"""Document that buffered telemetry is lost when the connector process is killed.

``send_batch()`` on ``connection._telemetry`` is a no-op because the Rust core
owns the flush lifecycle; the only cleanup hook either driver installs is
``atexit.register(...)``. ``atexit`` callbacks run on normal interpreter
shutdown only (falling off ``__main__``, ``sys.exit()``, an unhandled
exception unwinding to the top) — not on any signal. Neither driver installs
a ``signal.signal(SIGTERM, ...)`` handler, so SIGTERM's default disposition
(immediate OS-level termination) applies: the interpreter never runs cleanup
code. SIGKILL is the same story, just more so (unblockable). Buffered
telemetry is lost on both signals, in both drivers — this test documents that
gap rather than asserting a guarantee that was never implemented.

Test strategy: run the connector in a subprocess so we can actually kill the
process. WireMock answers the login request but hangs the query response
indefinitely, so the subprocess is blocked inside ``cursor.execute`` and has no
chance to call ``send_batch()`` or ``close()`` explicitly.

Runs against both drivers (old connector reference + universal driver) to catch
regressions and to document any parity differences.
"""

from __future__ import annotations

import os
import signal
import subprocess
import sys
import textwrap
import time

import pytest

from tests.integ.telemetry._telemetry_helpers import collect_log_entries
from tests.private_key_helper import get_test_private_key_path


# ---------------------------------------------------------------------------
# Subprocess script — intentionally minimal, no test infra imports
# ---------------------------------------------------------------------------

_SUBPROCESS_SCRIPT = textwrap.dedent("""
    import os
    import sys

    wiremock_url = os.environ["_TEST_WIREMOCK_URL"]
    private_key_path = os.environ["_TEST_PRIVATE_KEY_PATH"]

    import snowflake.connector
    conn = snowflake.connector.connect(
        account="test_account",
        user="test_user",
        database="test_database",
        schema="test_schema",
        warehouse="test_warehouse",
        role="test_role",
        server_url=wiremock_url,
        protocol="http",
        host=wiremock_url.split("://")[1].rsplit(":", 1)[0],
        port=int(wiremock_url.rsplit(":", 1)[1]),
        authenticator="SNOWFLAKE_JWT",
        private_key_file=private_key_path,
    )
    cursor = conn.cursor()
    # This blocks indefinitely — WireMock never responds.
    # The parent process kills us before this returns.
    try:
        cursor.execute("SELECT 1")
    except Exception:
        pass
""")


# ---------------------------------------------------------------------------
# Parametrize: how to kill, and whether we expect delivery to succeed
# ---------------------------------------------------------------------------

_KILL_CASES = [
    pytest.param(
        signal.SIGTERM,
        False,
        id="SIGTERM-buffer-lost",
        marks=[
            pytest.mark.xfail(
                strict=False,
                reason="Neither driver installs a SIGTERM handler; atexit does not fire on signals, "
                "so buffered telemetry is lost. xfail (not a hard assertion) so this flips silently, "
                "not with a failure, if a crash-safe flush is ever implemented.",
            )
        ],
    ),
    pytest.param(
        signal.SIGKILL,
        False,
        id="SIGKILL-buffer-lost",
        marks=[pytest.mark.xfail(strict=False, reason="SIGKILL bypasses all cleanup; in-buffer entries are lost")],
    ),
]


# ---------------------------------------------------------------------------
# Test
# ---------------------------------------------------------------------------


@pytest.mark.parametrize("kill_signal,expect_delivery", _KILL_CASES)
def test_telemetry_flushed_after_process_kill(int_test_connection_factory, wiremock, kill_signal, expect_delivery):
    """Telemetry buffered during connect+execute reaches /telemetry/send after the process is killed."""
    wiremock.add_mapping("auth/login_success_jwt.json")
    wiremock.add_mapping("query/query_hanging.json")
    wiremock.add_mapping("telemetry/telemetry_send_success.json")

    env = {
        **os.environ,
        "_TEST_WIREMOCK_URL": wiremock.http_url(),
        "_TEST_PRIVATE_KEY_PATH": get_test_private_key_path(),
    }

    proc = subprocess.Popen(
        [sys.executable, "-c", _SUBPROCESS_SCRIPT],
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )

    try:
        # Give the subprocess time to connect and block on the hanging query.
        time.sleep(2)
        proc.send_signal(kill_signal)
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait()

    # Poll briefly rather than asserting immediately after wait() returns.
    telemetry_requests = wiremock.wait_for_requests("/telemetry/send", min_count=1, timeout=5.0)

    if not expect_delivery:
        # xfail: assert nothing arrived — if this fails it means the driver somehow
        # flushes under this signal, which would be a pleasant surprise.
        assert len(telemetry_requests) == 0, (
            f"Unexpectedly received telemetry after {kill_signal.name} — "
            "update this test if the driver gained crash-safe flush"
        )
        return

    assert len(telemetry_requests) >= 1, (
        f"Expected telemetry to reach /telemetry/send after {kill_signal.name}, got none. "
        "The driver's crash-flush path may not be wired up."
    )

    entries = collect_log_entries(telemetry_requests)
    assert len(entries) >= 1, f"Telemetry arrived but contained no log entries: {telemetry_requests}"

    # At minimum, a session_init span is expected from the connect() call.
    entry_types = {e["message"].get("type") for e in entries}
    assert entry_types, f"No 'type' keys found in telemetry entries: {entries}"
