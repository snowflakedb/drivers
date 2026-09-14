"""Smoke coverage for write_pandas MATCH_BY_COLUMN_NAME vs the legacy $1: projection.

These tests are meant to run on both the universal driver and
``hatch run reference-pandas:test`` so the two pipelines can be compared.
"""

from __future__ import annotations

import random

from uuid import uuid4

import pandas as pd
import pytest

from snowflake.connector.errors import ProgrammingError
from snowflake.connector.pandas_tools import write_pandas
from tests.compatibility import IS_UNIVERSAL_DRIVER
from tests.e2e.types.utils import assert_connection_is_open


WEIRD_LABELS = [
    " c o l ",
    '"col',
    '"c""ol',
    "'col",
    '"col"',
    "Year(1)",
    "$col",
    "#hash",
    "col-name",
    "col.with.dots",
    "チリヌル",
    "ป็นมนุ",
    "name with whitespace",
]


def _table(prefix: str) -> str:
    return f"{prefix}_{uuid4().hex[:8]}"


def _q(name: str) -> str:
    return '"' + str(name).replace('"', '""') + '"'


def _quoted_schema(tmp_schema: str) -> str:
    return tmp_schema.upper()


def _write(connection, df, table_name, tmp_schema, **kwargs):
    quote_identifiers = kwargs.get("quote_identifiers", True)
    defaults = {
        "schema": _quoted_schema(tmp_schema) if quote_identifiers else tmp_schema,
        "quote_identifiers": True,
        "auto_create_table": True,
        "table_type": "temp",
    }
    defaults.update(kwargs)
    return write_pandas(connection, df, table_name, **defaults)


class TestWritePandasWeirdColumnNames:
    @pytest.mark.parametrize("label", WEIRD_LABELS)
    def test_should_round_trip_a_hostile_column_label(self, execute_query, connection, cursor, tmp_schema, label):
        assert_connection_is_open(execute_query)
        table_name = _table("WP_WEIRD")
        df = pd.DataFrame({label: ["alpha", "beta"]})

        success, _, nrows, _ = _write(connection, df, table_name, tmp_schema)

        assert success
        assert nrows == 2
        rows = cursor.execute(
            f"SELECT {_q(label)} FROM {_q(_quoted_schema(tmp_schema))}.{_q(table_name)} ORDER BY 1"
        ).fetchall()
        assert [row[0] for row in rows] == ["alpha", "beta"]


class TestWritePandasCaseSensitivity:
    def test_should_match_mixed_case_when_identifiers_are_quoted(self, execute_query, connection, cursor, tmp_schema):
        assert_connection_is_open(execute_query)
        table_name = _table("WP_CASE_Q")
        fq = f"{_q(_quoted_schema(tmp_schema))}.{_q(table_name)}"
        cursor.execute(f"CREATE OR REPLACE TEMPORARY TABLE {fq} ({_q('MyCol')} VARCHAR)")

        success, *_ = write_pandas(
            connection,
            pd.DataFrame({"MyCol": ["quoted"]}),
            table_name,
            schema=_quoted_schema(tmp_schema),
            quote_identifiers=True,
        )

        assert success
        rows = cursor.execute(f"SELECT {_q('MyCol')} FROM {fq} ORDER BY 1").fetchall()
        assert rows == [("quoted",)]

    def test_should_not_match_different_case_when_identifiers_are_quoted(
        self, execute_query, connection, cursor, tmp_schema
    ):
        assert_connection_is_open(execute_query)
        table_name = _table("WP_CASE_MISMATCH")
        fq = f"{_q(_quoted_schema(tmp_schema))}.{_q(table_name)}"
        cursor.execute(f"CREATE OR REPLACE TEMPORARY TABLE {fq} ({_q('MyCol')} VARCHAR)")

        df = pd.DataFrame({"mycol": ["lower"]})
        kwargs = {
            "schema": _quoted_schema(tmp_schema),
            "quote_identifiers": True,
        }
        if IS_UNIVERSAL_DRIVER:
            success, *_ = write_pandas(connection, df, table_name, **kwargs)
            assert success
            rows = cursor.execute(f"SELECT {_q('MyCol')} FROM {fq} ORDER BY 1").fetchall()
            assert rows == [(None,)]
        else:
            with pytest.raises(ProgrammingError):
                write_pandas(connection, df, table_name, **kwargs)

    def test_should_match_any_case_when_identifiers_are_unquoted(self, execute_query, connection, cursor, tmp_schema):
        assert_connection_is_open(execute_query)
        table_name = _table("WP_CASE_UNQ").upper()
        fq = f"{tmp_schema}.{table_name}"
        cursor.execute(f"CREATE OR REPLACE TEMPORARY TABLE {fq} (MYCOL VARCHAR)")

        success, *_ = write_pandas(
            connection,
            pd.DataFrame({"mycol": ["folded"]}),
            table_name,
            schema=tmp_schema,
            quote_identifiers=False,
        )

        assert success
        rows = cursor.execute(f"SELECT MYCOL FROM {fq} ORDER BY 1").fetchall()
        assert rows == [("folded",)]

    def test_should_load_into_columns_that_differ_only_by_case(self, execute_query, connection, cursor, tmp_schema):
        assert_connection_is_open(execute_query)
        table_name = _table("WP_CASE_PAIR")
        fq = f"{_q(_quoted_schema(tmp_schema))}.{_q(table_name)}"
        cursor.execute(f"CREATE OR REPLACE TEMPORARY TABLE {fq} ({_q('Foo')} VARCHAR, {_q('FOO')} VARCHAR)")

        success, *_ = write_pandas(
            connection,
            pd.DataFrame({"Foo": ["mixed"], "FOO": ["upper"]}),
            table_name,
            schema=_quoted_schema(tmp_schema),
            quote_identifiers=True,
        )

        assert success
        rows = cursor.execute(f"SELECT {_q('Foo')}, {_q('FOO')} FROM {fq} ORDER BY {_q('Foo')}").fetchall()
        assert rows == [("mixed", "upper")]

    def test_should_fill_only_the_matching_case_when_a_case_sibling_exists(
        self, execute_query, connection, cursor, tmp_schema
    ):
        assert_connection_is_open(execute_query)
        table_name = _table("WP_CASE_SIBLING")
        fq = f"{_q(_quoted_schema(tmp_schema))}.{_q(table_name)}"
        cursor.execute(f"CREATE OR REPLACE TEMPORARY TABLE {fq} ({_q('Foo')} VARCHAR, {_q('FOO')} VARCHAR)")

        success, *_ = write_pandas(
            connection,
            pd.DataFrame({"Foo": ["mixed-only"]}),
            table_name,
            schema=_quoted_schema(tmp_schema),
            quote_identifiers=True,
        )

        assert success
        rows = cursor.execute(f"SELECT {_q('Foo')}, {_q('FOO')} FROM {fq} ORDER BY {_q('Foo')}").fetchall()
        assert rows == [("mixed-only", None)]


