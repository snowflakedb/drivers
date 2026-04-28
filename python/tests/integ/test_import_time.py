"""
Integration tests for import time of snowflake.connector.

These tests run imports in a subprocess so that in-memory cached modules
from the test process do not affect the measurement.
"""

from __future__ import annotations

import functools
import platform
import subprocess
import sys

import pytest

from tests.compatibility import IS_UNIVERSAL_DRIVER


@functools.lru_cache(maxsize=1)
def _is_rosetta() -> bool:
    """Detect if running x86_64 under Rosetta 2 on Apple Silicon."""
    if platform.system() != "Darwin" or platform.machine() != "x86_64":
        return False
    try:
        result = subprocess.run(
            ["sysctl", "-n", "sysctl.proc_translated"],
            capture_output=True,
            text=True,
            timeout=5,
        )
        return result.stdout.strip() == "1"
    except Exception:
        return False


_IMPORT_TIME_SCRIPT = """\
import time

start = time.monotonic()
from snowflake.connector import connect  # noqa: E402, F401
str(connect)
elapsed = time.monotonic() - start

print(f"{elapsed:.6f}")
"""

_NUM_RUNS = 5
_MAX_IMPORT_TIME_SECONDS = 0.4 if IS_UNIVERSAL_DRIVER else 0.6


class TestImportTime:
    """Verify that importing snowflake.connector.connect stays within budget."""

    @pytest.mark.skipif(_is_rosetta(), reason="Import timing unreliable under Rosetta 2 emulation")
    def test_import_connect_time(self):
        """Importing ``from snowflake.connector import connect`` must complete
        within the allowed time budget.

        The import is executed in fresh subprocesses so that any modules
        already loaded by the test runner have no effect on the measurement.
        The mean of multiple runs is used to reduce noise.
        """
        times = []
        for _ in range(_NUM_RUNS):
            result = subprocess.run(
                [sys.executable, "-c", _IMPORT_TIME_SCRIPT],
                capture_output=True,
                text=True,
                timeout=30,
            )
            assert result.returncode == 0, f"Import script failed.\nstdout: {result.stdout}\nstderr: {result.stderr}"
            elapsed_output = result.stdout.strip()
            try:
                elapsed = float(elapsed_output)
            except ValueError:
                raise AssertionError(
                    "Import script produced non-numeric timing output.\n"
                    f"parsed stdout: {elapsed_output!r}\n"
                    f"stdout: {result.stdout}\n"
                    f"stderr: {result.stderr}"
                ) from None
            times.append(elapsed)

        mean_elapsed = sum(times) / len(times)
        assert mean_elapsed < _MAX_IMPORT_TIME_SECONDS, (
            f"Importing snowflake.connector.connect took {mean_elapsed:.3f}s on average "
            f"(runs: {', '.join(f'{t:.3f}s' for t in times)}), "
            f"which exceeds the {_MAX_IMPORT_TIME_SECONDS}s budget"
        )
