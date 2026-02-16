from __future__ import annotations

import ctypes

from ctypes import c_char_p

from ..protobuf_gen.database_driver_v1_services import DatabaseDriverClient
from ..protobuf_gen.proto_exception import ProtoTransportException
from .c_api import sf_core_api_call_proto, sf_core_free_buffer


class ProtoTransport:
    def handle_message(self, api: str, method: str, message: bytes) -> tuple[int, bytes]:
        response = ctypes.POINTER(ctypes.c_ubyte)()
        response_len = ctypes.c_size_t()
        api_bytes: c_char_p = ctypes.c_char_p(api.encode("utf-8"))
        method_bytes: c_char_p = ctypes.c_char_p(method.encode("utf-8"))
        message_buf = (ctypes.c_ubyte * len(message))()
        message_buf[:] = message  # type: ignore
        code = sf_core_api_call_proto(
            api_bytes,
            method_bytes,
            ctypes.cast(message_buf, ctypes.POINTER(ctypes.c_ubyte)),
            len(message),
            ctypes.byref(response),
            ctypes.byref(response_len),
        )
        if code == 0 or code == 1 or code == 2:
            result = bytes(response[: response_len.value])
            sf_core_free_buffer(response, response_len.value)
            return (code, result)

        raise ProtoTransportException(f"Unknown error code: {code}")


_DATABASE_DRIVER_CLIENT: DatabaseDriverClient | None = None


def database_driver_client() -> DatabaseDriverClient:
    global _DATABASE_DRIVER_CLIENT
    if _DATABASE_DRIVER_CLIENT is None:
        _DATABASE_DRIVER_CLIENT = DatabaseDriverClient(ProtoTransport())
    return _DATABASE_DRIVER_CLIENT
