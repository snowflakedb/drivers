"""FILE type tests for Universal Driver.

A FILE column (produced by TO_FILE) is not decoded by any driver: it is read as
the raw, undecoded JSON document the server sends, the same representation
used for VARIANT.
"""

from __future__ import annotations

import json

from snowflake.connector.constants import FIELD_ID_TO_NAME

from .utils import assert_type


EXPECTED_FILE_DOCUMENT = {
    "RELATIVE_PATH": "some_new_file.jpeg",
    "STAGE": "@myStage",
    "STAGE_FILE_URL": "some_new_file.jpeg",
    "SIZE": 123,
    "ETAG": "xxx",
    "CONTENT_TYPE": "image/jpeg",
    "LAST_MODIFIED": "2025-01-01",
}


def _create_file_table(cursor, table_name: str) -> None:
    cursor.execute(
        f"CREATE OR REPLACE TEMPORARY TABLE {table_name} AS SELECT "
        "TO_FILE(OBJECT_CONSTRUCT("
        "'RELATIVE_PATH', 'some_new_file.jpeg', "
        "'STAGE', '@myStage', "
        "'STAGE_FILE_URL', 'some_new_file.jpeg', "
        "'SIZE', 123, "
        "'ETAG', 'xxx', "
        "'CONTENT_TYPE', 'image/jpeg', "
        "'LAST_MODIFIED', '2025-01-01')) AS file_col"
    )


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
