"""Unit tests for pandas_tools module.

Uses MagicMock for DataFrames — no pandas dependency required.
"""

from __future__ import annotations

import inspect

from unittest.mock import MagicMock, patch

import pytest

import snowflake.connector._internal.write_pandas_operation as _wpo

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
from snowflake.connector.pandas_tools import make_pd_writer, pd_writer


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
        sql = op._build_create_stage_sql("MY_STAGE")
        assert "CREATE TEMPORARY STAGE MY_STAGE" in sql
        assert "TYPE=PARQUET" in sql
        assert "COMPRESSION=auto" in sql

    def test_snappy(self):
        op: WritePandasOperation = _make_op(compression="snappy")
        sql = op._build_create_stage_sql("MY_STAGE")
        assert "COMPRESSION=snappy" in sql

    def test_with_binary_as_text_false(self):
        op: WritePandasOperation = _make_op(auto_create_table=True)
        sql = op._build_create_stage_sql("MY_STAGE")
        assert "BINARY_AS_TEXT=FALSE" in sql

    def test_without_binary_as_text(self):
        op: WritePandasOperation = _make_op()
        sql = op._build_create_stage_sql("MY_STAGE")
        assert "BINARY_AS_TEXT" not in sql


# ---------------------------------------------------------------------------
# SQL generation — CREATE FILE FORMAT
# ---------------------------------------------------------------------------


class TestBuildCreateFileFormatSql:
    def test_basic(self):
        op: WritePandasOperation = _make_op()
        sql = op._build_create_file_format_sql("MY_FF")
        assert "CREATE TEMPORARY FILE FORMAT MY_FF" in sql
        assert "TYPE=PARQUET" in sql
        assert "COMPRESSION=auto" in sql

    def test_with_use_logical_type_true(self):
        op: WritePandasOperation = _make_op(use_logical_type=True)
        sql = op._build_create_file_format_sql("MY_FF")
        assert "USE_LOGICAL_TYPE=TRUE" in sql

    def test_with_use_logical_type_false(self):
        op: WritePandasOperation = _make_op(use_logical_type=False)
        sql = op._build_create_file_format_sql("MY_FF")
        assert "USE_LOGICAL_TYPE=FALSE" in sql


# ---------------------------------------------------------------------------
# SQL generation — COPY INTO
# ---------------------------------------------------------------------------


class TestBuildCopyIntoSql:
    def test_basic_with_quoting(self):
        op: WritePandasOperation = _make_op(df=_mock_df(columns=["A", "B"]))
        sql = op._build_copy_into_sql("@MY_STAGE", "MY_TABLE", None)
        assert "COPY INTO MY_TABLE" in sql
        assert '$1:"A" AS "A"' in sql
        assert '$1:"B" AS "B"' in sql
        assert "TYPE=PARQUET" in sql
        assert "PURGE=TRUE" in sql
        assert "ON_ERROR=abort_statement" in sql

    def test_with_column_type_map(self):
        op: WritePandasOperation = _make_op(df=_mock_df(columns=["A"]))
        sql = op._build_copy_into_sql("@MY_STAGE", "MY_TABLE", {"A": "NUMBER(38,0)"})
        assert '$1:"A"::NUMBER(38,0)' in sql

    def test_no_quoting(self):
        op: WritePandasOperation = _make_op(df=_mock_df(columns=["A"]), quote_identifiers=False)
        sql = op._build_copy_into_sql("@MY_STAGE", "MY_TABLE", None)
        assert '$1:"A" AS A' in sql
        assert '"A" AS "A"' not in sql

    def test_with_vectorized_scanner(self):
        op: WritePandasOperation = _make_op(use_vectorized_scanner=True)
        sql = op._build_copy_into_sql("@MY_STAGE", "MY_TABLE", None)
        assert "USE_VECTORIZED_SCANNER=TRUE" in sql


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
