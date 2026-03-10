import ctypes
import logging
import sys

from enum import Enum
from importlib import resources
from typing import Any

from ..logging import get_sf_core_logger


_CORE_LIB_NAME = "libsf_core"


class CORE_API(Enum):
    DATABASE_DRIVER_API_V1 = 1


class CAPIHandle(ctypes.Structure):
    _fields_ = [("id", ctypes.c_int64), ("magic", ctypes.c_int64)]


def _get_core_path() -> Any:
    # Define the file name for each platform.
    # On Windows, cdylib crates produce "sf_core.dll" (no lib prefix).
    # On Unix, they produce "libsf_core.so" / "libsf_core.dylib".
    if sys.platform.startswith("win"):
        lib_name = "sf_core.dll"
    elif sys.platform.startswith("darwin"):
        lib_name = f"{_CORE_LIB_NAME}.dylib"
    else:
        lib_name = f"{_CORE_LIB_NAME}.so"

    files = resources.files("snowflake.connector")
    return files.joinpath("_core").joinpath(lib_name)


def _load_core() -> ctypes.CDLL:
    path = _get_core_path()
    with resources.as_file(path) as lib_path:
        if sys.platform.startswith("win"):
            import os
            import struct

            dll_dir = os.fspath(lib_path.parent)
            os.add_dll_directory(dll_dir)

            # --- TEMPORARY DIAGNOSTIC (remove after Windows ARM64 is green) ---
            # Log the DLL's PE machine type and try pre-loading dependencies
            # to isolate which import causes WinError 127.
            try:
                with open(str(lib_path), "rb") as f:
                    f.seek(0x3C)
                    pe_offset = struct.unpack("<I", f.read(4))[0]
                    f.seek(pe_offset + 4)
                    machine = struct.unpack("<H", f.read(2))[0]
                    arch_map = {0x8664: "x86_64", 0xAA64: "ARM64", 0x14C: "x86"}
                    logging.getLogger(__name__).warning(
                        "sf_core.dll PE machine: 0x%X (%s)", machine, arch_map.get(machine, "unknown")
                    )
            except Exception:
                pass

            # Try pre-loading known dependencies to find which one fails
            for dep_name in ["libcrypto-3-arm64.dll", "libssl-3-arm64.dll"]:
                dep_path = os.path.join(dll_dir, dep_name)
                if os.path.exists(dep_path):
                    try:
                        ctypes.CDLL(dep_path)
                        logging.getLogger(__name__).warning("Pre-loaded OK: %s", dep_name)
                    except OSError as dep_err:
                        logging.getLogger(__name__).warning("Pre-load FAILED: %s: %s", dep_name, dep_err)
            # --- END TEMPORARY DIAGNOSTIC ---

        core = ctypes.CDLL(str(lib_path))
    return core


try:
    core = _load_core()
except OSError as err:
    core_path = _get_core_path()
    msg = f"Couldn't load core driver dependency: {err} (path={core_path})"
    raise RuntimeError(msg) from err

LOGGER_CALLBACK = ctypes.CFUNCTYPE(
    ctypes.c_uint32, ctypes.c_uint32, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_uint32, ctypes.c_char_p
)
core.sf_core_init_logger.argtypes = [LOGGER_CALLBACK]
core.sf_core_init_logger.restype = ctypes.c_uint32

core.sf_core_api_call_proto.restype = ctypes.c_uint32
core.sf_core_api_call_proto.argtypes = [
    ctypes.c_char_p,  # const char* api
    ctypes.c_char_p,  # const char* method
    ctypes.POINTER(ctypes.c_ubyte),  # const char* request
    ctypes.c_size_t,  # size_t request_len
    ctypes.POINTER(ctypes.POINTER(ctypes.c_ubyte)),  # char* const* response
    ctypes.POINTER(ctypes.c_size_t),  # size_t* response_len
]

core.sf_core_free_buffer.restype = None
core.sf_core_free_buffer.argtypes = [
    ctypes.POINTER(ctypes.c_ubyte),  # uint8_t* buffer
    ctypes.c_size_t,  # size_t len
]


def sf_core_api_call_proto(
    api: ctypes.c_char_p,
    method: ctypes.c_char_p,
    request: Any,
    request_len: int,
    response: Any,
    response_len: Any,
) -> int:
    return core.sf_core_api_call_proto(api, method, request, request_len, response, response_len)  # type: ignore


def sf_core_free_buffer(buffer: Any, length: int) -> None:
    core.sf_core_free_buffer(buffer, length)


def sf_core_init_logger(callback: Any) -> None:
    core.sf_core_init_logger(callback)


level_map = {
    # sf_core level -> python logging level
    0: logging.ERROR,
    1: logging.WARNING,
    2: logging.INFO,
    3: logging.DEBUG,
}


def logger_callback(level: int, message: bytes, filename: bytes, line: int, function: bytes) -> int:
    py_level = level_map.get(level)
    if py_level is None:
        return 0

    sf_core_logger = get_sf_core_logger()
    # Respect the logger's configured level - skip if not enabled
    if not sf_core_logger.isEnabledFor(py_level):
        return 0

    record = sf_core_logger.makeRecord(
        sf_core_logger.name,
        py_level,
        filename.decode("utf-8"),
        line,
        message.decode("utf-8"),
        (),
        None,
        func=function.decode("utf-8"),
    )
    sf_core_logger.handle(record)
    return 0


c_logger_callback = LOGGER_CALLBACK(logger_callback)


def register_default_logger_callback() -> None:
    """
    Register the default logger callback with the core API.
    Call this function explicitly to set up logging.
    """
    sf_core_init_logger(c_logger_callback)
