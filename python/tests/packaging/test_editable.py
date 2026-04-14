"""E2E tests for editable (PEP 660) install packaging.

These tests verify that a PEP 660 editable install compiles the native Cython
extension (arrow_stream_iterator) and exports all expected symbols.  The Rust
core build is skipped via SKIP_CORE_BUILD=1 to keep the test fast; sf_core
loading is already covered by test_sdist.py.
"""

from __future__ import annotations

import os
import subprocess
import sys
import venv

from pathlib import Path
from tempfile import TemporaryDirectory

import pytest


def get_python_dir() -> Path:
    """Return the python/ project root (parent of tests/)."""
    # tests/packaging/test_editable.py -> python/
    return Path(__file__).parent.parent.parent


class TestEditableInstall:
    """Tests for PEP 660 editable install."""

    @pytest.fixture
    def temp_env(self):
        """Temporary directory for the test environment."""
        with TemporaryDirectory(prefix="editable_test_") as tmpdir:
            yield Path(tmpdir)

    def _run_command(
        self,
        cmd: list[str],
        cwd: Path | None = None,
        env: dict[str, str] | None = None,
        timeout: int = 300,
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

    def _create_venv(self, venv_path: Path) -> Path:
        """Create a venv and return the path to its Python executable."""
        venv.create(venv_path, with_pip=True)
        python_exe = venv_path / "Scripts" / "python.exe" if sys.platform == "win32" else venv_path / "bin" / "python"
        pip_exe = venv_path / "Scripts" / "pip.exe" if sys.platform == "win32" else venv_path / "bin" / "pip"
        result = self._run_command([str(pip_exe), "install", "--upgrade", "pip"])
        if result.returncode != 0:
            pytest.fail(f"pip upgrade failed: {result.stderr}")
        return python_exe

    @pytest.mark.slow
    def test_editable_install_compiles_arrow_extension(self, temp_env: Path) -> None:
        """Editable install must compile arrow_stream_iterator and export ArrowStreamTableIterator.

        Regression test for the BuildHook gap where target_name == 'editable'
        caused an early return, leaving the Cython extension uncompiled.

        Steps:
        1. Create a fresh venv (no pre-built .so present).
        2. pip install -e <python_dir> — exercises the PEP 660 editable path.
        3. Assert ArrowStreamTableIterator is importable from the compiled module.
        4. Assert the module is a native .so/.pyd (not a Python fallback).
        """
        python_dir = get_python_dir()
        venv_path = temp_env / "venv"

        python_exe = self._create_venv(venv_path)

        # Install in editable mode.  SKIP_CORE_BUILD=1 skips the Rust release
        # build — sf_core is covered by test_sdist.py.  Proto generation still
        # runs (it uses cargo, but only the fast proto_generator binary).
        result = self._run_command(
            [str(python_exe), "-m", "pip", "install", "--editable", str(python_dir)],
            env={"SKIP_CORE_BUILD": "1"},
            timeout=300,
        )
        if result.returncode != 0:
            pytest.fail(f"Editable install failed:\nstdout: {result.stdout}\nstderr: {result.stderr}")

        # Verify ArrowStreamTableIterator is exported by the compiled extension.
        result = self._run_command(
            [
                str(python_exe),
                "-c",
                (
                    "from snowflake.connector._internal.arrow_stream_iterator"
                    " import ArrowStreamTableIterator;"
                    " print('ArrowStreamTableIterator imported successfully')"
                ),
            ]
        )
        if result.returncode != 0:
            pytest.fail(
                f"ArrowStreamTableIterator import failed — Cython extension may not have been"
                f" compiled during editable install:\nstdout: {result.stdout}\nstderr: {result.stderr}"
            )
        assert "ArrowStreamTableIterator imported successfully" in result.stdout

        # Confirm the module resolves to a compiled native extension (.so / .pyd),
        # not a pure-Python fallback.
        result = self._run_command(
            [
                str(python_exe),
                "-c",
                (
                    "import snowflake.connector._internal.arrow_stream_iterator as m;"
                    " import pathlib;"
                    " f = pathlib.Path(m.__file__);"
                    " assert f.suffix in ('.so', '.pyd'), f'Expected native extension, got {f}';"
                    " print(f'Native extension confirmed: {f.name}')"
                ),
            ]
        )
        if result.returncode != 0:
            pytest.fail(
                f"arrow_stream_iterator is not a compiled native extension:"
                f"\nstdout: {result.stdout}\nstderr: {result.stderr}"
            )
        assert "Native extension confirmed" in result.stdout
