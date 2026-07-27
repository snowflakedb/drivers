"""Unit tests for pandas_tools module.

Uses MagicMock for DataFrames — no pandas dependency required.
"""

from __future__ import annotations

import inspect
import warnings

from unittest.mock import MagicMock, patch

import pytest

import snowflake.connector._internal.write_pandas_operation as _wpo
import snowflake.connector.aio.pandas_tools as _aio_pt

from snowflake.connector._internal.write_pandas_operation import (
    WritePandasConfig,
    WritePandasOperation,
    WritePandasResult,
    _convert_value_to_sql_option,
    escape_path_for_sql,
    generate_temp_name,
    qualify_name,
    quote_identifier,
)
from snowflake.connector.errors import ProgrammingError
from snowflake.connector.pandas_tools import make_pd_writer, pd_writer, write_pandas
from tests.compatibility import NEW_DRIVER_ONLY


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _mock_df(columns=None, length=10, dtypes=None, index=None):
    """Build a MagicMock that satisfies WritePandasConfig's DataFrame contract."""
    df = MagicMock()
    df.__len__ = MagicMock(return_value=length)
    df.columns = columns or ["COL_A", "COL_B"]
    if dtypes is None:
        dtypes = [MagicMock(spec=[]) for _ in df.columns]
    df.dtypes = dtypes
    if index is not None:
        df.index = index
    return df


def _mock_conn():
    return MagicMock()


def _make_cfg(**overrides):
    """Create a WritePandasConfig with sensible mock defaults."""
    defaults = {
        "conn": _mock_conn(),
        "df": _mock_df(),
        "table_name": "TARGET",
    }
    defaults.update(overrides)
    conn = defaults.pop("conn")
    df = defaults.pop("df")
    table_name = defaults.pop("table_name")
    return WritePandasConfig(conn, df, table_name, **defaults)


def _make_op(**overrides):
    """Create a WritePandasOperation from a config with mock defaults."""
    return WritePandasOperation(_make_cfg(**overrides))


# ---------------------------------------------------------------------------
# Module-level utilities
# ---------------------------------------------------------------------------


class TestQuoteIdentifier:
    def test_basic(self):
        assert quote_identifier("my_table") == '"my_table"'

    def test_embedded_double_quotes(self):
        assert quote_identifier('has"quote') == '"has""quote"'

    def test_empty_string(self):
        assert quote_identifier("") == '""'


class TestQualifyName:
    def test_name_only(self):
        assert qualify_name(None, None, "t", False) == "t"

    def test_schema_and_name(self):
        assert qualify_name(None, "s", "t", False) == "s.t"

    def test_database_schema_name(self):
        assert qualify_name("d", "s", "t", False) == "d.s.t"

    def test_with_quoting(self):
        assert qualify_name("d", "s", "t", True) == '"d"."s"."t"'


class TestEscapePathForSql:
    def test_backslash(self):
        assert escape_path_for_sql("C:\\Users\\data") == "C:\\\\Users\\\\data"

    def test_single_quote(self):
        assert escape_path_for_sql("/tmp/it's/file") == "/tmp/it\\'s/file"


class TestGenerateTempName:
    def test_format(self):
        name = generate_temp_name("STAGE")
        assert name.startswith("__WRITE_PANDAS_STAGE_")
        assert len(name) > len("__WRITE_PANDAS_STAGE_")


class TestConvertValueToSqlOption:
    def test_string_value(self):
        assert _convert_value_to_sql_option("my_volume") == "'my_volume'"

    def test_already_quoted_string(self):
        assert _convert_value_to_sql_option("'my_volume'") == "'my_volume'"

    def test_string_with_single_quote(self):
        assert _convert_value_to_sql_option("it's") == "'it''s'"


# ---------------------------------------------------------------------------
# WritePandasConfig — Validation
# ---------------------------------------------------------------------------


