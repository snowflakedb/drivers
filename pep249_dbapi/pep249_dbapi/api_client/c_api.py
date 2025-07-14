from enum import Enum
import ctypes

class CORE_API(Enum):
    DATABASE_DRIVER_API_V1 = 1

class CAPIHandle(ctypes.Structure):
    _fields_ = [("id", ctypes.c_int64), ("magic", ctypes.c_int64)]

try:
    import os
    if "CORE_PATH" not in os.environ:
        raise ValueError("CORE_PATH environment variable not set")
    core = ctypes.CDLL(os.environ["CORE_PATH"])
except OSError as e:
    print(f"Error loading library {e}")

core.sf_core_api_init.argtypes = [ctypes.c_uint]
core.sf_core_api_init.restype = CAPIHandle

core.sf_core_api_write.restype = ctypes.c_uint
core.sf_core_api_write.argtypes = [CAPIHandle, ctypes.c_char_p, ctypes.c_size_t]

core.sf_core_api_read.restype = ctypes.c_uint
core.sf_core_api_read.argtypes = [CAPIHandle, ctypes.c_char_p, ctypes.c_size_t]

core.sf_core_api_flush.restype = ctypes.c_uint
core.sf_core_api_flush.argtypes = [CAPIHandle]

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