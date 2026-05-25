"""Initialize the core API with the default logger callback."""

from ..logging import get_connector_logger
from .c_api import c_logger_callback, sf_core_init


sf_core_init(c_logger_callback)

from snowflake.connector.version import __version__  # noqa: E402


get_connector_logger().info("Python connector starting v%s", __version__)