class TestConfigValidation:
    def test_should_raise_when_database_without_schema(self):
        with pytest.raises(ProgrammingError):
            _make_cfg(database="mydb")

    def test_should_raise_for_invalid_compression(self):
        with pytest.raises(ProgrammingError, match="bzip2"):
            _make_cfg(compression="bzip2")

    def test_should_raise_for_invalid_table_type(self):
        with pytest.raises(ValueError, match="bogus"):
            _make_cfg(table_type="bogus")

    def test_should_raise_for_invalid_iceberg_config_key(self):
        with pytest.raises(ProgrammingError, match="UNKNOWN_KEY"):
            _make_cfg(iceberg_config={"UNKNOWN_KEY": "val"})

    def test_should_default_chunk_size_to_df_length(self):
        cfg = _make_cfg(df=_mock_df(length=42))
        assert cfg.chunk_size == 42

    def test_should_respect_explicit_chunk_size(self):
        cfg = _make_cfg(chunk_size=5)
        assert cfg.chunk_size == 5


# ---------------------------------------------------------------------------
# WritePandasConfig — Branching properties
# ---------------------------------------------------------------------------


class TestBranchingProperties:
    def test_defaults_need_nothing(self):
        cfg = _make_cfg()
        assert not cfg.needs_inference
        assert not cfg.needs_table_creation
        assert not cfg.needs_truncate
        assert not cfg.needs_swap
        assert not cfg.binary_as_text_false_on_stage
        assert not cfg.binary_as_text_false_on_copy

    def test_auto_create_table(self):
        cfg = _make_cfg(auto_create_table=True)
        assert cfg.needs_inference
        assert cfg.needs_table_creation
        assert not cfg.needs_truncate
        assert not cfg.needs_swap
        assert cfg.binary_as_text_false_on_stage
        assert cfg.binary_as_text_false_on_copy

    def test_overwrite_only(self):
        cfg = _make_cfg(overwrite=True)
        assert cfg.needs_inference
        assert cfg.needs_table_creation
        assert cfg.needs_truncate
        assert not cfg.needs_swap
        assert cfg.binary_as_text_false_on_stage
        assert cfg.binary_as_text_false_on_copy

    def test_overwrite_with_auto_create(self):
        cfg = _make_cfg(overwrite=True, auto_create_table=True)
        assert cfg.needs_inference
        assert cfg.needs_table_creation
        assert not cfg.needs_truncate
        assert cfg.needs_swap
        assert cfg.binary_as_text_false_on_stage
        assert cfg.binary_as_text_false_on_copy

    def test_infer_schema_only(self):
        cfg = _make_cfg(infer_schema=True)
        assert cfg.needs_inference
        assert not cfg.needs_table_creation
        assert not cfg.needs_truncate
        assert not cfg.needs_swap
        assert not cfg.binary_as_text_false_on_stage
        assert cfg.binary_as_text_false_on_copy


# ---------------------------------------------------------------------------
# WritePandasConfig — qualify
# ---------------------------------------------------------------------------


class TestConfigQualify:
    def test_name_only_quoted(self):
        cfg = _make_cfg(quote_identifiers=True)
        assert cfg.qualify("MY_STAGE") == '"MY_STAGE"'

    def test_schema_and_name(self):
        cfg = _make_cfg(schema="s", quote_identifiers=False)
        assert cfg.qualify("MY_TABLE") == "s.MY_TABLE"

    def test_full_qualification(self):
        cfg = _make_cfg(database="d", schema="s", quote_identifiers=True)
        assert cfg.qualify("t") == '"d"."s"."t"'


# ---------------------------------------------------------------------------
# WritePandasConfig — DataFrame inspection
# ---------------------------------------------------------------------------


