"""E2E tests for wheel packaging.

These tests verify that a wheel can be built from source, including
Rust core compilation, Cython extension building, and protobuf generation.

With SKIP_CORE_BUILD=true in CI's build_wheel job, the packaging_tests job
is the only place that exercises the full wheel build pipeline end-to-end.
"""

from __future__ import annotations

import os
import subprocess
import sys
import zipfile

from pathlib import Path
from tempfile import TemporaryDirectory

import pytest


def get_python_dir() -> Path:
    """Get the python project directory."""
    return Path(__file__).parent.parent.parent


class TestWheelPackaging:
    """Tests for wheel build and contents."""

    def _run_command(
        self,
        cmd: list[str],
        cwd: Path | None = None,
        env: dict[str, str] | None = None,
        timeout: int = 900,
    ) -> subprocess.CompletedProcess:
        full_env = os.environ.copy()
        if env:
            full_env.update(env)

        return subprocess.run(
            cmd,
            cwd=cwd,
            env=full_env,
            capture_output=True,
            text=True,
            timeout=timeout,
        )

    @pytest.mark.slow
    def test_wheel_build_from_source(self) -> None:
        """Test that a wheel can be built from source including Rust compilation.

        Exercises the full hatch_build.py pipeline:
        1. Proto generation (Rust proto_generator)
        2. Cython extension compilation
        3. sf_core Rust library compilation (release mode)
        4. Wheel packaging with all artifacts
        """
        python_dir = get_python_dir()

        with TemporaryDirectory(prefix="wheel_test_") as tmpdir:
            dist_dir = Path(tmpdir) / "dist"
            dist_dir.mkdir()

            env = {"SKIP_CORE_BUILD": ""}

            result = self._run_command(
                [sys.executable, "-m", "build", "--wheel", "--installer", "uv", "--outdir", str(dist_dir)],
                cwd=python_dir,
                env=env,
                timeout=900,
            )

            if result.returncode != 0:
                pytest.fail(f"Failed to build wheel:\nstdout: {result.stdout}\nstderr: {result.stderr}")

            wheels = list(dist_dir.glob("*.whl"))
            assert wheels, f"No wheel found in {dist_dir}"

            self._verify_wheel_contents(wheels[0])

    def _verify_wheel_contents(self, wheel_path: Path) -> None:
        """Verify the wheel contains expected build artifacts."""
        with zipfile.ZipFile(wheel_path, "r") as whl:
            names = whl.namelist()

            core_lib_suffixes = (".so", ".dll", ".dylib")
            core_files = [n for n in names if "_core/" in n and any(n.endswith(s) for s in core_lib_suffixes)]
            assert core_files, (
                f"sf_core native library not found in wheel. Wheel _core contents: {[n for n in names if '_core' in n]}"
            )

            pb2_files = [n for n in names if n.endswith("_pb2.py")]
            assert pb2_files, "No protobuf generated files (*_pb2.py) found in wheel"

            arrow_ext_patterns = ("arrow_stream_iterator.cpython", "arrow_stream_iterator.pyd")
            arrow_files = [n for n in names if any(p in n for p in arrow_ext_patterns)]
            assert arrow_files, (
                f"Cython arrow_stream_iterator extension not found in wheel. "
                f"Wheel contents: {[n for n in names if 'arrow' in n]}"
            )
