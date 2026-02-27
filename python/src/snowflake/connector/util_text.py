"""BACKWARD COMPATIBILITY MODULE ONLY"""

import random
import string
from collections.abc import Sequence

from ._internal.text_utils import split_statements  # noqa


def random_string(
    length: int = 10,
    prefix: str = "",
    suffix: str = "",
    choices: Sequence[str] = string.ascii_lowercase,
) -> str:
    random_part = "".join(random.choice(choices) for _ in range(length))
    return f"{prefix}{random_part}{suffix}"
