# PEP 440 compliant version string (used by hatch for packaging)
__version__ = "5.0.0dev"

# Compatibility with old driver pattern — extract leading digits from each segment
VERSION = (*[int(s) for seg in __version__.split(".")[:3] if (s := "".join(c for c in seg if c.isdigit()))], None)
