"""write_pandas tests (Python-specific).

This module tests the write_pandas function that writes Pandas DataFrames
to Snowflake tables via the Parquet stage upload pipeline.
"""

from __future__ import annotations

import io
import math
import re
import warnings

from datetime import UTC, date, datetime
from uuid import uuid4

import pandas as pd
import pytest

from snowflake.connector.cursor import DictCursor
from snowflake.connector.errors import ProgrammingError
from snowflake.connector.pandas_tools import write_pandas
from tests.e2e.types.utils import assert_connection_is_open


SAMPLE_DATA = [
    ("Alice", 100),
    ("Bob", 200),
    ("Charlie", 300),
    ("Diana", 400),
    ("Eve", 500),
]
SAMPLE_DF = pd.DataFrame(SAMPLE_DATA, columns=["NAME", "SCORE"])
SHOE_DF = pd.DataFrame(
    [(1, 4.5, "Nike"), (2, 7.5, "Adidas"), (3, 10.5, "Puma")],
    columns=["id", "foot_size", "shoe_make"],
)
EXTRA_COLUMN_DF = pd.DataFrame([(1, "dash", 1000, 32)], columns=["id", "name", "points", "age"])


def _quote_identifier(identifier: str) -> str:
    return '"' + identifier.replace('"', '""') + '"'


def _table(prefix: str) -> str:
    return f"{prefix}_{uuid4().hex[:8]}".upper()


def _schema_arg(tmp_schema: str, quote_identifiers: bool) -> str:
    # Unquoted CREATE SCHEMA stores the name uppercase; quote_identifiers=True
    # quotes the schema as given, so a lowercase tmp_schema would miss the object.
    return tmp_schema.upper() if quote_identifiers else tmp_schema


def _parquet_field_names(df: pd.DataFrame) -> list[str]:
    import pyarrow.parquet as pq

    buffer = io.BytesIO()
    df.to_parquet(buffer)
    return list(pq.read_schema(io.BytesIO(buffer.getvalue())).names)


def _fq_table(tmp_schema: str, table_name: str, quote_identifiers: bool) -> str:
    schema = _schema_arg(tmp_schema, quote_identifiers)
    if quote_identifiers:
        return f"{schema}.{_quote_identifier(table_name)}"
    return f"{schema}.{table_name}"


def _sql_ident(name: str, quote_identifiers: bool) -> str:
    return _quote_identifier(name) if quote_identifiers else name


def _missing_name_identifier(quote_identifiers: bool) -> str:
    ident = _quote_identifier("name") if quote_identifiers else "NAME"
    return re.escape(f"invalid identifier '{ident}'")


def _seed_table(connection, table_name: str, schema: str, quote_identifiers: bool) -> None:
    write_pandas(
        connection,
        SHOE_DF,
        table_name,
        schema=schema,
        quote_identifiers=quote_identifiers,
        auto_create_table=True,
        table_type="temp",
    )


