"""FILE type tests for Universal Driver.

A FILE column (produced by TO_FILE) is not decoded by any driver: it is read as
the raw, undecoded JSON document the server sends, the same representation
used for VARIANT.
"""

from __future__ import annotations

import json

import pytest

from snowflake.connector.constants import FIELD_ID_TO_NAME

from .utils import assert_sequential_values, assert_type, parse_json_value


LARGE_RESULT_SET_SIZE = 20_000

EXPECTED_FILE_DOCUMENT = {
    "RELATIVE_PATH": "some_new_file.jpeg",
    "STAGE": "@myStage",
    "STAGE_FILE_URL": "some_new_file.jpeg",
    "SIZE": 123,
    "ETAG": "xxx",
    "CONTENT_TYPE": "image/jpeg",
    "LAST_MODIFIED": "2025-01-01",
}

FILE_EXPR = f"TO_FILE(PARSE_JSON('{json.dumps(EXPECTED_FILE_DOCUMENT)}'))"

EXPECTED_OTHER_FILE_DOCUMENT = {
    "RELATIVE_PATH": "quarterly_report.pdf",
    "STAGE": "@otherStage",
    "STAGE_FILE_URL": "reports/quarterly_report.pdf",
    "SIZE": 45678,
    "ETAG": "yyy",
    "CONTENT_TYPE": "application/pdf",
    "LAST_MODIFIED": "2025-06-30",
}

OTHER_FILE_EXPR = f"TO_FILE(PARSE_JSON('{json.dumps(EXPECTED_OTHER_FILE_DOCUMENT)}'))"


def _create_file_table(cursor, table_name: str) -> None:
    cursor.execute(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} AS SELECT {FILE_EXPR} AS file_col")


@pytest.fixture
def file_table(execute_query, tmp_schema):
    """Create a temporary table with an ID and a FILE column."""
    table_name = f"{tmp_schema}.file_table"
    execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (id INT, file_col FILE)")
    return table_name


