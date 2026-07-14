"""Initialize the core API with the default logger callback."""

from ..logging import _get_connector_stdlib_logger
from .c_api import c_logger_callback, register_default_logger_callback  # noqa: F401


register_default_logger_callback()

from snowflake.connector.version import __version__  # noqa: E402


_get_connector_stdlib_logger().info("Python connector starting v%s", __version__)
