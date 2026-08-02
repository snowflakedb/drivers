"""BACKWARD COMPATIBILITY MODULE ONLY — optional-dependency helpers for Snowpark.

Re-exports the shared machinery from ``_internal.extras`` and adds the
legacy surface Snowpark imports: ``MissingPandas``, ``ModuleLikeObject``,
``installed_pandas``, ``installed_pyarrow``.
"""

from __future__ import annotations

from types import ModuleType

from ._internal.extras import MissingOptionalDependency, pandas, pyarrow  # noqa: F401


class MissingPandas(MissingOptionalDependency):
    _dep_name = "pandas"


ModuleLikeObject = ModuleType | MissingOptionalDependency

installed_pandas: bool = not isinstance(pandas, MissingOptionalDependency)
installed_pyarrow: bool = installed_pandas  # pyarrow availability tracks pandas