class TestWritePandasExtraAndMissingColumns:
    def test_should_leave_extra_table_columns_null(self, execute_query, connection, cursor, tmp_schema):
        assert_connection_is_open(execute_query)
        table_name = _table("WP_EXTRA_TBL")
        fq = f"{_q(_quoted_schema(tmp_schema))}.{_q(table_name)}"
        cursor.execute(
            f"CREATE OR REPLACE TEMPORARY TABLE {fq} ({_q('A')} VARCHAR, {_q('B')} VARCHAR, {_q('EXTRA')} VARCHAR)"
        )

        success, *_ = write_pandas(
            connection,
            pd.DataFrame({"A": ["x"], "B": ["y"]}),
            table_name,
            schema=_quoted_schema(tmp_schema),
            quote_identifiers=True,
        )

        assert success
        rows = cursor.execute(f"SELECT {_q('A')}, {_q('B')}, {_q('EXTRA')} FROM {fq} ORDER BY 1").fetchall()
        assert rows == [("x", "y", None)]

    def test_should_handle_dataframe_columns_missing_from_the_table(
        self, execute_query, connection, cursor, tmp_schema
    ):
        assert_connection_is_open(execute_query)
        table_name = _table("WP_EXTRA_DF")
        fq = f"{_q(_quoted_schema(tmp_schema))}.{_q(table_name)}"
        cursor.execute(f"CREATE OR REPLACE TEMPORARY TABLE {fq} ({_q('A')} VARCHAR, {_q('B')} VARCHAR)")
        df = pd.DataFrame({"A": ["x"], "B": ["y"], "EXTRA": ["z"]})

        if IS_UNIVERSAL_DRIVER:
            success, *_ = write_pandas(
                connection,
                df,
                table_name,
                schema=_quoted_schema(tmp_schema),
                quote_identifiers=True,
            )
            assert success
            rows = cursor.execute(f"SELECT {_q('A')}, {_q('B')} FROM {fq} ORDER BY 1").fetchall()
            assert rows == [("x", "y")]
        else:
            with pytest.raises(ProgrammingError):
                write_pandas(
                    connection,
                    df,
                    table_name,
                    schema=_quoted_schema(tmp_schema),
                    quote_identifiers=True,
                )


class TestWritePandasRandomizedData:
    def test_should_round_trip_seeded_random_rows(self, execute_query, connection, cursor, tmp_schema):
        assert_connection_is_open(execute_query)
        table_name = _table("WP_RAND")
        rng = random.Random(20260914)
        expected = [(i, rng.random(), rng.choice(["red", "green", "blue"])) for i in range(25)]
        df = pd.DataFrame(expected, columns=["ID", "VAL", "TAG"])

        success, _, nrows, _ = _write(connection, df, table_name, tmp_schema, quote_identifiers=False)

        assert success
        assert nrows == 25
        rows = cursor.execute(f"SELECT ID, VAL, TAG FROM {tmp_schema}.{table_name} ORDER BY ID").fetchall()
        assert len(rows) == 25
        assert [row[0] for row in rows] == list(range(25))
        assert [row[2] for row in rows] == [row[2] for row in expected]
        for got, want in zip(rows, expected, strict=True):
            assert got[1] == pytest.approx(want[1])
