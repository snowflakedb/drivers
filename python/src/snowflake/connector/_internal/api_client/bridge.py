from __future__ import annotations

import asyncio
import ctypes

from ctypes import c_char_p
from typing import Any

from ..protobuf_gen.proto_exception import ProtoTransportException
from .c_api import (
    RESPONSE_CALLBACK,
    sf_core_api_call_proto,
    sf_core_api_call_proto_async,
    sf_core_api_cancel,
    sf_core_free_buffer,
)


class ProtoTransport:
    """Bridge between Python proto RPC calls and the Rust core FFI layer.

    :meth:`handle_message_async` - uses the callback-based async FFI.
        It creates a Future and builds a callback (C fn ptr).
        (Callback resolves the Future when Tokio tasks completes.)
        It submits the request to core and awaits the Future.

    :meth:`handle_message` - uses the blocking FFI call.

    Note on lifetime correctness: the C callback object **must** outlive the Rust task that may invoke it.
    We pin it to the Future so it stays alive until the Future is resolved.
    So, it's garbage-collected only after the awaiting coroutine resumes.
    """

    async def handle_message_async(self, api: str, method: str, message: bytes) -> tuple[int, bytes]:
        loop = asyncio.get_running_loop()
        future: asyncio.Future[tuple[int, bytes]] = loop.create_future()

        def on_response(
            user_data_ptr: int,
            status: int,
            response_ptr: Any,
            response_len: int,
        ) -> None:
            # Copy response bytes BEFORE returning. Rust frees the buffer next.
            # `string_at` is a single memcpy; avoids the O(n) Python __getitem__ loop you get from `bytes(ptr[:n])`
            response_bytes = ctypes.string_at(response_ptr, response_len)

            def _set() -> None:
                # The Future may have been canceled while Rust was working.
                if not future.done():
                    future.set_result((status, response_bytes))

            loop.call_soon_threadsafe(_set)

        callback_ref = RESPONSE_CALLBACK(on_response)

        # Pin the callback to the Future so it cannot be garbage-collected before Rust invokes it.
        # Without this, a coroutine that gets canceled could free the callback object while Rust still holds the fn ptr.
        #
        # Note: this creates a deliberate reference cycle (future -> callback_ref -> on_response closure -> future).
        # It is broken by the cycle GC after the Future resolves and the awaiting coroutine drops its reference.
        # Do not optimize this pin away thinking it is redundant.
        future._proto_transport_callback_ref = callback_ref  # type: ignore[attr-defined]

        message_buf = (ctypes.c_ubyte * len(message)).from_buffer_copy(message)

        async_handle = sf_core_api_call_proto_async(
            api.encode("utf-8"),
            method.encode("utf-8"),
            ctypes.cast(message_buf, ctypes.POINTER(ctypes.c_ubyte)),
            len(message),
            callback_ref,
            None,  # user_data not needed, as we capture future in the closure
        )

        try:
            status, response_bytes = await future
        except asyncio.CancelledError:
            sf_core_api_cancel(async_handle)
            raise

        if status in (0, 1, 2):
            return status, response_bytes

        raise ProtoTransportException(f"Unknown error code: {status}")

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
        if code in (0, 1, 2):
            result = bytes(response[: response_len.value])
            sf_core_free_buffer(response, response_len.value)
            return code, result

        raise ProtoTransportException(f"Unknown error code: {code}")
