# PEP 440 compliant version string (used by hatch for packaging)
__version__ = "2026.0.0"

# Compatibility with old driver pattern
VERSION = (*[int(n) for n in __version__.split(".")], None)
