"""Custom hatch build hook to build Rust core library before packaging."""

import subprocess
import sys
from pathlib import Path

from hatchling.builders.hooks.plugin.interface import BuildHookInterface


class RustCoreBuildHook(BuildHookInterface):

    PLUGIN_NAME = "rust-core"

    def initialize(self, version, build_data):
        _, _, lib_path = self.get_core_lib_path(Path(self.root))
        if not lib_path.exists():
            raise RuntimeError(f"Missing core lib in {lib_path}")

    @classmethod
    def build_core(cls, root):
        """Build the Rust core library if it doesn't exist."""
        # Get project root (parent of python directory)
        project_root = Path(root).parent
        python_dir = Path(root)
        core_module_dir, lib_name, lib_path = cls.get_core_lib_path(python_dir)

        # Check if library already exists
        # if lib_path.exists():
        #     print(f"✓ Rust core library already exists at {lib_path}")
        #     return

        print("=" * 70)
        print("Building Rust core library...")
        print(f"Project root: {project_root}")
        print(f"Target directory: {core_module_dir}")
        print("=" * 70)

        try:
            # Build the Rust library
            result = subprocess.run(
                [
                    "cargo",
                    "build",
                    "--package",
                    "sf_core",
                    f"--target-dir={core_module_dir}",
                ],
                cwd=project_root,
                check=True,
                capture_output=True,
                text=True,
            )

            print(result.stdout)

            # Move files from debug/ to parent directory
            debug_dir = core_module_dir / "debug"
            if debug_dir.exists():
                print(f"Moving build artifacts from {debug_dir} to {core_module_dir}")
                
                # Move the main library file
                debug_lib = debug_dir / lib_name
                if debug_lib.exists():
                    debug_lib.rename(lib_path)
                    print(f"✓ Moved {lib_name}")
                
                # Move other potentially needed files (like .rlib, .d files)
                for ext in ["*.rlib", "*.d"]:
                    for file in debug_dir.glob(ext):
                        target = core_module_dir / file.name
                        if not target.exists():
                            file.rename(target)
                
                # Clean up debug directory (optional - you may want to keep it)
                # Note: Not removing debug/ as it may contain other artifacts
                print(f"✓ Rust core library built successfully: {lib_path}")

        except subprocess.CalledProcessError as e:
            print("=" * 70, file=sys.stderr)
            print("ERROR: Failed to build Rust core library", file=sys.stderr)
            print("=" * 70, file=sys.stderr)
            print(e.stderr, file=sys.stderr)
            print("\nMake sure Rust toolchain is installed:", file=sys.stderr)
            print("  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh", file=sys.stderr)
            print("=" * 70, file=sys.stderr)
            raise

        except FileNotFoundError:
            print("=" * 70, file=sys.stderr)
            print("ERROR: cargo not found", file=sys.stderr)
            print("=" * 70, file=sys.stderr)
            print("Make sure Rust toolchain is installed:", file=sys.stderr)
            print("  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh", file=sys.stderr)
            print("=" * 70, file=sys.stderr)
            raise

    @classmethod
    def get_core_lib_path(cls, python_dir: Path) -> tuple[Path, str, Path]:
        core_module_dir = python_dir / "src" / "snowflake" / "ud_connector" / "_core"

        # Determine the library file name based on platform
        if sys.platform == "darwin":
            lib_name = "libsf_core.dylib"
        elif sys.platform == "win32":
            lib_name = "libsf_core.dll"
        else:
            lib_name = "libsf_core.so"

        lib_path = core_module_dir / lib_name
        return core_module_dir, lib_name, lib_path


if __name__ == '__main__':
    RustCoreBuildHook.build_core(root=Path(__name__).parent)