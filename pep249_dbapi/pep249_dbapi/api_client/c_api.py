from enum import Enum
import ctypes
import logging

class CORE_API(Enum):
    DATABASE_DRIVER_API_V1 = 1

class CAPIHandle(ctypes.Structure):
    _fields_ = [("id", ctypes.c_int64), ("magic", ctypes.c_int64)]

try:
    import os
    if "CORE_PATH" not in os.environ:
        raise ImportError(
            "CORE_PATH environment variable not set. Set CORE_PATH to the built core library, e.g. ../target/debug/libsf_core.dylib"
        )
    core = ctypes.CDLL(os.environ["CORE_PATH"])
except OSError as e:
    print(f"Error loading library {e}")

core.sf_core_api_init.argtypes = [ctypes.c_uint]
core.sf_core_api_init.restype = CAPIHandle

core.sf_core_api_write.restype = ctypes.c_uint
core.sf_core_api_write.argtypes = [CAPIHandle, ctypes.c_char_p, ctypes.c_size_t]

core.sf_core_api_read.restype = ctypes.c_uint
core.sf_core_api_read.argtypes = [CAPIHandle, ctypes.c_char_p, ctypes.c_size_t]

core.sf_core_api_flush.restype = None
core.sf_core_api_flush.argtypes = [CAPIHandle]

LOGGER_CALLBACK = ctypes.CFUNCTYPE(ctypes.c_uint32, ctypes.c_uint32, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_uint32, ctypes.c_char_p)
core.sf_core_init_logger.argtypes = [LOGGER_CALLBACK]
core.sf_core_init_logger.restype = ctypes.c_uint32

def sf_core_api_read(channel, buf, len):
    core.sf_core_api_read(channel, buf, len)

def sf_core_api_write(channel, buf, len):
    core.sf_core_api_write(channel, buf, len)

def sf_core_api_flush(channel):
    core.sf_core_api_flush(channel)

def sf_core_api_init(api_id):
    if api_id not in CORE_API:
        raise ValueError(f"Invalid API ID: {api_id}")
    return core.sf_core_api_init(api_id.value)

def sf_core_init_logger(callback):
    core.sf_core_init_logger(callback)

level_map = {
    # sf_core level -> python logging level
    0: logging.ERROR,
    1: logging.WARNING,
    2: logging.INFO,
    3: logging.DEBUG,
    4: logging.NOTSET,
}

def logger_callback(level, message, filename, line, function):
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
