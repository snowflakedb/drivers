"""Initialize the core API with the default logger callback."""

from .c_api import c_logger_callback, sf_core_init


sf_core_init(c_logger_callback)
