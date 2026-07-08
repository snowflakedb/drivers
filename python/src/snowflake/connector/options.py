"""BACKWARD COMPATIBILITY MODULE ONLY — optional-dependency helpers for Snowpark.

Kept separate from ``_internal.extras`` on purpose: Snowpark subclasses
``MissingOptionalDependency`` (e.g. ``MissingModin``, ``MissingOpenTelemetry``)
with a class-attribute ``_dep_name`` and *no-argument* construction, whereas
``extras.MissingOptionalDependency.__init__`` requires a ``dep`` argument.
Re-exporting extras would break those subclasses and ``isinstance`` checks.
"""

from __future__ import annotations

import importlib

from types import ModuleType

from . import errors


class MissingOptionalDependency:
    """Placeholder for an absent optional dependency.

    Any attribute access raises ``MissingDependencyError`` so callers get a clear
    message rather than an ``AttributeError``. Subclasses set ``_dep_name``.
    """

    _dep_name = "not set"

    def __getattr__(self, item: str) -> None:
        raise errors.MissingDependencyError(self._dep_name)


class MissingPandas(MissingOptionalDependency):
    _dep_name = "pandas"


ModuleLikeObject = ModuleType | MissingOptionalDependency


def _import_or_missing_pandas() -> tuple[ModuleLikeObject, ModuleLikeObject, bool]:
    try:
        pandas = importlib.import_module("pandas")
        pyarrow = importlib.import_module("pyarrow")
        return pandas, pyarrow, True
    except ImportError:
        return MissingPandas(), MissingPandas(), False


pandas, pyarrow, installed_pandas = _import_or_missing_pandas()
installed_pyarrow = installed_pandas  # pyarrow availability tracks pandas
