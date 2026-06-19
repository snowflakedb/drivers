"""Core library loading and shared FFI types."""

from __future__ import annotations

import ctypes
import os
import sys

from enum import Enum
from importlib import resources
from typing import TYPE_CHECKING


if TYPE_CHECKING:
    from typing import Any


_CORE_LIB_STEM = "sf_core"
_CORE_LIB_NAME = f"lib{_CORE_LIB_STEM}"


class CORE_API(Enum):
    DATABASE_DRIVER_API_V1 = 1


class CAPIHandle(ctypes.Structure):
    _fields_ = [("id", ctypes.c_int64), ("magic", ctypes.c_int64)]


def _get_core_path() -> Any:
    # Define the file name for each platform.
    # On Windows, cdylib crates produce "sf_core.dll" (no lib prefix).
    # On Unix, they produce "libsf_core.so" / "libsf_core.dylib".
    if sys.platform.startswith("win"):
        lib_name = f"{_CORE_LIB_STEM}.dll"
    elif sys.platform.startswith("darwin"):
        lib_name = f"{_CORE_LIB_NAME}.dylib"
    else:
        lib_name = f"{_CORE_LIB_NAME}.so"

    files = resources.files("snowflake.connector")
    return files.joinpath("_core").joinpath(lib_name)


def _load_core() -> ctypes.CDLL:
    path = _get_core_path()
    with resources.as_file(path) as lib_path:
        lib_path_str = str(lib_path)
        if sys.platform.startswith("win"):
            # ctypes.CDLL on Python 3.8+ uses restricted DLL search; register
            # _core/ so the Windows loader finds sf_core.dll's co-located deps.
            os.add_dll_directory(os.fspath(lib_path.parent))
        try:
            return ctypes.CDLL(lib_path_str)
        except OSError as err:
            raise OSError(f"Couldn't load core driver (path={lib_path_str})") from err


try:
    core = _load_core()
except OSError as err:
    raise RuntimeError("Couldn't load core driver dependency") from err
