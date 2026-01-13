#!/usr/bin/env python

import os
import sys
import warnings

from setuptools import Extension, setup
from setuptools.command.build_ext import build_ext

CONNECTOR_SRC_DIR = os.path.join("src", "snowflake", "ud_connector")
NANOARROW_SRC_DIR = os.path.join(CONNECTOR_SRC_DIR, "nanoarrow_cpp", "ArrowIterator")

# Parse command line flags
options_def = {
    "--debug",
}

options = {e.lstrip("-"): False for e in options_def}

for flag in options_def:
    if flag in sys.argv:
        options[flag.lstrip("-")] = True
        sys.argv.remove(flag)

extensions = None
cmd_class = {}

_POSITIVE_VALUES = ("y", "yes", "t", "true", "1", "on")
SNOWFLAKE_DISABLE_COMPILE_ARROW_EXTENSIONS = (
    os.environ.get("SNOWFLAKE_DISABLE_COMPILE_ARROW_EXTENSIONS", "false").lower()
    in _POSITIVE_VALUES
)

try:
    from Cython.Build import cythonize

    _ABLE_TO_COMPILE_EXTENSIONS = True
except ImportError:
    warnings.warn(
        "Cannot compile native C code, because of a missing build dependency (Cython)",
        stacklevel=1,
    )
    _ABLE_TO_COMPILE_EXTENSIONS = False

if _ABLE_TO_COMPILE_EXTENSIONS and not SNOWFLAKE_DISABLE_COMPILE_ARROW_EXTENSIONS:
    extensions = cythonize(
        [
            Extension(
                name="snowflake.ud_connector._arrow_batch_iterator",
                sources=[os.path.join(NANOARROW_SRC_DIR, "arrow_batch_iterator.pyx")],
                language="c++",
            ),
        ],
    )

    class MyBuildExt(build_ext):
        def build_extension(self, ext):
            if options["debug"]:
                ext.extra_compile_args.append("-g")
                ext.extra_link_args.append("-g")
                ext.extra_compile_args.append("-O0")
                ext.extra_link_args.append("-O0")
            current_dir = os.getcwd()

            if ext.name == "snowflake.ud_connector._arrow_batch_iterator":
                NANOARROW_CPP_SRC_DIR = os.path.join(CONNECTOR_SRC_DIR, "nanoarrow_cpp")
                NANOARROW_ARROW_ITERATOR_SRC_DIR = os.path.join(
                    NANOARROW_CPP_SRC_DIR, "ArrowIterator"
                )
                NANOARROW_LOGGING_SRC_DIR = os.path.join(
                    NANOARROW_CPP_SRC_DIR, "Logging"
                )

                ext.sources += [
                    os.path.join(
                        NANOARROW_ARROW_ITERATOR_SRC_DIR,
                        *((file,) if isinstance(file, str) else file),
                    )
                    for file in {
                        "ArrayConverter.cpp",
                        "BinaryConverter.cpp",
                        "BooleanConverter.cpp",
                        "CArrowBatchIterator.cpp",
                        "CArrowIterator.cpp",
                        "CArrowStreamIterator.cpp",
                        "CArrowTableIterator.cpp",
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
                        ("Python", "Common.cpp"),
                        ("Python", "Helpers.cpp"),
                        ("Util", "time.cpp"),
                    }
                ]
                ext.sources.append(
                    os.path.join(NANOARROW_LOGGING_SRC_DIR, "logging.cpp")
                )
                ext.include_dirs.append(NANOARROW_ARROW_ITERATOR_SRC_DIR)
                ext.include_dirs.append(NANOARROW_LOGGING_SRC_DIR)

                if sys.platform == "win32":
                    if not any("/std" in s for s in ext.extra_compile_args):
                        ext.extra_compile_args.append("/std:c++17")
                elif sys.platform == "linux" or sys.platform == "darwin":
                    if "std=" not in os.environ.get("CXXFLAGS", ""):
                        ext.extra_compile_args.append("-std=c++11")
                        ext.extra_compile_args.append("-D_GLIBCXX_USE_CXX11_ABI=0")
                    # Define endianness for flatcc
                    ext.extra_compile_args.append("-DFLATBUFFERS_LITTLEENDIAN=1")
                    ext.extra_compile_args.append("-DFLATBUFFERS_PROTOCOL_IS_LE=1")
                    if (
                        sys.platform == "darwin"
                        and "macosx-version-min" not in os.environ.get("CXXFLAGS", "")
                    ):
                        ext.extra_compile_args.append("-mmacosx-version-min=10.13")

                ext.library_dirs.append(
                    os.path.join(
                        current_dir, self.build_lib, "snowflake", "ud_connector"
                    )
                )

                if sys.platform == "linux":
                    ext.extra_link_args += ["-Wl,-rpath,$ORIGIN"]
                elif sys.platform == "darwin":
                    ext.extra_link_args += ["-rpath", "@loader_path"]

            original__compile = self.compiler._compile

            # the following is required by nanoarrow to compile c files
            def new__compile(obj, src: str, ext, cc_args, extra_postargs, pp_opts):
                if (
                    src.endswith("nanoarrow.c")
                    or src.endswith("nanoarrow_ipc.c")
                    or src.endswith("flatcc.c")
                ):
                    # Remove C++ standard flags and add C99 for C files
                    # Keep other flags like endianness defines
                    extra_postargs = [
                        s
                        for s in extra_postargs
                        if s not in ("-std=c++17", "-std=c++11")
                    ]
                    extra_postargs.append("-std=c99")
                return original__compile(
                    obj, src, ext, cc_args, extra_postargs, pp_opts
                )

            self.compiler._compile = new__compile

            try:
                build_ext.build_extension(self, ext)
            finally:
                self.compiler._compile = original__compile

    cmd_class = {"build_ext": MyBuildExt}

setup(
    ext_modules=extensions,
    cmdclass=cmd_class,
)
