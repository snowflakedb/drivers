"""Core initialization and logger callback."""

from __future__ import annotations

import ctypes
import logging

from typing import TYPE_CHECKING

from ...logging import _get_sf_core_stdlib_logger
from ...logging.config import LoggingConfiguration
from ._common import core


if TYPE_CHECKING:
    from typing import Any

LOGGER_CALLBACK = ctypes.CFUNCTYPE(
    ctypes.c_uint32,
    ctypes.c_uint32,
    ctypes.c_char_p,
    ctypes.c_char_p,
    ctypes.c_uint32,
    ctypes.c_char_p,
    ctypes.c_char_p,
)


class SfCoreInitResult(ctypes.Structure):
    _fields_ = [
        ("status", ctypes.c_uint32),
        ("troubleshooting_enabled", ctypes.c_uint32),
    ]


core.sf_core_init.argtypes = [LOGGER_CALLBACK]
core.sf_core_init.restype = SfCoreInitResult

core.sf_core_log_event.argtypes = [
    ctypes.c_uint32,
    ctypes.c_char_p,
    ctypes.c_char_p,
    ctypes.c_uint32,
    ctypes.c_char_p,
    ctypes.c_char_p,
]
core.sf_core_log_event.restype = ctypes.c_uint32


def sf_core_init(callback: Any) -> SfCoreInitResult:
    return core.sf_core_init(callback)  # type: ignore[no-any-return]


def sf_core_log_event(
    *,
    level: int,
    message: str,
    file: str,
    line: int,
    function: str,
    logger_name: str,
) -> int:
    """Send a wrapper log event to sf_core; return its status code.

    ``0`` means the event was accepted by the tracing pipeline; any non-zero
    value means it was not delivered (e.g. ``sf_core_init`` has not run yet).
    """
    return int(
        core.sf_core_log_event(
            level,
            message.encode("utf-8"),
            file.encode("utf-8"),
            line,
            function.encode("utf-8"),
            logger_name.encode("utf-8"),
        )
    )


def _sf_core_level_to_python(level: int) -> int | None:
    """Map sf_core wire level to stdlib logging level.

    DEBUG is the finest level Python supports. Core's ``normalize_event`` collapses
    finer levels (Rust ``tracing::trace!``, legacy TRACE) to wire level 3 before
    the callback, so level 4 should not normally reach this path. The ``>= 3``
    branch is defensive: any wire level 3 or higher is delivered as DEBUG rather
    than dropped.
    """
    if level >= 3:
        return logging.DEBUG
    match level:
        case 0:
            return logging.ERROR
        case 1:
            return logging.WARNING
        case 2:
            return logging.INFO
        case _:
            return None


def logger_callback(
    level: int,
    message: bytes,
    filename: bytes,
    line: int,
    function: bytes,
    logger_name: bytes,
) -> int:
    py_level = _sf_core_level_to_python(level)
    if py_level is None:
        return 0

    name = logger_name.decode("utf-8")
    target_logger = logging.getLogger(name) if name else _get_sf_core_stdlib_logger()

    if not target_logger.isEnabledFor(py_level):
        return 0

    record = target_logger.makeRecord(
        target_logger.name,
        py_level,
        filename.decode("utf-8"),
        line,
        message.decode("utf-8"),
        (),
        None,
        func=function.decode("utf-8"),
    )
    target_logger.handle(record)
    return 0


c_logger_callback = LOGGER_CALLBACK(logger_callback)


def register_default_logger_callback() -> None:
    """Register the default logger callback with the core API."""
    result = sf_core_init(c_logger_callback)
    if result.status != 0:
        msg = f"sf_core_init failed (status={result.status})"
        raise RuntimeError(msg)

    LoggingConfiguration.initialize(troubleshooting_enabled=bool(result.troubleshooting_enabled))
