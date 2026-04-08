"""Custom Hatch build hooks."""

from __future__ import annotations

import os
import platform
import shutil
import subprocess
import sys
import warnings

from contextlib import nullcontext
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
    """Build hook for compiling Cython extensions and generating protobuf code."""

    PLUGIN_NAME = "nanoarrow"

    # Relative paths from src/
    SRC_DIR = Path("src")
    CONNECTOR_DIR = Path("snowflake") / "connector"
    INTERNAL_DIR = CONNECTOR_DIR / "_internal"
    NANOARROW_CPP_DIR = INTERNAL_DIR / "nanoarrow_cpp"
    ARROW_ITERATOR_DIR = NANOARROW_CPP_DIR / "ArrowIterator"
    LOGGING_DIR = NANOARROW_CPP_DIR / "Logging"

    # Proto generation
    PROTO_INPUT = Path("protobuf") / "database_driver_v1.proto"
    PROTOBUF_GEN_DIR = SRC_DIR / INTERNAL_DIR / "protobuf_gen"

    # Extension module name
    EXTENSION_NAME = "snowflake.connector._internal.arrow_stream_iterator"
    PYX_SOURCE = ARROW_ITERATOR_DIR / "arrow_stream_iterator.pyx"

    # C++ source files in ArrowIterator directory
    CPP_SOURCES = [
        "ArrayConverter.cpp",
        "ArrowTableConverter.cpp",
        "BinaryConverter.cpp",
        "BooleanConverter.cpp",
        "CArrowIterator.cpp",
        "CArrowStreamIterator.cpp",
        "CArrowStreamTableIterator.cpp",
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
        """Initialize the build hook: generate protobuf code and compile extensions."""
        self._generate_protobuf()

        if self.target_name not in ("wheel", "editable"):
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

        build_data["pure_python"] = False
        build_data["infer_tag"] = True

    def _generate_protobuf(self) -> None:
        """Generate Python protobuf code using the Rust proto_generator binary."""
        python_dir = Path(self.root)
        proto_input = (python_dir / self.PROTO_INPUT).resolve()
        protobuf_gen_dir = python_dir / self.PROTOBUF_GEN_DIR

        if not proto_input.exists():
            raise RuntimeError(
                f"Proto file not found at {proto_input}. "
                "Cannot build package without protobuf definitions."
            )

        cargo_manifest = python_dir / "Cargo.toml"
        if not cargo_manifest.exists():
            # When Cargo.toml is missing (e.g. building a wheel from sdist without the
            # Rust toolchain), fall back to previously generated files if they exist.
            if any(protobuf_gen_dir.glob("*_pb2.py")):
                warnings.warn(
                    "Cargo.toml not found; skipping protobuf regeneration. "
                    "Using previously generated protobuf files.",
                    stacklevel=1,
                )
                return
            raise RuntimeError(
                "Cargo.toml not found. Cannot build proto_generator. "
                "Ensure Cargo.toml symlink exists (ln -s ../Cargo.toml Cargo.toml) "
                "or build from a pre-built wheel."
            )

        cargo_manifest = cargo_manifest.resolve()
        protobuf_gen_dir.mkdir(parents=True, exist_ok=True)

        # Ensure protoc-gen-mypy is discoverable by protoc for .pyi stub generation.
        # mypy-protobuf is declared in build-system.requires, but the plugin binary
        # may not be on PATH when cargo shells out to protoc.
        env = os.environ.copy()
        scripts_dir = str(Path(sys.executable).parent)
        env["PATH"] = scripts_dir + os.pathsep + env.get("PATH", "")

        # Use a stable target dir when PROTO_CARGO_TARGET_DIR is set (enables
        # incremental Rust compilation and CI caching). Otherwise fall back to a
        # temporary directory that is cleaned up after the build.
        stable_target = os.environ.get("PROTO_CARGO_TARGET_DIR")
        if stable_target:
            Path(stable_target).mkdir(parents=True, exist_ok=True)
            dir_ctx = nullcontext(stable_target)
        else:
            dir_ctx = TemporaryDirectory()

        with dir_ctx as target_dir:
            cargo_args = [
                "cargo",
                "run",
                "--bin",
                "proto_generator",
                "--manifest-path",
                str(cargo_manifest),
                "--target-dir",
                target_dir,
                "--",
                "--generator",
                "python",
                "--input",
                str(proto_input),
                "--output",
                str(protobuf_gen_dir),
            ]

            try:
                result = subprocess.run(
                    cargo_args,
                    check=True,
                    capture_output=True,
                    text=True,
                    env=env,
                )
                print(result.stdout)
            except subprocess.CalledProcessError as e:
                print(f"Proto generation failed with exit code {e.returncode}")
                print(f"stdout: {e.stdout}")
                print(f"stderr: {e.stderr}")
                raise

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
        # Uses symlinked Cargo.toml in dev (points to ../Cargo.toml) or actual file in sdist
        python_dir = Path(__file__).parent
        target_dir = python_dir / "src" / "snowflake" / "connector" / "_core"
        cargo_manifest = python_dir / "Cargo.toml"

        if not cargo_manifest.exists():
            warnings.warn(
                "Cargo.toml not found. Cannot build sf_core native extension. "
                "Ensure symlinks are set up (ln -s ../Cargo.toml Cargo.toml) or "
                "install from a pre-built wheel.",
                stacklevel=1,
            )
            return

        # Resolve symlinks so cargo uses the actual path (important for workspace resolution)
        cargo_manifest = cargo_manifest.resolve()

        # Ensure target directory exists
        target_dir.mkdir(parents=True, exist_ok=True)

        # Build the Rust core library in release mode with optimizations.
        # On Windows ARM64, disable strip — strip=true on a cdylib removes
        # the .pdata exception-unwind tables, causing WinError 127 at load time.
        extra_cargo_args: list[str] = []
        if sys.platform == "win32" and platform.machine() == "ARM64":
            extra_cargo_args = ["--config", "profile.release.strip=false"]

        # Use a stable target dir when CORE_CARGO_TARGET_DIR is set (enables
        # incremental Rust compilation and CI caching). Otherwise fall back to a
        # temporary directory that is cleaned up after the build.
        stable_target = os.environ.get("CORE_CARGO_TARGET_DIR")
        if stable_target:
            Path(stable_target).mkdir(parents=True, exist_ok=True)
            dir_ctx = nullcontext(stable_target)
        else:
            dir_ctx = TemporaryDirectory()

        with dir_ctx as build_dir:
            cargo_args = [
                "cargo",
                "build",
                "--release",
                "--package",
                "sf_core",
                "--manifest-path",
                str(cargo_manifest),
                "--target-dir",
                str(build_dir),
                *extra_cargo_args,
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

            # Copy built artifacts from release directory to _core directory.
            release_dir = Path(build_dir) / "release"
            if not release_dir.exists():
                raise Exception("Core binary not present")
            # Use iterdir(), not rglob() — release/deps/ contains proc-macro DLLs
            # built for the host (build-time tools, not runtime deps).
            # Bundling them into the wheel causes spurious load errors.
            found_core = False
            for file in release_dir.iterdir():
                if file.is_file() and file.suffix in (".dylib", ".so", ".dll"):
                    dest = target_dir / file.name
                    tmp = target_dir / (file.name + ".tmp")
                    shutil.copy2(file, tmp)
                    os.replace(tmp, dest)
                    found_core = True
            if not found_core:
                raise Exception(
                    f"No shared library (.dll/.so/.dylib) found in {release_dir}"
                )

            # sf_core.dll is built with OPENSSL_STATIC=1 (arm64-windows-static-md triplet),
            # which embeds OpenSSL at link time — sf_core.dll has no runtime OpenSSL DLL dep.
            # Dynamic OpenSSL (arm64-windows triplet) would require these DLLs to be
            # co-located with sf_core.dll, because Python 3.8+ ctypes uses restricted DLL
            # search (LOAD_LIBRARY_SEARCH_DEFAULT_DIRS) that does NOT include PATH.
            # The bundling code below is a safety net for non-static builds.
            if sys.platform == "win32":
                openssl_bin = os.environ.get("OPENSSL_DIR", "")
                if openssl_bin:
                    openssl_bin_dir = Path(openssl_bin) / "bin"
                    if openssl_bin_dir.is_dir():
                        for dll in openssl_bin_dir.glob("*.dll"):
                            shutil.copy2(dll, target_dir)
                            print(f"Bundled OpenSSL DLL: {dll.name}")
