"""
Public API for ci/test_matrix/mappings.

Re-exports every (OS, Arch) lookup table used by generate_matrix.py. Each
driver exposes a single dict-of-dicts (`*_PLATFORM`); cross-driver tables
live in shared.py.
"""

from .shared import GHA_RUNNER
from .odbc import ODBC_PLATFORM
from .python import PYTHON_PLATFORM, SDIST_PY
from .core import CORE_PLATFORM
from .dotnet import DOTNET_PLATFORM, DOTNET_TFM

__all__ = [
    # shared
    "GHA_RUNNER",
    # per-driver platform tables
    "ODBC_PLATFORM",
    "PYTHON_PLATFORM", "SDIST_PY",
    "CORE_PLATFORM",
    "DOTNET_PLATFORM", "DOTNET_TFM",
]
