"""Asynchronous proto API (callback-based, returns immediately)."""

from __future__ import annotations

import ctypes

from typing import TYPE_CHECKING

from ._common import core


if TYPE_CHECKING:
    from typing import Any

RESPONSE_CALLBACK = ctypes.CFUNCTYPE(
    None,  # return type (must not raise/unwind)
    ctypes.c_void_p,  # user_data
    ctypes.c_size_t,  # status (0=ok, 1=app error, 2=transport error)
    ctypes.POINTER(ctypes.c_ubyte),  # response_ptr
    ctypes.c_size_t,  # response_len
)
core.sf_core_api_call_proto_async.restype = ctypes.c_uint64
core.sf_core_api_call_proto_async.argtypes = [
    ctypes.c_char_p,  # const char* api
    ctypes.c_char_p,  # const char* method
    ctypes.POINTER(ctypes.c_ubyte),  # const uint8_t* request
    ctypes.c_size_t,  # size_t request_len
    RESPONSE_CALLBACK,  # callback
    ctypes.c_void_p,  # user_data
]
core.sf_core_api_cancel.restype = None
core.sf_core_api_cancel.argtypes = [ctypes.c_uint64]


def sf_core_api_call_proto_async(
    api: bytes,
    method: bytes,
    request: Any,
    request_len: int,
    callback: Any,
    user_data: Any,
) -> int:
    return core.sf_core_api_call_proto_async(api, method, request, request_len, callback, user_data)  # type: ignore


def sf_core_api_cancel(async_handle: int) -> None:
    core.sf_core_api_cancel(async_handle)
