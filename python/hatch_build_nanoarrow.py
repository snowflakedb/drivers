"""Custom Hatch build hook for building Cython extensions with nanoarrow C++ code."""

from __future__ import annotations

import os
import sys
import warnings
from pathlib import Path
from typing import Any

from hatchling.builders.hooks.plugin.interface import BuildHookInterface


class CythonBuildHook(BuildHookInterface):
    """Build hook for compiling Cython extensions with custom C++ sources."""

    PLUGIN_NAME = "cython"

    def initialize(self, version: str, build_data: dict[str, Any]) -> None:
        """Initialize the build hook and compile extensions."""
        if self.target_name != "wheel":
            return

        # Check if compilation should be disabled
        positive_values = ("y", "yes", "t", "true", "1", "on")
        if (
            os.environ.get(
                "SNOWFLAKE_DISABLE_COMPILE_ARROW_EXTENSIONS", "false"
            ).lower()
            in positive_values
        ):
            return

        try:
            self._build_extensions()
        except ImportError as e:
            warnings.warn(
                f"Cannot compile native C code, because of a missing build dependency: {e}",
                stacklevel=1,
            )

    def _build_extensions(self) -> None:
        """Build the Cython extensions."""
        from Cython.Build import cythonize
        from setuptools import Distribution, Extension
        from setuptools.command.build_ext import build_ext

        # Define paths
        src_root = Path(self.root) / "src"
        connector_dir = src_root / "snowflake" / "ud_connector"
        internal_dir = connector_dir / "_internal"
        nanoarrow_cpp_dir = internal_dir / "nanoarrow_cpp"
        arrow_iterator_dir = nanoarrow_cpp_dir / "ArrowIterator"
        logging_dir = nanoarrow_cpp_dir / "Logging"

        # Define the extension
        ext = Extension(
            name="snowflake.ud_connector._internal.arrow_stream_iterator",
            sources=[str(arrow_iterator_dir / "arrow_stream_iterator.pyx")],
            language="c++",
        )

        # Add C++ source files
        cpp_sources = [
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

        # Add subdirectory sources
        subdirectory_sources = [
            ("Python", "Common.cpp"),
            ("Python", "Helpers.cpp"),
            ("Util", "time.cpp"),
        ]

        for src in cpp_sources:
            ext.sources.append(str(arrow_iterator_dir / src))

        for subdir, filename in subdirectory_sources:
            ext.sources.append(str(arrow_iterator_dir / subdir / filename))

        # Add logging source
        ext.sources.append(str(logging_dir / "logging.cpp"))

        # Add include directories
        ext.include_dirs.append(str(arrow_iterator_dir))
        ext.include_dirs.append(str(logging_dir))

        # Platform-specific compile flags
        if sys.platform == "win32":
            if not any("/std" in s for s in ext.extra_compile_args):
                ext.extra_compile_args.append("/std:c++17")
        elif sys.platform in ("linux", "darwin"):
            if "std=" not in os.environ.get("CXXFLAGS", ""):
                ext.extra_compile_args.extend(
                    ["-std=c++11", "-D_GLIBCXX_USE_CXX11_ABI=0"]
                )
            # Define endianness for flatcc
            ext.extra_compile_args.extend(
                [
                    "-DFLATBUFFERS_LITTLEENDIAN=1",
                    "-DFLATBUFFERS_PROTOCOL_IS_LE=1",
                ]
            )
            if sys.platform == "darwin" and "macosx-version-min" not in os.environ.get(
                "CXXFLAGS", ""
            ):
                ext.extra_compile_args.append("-mmacosx-version-min=10.13")

        # Platform-specific link flags
        if sys.platform == "linux":
            ext.extra_link_args += ["-Wl,-rpath,$ORIGIN"]
        elif sys.platform == "darwin":
            ext.extra_link_args += ["-rpath", "@loader_path"]

        # Cythonize the extension
        extensions = cythonize([ext])

        # Create custom build_ext command
        class CustomBuildExt(build_ext):
            def build_extension(self, ext):
                original_compile = self.compiler._compile

                def new_compile(
                    obj, src: str, ext_arg, cc_args, extra_postargs, pp_opts
                ):
                    # Handle C files differently from C++ files
                    if src.endswith(("nanoarrow.c", "nanoarrow_ipc.c", "flatcc.c")):
                        extra_postargs = [
                            s
                            for s in extra_postargs
                            if s not in ("-std=c++17", "-std=c++11")
                        ]
                        extra_postargs.append("-std=c99")
                    return original_compile(
                        obj, src, ext_arg, cc_args, extra_postargs, pp_opts
                    )

                self.compiler._compile = new_compile

                try:
                    build_ext.build_extension(self, ext)
                finally:
                    self.compiler._compile = original_compile

        # Build using setuptools Distribution
        dist = Distribution({"ext_modules": extensions})
        dist.package_dir = {"": "src"}

        cmd = CustomBuildExt(dist)
        cmd.ensure_finalized()

        # Set the build directory to be inside _internal so the .so file is placed correctly
        cmd.build_lib = str(src_root)
        cmd.inplace = True

        cmd.run()
