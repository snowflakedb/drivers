"""
Unit tests for the extras module (optional dependency management).
"""

import warnings

import pytest

from snowflake.connector._internal.errorcode import ER_NO_NUMPY, ER_NO_PYARROW
from snowflake.connector.errors import MissingDependencyError, ProgrammingError


# Resolving MissingPandas fires the backward-compat DeprecationWarning; the
# warning contract itself is asserted in test_backward_compatibility_warnings.py,
# so suppress it at import here.
with warnings.catch_warnings():
    warnings.simplefilter("ignore")
    from snowflake.connector._common.extras import (
        DEP_NUMPY,
        DEP_PANDAS,
        DEP_PYARROW,
        DEP_TZLOCAL,
        MissingOptionalDependency,
        MissingPandas,
        ModuleLikeObject,
        check_dependency,
        installed_pandas,
        installed_pyarrow,
    )


class TestCheckDependencyNumpy:
    """Tests for check_dependency handling of the numpy dependency."""

    def test_raises_programming_error_when_numpy_missing(self):
        missing = MissingOptionalDependency(DEP_NUMPY)
        with pytest.raises(ProgrammingError) as exc_info:
            check_dependency(missing)
        assert exc_info.value.errno == ER_NO_NUMPY

    def test_error_message_mentions_numpy(self):
        missing = MissingOptionalDependency(DEP_NUMPY)
        with pytest.raises(ProgrammingError, match="(?i)numpy"):
            check_dependency(missing)

    def test_no_error_when_numpy_is_real_module(self):
        np = pytest.importorskip("numpy")
        check_dependency(np)


class TestCheckDependencyPyarrowPandas:
    """Tests for check_dependency handling of pyarrow/pandas."""

    @pytest.mark.parametrize("dep_name", [DEP_PYARROW, DEP_PANDAS])
    def test_raises_programming_error_with_pyarrow_errno(self, dep_name):
        missing = MissingOptionalDependency(dep_name)
        with pytest.raises(ProgrammingError) as exc_info:
            check_dependency(missing)
        assert exc_info.value.errno == ER_NO_PYARROW

    @pytest.mark.parametrize("dep_name", [DEP_PYARROW, DEP_PANDAS])
    def test_error_message_contains_install_link(self, dep_name):
        missing = MissingOptionalDependency(dep_name)
        with pytest.raises(ProgrammingError, match="python-connector-pandas"):
            check_dependency(missing)


class TestCheckDependencyUnknown:
    """Tests for check_dependency handling of an unknown/generic dependency."""

    def test_raises_missing_dependency_error_for_unknown_dep(self):
        missing = MissingOptionalDependency("some_unknown_lib")
        with pytest.raises(MissingDependencyError):
            check_dependency(missing)


class TestCheckDependencyTzlocal:
    """Tests for check_dependency handling of the tzlocal dependency."""

    def test_raises_missing_dependency_error_when_tzlocal_missing(self):
        missing = MissingOptionalDependency(DEP_TZLOCAL)
        with pytest.raises(MissingDependencyError):
            check_dependency(missing)

    def test_no_error_when_tzlocal_is_real_module(self):
        tz = pytest.importorskip("tzlocal")
        check_dependency(tz)


class TestMissingOptionalDependency:
    """Tests for the MissingOptionalDependency stub class."""

    def test_dep_name_property(self):
        stub = MissingOptionalDependency(DEP_NUMPY)
        assert stub.dep_name == DEP_NUMPY

    def test_getattr_raises_missing_dependency_error(self):
        stub = MissingOptionalDependency(DEP_NUMPY)
        with pytest.raises(MissingDependencyError):
            _ = stub.some_attribute


class TestSnowparkCompatNames:
    """Tests for the Snowpark-only names folded in from the deleted ``options.py``.

    The warning-emission contract for ``MissingPandas`` (a class, so it goes
    through the module's PEP 562 ``__getattr__``) is covered separately in
    ``test_backward_compatibility_warnings.py``; these tests cover behavior only.
    """

    def test_missing_pandas_no_arg_resolves_dep_name(self):
        stub = MissingPandas()
        assert stub.dep_name == DEP_PANDAS

    def test_missing_pandas_is_missing_optional_dependency_subclass(self):
        assert issubclass(MissingPandas, MissingOptionalDependency)

    def test_missing_pandas_check_dependency_raises_pyarrow_errno(self):
        """check_dependency special-cases DEP_PANDAS/DEP_PYARROW identically."""
        with pytest.raises(ProgrammingError) as exc_info:
            check_dependency(MissingPandas())
        assert exc_info.value.errno == ER_NO_PYARROW

    def test_installed_pandas_is_bool_consistent_with_pandas_module(self):
        from snowflake.connector._common import extras as extras_module

        assert isinstance(installed_pandas, bool)
        assert installed_pandas == (not isinstance(extras_module.pandas, MissingOptionalDependency))

    def test_installed_pyarrow_is_bool_consistent_with_pyarrow_module(self):
        """installed_pyarrow checks pyarrow independently rather than mirroring
        installed_pandas (a fixed port artifact from v4's options.py, where
        pandas+pyarrow were always imported as one coupled unit)."""
        from snowflake.connector._common import extras as extras_module

        assert isinstance(installed_pyarrow, bool)
        assert installed_pyarrow == (not isinstance(extras_module.pyarrow, MissingOptionalDependency))

    def test_module_like_object_is_expected_union_type(self):
        from types import ModuleType

        assert ModuleLikeObject == ModuleType | MissingOptionalDependency
