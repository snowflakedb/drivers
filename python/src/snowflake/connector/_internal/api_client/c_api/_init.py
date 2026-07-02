"""Core initialization and logger callback."""

from __future__ import annotations

import ctypes
import logging

from typing import TYPE_CHECKING

from ...logging import get_sf_core_logger
from ._common import core


if TYPE_CHECKING:
    from typing import Any

LOGGER_CALLBACK = ctypes.CFUNCTYPE(
    ctypes.c_uint32, ctypes.c_uint32, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_uint32, ctypes.c_char_p
)
core.sf_core_init.argtypes = [LOGGER_CALLBACK]
core.sf_core_init.restype = ctypes.c_uint32


def sf_core_init(callback: Any) -> None:
    core.sf_core_init(callback)


level_map = {
    # sf_core level -> python logging level
    0: logging.ERROR,
    1: logging.WARNING,
    2: logging.INFO,
    3: logging.DEBUG,
}


def logger_callback(level: int, message: bytes, filename: bytes, line: int, function: bytes) -> int:
    py_level = level_map.get(level)
    if py_level is None:
        return 0

    sf_core_logger = get_sf_core_logger()
    if not sf_core_logger.isEnabledFor(py_level):
        return 0

    record = sf_core_logger.makeRecord(
        sf_core_logger.name,
        py_level,
        filename.decode("utf-8"),
        line,
        message.decode("utf-8"),
        (),
        None,
        func=function.decode("utf-8"),
    )
    sf_core_logger.handle(record)
    return 0


c_logger_callback = LOGGER_CALLBACK(logger_callback)


def register_default_logger_callback() -> None:
    """Register the default logger callback with the core API."""
    sf_core_init(c_logger_callback)
    logging.getLogger("sf_core").setLevel(logging.INFO)
