import ctypes
import functools
import logging
import os
import sys

from enum import Enum
from importlib import resources
from typing import Any

from ..logging import get_sf_core_logger


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


# Performance instrumentation FFI bindings (see sf_core/src/c_api.rs).
# These symbols are always present in libsf_core; when the perf_timing feature
# is off they return empty/no-op results. Callers use sf_core_perf_enabled() to
# check whether real data is available.
core.sf_core_perf_enabled.argtypes = []
core.sf_core_perf_enabled.restype = ctypes.c_bool


class CoreInstrumentationData(ctypes.Structure):
    """Mirrors #[repr(C)] CoreInstrumentationData from sf_core::perf_timing."""

    _fields_ = [
        ("core_batch_wait_ns", ctypes.c_uint64),
        ("core_chunk_download_ns", ctypes.c_uint64),
        ("core_arrow_decode_ns", ctypes.c_uint64),
    ]


core.sf_core_get_perf_data.argtypes = []
core.sf_core_get_perf_data.restype = CoreInstrumentationData

core.sf_core_reset_perf_metrics.argtypes = []
core.sf_core_reset_perf_metrics.restype = None


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


@functools.lru_cache(maxsize=1)
def sf_core_perf_enabled() -> bool:
    return bool(core.sf_core_perf_enabled())


def sf_core_get_perf_data() -> dict[str, float]:
    """Atomically read-and-reset perf counters, returning seconds."""
    data: CoreInstrumentationData = core.sf_core_get_perf_data()
    return {
        "core_batch_wait_s": data.core_batch_wait_ns / 1e9,
        "core_chunk_download_s": data.core_chunk_download_ns / 1e9,
        "core_arrow_decode_s": data.core_arrow_decode_ns / 1e9,
    }


def sf_core_reset_perf_metrics() -> None:
    core.sf_core_reset_perf_metrics()
