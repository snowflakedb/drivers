"""
Integration tests for import time of snowflake.connector.

These tests run imports in a subprocess so that in-memory cached modules
from the test process do not affect the measurement.
"""

from __future__ import annotations

import math
import subprocess
import sys

from tests.compatibility import IS_UNIVERSAL_DRIVER


_IMPORT_TIME_SCRIPT = """\
import time

start = time.monotonic()
from snowflake.connector import connect  # noqa: E402, F401
str(connect)
elapsed = time.monotonic() - start

print(f"{elapsed:.6f}")
"""

_NUM_RUNS = 10
_MAX_IMPORT_TIME_SECONDS = 0.4 if IS_UNIVERSAL_DRIVER else 0.6


class TestImportTime:
    """Verify that importing snowflake.connector.connect stays within budget."""

    def test_import_connect_time(self):
        """Importing ``from snowflake.connector import connect`` must complete
        within the allowed time budget.

        The import is executed in fresh subprocesses so that any modules
        already loaded by the test runner have no effect on the measurement.
        The 90th percentile of multiple runs is used to reduce noise while
        still catching consistently slow runs.
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

        sorted_times = sorted(times)
        p90_elapsed = sorted_times[math.ceil(0.9 * len(sorted_times)) - 1]
        assert p90_elapsed < _MAX_IMPORT_TIME_SECONDS, (
            f"Importing snowflake.connector.connect had p90 {p90_elapsed:.3f}s "
            f"(runs: {', '.join(f'{t:.3f}s' for t in times)}), "
            f"which exceeds the {_MAX_IMPORT_TIME_SECONDS}s budget"
        )
