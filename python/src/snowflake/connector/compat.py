"""BACKWARD COMPATIBILITY MODULE ONLY"""

import platform

from typing import Any

from ._internal.decorators import backward_compatibility


IS_LINUX = platform.system() == "Linux"
IS_WINDOWS = platform.system() == "Windows"
IS_MACOS = platform.system() == "Darwin"


@backward_compatibility
def IS_UNICODE(v: Any) -> bool:
    """To check whether v is a Unicode string."""
    return isinstance(v, str)


# backward compatibility
IS_STR = IS_UNICODE
