import sys
from enum import Enum
import ctypes
import logging
from importlib import resources


_CORE_LIB_NAME = "libsf_core"


class CORE_API(Enum):
    DATABASE_DRIVER_API_V1 = 1


class CAPIHandle(ctypes.Structure):
    _fields_ = [("id", ctypes.c_int64), ("magic", ctypes.c_int64)]


def _get_core_path():
    # Define the file name for each platform
    if sys.platform.startswith("win"):
        lib_name = f"{_CORE_LIB_NAME}.dll"
    elif sys.platform.startswith("darwin"):
        lib_name = f"{_CORE_LIB_NAME}.dylib"
    else:
        lib_name = f"{_CORE_LIB_NAME}.so"

    files = resources.files("snowflake.ud_connector._core")
    return files.joinpath(lib_name)

def _load_core() -> ctypes.CDLL:
    # This context manager is the safe way to get a
    # file path from importlib.resources. It handles cases
    # where the file is inside a zip and needs to be extracted
    # to a temporary location.
    path = _get_core_path()
    with resources.as_file(path) as lib_path:
        core = ctypes.CDLL(str(lib_path))
    return core


try:
    core = _load_core()
except OSError as e:
    raise RuntimeError("Missing core driver dependency")

LOGGER_CALLBACK = ctypes.CFUNCTYPE(ctypes.c_uint32, ctypes.c_uint32, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_uint32, ctypes.c_char_p)
core.sf_core_init_logger.argtypes = [LOGGER_CALLBACK]
core.sf_core_init_logger.restype = ctypes.c_uint32

core.sf_core_api_call_proto.restype = ctypes.c_uint32
core.sf_core_api_call_proto.argtypes = [
    ctypes.c_char_p,  # const char* api
    ctypes.c_char_p,  # const char* method
    ctypes.POINTER(ctypes.c_ubyte),  # const char* request
    ctypes.c_size_t,  # size_t request_len
    ctypes.POINTER(ctypes.POINTER(ctypes.c_ubyte)),  # char* const* response
    ctypes.POINTER(ctypes.c_size_t)  # size_t* response_len
]

def sf_core_api_call_proto(api, method, request, request_len, response, response_len):
    return core.sf_core_api_call_proto(api, method, request, request_len, response, response_len)

def sf_core_init_logger(callback):
    core.sf_core_init_logger(callback)

level_map = {
    # sf_core level -> python logging level
    0: logging.ERROR,
    1: logging.WARNING,
    2: logging.INFO,
    3: logging.DEBUG,
}

def logger_callback(level, message, filename, line, function):
    if level not in level_map:
        return 0
    logger = logging.getLogger("sf_core")
    record = logger.makeRecord("sf_core", level_map[level], filename.decode('utf-8'), line, message.decode('utf-8'), [], None, func=function.decode('utf-8'))
    logger.handle(record)
    return 0

c_logger_callback = LOGGER_CALLBACK(logger_callback)

def register_default_logger_callback():
    """
    Registers the default logger callback with the core API.
    Call this function explicitly to set up logging.
    """
    sf_core_init_logger(c_logger_callback)
