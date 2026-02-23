"""E2E tests for sdist packaging.

These tests verify that the sdist can be built and installed correctly,
ensuring that all necessary sources (including Rust code) are properly included.
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
    """Get the python project directory."""
    # tests/packaging/test_sdist.py -> python/
    return Path(__file__).parent.parent.parent


class TestSdistPackaging:
    """Tests for sdist build and installation."""

    @pytest.fixture
    def temp_env(self):
        """Create a temporary directory for the test environment."""
        with TemporaryDirectory(prefix="sdist_test_") as tmpdir:
            yield Path(tmpdir)

    def _run_command(
        self,
        cmd: list[str],
        cwd: Path | None = None,
        env: dict[str, str] | None = None,
        timeout: int = 600,
    ) -> subprocess.CompletedProcess:
        """Run a command and return the result."""
        full_env = os.environ.copy()
        if env:
            full_env.update(env)

        result = subprocess.run(
            cmd,
            cwd=cwd,
            env=full_env,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
        return result

    def _create_venv(self, venv_path: Path) -> Path:
        """Create a virtual environment and return the python executable path."""
        venv.create(venv_path, with_pip=True)

        if sys.platform == "win32":
            python_exe = venv_path / "Scripts" / "python.exe"
            pip_exe = venv_path / "Scripts" / "pip.exe"
        else:
            python_exe = venv_path / "bin" / "python"
            pip_exe = venv_path / "bin" / "pip"

        # Upgrade pip to avoid issues with older pip versions
        result = self._run_command([str(pip_exe), "install", "--upgrade", "pip"])
        if result.returncode != 0:
            pytest.fail(f"Failed to upgrade pip: {result.stderr}")

        return python_exe

    def _build_sdist(self, python_dir: Path, output_dir: Path) -> Path:
        """Build sdist and return the path to the tarball."""
        # Use python -m build to create sdist (standard PEP 517 build)
        result = self._run_command(
            [sys.executable, "-m", "build", "--sdist", "--outdir", str(output_dir)],
            cwd=python_dir,
        )

        if result.returncode != 0:
            pytest.fail(f"Failed to build sdist:\nstdout: {result.stdout}\nstderr: {result.stderr}")

        # Find the built sdist
        sdist_files = list(output_dir.glob("*.tar.gz"))
        if not sdist_files:
            pytest.fail(f"No sdist found in {output_dir}")

        return sdist_files[0]

    def _install_sdist(self, python_exe: Path, sdist_path: Path) -> None:
        """Install sdist into the virtual environment."""
        pip_exe = python_exe.parent / ("pip.exe" if sys.platform == "win32" else "pip")

        # Install the sdist (this will build from source)
        result = self._run_command(
            [str(pip_exe), "install", str(sdist_path), "--verbose"],
            timeout=600,  # Rust compilation can take a while
        )

        if result.returncode != 0:
            pytest.fail(f"Failed to install sdist:\nstdout: {result.stdout}\nstderr: {result.stderr}")

    def _verify_installation(self, python_exe: Path) -> None:
        """Verify the installation by importing the module and checking version."""
        # Test importing the main module
        result = self._run_command(
            [
                str(python_exe),
                "-c",
                "from snowflake.connector.version import VERSION; print(f'VERSION={VERSION}')",
            ]
        )

        if result.returncode != 0:
            pytest.fail(
                f"Failed to import snowflake.connector.version:\nstdout: {result.stdout}\nstderr: {result.stderr}"
            )

        assert "VERSION=" in result.stdout, f"Unexpected output: {result.stdout}"

        # Test importing the core API (verifies native library loads)
        result = self._run_command(
            [
                str(python_exe),
                "-c",
                "from snowflake.connector._internal.api_client import c_api; print('c_api imported successfully')",
            ]
        )

        if result.returncode != 0:
            pytest.fail(f"Failed to import c_api (native library):\nstdout: {result.stdout}\nstderr: {result.stderr}")

        assert "c_api imported successfully" in result.stdout

    @pytest.mark.slow
    def test_sdist_build_and_install(self, temp_env: Path) -> None:
        """Test that sdist can be built, installed, and the module imports correctly.

        This test verifies the complete sdist workflow:
        1. Build sdist from the python project
        2. Create a fresh virtual environment
        3. Install the sdist (which builds from source including Rust)
        4. Verify the module can be imported and version is accessible
        5. Verify the native library (sf_core) loads correctly
        """
        python_dir = get_python_dir()
        venv_path = temp_env / "venv"
        dist_dir = temp_env / "dist"
        dist_dir.mkdir()

        # Step 1: Build sdist
        sdist_path = self._build_sdist(python_dir, dist_dir)
        assert sdist_path.exists(), f"sdist not found at {sdist_path}"

        # Step 2: Create virtual environment
        python_exe = self._create_venv(venv_path)
        assert python_exe.exists(), f"Python executable not found at {python_exe}"

        # Step 3: Install sdist
        self._install_sdist(python_exe, sdist_path)

        # Step 4: Verify installation
        self._verify_installation(python_exe)

    @pytest.mark.slow
    def test_sdist_contains_rust_sources(self, temp_env: Path) -> None:
        """Test that the sdist contains all necessary Rust sources."""
        import tarfile

        python_dir = get_python_dir()
        dist_dir = temp_env / "dist"
        dist_dir.mkdir()

        # Build sdist
        sdist_path = self._build_sdist(python_dir, dist_dir)

        # Check contents
        with tarfile.open(sdist_path, "r:gz") as tar:
            names = tar.getnames()

            # Check for essential Rust files
            required_patterns = [
                "Cargo.toml",
                "Cargo.lock",
                "sf_core/Cargo.toml",
                "sf_core/src/lib.rs",
                "sf_core/src/c_api.rs",
                "proto_utils/Cargo.toml",
                "proto_utils/src/lib.rs",
                "proto_generator/Cargo.toml",
                "proto_generator/src/main.rs",
                "protobuf/database_driver_v1.proto",
                "error_trace/Cargo.toml",
                "error_trace/src/lib.rs",
                "error_trace_derive/Cargo.toml",
                "error_trace_derive/src/lib.rs",
            ]

            for pattern in required_patterns:
                matches = [n for n in names if n.endswith(pattern)]
                assert matches, f"Missing required file pattern: {pattern}"

            # Verify Cargo.toml is the minimal version (not full workspace)
            cargo_toml_entry = next(
                n
                for n in names
                if n.endswith("/Cargo.toml")
                and "sf_core" not in n
                and "proto_utils" not in n
                and "proto_generator" not in n
                and "error_trace" not in n
            )
            cargo_content = tar.extractfile(cargo_toml_entry)
            assert cargo_content is not None
            content = cargo_content.read().decode("utf-8")

            assert "sf_core" in content
            assert "proto_utils" in content
            assert "error_trace" in content
            assert "error_trace_derive" in content
            # Should NOT have other workspace members
            assert "odbc" not in content
            assert "jdbc_bridge" not in content
