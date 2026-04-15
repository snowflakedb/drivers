"""
Unit tests for the extras module (optional dependency management).
"""

import pytest

from snowflake.connector._internal.errorcode import ER_NO_NUMPY, ER_NO_PYARROW
from snowflake.connector._internal.extras import (
    DEP_NUMPY,
    DEP_PANDAS,
    DEP_PYARROW,
    DEP_TZLOCAL,
    MissingOptionalDependency,
    check_dependency,
)
from snowflake.connector.errors import MissingDependencyError, ProgrammingError


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
