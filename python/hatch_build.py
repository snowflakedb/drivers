"""Custom Hatch build hooks."""

import shutil
import subprocess
from pathlib import Path
from typing import Any

from hatchling.builders.hooks.plugin.interface import BuildHookInterface


class CustomBuildHook(BuildHookInterface):
    """Build hook that compiles the Rust core library before packaging."""

    def initialize(self, version: str, build_data: dict[str, Any]) -> None:
        """Run before the build process starts."""
        if self.target_name not in ("wheel", "sdist"):
            return

        print("Building Rust core library...")

        # Get paths relative to the Python wrapper directory
        python_dir = Path(__file__).parent
        cargo_manifest = python_dir.parent / "Cargo.toml"
        target_dir = python_dir / "src" / "snowflake" / "ud_connector" / "_core"

        # Ensure target directory exists
        target_dir.mkdir(parents=True, exist_ok=True)

        # Build the Rust core library
        cargo_args = [
            "cargo",
            "build",
            "--package",
            "sf_core",
            "--manifest-path",
            str(cargo_manifest),
            "--target-dir",
            str(target_dir),
        ]

        try:
            result = subprocess.run(
                cargo_args,
                check=True,
                capture_output=True,
                text=True,
            )
            print(result.stdout)
        except subprocess.CalledProcessError as e:
            print(f"Cargo build failed with exit code {e.returncode}")
            print(f"stdout: {e.stdout}")
            print(f"stderr: {e.stderr}")
            raise

        # Copy built artifacts from debug directory to _core directory
        debug_dir = target_dir / "debug"
        if debug_dir.exists():
            print(f"Copying artifacts from {debug_dir} to {target_dir}...")
            shutil.copytree(debug_dir, target_dir, dirs_exist_ok=True)

            # Clean up debug directory
            shutil.rmtree(debug_dir)
            print("Cleaned up debug directory")

        print("Rust core library build complete!")
