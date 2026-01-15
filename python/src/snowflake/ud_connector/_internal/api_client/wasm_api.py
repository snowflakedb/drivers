"""WASM-based API client using wasmtime.

This module provides an alternative to the native library (c_api.py) using
a WASM component loaded via wasmtime. This enables portable distribution
without platform-specific native libraries.

Usage:
    # Set environment variable to use WASM transport
    export SNOWFLAKE_USE_WASM=1
    
    # Or programmatically:
    from snowflake.ud_connector._internal.api_client import wasm_api
    wasm_api.enable_wasm_transport()
"""

import os
import logging
from importlib import resources
from typing import Optional, Tuple

logger = logging.getLogger(__name__)

# Lazy import wasmtime to avoid import errors when not using WASM
_wasmtime = None
_wasm_store = None
_wasm_instance = None

_WASM_FILE_NAME = "sf_core_wasm.wasm"


def _get_wasmtime():
    """Lazy import wasmtime."""
    global _wasmtime
    if _wasmtime is None:
        try:
            import wasmtime
            _wasmtime = wasmtime
        except ImportError:
            raise ImportError(
                "wasmtime is required for WASM transport. "
                "Install it with: pip install wasmtime"
            )
    return _wasmtime


def _get_wasm_path():
    """Get the path to the WASM component file."""
    # Look in the _core directory alongside the native library
    files = resources.files("snowflake.ud_connector._core")
    return files.joinpath(_WASM_FILE_NAME)


def _load_wasm_component():
    """Load and instantiate the WASM component."""
    global _wasm_store, _wasm_instance
    
    if _wasm_instance is not None:
        return _wasm_instance
    
    wasmtime = _get_wasmtime()
    
    # Get the WASM file path
    wasm_path = _get_wasm_path()
    
    with resources.as_file(wasm_path) as path:
        logger.info(f"Loading WASM component from: {path}")
        
        # Create engine with WASI support
        config = wasmtime.Config()
        config.wasm_component_model = True
        engine = wasmtime.Engine(config)
        
        # Create store with WASI context
        linker = wasmtime.Linker(engine)
        linker.define_wasi()
        
        # Configure WASI
        wasi_config = wasmtime.WasiConfig()
        wasi_config.inherit_stdout()
        wasi_config.inherit_stderr()
        wasi_config.inherit_env()
        
        _wasm_store = wasmtime.Store(engine)
        _wasm_store.set_wasi(wasi_config)
        
        # Load the component
        component = wasmtime.Component.from_file(engine, str(path))
        
        # Instantiate
        _wasm_instance = linker.instantiate(_wasm_store, component)
        
        logger.info("WASM component loaded successfully")
        return _wasm_instance


def sf_core_api_call_proto_wasm(api: str, method: str, request: bytes) -> Tuple[int, bytes]:
    """Call the sf_core API via WASM.
    
    This is the WASM equivalent of sf_core_api_call_proto from c_api.py.
    
    Args:
        api: The API name (e.g., "DatabaseDriver")
        method: The method name (e.g., "Connect")
        request: Protobuf-encoded request data
        
    Returns:
        Tuple of (return_code, response_bytes) where:
        - return_code 0 = success
        - return_code 1 = application error
        - return_code 2 = transport error
    """
    instance = _load_wasm_component()
    
    # Get the api-call export
    # The function signature from WIT:
    # api-call: func(api: string, method: string, request: list<u8>) -> result<list<u8>, list<u8>>
    api_call = instance.exports(_wasm_store)["snowflake:driver/api"]["api-call"]
    
    # Call the WASM function
    result = api_call(_wasm_store, api, method, list(request))
    
    # Handle the result based on the variant type
    # The WIT definition returns: variant api-result { ok(list<u8>), application-error(list<u8>), transport-error(string) }
    if hasattr(result, 'ok') and result.ok is not None:
        return (0, bytes(result.ok))
    elif hasattr(result, 'application_error') and result.application_error is not None:
        return (1, bytes(result.application_error))
    elif hasattr(result, 'transport_error') and result.transport_error is not None:
        return (2, result.transport_error.encode('utf-8'))
    else:
        # Fallback for different wasmtime API versions
        if isinstance(result, tuple):
            variant_name, value = result
            if variant_name == "ok":
                return (0, bytes(value))
            elif variant_name == "application-error":
                return (1, bytes(value))
            elif variant_name == "transport-error":
                return (2, value.encode('utf-8') if isinstance(value, str) else value)
        
        raise RuntimeError(f"Unexpected WASM result format: {result}")


def sf_core_init_logger_wasm(level: int = 2) -> int:
    """Initialize the WASM logger.
    
    Args:
        level: Log level (0=error, 1=warn, 2=info, 3=debug, 4=trace)
        
    Returns:
        0 on success, non-zero on failure
    """
    instance = _load_wasm_component()
    
    init_logger = instance.exports(_wasm_store)["snowflake:driver/api"]["init-logger"]
    return init_logger(_wasm_store, level)


def get_version_wasm() -> str:
    """Get the WASM driver version."""
    instance = _load_wasm_component()
    
    get_version = instance.exports(_wasm_store)["snowflake:driver/api"]["get-version"]
    return get_version(_wasm_store)


def is_wasm_available() -> bool:
    """Check if WASM transport is available."""
    try:
        _get_wasmtime()
        wasm_path = _get_wasm_path()
        with resources.as_file(wasm_path) as path:
            return path.exists()
    except (ImportError, FileNotFoundError):
        return False


def is_wasm_enabled() -> bool:
    """Check if WASM transport is enabled via environment variable."""
    return os.environ.get("SNOWFLAKE_USE_WASM", "").lower() in ("1", "true", "yes")


def enable_wasm_transport():
    """Enable WASM transport programmatically."""
    os.environ["SNOWFLAKE_USE_WASM"] = "1"


def disable_wasm_transport():
    """Disable WASM transport programmatically."""
    os.environ.pop("SNOWFLAKE_USE_WASM", None)


# Export compatibility with c_api module
__all__ = [
    "sf_core_api_call_proto_wasm",
    "sf_core_init_logger_wasm", 
    "get_version_wasm",
    "is_wasm_available",
    "is_wasm_enabled",
    "enable_wasm_transport",
    "disable_wasm_transport",
]

