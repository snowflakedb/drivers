# PEP 440 compliant version string (used by hatch for packaging)
__version__ = "0.1.0"

# Compatibility with old driver pattern: tuple of (major, minor, patch, None)
VERSION = (*[int(n) for n in __version__.split(".")], None)