class TestDataFrameInspection:
    def test_has_tz_aware_columns_false(self):
        cfg = _make_cfg()
        assert not cfg.has_tz_aware_columns()

    def test_has_tz_aware_columns_true(self):
        tz_dtype = MagicMock()
        tz_dtype.tz = "UTC"
        cfg = _make_cfg(df=_mock_df(dtypes=[tz_dtype, MagicMock(spec=[])]))
        assert cfg.has_tz_aware_columns()

    def test_is_standard_range_index_true(self):
        idx = MagicMock()
        idx.start = 0
        idx.step = 1
        with patch.object(_wpo, "pandas", new=MagicMock(RangeIndex=type(idx))):
            cfg = _make_cfg(df=_mock_df(index=idx))
            assert cfg.is_standard_range_index()

    def test_is_standard_range_index_false_wrong_start(self):
        idx = MagicMock()
        idx.start = 5
        idx.step = 1
        with patch.object(_wpo, "pandas", new=MagicMock(RangeIndex=type(idx))):
            cfg = _make_cfg(df=_mock_df(index=idx))
            assert not cfg.is_standard_range_index()


# ---------------------------------------------------------------------------
# SQL generation — CREATE STAGE
# ---------------------------------------------------------------------------


class TestBuildCreateStageSql:
    def test_basic(self):
        op: WritePandasOperation = _make_op()
        result = op._build_create_stage_sql("MY_STAGE")
        assert "CREATE TEMPORARY STAGE IDENTIFIER(?)" in result["operation"]
        assert "MY_STAGE" not in result["operation"]
        assert result["parameters"] == ("MY_STAGE",)
        assert "TYPE=PARQUET" in result["operation"]
        assert "COMPRESSION=auto" in result["operation"]

    def test_snappy(self):
        op: WritePandasOperation = _make_op(compression="snappy")
        result = op._build_create_stage_sql("MY_STAGE")
        assert "COMPRESSION=snappy" in result["operation"]

    def test_with_binary_as_text_false(self):
        op: WritePandasOperation = _make_op(auto_create_table=True)
        result = op._build_create_stage_sql("MY_STAGE")
        assert "BINARY_AS_TEXT=FALSE" in result["operation"]

    def test_without_binary_as_text(self):
        op: WritePandasOperation = _make_op()
        result = op._build_create_stage_sql("MY_STAGE")
        assert "BINARY_AS_TEXT" not in result["operation"]


# ---------------------------------------------------------------------------
# SQL generation — CREATE FILE FORMAT
# ---------------------------------------------------------------------------


class TestBuildCreateFileFormatSql:
    def test_basic(self):
        op: WritePandasOperation = _make_op()
        result = op._build_create_file_format_sql("MY_FF")
        assert "CREATE TEMPORARY FILE FORMAT IDENTIFIER(?)" in result["operation"]
        assert "MY_FF" not in result["operation"]
        assert result["parameters"] == ("MY_FF",)
        assert "TYPE=PARQUET" in result["operation"]
        assert "COMPRESSION=auto" in result["operation"]

    def test_with_use_logical_type_true(self):
        op: WritePandasOperation = _make_op(use_logical_type=True)
        result = op._build_create_file_format_sql("MY_FF")
        assert "USE_LOGICAL_TYPE=TRUE" in result["operation"]

    def test_with_use_logical_type_false(self):
        op: WritePandasOperation = _make_op(use_logical_type=False)
        result = op._build_create_file_format_sql("MY_FF")
        assert "USE_LOGICAL_TYPE=FALSE" in result["operation"]


# ---------------------------------------------------------------------------
# SQL generation — COPY INTO
# ---------------------------------------------------------------------------


