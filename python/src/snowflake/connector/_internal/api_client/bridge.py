from __future__ import annotations

import asyncio
import ctypes

from ctypes import c_char_p

from ..protobuf_gen.proto_exception import ProtoTransportException
from .c_api import sf_core_api_call_proto, sf_core_free_buffer


class ProtoTransport:
    """Serialize a protobuf RPC into a blocking FFI call.

    Exposes two entry points for the two driver code paths:

    - :meth:`handle_message` — the synchronous path used by the sync client.
      Calls into the FFI on the calling thread and blocks until it returns.
    - :meth:`handle_message_async` — the asynchronous path used by the async
      client. Offloads the same blocking FFI call to a worker thread via
      ``asyncio.to_thread`` so the event loop is not blocked.
    """

    async def handle_message_async(self, api: str, method: str, message: bytes) -> tuple[int, bytes]:
        # asyncio.to_thread hop is a temporary bridge until the FFI itself becomes non-blocking
        return await asyncio.to_thread(self.handle_message, api, method, message)

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
            return code, result

        raise ProtoTransportException(f"Unknown error code: {code}")
