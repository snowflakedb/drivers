"""Unit tests for the Snowpark-facing pandas helpers added in PR #518.

Covers:
- _stage_sql / _file_format_sql: pure SQL builder functions (no cursor)
- _create_temp_stage / _create_temp_file_format: orchestration with mock cursor

These are the functions Snowpark's analyzer_utils calls directly.
Both use IDENTIFIER(?) bindings, matching WritePandasOperation._build_create_stage_sql.
"""

from __future__ import annotations

from unittest.mock import MagicMock

from snowflake.connector._internal.write_pandas_operation import (
    _file_format_sql,
    _stage_sql,
)
from snowflake.connector.errors import ProgrammingError
from snowflake.connector.pandas_tools import (
    _create_temp_file_format,
    _create_temp_stage,
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _mock_cursor(*, fail_first: bool = False):
    """Return a MagicMock cursor.

    When fail_first=True the first execute call raises ProgrammingError and
    subsequent calls succeed — this exercises _create_temp_object's fallback.
    """
    cursor = MagicMock()
    if fail_first:
        cursor.execute.side_effect = [ProgrammingError("no create privilege"), MagicMock()]
    return cursor


# ---------------------------------------------------------------------------
# _stage_sql — pure function, no cursor
# ---------------------------------------------------------------------------


class TestStageSql:
    def test_gzip_maps_to_auto_compression(self):
        sql, _ = _stage_sql("MY_STAGE", "gzip", False)
        assert "COMPRESSION=auto" in sql

    def test_snappy_maps_to_snappy_compression(self):
        sql, _ = _stage_sql("MY_STAGE", "snappy", False)
        assert "COMPRESSION=snappy" in sql

    def test_default_is_not_scoped(self):
        sql, _ = _stage_sql("MY_STAGE", "gzip", False)
        assert sql.startswith("CREATE TEMPORARY STAGE")
        assert "SCOPED" not in sql

    def test_scoped_flag_produces_scoped_temporary(self):
        sql, _ = _stage_sql("MY_STAGE", "gzip", False, use_scoped=True)
        assert "CREATE SCOPED TEMPORARY STAGE" in sql

    def test_binary_as_text_false_when_true(self):
        sql, _ = _stage_sql("MY_STAGE", "gzip", True)
        assert "BINARY_AS_TEXT=FALSE" in sql

    def test_no_binary_as_text_when_false(self):
        sql, _ = _stage_sql("MY_STAGE", "gzip", False)
        assert "BINARY_AS_TEXT" not in sql

    def test_name_is_bound_via_identifier_not_inline(self):
        sql, params = _stage_sql("__WRITE_PANDAS_STAGE_abc123", "gzip", False)
        assert "IDENTIFIER(?)" in sql
        assert params == ("__WRITE_PANDAS_STAGE_abc123",)
        assert "__WRITE_PANDAS_STAGE_abc123" not in sql


# ---------------------------------------------------------------------------
# _file_format_sql — pure function, no cursor
# ---------------------------------------------------------------------------


class TestFileFormatSql:
    def test_gzip_maps_to_auto_compression(self):
        sql, _ = _file_format_sql("MY_FF", "gzip")
        assert "COMPRESSION=auto" in sql

    def test_snappy_maps_to_snappy_compression(self):
        sql, _ = _file_format_sql("MY_FF", "snappy")
        assert "COMPRESSION=snappy" in sql

    def test_default_is_not_scoped(self):
        sql, _ = _file_format_sql("MY_FF", "gzip")
        assert sql.startswith("CREATE TEMPORARY FILE FORMAT")
        assert "SCOPED" not in sql

    def test_scoped_flag_produces_scoped_temporary(self):
        sql, _ = _file_format_sql("MY_FF", "gzip", use_scoped=True)
        assert "CREATE SCOPED TEMPORARY FILE FORMAT" in sql

    def test_logical_type_suffix_appended_at_end(self):
        suffix = " USE_LOGICAL_TYPE=TRUE"
        sql, _ = _file_format_sql("MY_FF", "gzip", use_logical_type_suffix=suffix)
        assert sql.endswith(suffix)

    def test_empty_suffix_no_trailing_content(self):
        sql, _ = _file_format_sql("MY_FF", "gzip", use_logical_type_suffix="")
        assert not sql.endswith(" ")
        assert "USE_LOGICAL_TYPE" not in sql


# ---------------------------------------------------------------------------
# _create_temp_stage — orchestration (mock cursor)
# ---------------------------------------------------------------------------


class TestCreateTempStage:
    def test_returns_qualified_name_on_success(self):
        cursor = _mock_cursor()
        result = _create_temp_stage(cursor, "db", "sc", False, "gzip", False, False)
        assert "." in result  # qualified: db.sc.<name>
        assert result.startswith("db.sc.")

    def test_falls_back_to_bare_name_on_programming_error(self):
        cursor = _mock_cursor(fail_first=True)
        result = _create_temp_stage(cursor, "db", "sc", False, "gzip", False, False)
        assert "." not in result  # bare name only
        assert result.startswith("__WRITE_PANDAS_STAGE_")

    def test_auto_create_table_sets_binary_as_text_false(self):
        cursor = _mock_cursor()
        _create_temp_stage(cursor, None, None, False, "gzip", True, False)
        executed_sql = cursor.execute.call_args[0][0]
        assert "BINARY_AS_TEXT=FALSE" in executed_sql

    def test_overwrite_sets_binary_as_text_false(self):
        cursor = _mock_cursor()
        _create_temp_stage(cursor, None, None, False, "gzip", False, True)
        executed_sql = cursor.execute.call_args[0][0]
        assert "BINARY_AS_TEXT=FALSE" in executed_sql

    def test_neither_flag_omits_binary_as_text(self):
        cursor = _mock_cursor()
        _create_temp_stage(cursor, None, None, False, "gzip", False, False)
        executed_sql = cursor.execute.call_args[0][0]
        assert "BINARY_AS_TEXT" not in executed_sql

    def test_scoped_temp_object_flag_in_sql(self):
        cursor = _mock_cursor()
        _create_temp_stage(cursor, None, None, False, "gzip", False, False, use_scoped_temp_object=True)
        executed_sql = cursor.execute.call_args[0][0]
        assert "SCOPED TEMPORARY STAGE" in executed_sql


# ---------------------------------------------------------------------------
# _create_temp_file_format — orchestration (mock cursor)
# ---------------------------------------------------------------------------


class TestCreateTempFileFormat:
    def test_returns_qualified_name_on_success(self):
        cursor = _mock_cursor()
        result = _create_temp_file_format(cursor, "db", "sc", False, "gzip", "", False)
        assert result.startswith("db.sc.")

    def test_falls_back_to_bare_name_on_programming_error(self):
        cursor = _mock_cursor(fail_first=True)
        result = _create_temp_file_format(cursor, "db", "sc", False, "gzip", "", False)
        assert "." not in result
        assert result.startswith("__WRITE_PANDAS_FILE_FORMAT_")

    def test_logical_type_suffix_forwarded_to_sql(self):
        cursor = _mock_cursor()
        suffix = " USE_LOGICAL_TYPE=TRUE"
        _create_temp_file_format(cursor, None, None, False, "gzip", suffix, False)
        executed_sql = cursor.execute.call_args[0][0]
        assert "USE_LOGICAL_TYPE=TRUE" in executed_sql

    def test_scoped_flag_in_sql(self):
        cursor = _mock_cursor()
        _create_temp_file_format(cursor, None, None, False, "gzip", "", True)
        executed_sql = cursor.execute.call_args[0][0]
        assert "SCOPED TEMPORARY FILE FORMAT" in executed_sql
