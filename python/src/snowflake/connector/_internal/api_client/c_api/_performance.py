"""Performance instrumentation bindings."""

from __future__ import annotations

import ctypes
import functools

from ._common import core


# These symbols are always present in libsf_core; when the perf_timing feature
# is off they return empty/no-op results.
core.sf_core_perf_enabled.argtypes = []
core.sf_core_perf_enabled.restype = ctypes.c_bool


class CoreInstrumentationData(ctypes.Structure):
    """Mirrors #[repr(C)] CoreInstrumentationData from sf_core::perf_timing."""

    _fields_ = [
        ("core_batch_wait_ns", ctypes.c_uint64),
        ("core_chunk_download_ns", ctypes.c_uint64),
        ("core_arrow_decode_ns", ctypes.c_uint64),
    ]


core.sf_core_get_perf_data.argtypes = []
core.sf_core_get_perf_data.restype = CoreInstrumentationData

core.sf_core_reset_perf_metrics.argtypes = []
core.sf_core_reset_perf_metrics.restype = None


@functools.lru_cache(maxsize=1)
def sf_core_perf_enabled() -> bool:
    return bool(core.sf_core_perf_enabled())


def sf_core_get_perf_data() -> dict[str, float]:
    """Atomically read-and-reset perf counters, returning seconds."""
    data: CoreInstrumentationData = core.sf_core_get_perf_data()
    return {
        "core_batch_wait_s": data.core_batch_wait_ns / 1e9,
        "core_chunk_download_s": data.core_chunk_download_ns / 1e9,
        "core_arrow_decode_s": data.core_arrow_decode_ns / 1e9,
    }


def sf_core_reset_perf_metrics() -> None:
    core.sf_core_reset_perf_metrics()