class TestBuildCopyIntoSql:
    def test_basic_with_quoting(self):
        op: WritePandasOperation = _make_op(df=_mock_df(columns=["A", "B"]))
        result = op._build_copy_into_sql("@MY_STAGE", "MY_TABLE", None)
        assert "COPY INTO IDENTIFIER(?)" in result["operation"]
        assert '$1:"A" AS "A"' in result["operation"]
        assert '$1:"B" AS "B"' in result["operation"]
        assert "TYPE=PARQUET" in result["operation"]
        assert "PURGE=TRUE" in result["operation"]
        assert "ON_ERROR=?" in result["operation"]
        assert result["parameters"] == ("MY_TABLE", "abort_statement")

    def test_with_column_type_map(self):
        op: WritePandasOperation = _make_op(df=_mock_df(columns=["A"]))
        result = op._build_copy_into_sql("@MY_STAGE", "MY_TABLE", {"A": "NUMBER(38,0)"})
        assert '$1:"A"::NUMBER(38,0)' in result["operation"]

    def test_no_quoting(self):
        op: WritePandasOperation = _make_op(df=_mock_df(columns=["A"]), quote_identifiers=False)
        result = op._build_copy_into_sql("@MY_STAGE", "MY_TABLE", None)
        assert '$1:"A" AS A' in result["operation"]
        assert '"A" AS "A"' not in result["operation"]

    def test_with_vectorized_scanner(self):
        op: WritePandasOperation = _make_op(use_vectorized_scanner=True)
        result = op._build_copy_into_sql("@MY_STAGE", "MY_TABLE", None)
        assert "USE_VECTORIZED_SCANNER=TRUE" in result["operation"]

    def test_target_location_is_not_interpolated_into_sql(self):
        target = '"mydb"."myschema"."mytable"'
        op: WritePandasOperation = _make_op()
        result = op._build_copy_into_sql("@MY_STAGE", target, None)
        assert target not in result["operation"]
        assert result["parameters"][0] == target

    def test_on_error_is_not_interpolated_into_sql(self):
        payload = "CONTINUE ->>\n EXECUTE IMMEDIATE $$DROP TABLE foo$$;--"
        op: WritePandasOperation = _make_op(on_error=payload)
        result = op._build_copy_into_sql("@MY_STAGE", "MY_TABLE", None)
        assert payload not in result["operation"]
        assert result["parameters"][1] == payload

    def test_params_order_is_target_then_on_error(self):
        op: WritePandasOperation = _make_op(on_error="continue")
        result = op._build_copy_into_sql("@MY_STAGE", "MY_TARGET", None)
        assert result["parameters"] == ("MY_TARGET", "continue")

    def test_full_query_and_parameters(self):
        op: WritePandasOperation = _make_op(df=_mock_df(columns=["A"]))
        result = op._build_copy_into_sql("MY_STAGE", "MY_TABLE", None)
        assert result == {
            "operation": (
                'COPY INTO IDENTIFIER(?) ("A") '
                'FROM (SELECT $1:"A" AS "A" '
                "FROM '@MY_STAGE') "
                "FILE_FORMAT = (TYPE=PARQUET COMPRESSION=auto) "
                "PURGE=TRUE ON_ERROR=?"
            ),
            "parameters": ("MY_TABLE", "abort_statement"),
            "_force_qmark_paramstyle": True,
        }


# ---------------------------------------------------------------------------
# SQL generation — parameterised DDL helpers
# ---------------------------------------------------------------------------


class TestDropObject:
    def test_uses_identifier_binding(self):
        op: WritePandasOperation = _make_op()
        result = op._build_drop_object_sql('"mydb"."myschema"."mytable"', "TABLE")
        assert "IDENTIFIER(?)" in result["operation"]
        assert '"mydb"."myschema"."mytable"' not in result["operation"]
        assert result["parameters"] == ('"mydb"."myschema"."mytable"',)
        assert result["_force_qmark_paramstyle"] is True

    def test_object_type_is_literal(self):
        op: WritePandasOperation = _make_op()
        result = op._build_drop_object_sql("MY_TABLE", "STAGE")
        assert result["operation"] == "DROP STAGE IF EXISTS IDENTIFIER(?)"


class TestBuildTruncateTableSql:
    def test_uses_identifier_binding(self):
        op: WritePandasOperation = _make_op()
        result = op._build_truncate_table_sql('"db"."schema"."tbl"')
        assert result["operation"] == "TRUNCATE TABLE IF EXISTS IDENTIFIER(?)"
        assert '"db"."schema"."tbl"' not in result["operation"]
        assert result["parameters"] == ('"db"."schema"."tbl"',)
        assert result["_force_qmark_paramstyle"] is True


