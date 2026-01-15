"""API client for sf_core with support for both native and WASM transports.

The transport is selected based on the SNOWFLAKE_USE_WASM environment variable:
- SNOWFLAKE_USE_WASM=1: Use WASM transport (requires wasmtime)
- Otherwise: Use native library transport (default)
"""

from ..protobuf_gen.database_driver_v1_services import DatabaseDriverClient
import ctypes
import logging
from ..protobuf_gen.proto_exception import ProtoTransportException

logger = logging.getLogger(__name__)


def _use_wasm_transport() -> bool:
    """Check if WASM transport should be used."""
    try:
        from . import wasm_api
        return wasm_api.is_wasm_enabled() and wasm_api.is_wasm_available()
    except ImportError:
        return False


class NativeTransport:
    """Transport using the native C library via ctypes."""
    
    def __init__(self):
        from .c_api import sf_core_api_call_proto
        self._api_call = sf_core_api_call_proto
    
    def handle_message(self, api, method, message):
        response = ctypes.POINTER(ctypes.c_ubyte)()
        response_len = ctypes.c_size_t()
        api_bytes = ctypes.c_char_p(api.encode('utf-8'))
        method_bytes = ctypes.c_char_p(method.encode('utf-8'))
        message_buf = (ctypes.c_ubyte * len(message))()
        message_buf[:] = message
        code = self._api_call(
            api_bytes, 
            method_bytes, 
            ctypes.cast(message_buf, ctypes.POINTER(ctypes.c_ubyte)), 
            len(message), 
            ctypes.byref(response), 
            ctypes.byref(response_len)
        )
        if code == 0 or code == 1 or code == 2:
            return (code, bytes(response[:response_len.value]))

        raise ProtoTransportException(f"Unknown error code: {code}")


class WasmTransport:
    """Transport using the WASM component via wasmtime."""
    
    def __init__(self):
        from . import wasm_api
        self._api_call = wasm_api.sf_core_api_call_proto_wasm
    
    def handle_message(self, api, method, message):
        code, response = self._api_call(api, method, message)
        if code == 0 or code == 1 or code == 2:
            return (code, response)
        
        raise ProtoTransportException(f"Unknown error code: {code}")


class ProtoTransport:
    """Auto-selecting transport that uses WASM or native based on configuration."""
    
    def __init__(self):
        if _use_wasm_transport():
            logger.info("Using WASM transport for sf_core")
            self._transport = WasmTransport()
        else:
            logger.debug("Using native transport for sf_core")
            self._transport = NativeTransport()
    
    def handle_message(self, api, method, message):
        return self._transport.handle_message(api, method, message)


_DATABASE_DRIVER_CLIENT = None


def database_driver_client():
    """Get the singleton database driver client."""
    global _DATABASE_DRIVER_CLIENT
    if _DATABASE_DRIVER_CLIENT is None:
        _DATABASE_DRIVER_CLIENT = DatabaseDriverClient(ProtoTransport())
    return _DATABASE_DRIVER_CLIENT


def reset_client():
    """Reset the singleton client (useful for testing or reconfiguration)."""
    global _DATABASE_DRIVER_CLIENT
    _DATABASE_DRIVER_CLIENT = None
