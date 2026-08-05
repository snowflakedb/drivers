"""Initialize the core API with the default logger callback."""

import logging

from snowflake.connector.errors import OperationalError

from ..logging import _get_connector_stdlib_logger, _get_sf_core_stdlib_logger
from ..logging.config import LoggingConfiguration


try:
    from snowflake.connector._core import sf_core_python
except ImportError as err:
    raise OperationalError(
        msg=(
            "Couldn't load core driver dependency (sf_core_python). "
            "Ensure the package was installed from a pre-built wheel or built locally."
        )
    ) from err


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


def _logger_callback(
    level: int,
    message: str,
    filename: str,
    line: int,
    function: str,
    logger_name: str,
) -> int:
    py_level = _sf_core_level_to_python(level)
    if py_level is None:
        return 0

    target_logger = logging.getLogger(logger_name) if logger_name else _get_sf_core_stdlib_logger()

    if not target_logger.isEnabledFor(py_level):
        return 0

    record = target_logger.makeRecord(
        target_logger.name,
        py_level,
        filename,
        line,
        message,
        (),
        None,
        func=function,
    )
    target_logger.handle(record)
    return 0


def register_default_logger_callback() -> None:
    """Register the default logger callback with the core API."""
    status, troubleshooting_enabled = sf_core_python.init(_logger_callback)
    if status != 0:
        raise OperationalError(
            msg=(
                f"sf_core_python.init failed (status={status}). "
                "Ensure the package was installed from a pre-built wheel or built locally."
            )
        )
    LoggingConfiguration.initialize(troubleshooting_enabled=troubleshooting_enabled)


register_default_logger_callback()

from snowflake.connector.version import __version__  # noqa: E402


_get_connector_stdlib_logger().info("Python connector starting v%s", __version__)
