"""E2E tests for the Snowpark-facing pandas helpers added in PR #518.

These mirror legacy connector patterns:
  - SQL-shape tests: real connection, execute intercepted, DDL structure verified
  - Object-existence tests: stage/file format created for real, then verified usable
  - Fallback test: ProgrammingError on target-schema DDL → verify bare-name fallback

Unlike the write_pandas e2e tests that assert on ingested data, these tests assert
directly on the objects created by _create_temp_stage / _create_temp_file_format,
which use IDENTIFIER(?) bindings via the same pattern as WritePandasOperation.
"""

from __future__ import annotations

from snowflake.connector._internal.write_pandas_operation import _drop_object
from snowflake.connector.errors import ProgrammingError
from snowflake.connector.pandas_tools import (
    _create_temp_file_format,
    _create_temp_stage,
)


# ---------------------------------------------------------------------------
# TestCreateTempStageE2E
# ---------------------------------------------------------------------------


class TestCreateTempStageE2E:
    def test_should_create_temp_stage_and_allow_listing_files(self, function_connection):
        """Object-existence check: stage created inline is usable for LIST."""
        # When _create_temp_stage is called without a target schema
        with function_connection.cursor() as cursor:
            stage_name = _create_temp_stage(
                cursor,
                database=None,
                schema=None,
                quote_identifiers=False,
                compression="gzip",
                auto_create_table=False,
                overwrite=False,
            )
            try:
                # Then the stage exists and LIST succeeds
                assert stage_name.startswith("__WRITE_PANDAS_STAGE_")
                # LIST @stage does not support IDENTIFIER(?) binding; name is connector-generated
                cursor.execute(f"LIST @{stage_name}")
            finally:
                _drop_object(cursor, stage_name, "STAGE")

    def test_should_emit_binary_as_text_false_for_auto_create_table(self, function_connection):
        """SQL-shape: DDL emitted to Snowflake must contain BINARY_AS_TEXT=FALSE."""
        # When _create_temp_stage is called with auto_create_table=True
        with function_connection.cursor() as cursor:
            captured = []
            real_execute = cursor.execute

            def spy(sql, *args, **kwargs):
                captured.append(sql)
                return real_execute(sql, *args, **kwargs)

            cursor.execute = spy
            stage_name = _create_temp_stage(
                cursor,
                database=None,
                schema=None,
                quote_identifiers=False,
                compression="gzip",
                auto_create_table=True,
                overwrite=False,
            )
            try:
                # Then BINARY_AS_TEXT=FALSE is present in the CREATE STAGE DDL
                ddl_calls = [s for s in captured if "CREATE" in s.upper() and "STAGE" in s.upper()]
                assert ddl_calls, "No CREATE STAGE DDL was emitted"
                assert any("BINARY_AS_TEXT=FALSE" in s for s in ddl_calls)
            finally:
                _drop_object(cursor, stage_name, "STAGE")

    def test_should_fall_back_to_bare_name_when_schema_lacks_privilege(self, function_connection):
        """Fallback: ProgrammingError on qualified DDL → bare-name stage is created and usable."""
        # When the target schema DDL raises ProgrammingError
        with function_connection.cursor() as cursor:
            real_execute = cursor.execute

            def selective_execute(sql, *args, **kwargs):
                # Intercept only CREATE STAGE DDL targeting a qualified name.
                # The name is now bound via IDENTIFIER(?), so check params not SQL text.
                is_create_stage = "CREATE" in sql.upper() and "STAGE" in sql.upper()
                params = kwargs.get("params", ())
                if is_create_stage and params and "." in str(params[0]):
                    raise ProgrammingError("Insufficient privileges to create stage in target schema")
                return real_execute(sql, *args, **kwargs)

            cursor.execute = selective_execute

            result = _create_temp_stage(
                cursor,
                database="some_db",
                schema="some_schema",
                quote_identifiers=False,
                compression="gzip",
                auto_create_table=False,
                overwrite=False,
            )
            try:
                # Then a bare-name stage is returned and is usable via LIST
                assert "." not in result, "Expected bare name, got qualified"
                assert result.startswith("__WRITE_PANDAS_STAGE_")
                cursor.execute = real_execute
                # LIST @stage does not support IDENTIFIER(?) binding; name is connector-generated
                cursor.execute(f"LIST @{result}")
            finally:
                cursor.execute = real_execute
                _drop_object(cursor, result, "STAGE")


# ---------------------------------------------------------------------------
# TestCreateTempFileFormatE2E
# ---------------------------------------------------------------------------


class TestCreateTempFileFormatE2E:
    def test_should_create_temp_file_format_and_verify_it_exists(self, function_connection):
        """Object-existence check: SHOW FILE FORMATS returns the created format."""
        # When _create_temp_file_format is called without a target schema
        with function_connection.cursor() as cursor:
            fmt_name = _create_temp_file_format(
                cursor,
                database=None,
                schema=None,
                quote_identifiers=False,
                compression="gzip",
                sql_use_logical_type="",
                use_scoped_temp_object=False,
            )
            try:
                # Then the file format appears in SHOW FILE FORMATS
                assert fmt_name.startswith("__WRITE_PANDAS_FILE_FORMAT_")
                rows = cursor.execute("SHOW FILE FORMATS LIKE '%WRITE_PANDAS_FILE_FORMAT_%'").fetchall()
                assert len(rows) >= 1
            finally:
                _drop_object(cursor, fmt_name, "FILE FORMAT")

    def test_should_apply_scoped_temporary_when_flag_set(self, function_connection):
        """SQL-shape: SCOPED TEMPORARY FILE FORMAT emitted when use_scoped_temp_object=True."""
        # When _create_temp_file_format is called with use_scoped_temp_object=True
        with function_connection.cursor() as cursor:
            captured = []
            real_execute = cursor.execute

            def spy(sql, *args, **kwargs):
                captured.append(sql)
                return real_execute(sql, *args, **kwargs)

            cursor.execute = spy
            fmt_name = _create_temp_file_format(
                cursor,
                database=None,
                schema=None,
                quote_identifiers=False,
                compression="gzip",
                sql_use_logical_type="",
                use_scoped_temp_object=True,
            )
            try:
                # Then the DDL contains SCOPED TEMPORARY FILE FORMAT
                ddl_calls = [s for s in captured if "FILE FORMAT" in s.upper()]
                assert ddl_calls, "No CREATE FILE FORMAT DDL was emitted"
                assert any("SCOPED TEMPORARY FILE FORMAT" in s for s in ddl_calls)
            finally:
                cursor.execute = real_execute
                _drop_object(cursor, fmt_name, "FILE FORMAT")
