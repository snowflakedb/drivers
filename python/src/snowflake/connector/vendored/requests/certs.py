"""BACKWARD COMPATIBILITY MODULE ONLY"""

import ssl


def where() -> str:
    return ssl.get_default_verify_paths().cafile or ""