class TestBuildCreateTableSql:
    def test_target_location_uses_identifier_binding(self):
        op: WritePandasOperation = _make_op(df=_mock_df(columns=["A"]))
        result = op._build_create_table_sql('"db"."schema"."tbl"', None)
        assert "IDENTIFIER(?)" in result["operation"]
        assert '"db"."schema"."tbl"' not in result["operation"]
        assert result["parameters"] == ('"db"."schema"."tbl"',)
        assert result["_force_qmark_paramstyle"] is True

    def test_column_type_from_map(self):
        op: WritePandasOperation = _make_op(df=_mock_df(columns=["A"]))
        result = op._build_create_table_sql("MY_TABLE", {"A": "NUMBER(38,0)"})
        assert "NUMBER(38,0)" in result["operation"]

    def test_column_type_defaults_to_variant(self):
        op: WritePandasOperation = _make_op(df=_mock_df(columns=["A"]))
        result = op._build_create_table_sql("MY_TABLE", None)
        assert "VARIANT" in result["operation"]

    def test_table_type_prefix(self):
        op: WritePandasOperation = _make_op(df=_mock_df(columns=["A"]), table_type="temp")
        result = op._build_create_table_sql("MY_TABLE", None)
        assert "TEMP TABLE" in result["operation"]


class TestBuildInferColumnTypesSql:
    def test_stage_and_format_are_bound_params(self):
        op: WritePandasOperation = _make_op()
        result = op._build_infer_column_types_sql("MY_STAGE", "MY_FILE_FORMAT")
        assert result["operation"] == "SELECT * FROM TABLE(INFER_SCHEMA(LOCATION => ?, FILE_FORMAT => ?))"
        assert result["parameters"] == ("@MY_STAGE", "MY_FILE_FORMAT")
        assert result["_force_qmark_paramstyle"] is True


class TestBuildPutFileSql:
    def test_full_query(self):
        from pathlib import Path as _Path

        op: WritePandasOperation = _make_op()
        result = op._build_put_file_sql("MY_STAGE", _Path("/tmp/file0.txt"))
        assert result == {
            "operation": (
                "PUT 'file:///tmp/file0.txt' @MY_STAGE "
                "PARALLEL=4 AUTO_COMPRESS=FALSE "
                "SOURCE_COMPRESSION=AUTO_DETECT OVERWRITE=TRUE"
            )
        }

    def test_parallel_from_config(self):
        from pathlib import Path as _Path

        op: WritePandasOperation = _make_op(parallel=8)
        result = op._build_put_file_sql("MY_STAGE", _Path("/tmp/file0.txt"))
        assert result == {
            "operation": (
                "PUT 'file:///tmp/file0.txt' @MY_STAGE "
                "PARALLEL=8 AUTO_COMPRESS=FALSE "
                "SOURCE_COMPRESSION=AUTO_DETECT OVERWRITE=TRUE"
            )
        }

    def test_no_bindings_in_dict(self):
        from pathlib import Path as _Path

        op: WritePandasOperation = _make_op()
        result = op._build_put_file_sql("MY_STAGE", _Path("/tmp/file0.txt"))
        assert "parameters" not in result
        assert "_force_qmark_paramstyle" not in result


class TestBuildPutDirectorySql:
    def test_full_query(self):
        op: WritePandasOperation = _make_op()
        result = op._build_put_directory_sql("MY_STAGE", "/tmp/upload_dir")
        assert result == {
            "operation": (
                "PUT 'file:///tmp/upload_dir/*' @MY_STAGE "
                "PARALLEL=4 AUTO_COMPRESS=FALSE "
                "SOURCE_COMPRESSION=AUTO_DETECT OVERWRITE=TRUE"
            )
        }

    def test_no_bindings_in_dict(self):
        op: WritePandasOperation = _make_op()
        result = op._build_put_directory_sql("MY_STAGE", "/tmp/upload_dir")
        assert "parameters" not in result
        assert "_force_qmark_paramstyle" not in result


