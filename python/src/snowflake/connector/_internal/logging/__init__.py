"""Logging for the snowflake.connector module.

This package owns **configuration** of the ``snowflake.connector`` and
``snowflake.connector._core`` stdlib loggers (``setup_logging`` and the internal
``_get_*_stdlib_logger`` accessors), including the ``sf_core`` FFI callback
logger, plus the public :func:`get_logger` entry point that wrapper code uses.

Round-trip logging (:class:`.core_logger.CoreLogger`, which sends wrapper logs
to sf_core over a direct FFI call and receives them back through the FFI
callback) lives in :mod:`.core_logger` so its FFI import stays off this
module's load path. See ``doc/logging/logging-architecture.md``.
"""

from __future__ import annotations

import functools

from typing import TYPE_CHECKING

from .config import (
    CONNECTOR_LOGGER_NAME,
    SF_CORE_LOGGER_NAME,
    _get_connector_stdlib_logger,
    _get_sf_core_stdlib_logger,
    _needs_handler,
    setup_logging,
)
from .native_extension_logger import NativeExtensionLogger, get_native_extension_logger


if TYPE_CHECKING:
    from .core_logger import CoreLogger


@functools.cache
def get_logger(name: str) -> CoreLogger:
    from .core_logger import CoreLogger

    return CoreLogger(name)


__all__ = [
    "CONNECTOR_LOGGER_NAME",
    "SF_CORE_LOGGER_NAME",
    "NativeExtensionLogger",
    "_get_connector_stdlib_logger",
    "_get_sf_core_stdlib_logger",
    "_needs_handler",
    "get_logger",
    "get_native_extension_logger",
    "setup_logging",
]