class TestFileTypeCasting:
    """Tests for FILE column value decoding."""

    def test_should_cast_file_values_to_appropriate_type(self, cursor, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # When A FILE column is populated via TO_FILE and queried
        table_name = f"{tmp_schema}.file_table"
        _create_file_table(cursor, table_name)
        rows = cursor.execute(f"SELECT * FROM {table_name}").fetchall()

        # Then the FILE column should be returned as appropriate type with the expected JSON document
        values = [row[0] for row in rows]
        assert_type(values, str)
        assert [json.loads(v) for v in values] == [EXPECTED_FILE_DOCUMENT]


class TestFileMetadata:
    """Tests for FILE column metadata."""

    def test_should_report_a_file_column_with_a_dedicated_type_code(self, cursor, tmp_schema):
        # Given Snowflake client is logged in
        pass

        # When A FILE column is populated via TO_FILE and queried
        table_name = f"{tmp_schema}.file_table"
        _create_file_table(cursor, table_name)
        cursor.execute(f"SELECT * FROM {table_name}").fetchall()

        # Then the column should report type code FILE
        assert FIELD_ID_TO_NAME[cursor.description[0].type_code] == "FILE"


class TestFileLiteral:
    """Tests for FILE values selected without a table."""

    def test_should_select_a_file_value_built_by_to_file_without_a_table(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT TO_FILE(PARSE_JSON('{"RELATIVE_PATH": "some_new_file.jpeg", ...}'))" is executed
        result = execute_query(f"SELECT {FILE_EXPR} AS file_col", single_row=True)

        # Then the result should contain the expected FILE JSON document
        assert_type(result, str)
        assert json.loads(result[0]) == EXPECTED_FILE_DOCUMENT

    def test_should_handle_null_file_values_from_literals(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query "SELECT TO_FILE(PARSE_JSON('...')), TO_FILE(NULL)" is executed
        result = execute_query(f"SELECT {FILE_EXPR} AS file_col, TO_FILE(NULL) AS null_col", single_row=True)

        # Then the result should contain the expected FILE JSON document and NULL
        assert json.loads(result[0]) == EXPECTED_FILE_DOCUMENT
        assert result[1] is None


class TestFileTable:
    """Tests for FILE values stored in a table."""

    def test_should_select_file_values_from_table(self, execute_query, file_table):
        # Given Snowflake client is logged in
        pass

        # And A temporary table with an ID and a FILE column is created
        pass

        # And The table is populated with two different FILE values
        execute_query(f"INSERT INTO {file_table} SELECT 1, {FILE_EXPR} UNION ALL SELECT 2, {OTHER_FILE_EXPR}")

        # When Query "SELECT * FROM {table} ORDER BY ID" is executed
        rows = execute_query(f"SELECT id, file_col FROM {file_table} ORDER BY id")

        # Then the result should contain the inserted FILE JSON documents in order
        assert_type([row[1] for row in rows], str)
        assert [parse_json_value(row[1]) for row in rows] == [EXPECTED_FILE_DOCUMENT, EXPECTED_OTHER_FILE_DOCUMENT]

    def test_should_handle_null_file_values_from_table(self, execute_query, file_table):
        # Given Snowflake client is logged in
        pass

        # And A temporary table with an ID and a FILE column is created
        pass

        # And The table is populated with a FILE value, a NULL and another FILE value
        execute_query(
            f"INSERT INTO {file_table} SELECT 1, {FILE_EXPR} "
            f"UNION ALL SELECT 2, TO_FILE(NULL) "
            f"UNION ALL SELECT 3, {OTHER_FILE_EXPR}"
        )

        # When Query "SELECT * FROM {table} ORDER BY ID" is executed
        rows = execute_query(f"SELECT id, file_col FROM {file_table} ORDER BY id")

        # Then the result should contain the inserted FILE JSON documents and NULL in order
        assert [parse_json_value(row[1]) for row in rows] == [
            EXPECTED_FILE_DOCUMENT,
            None,
            EXPECTED_OTHER_FILE_DOCUMENT,
        ]


class TestFileMultipleChunks:
    """Tests for FILE type with multiple chunks downloading."""

    def test_should_download_file_data_in_multiple_chunks(self, execute_query):
        # Given Snowflake client is logged in
        pass

        # When Query generating 20000 FILE values is executed
        rows = execute_query(
            "SELECT id, TO_FILE(OBJECT_CONSTRUCT("
            "'RELATIVE_PATH', 'file_' || id || '.jpeg', "
            "'STAGE', '@myStage', "
            "'STAGE_FILE_URL', 'file_' || id || '.jpeg', "
            "'SIZE', id, "
            "'ETAG', 'xxx', "
            "'CONTENT_TYPE', 'image/jpeg', "
            "'LAST_MODIFIED', '2025-01-01')) AS file_col "
            "FROM (SELECT (ROW_NUMBER() OVER (ORDER BY seq8()) - 1) AS id "
            f"FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE}))) "
            "ORDER BY id"
        )

        # Then All 20000 rows should be fetched with the expected FILE JSON documents
        assert len(rows) == LARGE_RESULT_SET_SIZE
        assert_type([row[1] for row in rows], str)

        def expected_row(i):
            return (
                i,
                {
                    "RELATIVE_PATH": f"file_{i}.jpeg",
                    "STAGE": "@myStage",
                    "STAGE_FILE_URL": f"file_{i}.jpeg",
                    "SIZE": i,
                    "ETAG": "xxx",
                    "CONTENT_TYPE": "image/jpeg",
                    "LAST_MODIFIED": "2025-01-01",
                },
            )

        def compare_row(actual, expected):
            return actual[0] == expected[0] and parse_json_value(actual[1]) == expected[1]

        assert_sequential_values(rows, LARGE_RESULT_SET_SIZE, transform=expected_row, compare=compare_row)
