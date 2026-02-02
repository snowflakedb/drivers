"""Custom Hatch build hooks."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import warnings

from pathlib import Path
from tempfile import TemporaryDirectory
from typing import Any

from hatchling.builders.hooks.plugin.interface import BuildHookInterface


# Graceful fallback if Cython or setuptools are not available
try:
    from Cython.Build import cythonize
    from setuptools import Distribution, Extension
    from setuptools.command.build_ext import build_ext

    CYTHON_AVAILABLE = True
except ImportError:
    CYTHON_AVAILABLE = False
    cythonize = None  # type: ignore[assignment,misc]
    Distribution = None  # type: ignore[assignment,misc]
    Extension = None  # type: ignore[assignment,misc]
    build_ext = None  # type: ignore[assignment,misc]


class BuildHook(BuildHookInterface):
    """Build hook for compiling Cython extensions with nanoarrow C++ code."""

    PLUGIN_NAME = "nanoarrow"

    # Relative paths from src/
    CONNECTOR_DIR = Path("snowflake") / "connector"
    INTERNAL_DIR = CONNECTOR_DIR / "_internal"
    NANOARROW_CPP_DIR = INTERNAL_DIR / "nanoarrow_cpp"
    ARROW_ITERATOR_DIR = NANOARROW_CPP_DIR / "ArrowIterator"
    LOGGING_DIR = NANOARROW_CPP_DIR / "Logging"

    # Extension module name
    EXTENSION_NAME = "snowflake.connector._internal.arrow_stream_iterator"
    PYX_SOURCE = ARROW_ITERATOR_DIR / "arrow_stream_iterator.pyx"

    # C++ source files in ArrowIterator directory
    CPP_SOURCES = [
        "ArrayConverter.cpp",
        "BinaryConverter.cpp",
        "BooleanConverter.cpp",
        "CArrowIterator.cpp",
        "CArrowStreamIterator.cpp",
        "CArrowTableIterator.cpp",
        "ConverterUtil.cpp",
        "DateConverter.cpp",
        "DecFloatConverter.cpp",
        "DecimalConverter.cpp",
        "FixedSizeListConverter.cpp",
        "FloatConverter.cpp",
        "IntConverter.cpp",
        "IntervalConverter.cpp",
        "MapConverter.cpp",
        "ObjectConverter.cpp",
        "SnowflakeType.cpp",
        "StringConverter.cpp",
        "TimeConverter.cpp",
        "TimeStampConverter.cpp",
        "flatcc.c",
        "nanoarrow.c",
        "nanoarrow_ipc.c",
    ]

    # Sources in subdirectories of ArrowIterator
    SUBDIRECTORY_SOURCES = [
        ("Python", "Common.cpp"),
        ("Python", "Helpers.cpp"),
        ("Util", "time.cpp"),
    ]

    # C files that need special handling (C99 instead of C++11)
    C_FILES_FOR_C99 = ("nanoarrow.c", "nanoarrow_ipc.c", "flatcc.c")

    # Environment variable to disable compilation
    DISABLE_COMPILE_ENV_VAR = "SNOWFLAKE_DISABLE_COMPILE_ARROW_EXTENSIONS"
    POSITIVE_VALUES = ("y", "yes", "t", "true", "1", "on")

    # Vendored C files that should have warnings suppressed
    VENDORED_C_FILES = ("nanoarrow.c", "nanoarrow_ipc.c", "flatcc.c")

    def initialize(self, version: str, build_data: dict[str, Any]) -> None:
        """Initialize the build hook and compile extensions."""
        if self.target_name != "wheel":
            # For source distribution
            return

        if os.environ.get(self.DISABLE_COMPILE_ENV_VAR, "false").lower() in self.POSITIVE_VALUES:
            return

        if not CYTHON_AVAILABLE:
            warnings.warn(
                "Cannot compile native C code, because of a missing build dependency "
                "(Cython or setuptools). The package will be installed without the "
                "native Arrow extension.",
                stacklevel=1,
            )
            return

        self._build_extensions()
        self._build_core()

    def _build_extensions(self) -> None:
        """Build the Cython extensions."""
        src_root = Path(self.root) / "src"
        arrow_iterator_dir = src_root / self.ARROW_ITERATOR_DIR
        logging_dir = src_root / self.LOGGING_DIR

        # Define the extension
        ext = Extension(
            name=self.EXTENSION_NAME,
            sources=[str(src_root / self.PYX_SOURCE)],
            language="c++",
        )

        # Add C++ source files
        for src in self.CPP_SOURCES:
            ext.sources.append(str(arrow_iterator_dir / src))

        # Add subdirectory sources
        for subdir, filename in self.SUBDIRECTORY_SOURCES:
            ext.sources.append(str(arrow_iterator_dir / subdir / filename))

        # Add logging source
        ext.sources.append(str(logging_dir / "logging.cpp"))

        # Add include directories
        ext.include_dirs.append(str(arrow_iterator_dir))
        ext.include_dirs.append(str(logging_dir))

        # Apply platform-specific flags
        self._apply_compile_flags(ext)
        self._apply_link_flags(ext)

        # Cythonize and build
        extensions = cythonize([ext])
        self._run_build(extensions, src_root)

    def _apply_compile_flags(self, ext: Extension) -> None:
        """Apply platform-specific compile flags to the extension."""
        if sys.platform == "win32":
            if not any("/std" in s for s in ext.extra_compile_args):
                ext.extra_compile_args.append("/std:c++17")
        elif sys.platform in ("linux", "darwin"):
            if "std=" not in os.environ.get("CXXFLAGS", ""):
                ext.extra_compile_args.extend(["-std=c++11", "-D_GLIBCXX_USE_CXX11_ABI=0"])
            # Define endianness for flatcc
            ext.extra_compile_args.extend(
                [
                    "-DFLATBUFFERS_LITTLEENDIAN=1",
                    "-DFLATBUFFERS_PROTOCOL_IS_LE=1",
                ]
            )
            if sys.platform == "darwin" and "macosx-version-min" not in os.environ.get("CXXFLAGS", ""):
                ext.extra_compile_args.append("-mmacosx-version-min=10.13")

    def _apply_link_flags(self, ext: Extension) -> None:
        """Apply platform-specific link flags to the extension."""
        if sys.platform == "linux":
            ext.extra_link_args += ["-Wl,-rpath,$ORIGIN"]
        elif sys.platform == "darwin":
            ext.extra_link_args += ["-rpath", "@loader_path"]

    def _run_build(self, extensions: list, src_root: Path) -> None:
        """Run the build using setuptools Distribution."""
        c_files_for_c99 = self.C_FILES_FOR_C99
        vendored_c_files = self.VENDORED_C_FILES
        is_unix = sys.platform in ("linux", "darwin")

        class CustomBuildExt(build_ext):
            """Custom build_ext that handles C files with C99 standard."""

            def build_extension(self, ext):
                original_compile = self.compiler._compile

                def new_compile(obj, src: str, ext_arg, cc_args, extra_postargs, pp_opts):
                    # Handle C files differently from C++ files (Unix only)
                    if is_unix and src.endswith(c_files_for_c99):
                        extra_postargs = [s for s in extra_postargs if s not in ("-std=c++17", "-std=c++11")]
                        extra_postargs.append("-std=c99")
                    # Suppress warnings for vendored/generated C files (Unix only, GCC/Clang flags)
                    if is_unix and src.endswith(vendored_c_files):
                        extra_postargs = list(extra_postargs)
                        extra_postargs.extend([
                            "-Wno-unused-variable",
                            "-Wno-unused-const-variable",
                            "-Wno-unreachable-code",
                        ])
                    return original_compile(obj, src, ext_arg, cc_args, extra_postargs, pp_opts)

                self.compiler._compile = new_compile

                try:
                    build_ext.build_extension(self, ext)
                finally:
                    self.compiler._compile = original_compile

        dist = Distribution({"ext_modules": extensions})
        dist.package_dir = {"": "src"}

        cmd = CustomBuildExt(dist)
        cmd.ensure_finalized()
        cmd.build_lib = str(src_root)
        cmd.inplace = True
        cmd.run()

    def _build_core(self) -> None:
        """Build the Rust core library in release mode for distribution."""

        if os.environ.get("SKIP_CORE_BUILD", "").lower() in ["true", "1"]:
            return
        # Get paths relative to the Python wrapper directory
        python_dir = Path(__file__).parent
        cargo_manifest = python_dir.parent / "Cargo.toml"
        target_dir = python_dir / "src" / "snowflake" / "connector" / "_core"

        # Ensure target directory exists
        target_dir.mkdir(parents=True, exist_ok=True)

        # Build the Rust core library in release mode with optimizations
        with TemporaryDirectory() as temp_dir:
            cargo_args = [
                "cargo",
                "build",
                "--release",
                "--package",
                "sf_core",
                "--manifest-path",
                str(cargo_manifest),
                "--target-dir",
                str(temp_dir),
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

            # Copy built artifacts from release directory to _core directory
            release_dir = Path(temp_dir) / "release"
            if not release_dir.exists():
                raise Exception("Core binary not present")
            for file in release_dir.rglob("*"):
                if file.is_file() and file.suffix in (".dylib", ".so", ".dll"):
                    shutil.copy2(file, target_dir)
