"""Initialize the core API with the default logger callback."""

import logging

from .c_api import c_logger_callback, sf_core_init


sf_core_init(c_logger_callback)

from snowflake.connector.version import __version__  # noqa: E402


logging.getLogger("snowflake.connector").info("Python connector starting v%s", __version__)