class TestBuildRenameTableSql:
    def test_uses_identifier_bindings(self):
        op: WritePandasOperation = _make_op(table_name="TARGET", quote_identifiers=False)
        result = op._build_rename_table_sql('"db"."schema"."tmp_tbl"')
        assert result["operation"] == "ALTER TABLE IDENTIFIER(?) RENAME TO IDENTIFIER(?)"
        assert result["parameters"] == ('"db"."schema"."tmp_tbl"', "TARGET")
        assert result["_force_qmark_paramstyle"] is True

    def test_original_table_not_in_sql(self):
        op: WritePandasOperation = _make_op(table_name="MY_TABLE", quote_identifiers=False)
        result = op._build_rename_table_sql("TMP_TABLE")
        assert "MY_TABLE" not in result["operation"]
        assert "TMP_TABLE" not in result["operation"]


# ---------------------------------------------------------------------------
# Iter chunks
# ---------------------------------------------------------------------------


class TestIterChunks:
    def test_single_chunk(self):
        op: WritePandasOperation = _make_op(df=_mock_df(length=5), chunk_size=10)
        chunks = list(op._iter_chunks())
        assert len(chunks) == 1
        assert chunks[0][0] == 0

    def test_multiple_chunks(self):
        op: WritePandasOperation = _make_op(df=_mock_df(length=5), chunk_size=2)
        chunks = list(op._iter_chunks())
        assert len(chunks) == 3
        assert [idx for idx, _ in chunks] == [0, 1, 2]

    def test_empty_df(self):
        op: WritePandasOperation = _make_op(df=_mock_df(length=0))
        chunks = list(op._iter_chunks())
        assert len(chunks) == 1
        assert chunks[0][0] == 0


# ---------------------------------------------------------------------------
# Pipeline flow — orchestration via mocked pipeline steps
# ---------------------------------------------------------------------------


class TestPipelineFlow:
    """Verify execute() calls pipeline steps in the correct order."""

    @pytest.fixture(autouse=True)
    def _patch_pandas(self):
        with patch.object(_wpo, "pandas", new=MagicMock(RangeIndex=MagicMock)):
            yield

    @pytest.fixture()
    def conn(self):
        c = _mock_conn()
        c.cursor.return_value = MagicMock()
        return c

    def test_basic_flow(self, conn):
        op: WritePandasOperation = _make_op(conn=conn)
        with (
            patch.object(op, "_create_stage", return_value="@STAGE") as mock_stage,
            patch.object(op, "_upload_to_stage", return_value=(1, 10)) as mock_upload,
            patch.object(op, "_copy_into", return_value=[("f", "LOADED")]) as mock_copy,
        ):
            result = op.execute()

        assert result == WritePandasResult(True, 1, 10, [("f", "LOADED")])
        mock_stage.assert_called_once()
        mock_upload.assert_called_once()
        mock_copy.assert_called_once()

    def test_auto_create_triggers_inference_and_create_table(self, conn):
        op: WritePandasOperation = _make_op(conn=conn, auto_create_table=True, table_type="temp")
        with (
            patch.object(op, "_create_stage", return_value="@STAGE"),
            patch.object(op, "_upload_to_stage", return_value=(1, 10)),
            patch.object(op, "_create_file_format", return_value="MY_FF") as mock_ff,
            patch.object(op, "_infer_column_types", return_value={"A": "NUMBER"}) as mock_infer,
            patch.object(op, "_create_table") as mock_create,
            patch.object(op, "_copy_into", return_value=[("f", "LOADED")]),
        ):
            result = op.execute()

        assert result.success
        mock_ff.assert_called_once()
        mock_infer.assert_called_once()
        mock_create.assert_called_once()

    def test_overwrite_with_auto_create_triggers_swap(self, conn):
        op: WritePandasOperation = _make_op(conn=conn, overwrite=True, auto_create_table=True, table_type="temp")
        with (
            patch.object(op, "_create_stage", return_value="@STAGE"),
            patch.object(op, "_upload_to_stage", return_value=(1, 5)),
            patch.object(op, "_create_file_format", return_value="MY_FF"),
            patch.object(op, "_infer_column_types", return_value={"A": "NUMBER"}),
            patch.object(op, "_create_table"),
            patch.object(op, "_copy_into", return_value=[("f", "LOADED")]),
            patch.object(op, "_swap_tables") as mock_swap,
        ):
            result = op.execute()

        assert result.success
        mock_swap.assert_called_once()

    def test_overwrite_without_auto_create_triggers_truncate(self, conn):
        op: WritePandasOperation = _make_op(conn=conn, overwrite=True)
        with (
            patch.object(op, "_create_stage", return_value="@STAGE"),
            patch.object(op, "_upload_to_stage", return_value=(1, 5)),
            patch.object(op, "_create_file_format", return_value="MY_FF"),
            patch.object(op, "_infer_column_types", return_value={"A": "NUMBER"}),
            patch.object(op, "_create_table"),
            patch.object(op, "_truncate_table") as mock_truncate,
            patch.object(op, "_copy_into", return_value=[("f", "LOADED")]),
        ):
            result = op.execute()

        assert result.success
        mock_truncate.assert_called_once()

    def test_failed_copy_returns_failure(self, conn):
        op: WritePandasOperation = _make_op(conn=conn)
        with (
            patch.object(op, "_create_stage", return_value="@STAGE"),
            patch.object(op, "_upload_to_stage", return_value=(1, 5)),
            patch.object(op, "_copy_into", return_value=[("f", "LOAD_FAILED")]),
        ):
            result = op.execute()

        assert not result.success


