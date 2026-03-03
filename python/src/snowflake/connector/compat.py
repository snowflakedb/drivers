"""BACKWARD COMPATIBILITY MODULE ONLY"""

import platform


IS_LINUX = platform.system() == "Linux"
IS_WINDOWS = platform.system() == "Windows"
IS_MACOS = platform.system() == "Darwin"
