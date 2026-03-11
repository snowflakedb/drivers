"""BACKWARD COMPATIBILITY MODULE ONLY — optional dependency helpers."""

from __future__ import annotations

import importlib

from types import ModuleType
from typing import Union

from . import errors


class MissingOptionalDependency:
    """Placeholder returned when an optional dependency is absent.

    Raises MissingDependencyError on any attribute access so callers get a
    clear message rather than an AttributeError.
    """

    _dep_name = "not set"

    def __getattr__(self, item: str) -> None:
        raise errors.MissingDependencyError(self._dep_name)


class MissingPandas(MissingOptionalDependency):
    _dep_name = "pandas"


ModuleLikeObject = Union[ModuleType, MissingOptionalDependency]


def _import_or_missing_pandas() -> tuple[ModuleLikeObject, ModuleLikeObject, bool]:
    try:
        pandas = importlib.import_module("pandas")
        pyarrow = importlib.import_module("pyarrow")
        return pandas, pyarrow, True
    except ImportError:
        return MissingPandas(), MissingPandas(), False


pandas, pyarrow, installed_pandas = _import_or_missing_pandas()