# ---------------------------------------------------------------------------
# pd_writer / make_pd_writer  (pandas_tools public wrappers)
# ---------------------------------------------------------------------------


def _stub_table(name="my_table", schema="PUBLIC"):
    t = MagicMock()
    t.name = name
    t.schema = schema
    return t


def _stub_sa_conn(sf_conn=None):
    """Mimic SQLAlchemy's conn.connection.connection → raw SF connection."""
    sa = MagicMock()
    sa.connection.connection = sf_conn or MagicMock()
    return sa


@pytest.fixture()
def _bypass_deps():
    """Bypass @requires_dependency checks and provide a fake pandas.DataFrame."""
    mock_pandas = MagicMock()
    mock_pandas.DataFrame = MagicMock
    with (
        patch("snowflake.connector._internal.extras.check_dependency"),
        patch("snowflake.connector.pandas_tools.pandas", mock_pandas),
        patch("snowflake.connector.pandas_tools.sqlalchemy", MagicMock()),
    ):
        yield


class TestPdWriter:
    @pytest.fixture(autouse=True)
    def _deps(self, _bypass_deps):
        pass

    @patch("snowflake.connector.pandas_tools.write_pandas")
    def test_delegates_to_write_pandas(self, mock_wp):
        sf_conn = MagicMock()
        pd_writer(_stub_table(name="orders", schema="SALES"), _stub_sa_conn(sf_conn), ["A", "B"], [(1, 2)])

        kw = mock_wp.call_args.kwargs
        assert kw["conn"] is sf_conn
        assert kw["table_name"] == "ORDERS"
        assert kw["schema"] == "SALES"

    @patch("snowflake.connector.pandas_tools.write_pandas")
    def test_uppercases_table_name(self, mock_wp):
        pd_writer(_stub_table(name="lower"), _stub_sa_conn(), ["c"], [])
        assert mock_wp.call_args.kwargs["table_name"] == "LOWER"

    @patch("snowflake.connector.pandas_tools.write_pandas")
    def test_forwards_extra_kwargs(self, mock_wp):
        pd_writer(_stub_table(), _stub_sa_conn(), ["c"], [], parallel=8)
        assert mock_wp.call_args.kwargs["parallel"] == 8

    @pytest.mark.parametrize("forbidden", ["conn", "df", "table_name", "schema"])
    def test_rejects_reserved_kwargs(self, forbidden):
        with pytest.raises(ProgrammingError, match="cannot be passed to pd_writer"):
            pd_writer(_stub_table(), _stub_sa_conn(), ["c"], [], **{forbidden: "x"})


