"""FFI bindings to libsf_core, split by concern."""

from ._async import RESPONSE_CALLBACK, sf_core_api_call_proto_async
from ._common import CORE_API, CAPIHandle, core
from ._init import (
    LOGGER_CALLBACK,
    c_logger_callback,
    logger_callback,
    register_default_logger_callback,
    sf_core_init,
)
from ._performance import (
    CoreInstrumentationData,
    sf_core_get_perf_data,
    sf_core_perf_enabled,
    sf_core_reset_perf_metrics,
)
from ._sync import sf_core_api_call_proto, sf_core_free_buffer


__all__ = [
    "CORE_API",
    "CAPIHandle",
    "CoreInstrumentationData",
    "LOGGER_CALLBACK",
    "RESPONSE_CALLBACK",
    "c_logger_callback",
    "core",
    "logger_callback",
    "register_default_logger_callback",
    "sf_core_api_call_proto",
    "sf_core_api_call_proto_async",
    "sf_core_free_buffer",
    "sf_core_get_perf_data",
    "sf_core_init",
    "sf_core_perf_enabled",
    "sf_core_reset_perf_metrics",
]
