"""Snowpark-compat surface for ``options`` and ``pandas_tools``.

These lock in the contract Snowpark actually depends on, so the modules can't be
"simplified" in ways that silently break it:

* ``options.MissingOptionalDependency`` must construct with **no arguments** and
  carry a class-attribute ``_dep_name`` (Snowpark subclasses it as
  ``MissingModin``/``MissingOpenTelemetry`` and instantiates them no-arg). This is
  why the module is not a re-export of ``_internal.extras`` (whose constructor
  requires a ``dep`` argument).
* ``pandas_tools`` must export ``build_location_helper`` and the two temp-object
  helpers that snowpark's analyzer imports directly.
"""

from unittest.mock import MagicMock

import pytest

from snowflake.connector.errors import MissingDependencyError, ProgrammingError
from snowflake.connector.options import (
    MissingOptionalDependency,
    MissingPandas,
    installed_pandas,
    installed_pyarrow,
    pandas,
    pyarrow,
)
from snowflake.connector.pandas_tools import (
    _create_temp_file_format,
    _create_temp_stage,
    build_location_helper,
)


class TestMissingOptionalDependencyContract:
    """The legacy no-arg / class-attribute contract Snowpark relies on."""

    def test_missing_pandas_constructs_with_no_arguments(self):
        # Snowpark does ``pandas = MissingPandas()`` (no args) in utils.py.
        assert MissingPandas()._dep_name == "pandas"

    def test_subclass_with_class_attr_dep_name_constructs_no_arg(self):
        # Mirrors snowpark's ``class MissingModin(MissingOptionalDependency): _dep_name = "modin"``.
        class MissingModin(MissingOptionalDependency):
            _dep_name = "modin"

        missing = MissingModin()
        assert isinstance(missing, MissingOptionalDependency)
        with pytest.raises(MissingDependencyError, match="(?i)modin"):
            _ = missing.DataFrame

    def test_attribute_access_raises_naming_the_dependency(self):
        with pytest.raises(MissingDependencyError, match="(?i)pandas"):
            _ = MissingPandas().DataFrame


class TestInstalledFlags:
    def test_installed_flags_are_bool_and_agree(self):
        assert isinstance(installed_pandas, bool)
        assert installed_pandas == installed_pyarrow

    def test_when_pandas_present_modules_are_real(self):
        pytest.importorskip("pandas")
        assert installed_pandas is True
        assert not isinstance(pandas, MissingOptionalDependency)
        assert not isinstance(pyarrow, MissingOptionalDependency)


class TestPandasToolsSnowparkHelpers:
    """The three names snowpark imports from ``connector.pandas_tools``."""

    def test_build_location_helper_qualifies_and_quotes(self):
        assert build_location_helper("db", "sch", "t", True) == '"db"."sch"."t"'

    def test_build_location_helper_omits_missing_parts_unquoted(self):
        assert build_location_helper(None, None, "t", False) == "t"

    def test_create_temp_stage_executes_create_stage_sql(self):
        cursor = MagicMock()
        result = _create_temp_stage(cursor, "db", "sch", True, "gzip", False, False)
        sql = cursor.execute.call_args[0][0]
        assert "CREATE TEMPORARY STAGE" in sql
        assert "TYPE=PARQUET COMPRESSION=auto" in sql
        assert result.startswith('"db"."sch".')

    def test_create_temp_stage_binary_as_text_false_when_auto_create(self):
        cursor = MagicMock()
        _create_temp_stage(cursor, None, None, False, "gzip", True, False)
        sql = cursor.execute.call_args[0][0]
        assert "BINARY_AS_TEXT=FALSE" in sql

    def test_create_temp_stage_binary_as_text_false_when_overwrite(self):
        cursor = MagicMock()
        _create_temp_stage(cursor, None, None, False, "gzip", False, True)
        sql = cursor.execute.call_args[0][0]
        assert "BINARY_AS_TEXT=FALSE" in sql

    def test_create_temp_stage_scoped(self):
        cursor = MagicMock()
        _create_temp_stage(cursor, None, None, False, "gzip", False, False, use_scoped_temp_object=True)
        sql = cursor.execute.call_args[0][0]
        assert "SCOPED TEMPORARY STAGE" in sql

    def test_create_temp_stage_fallback_on_programming_error(self):
        cursor = MagicMock()
        cursor.execute.side_effect = [ProgrammingError(), None]
        result = _create_temp_stage(cursor, "db", "sch", True, "gzip", False, False)
        assert cursor.execute.call_count == 2
        fallback_sql = cursor.execute.call_args_list[1][0][0]
        assert '"db"."sch"' not in fallback_sql
        assert '"db"."sch"' not in result

    def test_create_temp_file_format_executes_create_file_format_sql(self):
        cursor = MagicMock()
        result = _create_temp_file_format(cursor, "db", "sch", True, "gzip", "")
        sql = cursor.execute.call_args[0][0]
        assert "CREATE TEMPORARY FILE FORMAT" in sql
        assert "TYPE=PARQUET COMPRESSION=auto" in sql
        assert result.startswith('"db"."sch".')

    def test_create_temp_file_format_with_logical_type_suffix(self):
        cursor = MagicMock()
        _create_temp_file_format(cursor, None, None, False, "gzip", " USE_LOGICAL_TYPE=TRUE")
        sql = cursor.execute.call_args[0][0]
        assert "USE_LOGICAL_TYPE=TRUE" in sql

    def test_create_temp_file_format_scoped(self):
        cursor = MagicMock()
        _create_temp_file_format(cursor, None, None, False, "gzip", "", use_scoped_temp_object=True)
        sql = cursor.execute.call_args[0][0]
        assert "SCOPED TEMPORARY FILE FORMAT" in sql