class TestMakePdWriter:
    @pytest.fixture(autouse=True)
    def _deps(self, _bypass_deps):
        pass

    @patch("snowflake.connector.pandas_tools.write_pandas")
    def test_returns_callable_that_delegates(self, mock_wp):
        writer = make_pd_writer(parallel=2)
        writer(_stub_table(), _stub_sa_conn(), ["c"], [])
        assert mock_wp.call_args.kwargs["parallel"] == 2

    @pytest.mark.parametrize("forbidden", ["table", "conn", "keys", "data_iter"])
    def test_rejects_reserved_kwargs(self, forbidden):
        with pytest.raises(ProgrammingError, match="cannot be passed to make_pd_writer"):
            make_pd_writer(**{forbidden: "x"})

    def test_rejects_all_pd_writer_positional_args(self):
        """Fail when pd_writer gains a positional arg not yet in make_pd_writer's reject list."""
        sig = inspect.signature(pd_writer)
        positional = [
            name for name, p in sig.parameters.items() if p.kind in (p.POSITIONAL_ONLY, p.POSITIONAL_OR_KEYWORD)
        ]
        assert positional, "pd_writer should have positional parameters"
        for name in positional:
            with pytest.raises(ProgrammingError, match="cannot be passed to make_pd_writer"):
                make_pd_writer(**{name: "sentinel"})


# ---------------------------------------------------------------------------
# BD#42: create_temp_table deprecation warning
# ---------------------------------------------------------------------------


class TestCreateTempTableDeprecation:
    @pytest.fixture(autouse=True)
    def _deps(self, _bypass_deps):
        pass

    def test_should_warn_when_create_temp_table_true(self):
        with patch("snowflake.connector.pandas_tools.WritePandasOperation") as mock_op:
            mock_op.return_value.execute.return_value = WritePandasResult(True, 1, 5, [])
            if NEW_DRIVER_ONLY("BD#42"):
                with pytest.warns(DeprecationWarning, match="create_temp_table"):
                    write_pandas(_mock_conn(), _mock_df(), "MY_TABLE", create_temp_table=True)
            else:
                with warnings.catch_warnings():
                    warnings.simplefilter("error", DeprecationWarning)
                    write_pandas(_mock_conn(), _mock_df(), "MY_TABLE", create_temp_table=True)

    def test_should_not_warn_when_table_type_also_set(self):
        with patch("snowflake.connector.pandas_tools.WritePandasOperation") as mock_op:
            mock_op.return_value.execute.return_value = WritePandasResult(True, 1, 5, [])
            with warnings.catch_warnings():
                warnings.simplefilter("error", DeprecationWarning)
                write_pandas(_mock_conn(), _mock_df(), "MY_TABLE", create_temp_table=True, table_type="temp")


# ---------------------------------------------------------------------------
# Sync / async write_pandas signature parity
# ---------------------------------------------------------------------------


class TestWritePandasSignatureParity:
    def test_sync_and_async_write_pandas_have_identical_signatures(self):
        """aio.write_pandas must accept exactly the same kwargs as the sync version."""
        from snowflake.connector.pandas_tools import write_pandas as sync_wp

        sync_params = inspect.signature(sync_wp).parameters
        async_params = inspect.signature(_aio_pt.write_pandas).parameters

        sync_names = {n for n, p in sync_params.items() if p.kind != inspect.Parameter.VAR_KEYWORD}
        async_names = {n for n, p in async_params.items() if p.kind != inspect.Parameter.VAR_KEYWORD}

        assert sync_names == async_names, (
            f"Signature mismatch — sync-only: {sync_names - async_names!r}, async-only: {async_names - sync_names!r}"
        )
