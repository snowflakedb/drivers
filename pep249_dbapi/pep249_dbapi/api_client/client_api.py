from ..protobuf_gen.database_driver_v1_services import DatabaseDriverClient
import ctypes
from .c_api import sf_core_api_call_proto
from ..protobuf_gen.proto_exception import ProtoTransportException

class ProtoTransport:
    def handle_message(self, api, method, message):
        response = ctypes.POINTER(ctypes.c_ubyte)()
        response_len = ctypes.c_size_t()
        api = ctypes.c_char_p(api.encode('utf-8'))
        method = ctypes.c_char_p(method.encode('utf-8'))
        message_buf = (ctypes.c_ubyte * len(message))()
        message_buf[:] = message
        code = sf_core_api_call_proto(api, method, ctypes.cast(message_buf, ctypes.POINTER(ctypes.c_ubyte)), len(message), ctypes.byref(response), ctypes.byref(response_len))
        if code == 0 or code == 1 or code == 2:
            return (code, bytes(response[:response_len.value]))

        raise ProtoTransportException(f"Unknown error code: {code}")
        

_DATABASE_DRIVER_CLIENT = None

def database_driver_client():
    global _DATABASE_DRIVER_CLIENT
    if _DATABASE_DRIVER_CLIENT is None:
        _DATABASE_DRIVER_CLIENT = DatabaseDriverClient(ProtoTransport())
    return _DATABASE_DRIVER_CLIENT
