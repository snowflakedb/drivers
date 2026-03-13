"""BACKWARD COMPATIBILITY MODULE ONLY"""

import ssl


def where() -> str:
    """Return the path to the default SSL certificate bundle."""
    return ssl.get_default_verify_paths().cafile or ""
