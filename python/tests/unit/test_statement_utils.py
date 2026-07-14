"""Unit tests for statement_utils.extract_rowcount."""

import pytest

from tests.compatibility import IS_UNIVERSAL_DRIVER

pytestmark = pytest.mark.skipif(not IS_UNIVERSAL_DRIVER, reason="Requires universal driver")

from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import (  # noqa: E402
    ResultSetDescriptor,
)
from snowflake.connector._internal.statement_utils import extract_rowcount  # noqa: E402


class TestExtractRowcount:
    def test_none_descriptor_is_unknown(self):
        assert extract_rowcount(None) == -1

    def test_present_rows_affected_is_returned(self):
        assert extract_rowcount(ResultSetDescriptor(rows_affected=42)) == 42

    def test_present_zero_rows_affected_is_zero(self):
        # SELECT / DML with zero rows must stay 0, not fall through to compat 1.
        assert extract_rowcount(ResultSetDescriptor(rows_affected=0)) == 0

    def test_zero_rows_affected_with_ddl_type_is_zero(self):
        # An explicit rows_affected takes precedence over the compat allowlist.
        assert extract_rowcount(ResultSetDescriptor(rows_affected=0, statement_type_id=0x6000)) == 0

    @pytest.mark.parametrize(
        "statement_type_id",
        [
            0x6000,  # DDL parent
            0x6100,  # CREATE SCHEMA
            0x6101,  # CREATE TABLE
            0x6300,  # DROP SCHEMA
            0x4100,  # ALTER SESSION SET/UNSET
            0x4300,  # USE SCHEMA
            0x5100,  # COMMIT (TCL)
            0x8101,  # BEGIN (legacy TCL block)
            0x8104,  # SET (legacy constant)
        ],
    )
    def test_legacy_no_result_success_is_one(self, statement_type_id):
        desc = ResultSetDescriptor(statement_type_id=statement_type_id)
        assert extract_rowcount(desc) == 1

    def test_manage_pats_is_not_no_result_success(self):
        # MANAGE_PATS (0x6244) is DDL-family by id but produces a result set.
        assert extract_rowcount(ResultSetDescriptor(statement_type_id=0x6244)) == -1

    def test_absent_statement_type_id_is_unknown(self):
        assert extract_rowcount(ResultSetDescriptor(query_id="q")) == -1

    def test_explicit_unknown_type_is_unknown(self):
        assert extract_rowcount(ResultSetDescriptor(statement_type_id=0x0000)) == -1

    @pytest.mark.parametrize(
        "statement_type_id",
        [
            0xBEEF,  # garbage / unrecognized family
            0xB000,  # unmapped family
            0xF000,  # unmapped family
            0xA000,  # MULTI_STMT parent (not a no-result-success family)
            0xA100,  # MULTI_STMT child
        ],
    )
    def test_unrecognized_family_is_unknown(self, statement_type_id):
        # Only known no-result families (SYSCMD/TCL/DDL/MISC) get the compat 1;
        # anything outside them is reported as unknown rather than a spurious 1.
        desc = ResultSetDescriptor(statement_type_id=statement_type_id)
        assert extract_rowcount(desc) == -1

    @pytest.mark.parametrize(
        "statement_type_id",
        [
            0x1000,  # SELECT
            0x2000,  # EXPLAIN
            0x3600,  # COPY (DML family, cursor-producing)
            0x4400,  # SHOW
            0x4500,  # DESCRIBE
            0x4701,  # LIST_FILES
            0x7102,  # PUT_FILES (stage file ops)
            0x9000,  # CALL
        ],
    )
    def test_cursor_types_without_rows_affected_are_unknown(self, statement_type_id):
        desc = ResultSetDescriptor(statement_type_id=statement_type_id)
        assert extract_rowcount(desc) == -1
