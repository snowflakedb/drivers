"""Logging configuration for snowflake.connector."""

from __future__ import annotations

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
    """Return True if the logger has no handlers or only NullHandler(s)."""
    if not logger.handlers:
        return True
    return all(isinstance(h, logging.NullHandler) for h in logger.handlers)


def setup_logging(
    level: int = logging.INFO,
    sf_core_level: int = logging.INFO,
    format_string: str | None = None,
    stream: object | None = None,
) -> None:
    """Configure basic logging for the snowflake.connector module.

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

    _connector_logger.setLevel(level)
    if _needs_handler(_connector_logger):
        _connector_logger.addHandler(handler)
        _connector_logger.propagate = False

    _sf_core_logger.setLevel(sf_core_level)
    if _needs_handler(_sf_core_logger):
        _sf_core_logger.addHandler(handler)
        _sf_core_logger.propagate = False


def _get_connector_stdlib_logger() -> logging.Logger:
    """Return the package stdlib logger used by logging infrastructure.

    Internal — not for wrapper contributor use.  Call :func:`logging.get_logger`
    to obtain a :class:`~.core_logger.CoreLogger` for module logging.
    """
    return _connector_logger


def _get_sf_core_stdlib_logger() -> logging.Logger:
    """Return the stdlib logger that receives core-originated FFI callback events.

    Internal — used by :mod:`~.api_client` callback dispatch only.
    Wrapper code should use :func:`logging.get_logger` instead.
    """
    return _sf_core_logger


class LoggingConfiguration:
    """Process-wide logging knobs — currently just troubleshooting mode."""

    _instance: LoggingConfiguration | None = None

    def __init__(self, *, troubleshooting_enabled: bool) -> None:
        self._troubleshooting_enabled = troubleshooting_enabled

    @classmethod
    def initialize(cls, *, troubleshooting_enabled: bool) -> LoggingConfiguration:
        """Create the process-wide instance. Call once after ``sf_core_python.init``."""
        if cls._instance is None:
            cls._instance = cls(troubleshooting_enabled=troubleshooting_enabled)
        return cls._instance

    def is_troubleshooting_enabled(self) -> bool:
        """Return whether wrapper logs should bypass the ``CoreLogger`` pre-filter."""
        return self._troubleshooting_enabled
