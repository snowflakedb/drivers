"""
Logging configuration for the snowflake.connector module.

This module provides utilities to configure logging for the Snowflake connector,
including the native sf_core library logs.
"""

import logging


# Logger names
CONNECTOR_LOGGER_NAME = "snowflake.connector"
SF_CORE_LOGGER_NAME = "snowflake.connector._core"

# Set up loggers once at module load with NullHandler (standard library practice
# per https://docs.python.org/3/howto/logging.html#configuring-logging-for-a-library).
# Propagation is left ON so that external log capture (e.g. pytest caplog, root-level
# handlers configured by the application) works out of the box -- matching
# the behavior of snowflake-connector-python.
# When setup_logging() is called explicitly, propagation is turned OFF on the
# loggers that receive a dedicated handler to prevent duplicate output.
_sf_core_logger = logging.getLogger(SF_CORE_LOGGER_NAME)
_sf_core_logger.addHandler(logging.NullHandler())

_connector_logger = logging.getLogger(CONNECTOR_LOGGER_NAME)
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
    format_string: str | None = None,
    stream: object | None = None,
) -> None:
    """
    Configure basic logging for the snowflake.connector module.

    This function sets up logging handlers and formatters for both the
    snowflake.connector logger and the sf_core logger (which receives
    logs from the native Rust library).

    Propagation behaviour
    ---------------------
    By default (before this function is called) both loggers have
    ``propagate=True``.  This means log records bubble up to the root
    logger so that application-level handlers and test frameworks such as
    ``pytest caplog`` capture connector output without any extra
    configuration.

    When this function adds a dedicated StreamHandler to a logger it also
    sets ``propagate=False`` on that logger to prevent every message from
    appearing twice — once via the dedicated handler and once via the root
    handler chain.

    **The propagate flag is only changed when a handler is actually added.**
    If the logger already has a non-NullHandler (meaning the caller has
    configured it themselves), this function leaves ``propagate`` and the
    existing handler untouched and only updates the logger's *level*.
    This respects application-level logging configuration while still
    honouring the requested verbosity.

    Summary:
      * No prior handler → adds StreamHandler, sets propagate=False.
      * Prior non-Null handler exists → updates level only, propagate unchanged.
      * Called multiple times with no prior handler → second call is a no-op
        for handler/propagation (handler already present from the first call).

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

    formatter = logging.Formatter(format_string)
    handler = logging.StreamHandler(stream)  # type: ignore [arg-type]
    handler.setFormatter(formatter)

    # Level is always updated regardless of whether a handler is added.
    _connector_logger.setLevel(level)
    if _needs_handler(_connector_logger):
        # No real handler yet: attach ours and stop propagation to avoid duplicate output via the root logger.
        _connector_logger.addHandler(handler)
        _connector_logger.propagate = False

    _sf_core_logger.setLevel(sf_core_level)
    if _needs_handler(_sf_core_logger):
        _sf_core_logger.addHandler(handler)
        _sf_core_logger.propagate = False


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