class TestWritePandas:
    """Tests for write_pandas function."""

    def test_should_write_a_dataframe_to_a_pre_created_table_and_read_it_back(
        self, execute_query, connection, cursor, tmp_schema
    ):
        table_name = _table("WP_BASIC")
        fq_table = f"{tmp_schema}.{table_name}"

        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And A temporary table with columns name STRING and score INT exists
        cursor.execute(f"CREATE OR REPLACE TEMPORARY TABLE {fq_table} (NAME STRING, SCORE INT)")

        # When write_pandas is called with the sample DataFrame
        success, nchunks, nrows, _ = write_pandas(
            connection,
            SAMPLE_DF,
            table_name,
            schema=tmp_schema,
            quote_identifiers=False,
        )

        # Then write_pandas should return success with correct chunk and row counts
        assert success
        assert nchunks == 1
        assert nrows == len(SAMPLE_DATA)

        # And SELECT from the table should return all original rows
        result = cursor.execute(f"SELECT * FROM {fq_table}").fetchall()
        assert set(result) == set(SAMPLE_DATA)

    def test_should_auto_create_a_table_from_dataframe_schema(self, execute_query, connection, cursor, tmp_schema):
        table_name = _table("WP_AUTOCREATE")
        fq_table = f"{tmp_schema}.{table_name}"

        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When write_pandas is called with auto_create_table=True and table_type="temp"
        success, nchunks, nrows, _ = write_pandas(
            connection,
            SAMPLE_DF,
            table_name,
            schema=tmp_schema,
            quote_identifiers=False,
            auto_create_table=True,
            table_type="temp",
        )

        # Then write_pandas should return success with correct chunk and row counts
        assert success
        assert nchunks == 1
        assert nrows == len(SAMPLE_DATA)

        # And SELECT from the table should return all original rows
        result = cursor.execute(f"SELECT * FROM {fq_table}").fetchall()
        assert set(result) == set(SAMPLE_DATA)

    def test_should_overwrite_existing_data_with_new_data(self, execute_query, connection, cursor, tmp_schema):
        table_name = _table("WP_OVERWRITE")
        fq_table = f"{tmp_schema}.{table_name}"
        initial_data = [("Frank", 10), ("Grace", 20), ("Hank", 30)]
        initial_df = pd.DataFrame(initial_data, columns=["NAME", "SCORE"])
        new_data = [("Ivy", 99)]
        new_df = pd.DataFrame(new_data, columns=["NAME", "SCORE"])

        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And A temporary table with columns name STRING and score INT exists
        cursor.execute(f"CREATE OR REPLACE TEMPORARY TABLE {fq_table} (NAME STRING, SCORE INT)")

        # And The table contains initial data
        write_pandas(connection, initial_df, table_name, schema=tmp_schema, quote_identifiers=False)

        # When write_pandas is called with new data and overwrite=True
        success, nchunks, nrows, _ = write_pandas(
            connection,
            new_df,
            table_name,
            schema=tmp_schema,
            quote_identifiers=False,
            overwrite=True,
        )

        # Then write_pandas should return success with correct chunk and row counts
        assert success
        assert nchunks == 1
        assert nrows == 1

        # And The table should contain only the new data
        result = cursor.execute(f"SELECT * FROM {fq_table}").fetchall()
        assert result == new_data

    def test_should_write_dataframe_in_multiple_chunks(self, execute_query, connection, cursor, tmp_schema):
        table_name = _table("WP_CHUNKED")
        fq_table = f"{tmp_schema}.{table_name}"
        chunk_size = 2
        expected_chunks = math.ceil(len(SAMPLE_DATA) / chunk_size)

        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And A temporary table with columns name STRING and score INT exists
        cursor.execute(f"CREATE OR REPLACE TEMPORARY TABLE {fq_table} (NAME STRING, SCORE INT)")

        # When write_pandas is called with chunk_size=2
        success, nchunks, nrows, _ = write_pandas(
            connection,
            SAMPLE_DF,
            table_name,
            schema=tmp_schema,
            quote_identifiers=False,
            chunk_size=chunk_size,
        )

        # Then write_pandas should return 3 chunks for a 5-row DataFrame
        assert success
        assert nchunks == expected_chunks
        assert nrows == len(SAMPLE_DATA)

        # And All original rows should be present in the table
        result = cursor.execute(f"SELECT * FROM {fq_table}").fetchall()
        assert set(result) == set(SAMPLE_DATA)

    def test_should_round_trip_multiple_data_types_through_write_pandas(self, execute_query, connection, tmp_schema):
        table_name = _table("WP_TYPES")
        fq_table = f"{tmp_schema}.{table_name}"
        ts_tz = datetime(2026, 4, 1, 9, 30, 29, tzinfo=UTC)
        ts_ntz = datetime(2026, 4, 2, 14, 15, 59)
        types_df = pd.DataFrame(
            {
                "COL_INT": [1, 2],
                "COL_FLOAT": [1.25, 2.75],
                "COL_STR": ["hello", "world"],
                "COL_BOOL": [True, False],
                "COL_DATE": [date(2026, 4, 1), date(2026, 4, 2)],
                "COL_BINARY": [b"\xde\xad", b"\xbe\xef"],
                "COL_TS_TZ": [ts_tz, ts_tz],
                "COL_TS_NTZ": [ts_ntz, ts_ntz],
            }
        )

        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When write_pandas is called with a multi-type DataFrame using auto_create_table=True and use_logical_type=True
        success, nchunks, nrows, _ = write_pandas(
            connection,
            types_df,
            table_name,
            schema=tmp_schema,
            quote_identifiers=False,
            auto_create_table=True,
            table_type="temp",
            use_logical_type=True,
        )

        # Then write_pandas should return success with correct chunk and row counts
        assert success
        assert nchunks == 1
        assert nrows == 2

        # And All values should match the original data including timestamps
        with connection.cursor(DictCursor) as cur:
            rows = cur.execute(f"SELECT * FROM {fq_table} ORDER BY COL_INT").fetchall()
        assert len(rows) == 2

        row0, row1 = rows[0], rows[1]

        assert row0["COL_INT"] == 1
        assert row1["COL_INT"] == 2
        assert row0["COL_FLOAT"] == pytest.approx(1.25)
        assert row1["COL_FLOAT"] == pytest.approx(2.75)
        assert row0["COL_STR"] == "hello"
        assert row1["COL_STR"] == "world"
        assert row0["COL_BOOL"] is True
        assert row1["COL_BOOL"] is False
        assert row0["COL_DATE"] == date(2026, 4, 1)
        assert row1["COL_DATE"] == date(2026, 4, 2)
        assert row0["COL_BINARY"] == b"\xde\xad"
        assert row1["COL_BINARY"] == b"\xbe\xef"
        assert row0["COL_TS_TZ"] == ts_tz
        assert row0["COL_TS_NTZ"] == ts_ntz
        assert row1["COL_TS_TZ"] == ts_tz
        assert row1["COL_TS_NTZ"] == ts_ntz


