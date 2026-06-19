"""Synchronous proto API (blocks until complete)."""

from __future__ import annotations

import ctypes

from typing import TYPE_CHECKING

from ._common import core


if TYPE_CHECKING:
    from typing import Any

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
