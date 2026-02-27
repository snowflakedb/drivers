"""BACKWARD COMPATIBILITY MODULE ONLY"""

import platform

from .version import __version__

OPERATING_SYSTEM = platform.system()
PLATFORM = platform.platform()
CLIENT_VERSION = __version__