class TestWritePandasValidation:
    """Tests for write_pandas input validation and warnings.

    Validation fires before any Snowflake interaction, so these tests
    use a real connection but never actually write data.
    """

    def test_should_raise_programming_error_when_database_is_set_without_schema(self, execute_query, connection):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When write_pandas is called with database but no schema
        kwargs = {"database": "mydb"}

        # Then ProgrammingError should be raised
        with pytest.raises(ProgrammingError):
            write_pandas(connection, SAMPLE_DF, "t", **kwargs)

    def test_should_raise_programming_error_for_invalid_compression(self, execute_query, connection):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When write_pandas is called with an unsupported compression value
        kwargs = {"compression": "bzip2"}

        # Then ProgrammingError should be raised
        with pytest.raises(ProgrammingError):
            write_pandas(connection, SAMPLE_DF, "t", **kwargs)

    def test_should_raise_value_error_for_invalid_table_type(self, execute_query, connection):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When write_pandas is called with an invalid table_type
        kwargs = {"table_type": "bogus"}

        # Then ValueError should be raised
        with pytest.raises(ValueError):
            write_pandas(connection, SAMPLE_DF, "t", **kwargs)

    def test_should_emit_user_warning_for_tz_aware_columns_without_use_logical_type(self, execute_query, connection):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And A DataFrame with a tz-aware datetime column
        tz_df = pd.DataFrame({"ts": [datetime(2024, 1, 1, tzinfo=UTC)]})

        # When write_pandas is called without use_logical_type=True
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            try:
                write_pandas(connection, tz_df, "t")
            except Exception:
                pass

        # Then UserWarning about timezone should be emitted
        assert any(issubclass(w.category, UserWarning) and "timezone" in str(w.message).lower() for w in caught), (
            f"Expected UserWarning about timezone, got: {[str(w.message) for w in caught]}"
        )

    def test_should_emit_user_warning_for_non_standard_dataframe_index(self, execute_query, connection):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # And A DataFrame with a string index
        string_idx_df = pd.DataFrame({"val": [10, 20]}, index=["a", "b"])

        # When write_pandas is called with the non-standard index DataFrame
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            try:
                write_pandas(connection, string_idx_df, "t")
            except Exception:
                pass

        # Then UserWarning about non-standard index should be emitted
        assert any(issubclass(w.category, UserWarning) and "index" in str(w.message).lower() for w in caught), (
            f"Expected UserWarning about index, got: {[str(w.message) for w in caught]}"
        )

    def test_should_handle_invalid_iceberg_config_keys(self, execute_query, connection, tmp_schema):
        # Given Snowflake client is logged in
        assert_connection_is_open(execute_query)

        # When write_pandas is called with iceberg_config containing invalid keys
        kwargs = {"iceberg_config": {"invalid_key": "value"}}

        # Then ProgrammingError should be raised
        with pytest.raises(ProgrammingError, match="INVALID_KEY"):
            write_pandas(
                connection,
                SAMPLE_DF,
                _table("WP_ICEBERG"),
                schema=tmp_schema,
                quote_identifiers=False,
                auto_create_table=True,
                **kwargs,
            )


