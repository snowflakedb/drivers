from __future__ import annotations

import importlib

from logging import getLogger
from types import ModuleType
from typing import Any

from snowflake.connector import errors
from snowflake.connector._internal.errorcode import ER_NO_PYARROW


logger = getLogger(__name__)

"""This module helps to manage optional dependencies.

It implements MissingOptionalDependency. If a module is unavailable an instance of this will be
returned. The point of this class is that if someone tries to use pyarrow code then by importing
pyarrow from this module if they did pyarrow.xxx then that would raise a MissingDependencyError.
"""


class MissingOptionalDependency:
    """A class to replace missing dependencies.

    The only thing this class is supposed to do is raise a MissingDependencyError when __getattr__ is called.
    This will be triggered whenever module.member is going to be called.
    """

    def __init__(self, dep: str) -> None:
        self._dep_name = dep

    def __getattr__(self, item: str) -> Any:
        raise errors.MissingDependencyError(self._dep_name)

    @property
    def dep_name(self) -> str:
        return self._dep_name


def _import_or_missing(module_name: str) -> ModuleType | MissingOptionalDependency:
    """Try importing an optional dependency.

    If available it returns the module. Otherwise, returns a MissingOptionalDependency stub.
    """
    try:
        mod = importlib.import_module(module_name)
        logger.info("%s is installed (version: %s)", module_name, mod.__version__)
        return mod
    except ImportError:
        logger.info("%s is not installed; %s-based features will be unavailable", module_name, module_name)
        return MissingOptionalDependency(dep=module_name)


def check_dependency(module: ModuleType | MissingOptionalDependency) -> None:
    """Raise ProgrammingError if optional dependency is not installed."""
    if isinstance(module, MissingOptionalDependency):
        if module.dep_name in ("pyarrow", "pandas"):
            msg = (
                f"Optional dependency: '{module.dep_name}' is not installed, please see the following link for"
                " install instructions: https://docs.snowflake.com/en/user-guide/python-connector-pandas.html#installation"
            )
            raise errors.ProgrammingError(msg=msg, errno=ER_NO_PYARROW)
        else:
            raise errors.MissingDependencyError(module.dep_name)


pyarrow = _import_or_missing("pyarrow")
pandas = _import_or_missing("pandas")
