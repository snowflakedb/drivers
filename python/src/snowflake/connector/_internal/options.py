from __future__ import annotations

import importlib

from logging import getLogger
from types import ModuleType

from snowflake.connector import errors


logger = getLogger(__name__)

"""This module helps to manage optional dependencies.

It implements MissingOptionalDependency as a base class. If a module is unavailable an instance of this will be
returned. These derived classes can be seen in this file pre-defined. The point of these classes is that if someone
tries to use pyarrow code then by importing pyarrow from this module if they did pyarrow.xxx then that would raise
a MissingDependencyError.
"""


class MissingOptionalDependency:
    """A class to replace missing dependencies.

    The only thing this class is supposed to do is raise a MissingDependencyError when __getattr__ is called.
    This will be triggered whenever module.member is going to be called.
    """

    _dep_name = "not set"

    def __getattr__(self, item):
        raise errors.MissingDependencyError(self._dep_name)


class MissingPyarrow(MissingOptionalDependency):
    """The class is specifically for pyarrow optional dependency."""

    _dep_name = "pyarrow"


class MissingPandas(MissingOptionalDependency):
    """The class is specifically for pandas optional dependency."""

    _dep_name = "pandas"


def _import_or_missing_pyarrow_option() -> tuple[ModuleType | MissingOptionalDependency, bool]:
    """This function tries importing pyarrow.

    If available it returns the pyarrow package with a flag of whether it was imported.
    """
    try:
        pa = importlib.import_module("pyarrow")
        logger.info("pyarrow is installed (version: %s)", pa.__version__)
        return pa, True
    except ImportError:
        logger.debug("pyarrow is not installed; arrow-based features will be unavailable")
        return MissingPyarrow(), False


def _import_or_missing_pandas_option() -> tuple[ModuleType | MissingOptionalDependency, bool]:
    """This function that try to import pandas.

    If available it returns the pandas package with a flag of whether it was imported.
    """
    try:
        pd = importlib.import_module("pandas")
        logger.info("pandas is installed (version: %s)", pd.__version__)
        return pd, True
    except ImportError:
        logger.debug("pandas is not installed; pandas-based features will be unavailable")
        return MissingPandas(), False


# Create actual constants to be imported from this file
pyarrow, installed_pyarrow = _import_or_missing_pyarrow_option()
pandas, installed_pandas = _import_or_missing_pandas_option()