class TestWritePandasIdentifierAndSchemaCoverage:
    """Quoted identifiers, special column names, overwrite/auto-create, and extra table columns."""

    def test_should_round_trip_with_default_quoted_identifiers(self, execute_query, connection, tmp_schema):
        table_name = _table("WP_QUOTED")
        fq_table = _fq_table(tmp_schema, table_name, True)
        df = pd.DataFrame(
            [(1, 4.5, "t1"), (2, 7.5, "t2"), (3, 10.5, "t3")],
            columns=["id", "foot_size", "shoe_model"],
        )

        assert_connection_is_open(execute_query)

        # When write_pandas auto-creates a table using its default quoted identifiers
        success, nchunks, nrows, _ = write_pandas(
            connection,
            df,
            table_name,
            schema=_schema_arg(tmp_schema, True),
            auto_create_table=True,
            table_type="temp",
        )

        # Then the quoted lowercase columns and values should round-trip
        assert success
        assert nchunks == 1
        assert nrows == 3

        with connection.cursor(DictCursor) as cur:
            rows = cur.execute(f'SELECT * FROM {fq_table} ORDER BY "id"').fetchall()
            columns = [col[0] for col in cur.description]
        assert columns == ["id", "foot_size", "shoe_model"]
        assert [(row["id"], row["foot_size"], row["shoe_model"]) for row in rows] == [
            (1, 4.5, "t1"),
            (2, 7.5, "t2"),
            (3, 10.5, "t3"),
        ]

    @pytest.mark.parametrize(
        "index, parquet_index_field",
        [
            (pd.Index([2, 3, 4], name="index"), "index"),
            (pd.Index([10, 11, 12]), "__index_level_0__"),
            (pd.Index([0.5, 1.5, 2.5]), "__index_level_0__"),
        ],
    )
    def test_should_not_materialize_the_dataframe_index_as_a_table_column(
        self, execute_query, connection, cursor, tmp_schema, index, parquet_index_field
    ):
        table_name = _table("WP_NOINDEX")
        fq_table = _fq_table(tmp_schema, table_name, True)
        df = pd.DataFrame({"a": [1, 2, 3], "b": [4, 5, 6]}, index=index)

        assert_connection_is_open(execute_query)

        # A non-default index is written into the Parquet file as a physical column;
        # a default RangeIndex is stored as metadata only and cannot leak.
        assert parquet_index_field in _parquet_field_names(df)

        # When write_pandas auto-creates and appends to a table from that DataFrame
        with warnings.catch_warnings():
            warnings.simplefilter("ignore", UserWarning)
            success, _, nrows, _ = write_pandas(
                connection,
                df,
                table_name,
                schema=_schema_arg(tmp_schema, True),
                auto_create_table=True,
                table_type="temp",
            )

            # Then only DataFrame columns should appear in the table
            assert success
            assert nrows == 3

            cursor.execute(f'SELECT * FROM {fq_table} ORDER BY "a"')
            columns = [col[0] for col in cursor.description]
            assert columns == ["a", "b"]

            success, _, nrows, _ = write_pandas(
                connection,
                pd.DataFrame({"a": [4], "b": [7]}, index=pd.Index([9], name=index.name)),
                table_name,
                schema=_schema_arg(tmp_schema, True),
            )
        assert success
        assert nrows == 1
        cursor.execute(f'SELECT * FROM {fq_table} ORDER BY "a"')
        columns = [col[0] for col in cursor.description]
        rows = cursor.fetchall()
        assert columns == ["a", "b"]
        assert rows == [(1, 4), (2, 5), (3, 6), (4, 7)]

    @pytest.mark.parametrize("quote_identifiers", [True, False])
    def test_should_replace_schema_when_overwriting_with_auto_create(
        self, execute_query, connection, cursor, tmp_schema, quote_identifiers
    ):
        table_name = _table("WP_OW_SCHEMA")
        schema = _schema_arg(tmp_schema, quote_identifiers)
        fq_table = _fq_table(tmp_schema, table_name, quote_identifiers)

        assert_connection_is_open(execute_query)
        _seed_table(connection, table_name, schema, quote_identifiers)

        # When overwrite and auto_create replace the existing table
        success, _, nrows, _ = write_pandas(
            connection,
            EXTRA_COLUMN_DF,
            table_name,
            schema=schema,
            quote_identifiers=quote_identifiers,
            overwrite=True,
            auto_create_table=True,
        )

        # Then the table should take the replacement schema and rows
        assert success
        assert nrows == 1
        order_column = _sql_ident("id", quote_identifiers)
        cursor.execute(f"SELECT * FROM {fq_table} ORDER BY {order_column}")
        columns = [col[0] for col in cursor.description]
        expected = ["id", "name", "points", "age"]
        if not quote_identifiers:
            expected = [c.upper() for c in expected]
        assert columns == expected
        assert cursor.fetchall() == [(1, "dash", 1000, 32)]

    @pytest.mark.parametrize("quote_identifiers", [True, False])
    def test_should_reject_extra_columns_when_the_existing_schema_cannot_change(
        self, execute_query, connection, cursor, tmp_schema, quote_identifiers
    ):
        table_name = _table("WP_OW_EXTRA")
        schema = _schema_arg(tmp_schema, quote_identifiers)
        fq_table = _fq_table(tmp_schema, table_name, quote_identifiers)

        assert_connection_is_open(execute_query)
        _seed_table(connection, table_name, schema, quote_identifiers)

        # When extra columns are written without replacing the table
        with pytest.raises(ProgrammingError, match=_missing_name_identifier(quote_identifiers)):
            write_pandas(
                connection,
                EXTRA_COLUMN_DF,
                table_name,
                schema=schema,
                quote_identifiers=quote_identifiers,
                overwrite=False,
                auto_create_table=True,
            )

        # Then the original rows should remain
        order_id = _sql_ident("id", quote_identifiers)
        order_foot = _sql_ident("foot_size", quote_identifiers)
        rows = cursor.execute(f"SELECT * FROM {fq_table} ORDER BY {order_id}, {order_foot}").fetchall()
        assert rows == [
            (1, 4.5, "Nike"),
            (2, 7.5, "Adidas"),
            (3, 10.5, "Puma"),
        ]

    @pytest.mark.parametrize("quote_identifiers", [True, False])
    def test_should_leave_the_table_empty_when_overwrite_truncates_then_copy_fails(
        self, execute_query, connection, cursor, tmp_schema, quote_identifiers
    ):
        table_name = _table("WP_OW_TRUNC")
        schema = _schema_arg(tmp_schema, quote_identifiers)
        fq_table = _fq_table(tmp_schema, table_name, quote_identifiers)

        assert_connection_is_open(execute_query)
        _seed_table(connection, table_name, schema, quote_identifiers)

        # When overwrite=True with auto_create_table=False truncates before COPY, a
        # COPY failure on extra columns leaves the table empty.
        with pytest.raises(ProgrammingError, match=_missing_name_identifier(quote_identifiers)):
            write_pandas(
                connection,
                EXTRA_COLUMN_DF,
                table_name,
                schema=schema,
                quote_identifiers=quote_identifiers,
                overwrite=True,
                auto_create_table=False,
            )

        # Then the table should remain empty
        count = cursor.execute(f"SELECT COUNT(*) FROM {fq_table}").fetchone()[0]
        assert count == 0

    def test_should_raise_when_the_target_table_does_not_exist(self, execute_query, connection, tmp_schema):
        assert_connection_is_open(execute_query)
        missing = _table("WP_MISSING")

        # When write_pandas targets a missing table without auto-create
        # Then it should report that this table, not some other object, does not exist
        with pytest.raises(ProgrammingError, match=rf"{re.escape(missing)}'? does not exist"):
            write_pandas(
                connection,
                SAMPLE_DF,
                missing,
                schema=_schema_arg(tmp_schema, True),
                auto_create_table=False,
            )

    @pytest.mark.parametrize(
        "column_names",
        [
            ["00 name", "bAl_ance"],
            ['c""ol', '"col"'],
            ["c''ol", "'col'"],
            ["チリヌル", "熊猫"],
            ['{"snow": {"fla": "ke"}}', "col\\with\\backslash"],
        ],
    )
    @pytest.mark.parametrize("auto_create_table", [True, False])
    def test_should_quote_special_column_names(
        self, execute_query, connection, cursor, tmp_schema, column_names, auto_create_table
    ):
        table_name = _table("WP_SPECIAL")
        schema = _schema_arg(tmp_schema, True)
        fq_table = _fq_table(tmp_schema, table_name, True)
        df = pd.DataFrame([("Mark", 10), ("Luke", 20)], columns=column_names)
        escaped = [name.replace('"', '""') for name in column_names]

        assert_connection_is_open(execute_query)

        # When write_pandas writes quoted special names to an existing or auto-created table
        if not auto_create_table:
            cursor.execute(
                f"CREATE OR REPLACE TEMPORARY TABLE {fq_table} "
                f'("{escaped[0]}" STRING, "{escaped[1]}" INT, "id" INT AUTOINCREMENT)'
            )

        success, nchunks, nrows, _ = write_pandas(
            connection,
            df,
            table_name,
            schema=schema,
            quote_identifiers=True,
            auto_create_table=auto_create_table,
            table_type="temp" if auto_create_table else "",
        )

        # Then all names and values should be preserved
        assert success
        assert nchunks == 1
        assert nrows == 2

        with connection.cursor(DictCursor) as cur:
            rows = cur.execute(f"SELECT * FROM {fq_table} ORDER BY 1").fetchall()
        values = {(row[column_names[0]], row[column_names[1]]) for row in rows}
        assert values == {("Mark", 10), ("Luke", 20)}
        if not auto_create_table:
            assert {row["id"] for row in rows} == {1, 2}

    def test_should_keep_case_distinct_column_names_when_auto_creating(self, execute_query, connection, tmp_schema):
        table_name = _table("WP_CASE")
        fq_table = _fq_table(tmp_schema, table_name, True)
        df = pd.DataFrame([(10, 11), (20, 21)], columns=["number", "Number"])

        assert_connection_is_open(execute_query)

        # When write_pandas auto-creates columns that differ only by case
        success, _, nrows, _ = write_pandas(
            connection,
            df,
            table_name,
            schema=_schema_arg(tmp_schema, True),
            quote_identifiers=True,
            auto_create_table=True,
            table_type="temp",
        )
        assert success
        assert nrows == 2

        with connection.cursor(DictCursor) as cur:
            rows = cur.execute(f'SELECT * FROM {fq_table} ORDER BY "number"').fetchall()
            columns = [col[0] for col in cur.description]

        # Then both case-distinct columns should remain independently addressable
        assert columns == ["number", "Number"]
        assert {(row["number"], row["Number"]) for row in rows} == {(10, 11), (20, 21)}

    @pytest.mark.parametrize("quote_identifiers", [True, False])
    def test_should_leave_autoincrement_columns_to_the_table(
        self, execute_query, connection, cursor, tmp_schema, quote_identifiers
    ):
        table_name = _table("WP_AI")
        schema = _schema_arg(tmp_schema, quote_identifiers)
        fq_table = _fq_table(tmp_schema, table_name, quote_identifiers)
        df = pd.DataFrame([("Mark", 10), ("Luke", 20)], columns=["name", "balance"])
        name_col = _sql_ident("name", quote_identifiers)
        balance_col = _sql_ident("balance", quote_identifiers)
        id_col = _sql_ident("id", quote_identifiers)

        assert_connection_is_open(execute_query)
        cursor.execute(
            f"CREATE OR REPLACE TEMPORARY TABLE {fq_table} "
            f"({name_col} STRING, {balance_col} INT, {id_col} INT AUTOINCREMENT)"
        )

        # When write_pandas supplies only the non-autoincrement columns
        success, _, nrows, _ = write_pandas(
            connection,
            df,
            table_name,
            schema=schema,
            quote_identifiers=quote_identifiers,
        )
        assert success
        assert nrows == 2

        with connection.cursor(DictCursor) as cur:
            rows = cur.execute(f"SELECT * FROM {fq_table} ORDER BY {id_col}").fetchall()

        # Then Snowflake should populate the omitted autoincrement column
        id_key = "id" if quote_identifiers else "ID"
        name_key = "name" if quote_identifiers else "NAME"
        balance_key = "balance" if quote_identifiers else "BALANCE"
        assert {row[id_key] for row in rows} == {1, 2}
        assert {(row[name_key], row[balance_key]) for row in rows} == {("Mark", 10), ("Luke", 20)}

    @pytest.mark.parametrize("quote_identifiers", [True, False])
    def test_should_leave_default_columns_to_the_table(
        self, execute_query, connection, cursor, tmp_schema, quote_identifiers
    ):
        table_name = _table("WP_DEF")
        schema = _schema_arg(tmp_schema, quote_identifiers)
        fq_table = _fq_table(tmp_schema, table_name, quote_identifiers)
        df = pd.DataFrame([("Mark", 10), ("Luke", 20)], columns=["name", "balance"])
        name_col = _sql_ident("name", quote_identifiers)
        balance_col = _sql_ident("balance", quote_identifiers)
        id_col = _sql_ident("id", quote_identifiers)
        ts_col = _sql_ident("ts", quote_identifiers)

        assert_connection_is_open(execute_query)
        cursor.execute(
            f"CREATE OR REPLACE TEMPORARY TABLE {fq_table} "
            f"({name_col} STRING, {balance_col} INT, "
            f"{id_col} VARCHAR(36) DEFAULT UUID_STRING(), "
            f"{ts_col} TIMESTAMP_LTZ DEFAULT CURRENT_TIMESTAMP)"
        )

        # When write_pandas supplies only the columns without defaults
        success, _, nrows, _ = write_pandas(
            connection,
            df,
            table_name,
            schema=schema,
            quote_identifiers=quote_identifiers,
        )
        assert success
        assert nrows == 2

        with connection.cursor(DictCursor) as cur:
            rows = cur.execute(f"SELECT * FROM {fq_table} ORDER BY {name_col}").fetchall()

        # Then Snowflake should populate each omitted default column
        id_key = "id" if quote_identifiers else "ID"
        ts_key = "ts" if quote_identifiers else "TS"
        name_key = "name" if quote_identifiers else "NAME"
        balance_key = "balance" if quote_identifiers else "BALANCE"
        assert {(row[name_key], row[balance_key]) for row in rows} == {("Mark", 10), ("Luke", 20)}
        for row in rows:
            assert row[id_key] is not None
            assert len(row[id_key]) == 36
            assert row[ts_key] is not None

    def test_should_write_an_empty_dataframe(self, execute_query, connection, cursor, tmp_schema):
        table_name = _table("WP_EMPTY")
        fq_table = _fq_table(tmp_schema, table_name, True)
        df = pd.DataFrame([], columns=["name", "balance"])

        assert_connection_is_open(execute_query)

        # When write_pandas receives an empty DataFrame with auto-create enabled
        success, nchunks, nrows, _ = write_pandas(
            connection,
            df,
            table_name,
            schema=_schema_arg(tmp_schema, True),
            auto_create_table=True,
            table_type="temp",
        )

        # Then it should create an empty table and report one empty chunk
        assert success
        assert nchunks == 1
        assert nrows == 0
        cursor.execute(f"SELECT * FROM {fq_table}")
        assert [col[0] for col in cursor.description] == ["name", "balance"]
        assert cursor.fetchall() == []

    def test_should_append_when_auto_create_targets_an_existing_table(
        self, execute_query, connection, cursor, tmp_schema
    ):
        table_name = _table("WP_AUTO_APPEND")
        fq_table = _fq_table(tmp_schema, table_name, True)
        df = pd.DataFrame({"id": [1, 2], "name": ["one", "two"]})

        assert_connection_is_open(execute_query)

        # When auto-create writes the same DataFrame to the same table twice
        for _ in range(2):
            success, _, nrows, _ = write_pandas(
                connection,
                df,
                table_name,
                schema=_schema_arg(tmp_schema, True),
                auto_create_table=True,
                table_type="temp",
            )
            assert success
            assert nrows == 2

        # Then the second call should append instead of replacing the table
        rows = cursor.execute(f'SELECT * FROM {fq_table} ORDER BY "id", "name"').fetchall()
        assert rows == [(1, "one"), (1, "one"), (2, "two"), (2, "two")]

    @pytest.mark.parametrize("quote_identifiers", [True, False])
    def test_should_map_reordered_dataframe_columns_and_leave_extra_target_columns_to_defaults(
        self, execute_query, connection, cursor, tmp_schema, quote_identifiers
    ):
        table_name = _table("WP_REORDER")
        schema = _schema_arg(tmp_schema, quote_identifiers)
        fq_table = _fq_table(tmp_schema, table_name, quote_identifiers)
        first = _sql_ident("first", quote_identifiers)
        second = _sql_ident("second", quote_identifiers)
        generated = _sql_ident("generated", quote_identifiers)
        df = pd.DataFrame([(20, 10), (40, 30)], columns=["second", "first"])

        assert_connection_is_open(execute_query)
        cursor.execute(
            f"CREATE OR REPLACE TEMPORARY TABLE {fq_table} ({first} INT, {second} INT, {generated} INT DEFAULT 99)"
        )

        # When DataFrame columns are reordered and the target has an extra defaulted column
        success, _, nrows, _ = write_pandas(
            connection,
            df,
            table_name,
            schema=schema,
            quote_identifiers=quote_identifiers,
        )

        # Then values should map by the explicit DataFrame column list and the default should apply
        assert success
        assert nrows == 2
        rows = cursor.execute(f"SELECT * FROM {fq_table} ORDER BY {first}").fetchall()
        assert rows == [(10, 20, 99), (30, 40, 99)]

    @pytest.mark.parametrize("quote_identifiers", [True, False])
    def test_should_reject_dataframe_columns_missing_from_the_target_table(
        self, execute_query, connection, cursor, tmp_schema, quote_identifiers
    ):
        table_name = _table("WP_EXTRA_DF")
        schema = _schema_arg(tmp_schema, quote_identifiers)
        fq_table = _fq_table(tmp_schema, table_name, quote_identifiers)
        existing = _sql_ident("existing", quote_identifiers)
        df = pd.DataFrame([(1, 2)], columns=["existing", "missing"])

        assert_connection_is_open(execute_query)
        cursor.execute(f"CREATE OR REPLACE TEMPORARY TABLE {fq_table} ({existing} INT)")

        # When the DataFrame contains a column absent from the target table
        # Then write_pandas should reject it as an invalid identifier
        with pytest.raises(ProgrammingError, match="invalid identifier"):
            write_pandas(
                connection,
                df,
                table_name,
                schema=schema,
                quote_identifiers=quote_identifiers,
            )

        assert cursor.execute(f"SELECT COUNT(*) FROM {fq_table}").fetchone()[0] == 0

    def test_should_infer_timestamp_types_when_logical_types_are_enabled(
        self, execute_query, connection, cursor, tmp_schema
    ):
        table_name = _table("WP_LOGICAL")
        fq_table = _fq_table(tmp_schema, table_name, True)
        df = pd.DataFrame(
            {
                "plain": [pd.Timestamp("2026-01-02 03:04:05")],
                "localized": [pd.Timestamp("2026-01-02 03:04:05", tz="UTC")],
            }
        )

        assert_connection_is_open(execute_query)

        # When write_pandas infers an auto-created table with logical types enabled
        success, _, nrows, _ = write_pandas(
            connection,
            df,
            table_name,
            schema=_schema_arg(tmp_schema, True),
            quote_identifiers=True,
            auto_create_table=True,
            table_type="temp",
            use_logical_type=True,
        )

        # Then naive and localized timestamps should infer NTZ and LTZ columns
        assert success
        assert nrows == 1
        description = cursor.execute(f"DESC TABLE {fq_table}").fetchall()
        types = {row[0]: row[1] for row in description}
        assert types["plain"].startswith("TIMESTAMP_NTZ")
        assert types["localized"].startswith("TIMESTAMP_LTZ")

    @pytest.mark.parametrize("table_type", ["", "temp", "temporary", "transient"])
    def test_should_create_the_requested_table_type(self, execute_query, connection, tmp_schema, table_type):
        table_name = _table("WP_TTYPE")
        fq_table = f"{tmp_schema}.{table_name}"
        assert_connection_is_open(execute_query)

        try:
            # When write_pandas auto-creates the requested table type
            success, _, _, _ = write_pandas(
                connection,
                SAMPLE_DF,
                table_name,
                schema=tmp_schema,
                quote_identifiers=False,
                auto_create_table=True,
                table_type=table_type,
            )
            assert success

            with connection.cursor(DictCursor) as cur:
                cur.execute(f"SHOW TABLES LIKE '{table_name}' IN SCHEMA {tmp_schema}")
                info = cur.fetchone()
            assert info is not None

            # Then SHOW TABLES should report the corresponding Snowflake table kind
            kind = info.get("kind") or info.get("KIND")
            if not table_type:
                assert kind == "TABLE"
            elif table_type == "temp":
                assert kind == "TEMPORARY"
            else:
                assert kind == table_type.upper()
        finally:
            with connection.cursor() as cur:
                cur.execute(f"DROP TABLE IF EXISTS {fq_table}")

    def test_should_write_to_a_table_whose_name_contains_a_single_quote(
        self, execute_query, connection, cursor, tmp_schema
    ):
        bare = f"test'table_{uuid4().hex[:8]}"
        quoted = _quote_identifier(bare)
        fq_table = f"{tmp_schema}.{quoted}"
        df = pd.DataFrame([[1], [2]], columns=["a"])

        assert_connection_is_open(execute_query)
        cursor.execute(f"CREATE OR REPLACE TEMPORARY TABLE {fq_table} (A INT)")

        # When write_pandas targets a pre-quoted table name containing a single quote
        success, _, nrows, _ = write_pandas(
            connection,
            df,
            quoted,
            schema=tmp_schema,
            quote_identifiers=False,
            auto_create_table=False,
        )

        # Then all rows should be loaded into that exact table
        assert success
        assert nrows == 2
        rows = cursor.execute(f"SELECT A FROM {fq_table} ORDER BY A").fetchall()
        assert rows == [(1,), (2,)]

    def test_should_write_a_column_name_that_already_contains_quotes(self, execute_query, connection, tmp_schema):
        table_name = _table("WP_CAT")
        fq_table = _fq_table(tmp_schema, table_name, True)
        df = pd.DataFrame([[1], [2]], columns=["col_'\"cat\"'"])

        assert_connection_is_open(execute_query)

        # When write_pandas auto-creates a column whose name already contains quotes
        success, _, nrows, _ = write_pandas(
            connection,
            df,
            table_name,
            schema=_schema_arg(tmp_schema, True),
            auto_create_table=True,
            table_type="temp",
        )

        # Then the quoted column name and rows should round-trip unchanged
        assert success
        assert nrows == 2
        expected_name = "col_'\"cat\"'"
        with connection.cursor(DictCursor) as cur:
            rows = cur.execute(f"SELECT * FROM {fq_table} ORDER BY 1").fetchall()
            columns = [col[0] for col in cur.description]
        assert columns == [expected_name]
        assert [row[expected_name] for row in rows] == [1, 2]
