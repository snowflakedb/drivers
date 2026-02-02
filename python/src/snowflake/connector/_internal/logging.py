"""
Logging configuration for the snowflake.connector module.

This module provides utilities to configure logging for the Snowflake connector,
including the native sf_core library logs.
"""

import logging

from typing import Optional


# Logger names
CONNECTOR_LOGGER_NAME = "snowflake.connector"
SF_CORE_LOGGER_NAME = "snowflake.connector._core"

# Set up loggers once at module load:
# - Disable propagation to prevent duplicate logs via parent loggers
# - Add NullHandler as a fallback (good practice for library loggers)
_sf_core_logger = logging.getLogger(SF_CORE_LOGGER_NAME)
_sf_core_logger.propagate = False
_sf_core_logger.addHandler(logging.NullHandler())

_connector_logger = logging.getLogger(CONNECTOR_LOGGER_NAME)
_connector_logger.propagate = False
_connector_logger.addHandler(logging.NullHandler())


def _needs_handler(logger: logging.Logger) -> bool:
    """
    Check if a logger needs a handler to be added.

    Returns True if the logger has no handlers or only has NullHandler(s).
    """
    if not logger.handlers:
        return True
    # Check if all existing handlers are NullHandlers
    return all(isinstance(h, logging.NullHandler) for h in logger.handlers)


def setup_logging(
    level: int = logging.INFO,
    sf_core_level: int = logging.INFO,
    format_string: Optional[str] = None,
    stream: Optional[object] = None,
) -> None:
    """
    Configure basic logging for the snowflake.connector module.

    This function sets up logging handlers and formatters for both the
    snowflake.connector logger and the sf_core logger (which receives
    logs from the native Rust library).

    Handlers are only added if the logger has no handlers or only has
    NullHandler(s). This prevents duplicate handlers if setup_logging
    is called multiple times.

    Args:
        level: Logging level for the snowflake.connector logger.
               Defaults to logging.INFO.
        sf_core_level: Logging level for the sf_core logger.
                       Defaults to logging.INFO.
        format_string: Custom format string for log messages.
                       If None, uses a default format.
        stream: Stream to write logs to. If None, uses sys.stderr.

    Example:
        >>> from snowflake.connector._internal.logging import setup_logging
        >>> import logging
        >>> setup_logging(level=logging.DEBUG, sf_core_level=logging.DEBUG)
    """
    if format_string is None:
        format_string = "%(asctime)s - %(name)s - %(levelname)s - %(message)s"

    # Create formatter
    formatter = logging.Formatter(format_string)

    # Create handler
    handler = logging.StreamHandler(stream)  # type: ignore [arg-type]
    handler.setFormatter(formatter)

    # Configure snowflake.connector logger
    _connector_logger.setLevel(level)
    if _needs_handler(_connector_logger):
        _connector_logger.addHandler(handler)

    # Configure sf_core logger with explicit INFO level
    _sf_core_logger.setLevel(sf_core_level)
    if _needs_handler(_sf_core_logger):
        _sf_core_logger.addHandler(handler)


def get_connector_logger() -> logging.Logger:
    """
    Get the snowflake.connector logger.

    Returns:
        The logger instance for snowflake.connector.
    """
    return _connector_logger


def get_sf_core_logger() -> logging.Logger:
    """
    Get the sf_core logger.

    This logger receives log messages from the native Rust library
    via the FFI callback mechanism.

    Returns:
        The logger instance for sf_core.
    """
    return _sf_core_logger
