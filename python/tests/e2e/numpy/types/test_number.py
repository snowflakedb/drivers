"""NUMBER type tests for Universal Driver -- numpy consumer.

Mirrors every scenario in ``tests/definitions/shared/types/number.feature``
using ``numpy=True`` cursor fetch (``fetchone`` / ``fetchall``).

NumPy-mode numeric behavior depends on the magnitude of the values a query
actually returns, not on the column's declared precision:

* ``scale == 0`` -> ``numpy.int64``, or Python ``int`` once a value exceeds int64
* ``scale > 0``  -> ``numpy.float64`` (precision loss vs Decimal), or ``Decimal``
  once a value carries more significant digits than float64 holds

Both fallbacks apply per result column, so a ``NUMBER(38,2)`` column reads back
as ``numpy.float64`` for small values and as ``Decimal`` for 38-digit ones.
"""

from __future__ import annotations

from decimal import Decimal

import pytest

from tests.conftest import with_paramstyle
from tests.e2e.numpy.utils import (
    assert_dtypes,
    execute_and_fetch,
    execute_and_fetch_multiple_batches,
    get_column,
    get_row,
    is_decimal,
    is_float,
    is_integer,
    is_object,
)
from tests.e2e.types.utils import assert_sequential_values, floats_equal


# Test constants ported from tests/e2e/types/test_number.py
NUMBER_TYPE_SYNONYMS = ["NUMBER", "DECIMAL", "NUMERIC"]
number_type_parametrize = pytest.mark.parametrize("num_type", NUMBER_TYPE_SYNONYMS)

NUMBER_38_DIGITS_INT = 12345678901234567890123456789012345678
NUMBER_38_DIGITS_SCALE2 = Decimal("123456789012345678901234567890123456.78")
NUMBER_38_DIGITS_SCALE10 = Decimal("1234567890123456789012345678.1234567890")
NUMBER_38_DIGITS_SCALE37 = Decimal("1.2345678901234567890123456789012345678")

NUMBER_5_2_MAX = 999.99
NUMBER_5_2_MIN = -999.99
NUMBER_8_0_MAX = 99999999
NUMBER_8_0_MIN = -99999999
NUMBER_38_0_MAX = 99999999999999999999999999999999999999
NUMBER_38_0_MIN = -99999999999999999999999999999999999999
NUMBER_38_37_MIN_POSITIVE = Decimal("0.0000000000000000000000000000000000001")

LARGE_RESULT_SET_SIZE = 30_000


# ---------------------------------------------------------------------------
# Scenarios
# ---------------------------------------------------------------------------


class TestFetchNumpyNumberTypeCasting:
    """Type-casting coverage for the NUMBER family via numpy fetch."""

    @number_type_parametrize
    def test_should_cast_number_values_to_appropriate_type_for_number_and_synonyms(self, cursor, num_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # When Query "SELECT 0::<type>(10,0), 123::<type>(10,0), 0.00::<type>(10,2), 123.45::<type>(10,2)" is executed
        df = execute_and_fetch(
            cursor,
            f"SELECT 0::{num_type}(10,0), 123::{num_type}(10,0), 0.00::{num_type}(10,2), 123.45::{num_type}(10,2)",
        )

        # Then All values should be returned as appropriate type matching [0, 123, 0.00, 123.45]
        assert_dtypes(df, [is_integer, is_integer, is_float, is_float])
        assert get_row(df, 0) == [0, 123, 0.0, 123.45]


class TestFetchNumpyNumberLiteral:
    """SELECT-with-literal coverage via numpy fetch."""

    @number_type_parametrize
    def test_should_select_number_literals_for_number_and_synonyms(self, cursor, num_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # When Query "SELECT 0::<type>(10,0), -456::<type>(10,0), 1.50::<type>(10,2),
        # -123.45::<type>(10,2), 123.456::<type>(15,3), -789.012::<type>(15,3)" is executed
        df = execute_and_fetch(
            cursor,
            f"SELECT 0::{num_type}(10,0), -456::{num_type}(10,0), "
            f"1.50::{num_type}(10,2), -123.45::{num_type}(10,2), "
            f"123.456::{num_type}(15,3), -789.012::{num_type}(15,3)",
        )

        # Then Result should contain [0, -456, 1.50, -123.45, 123.456, -789.012]
        assert_dtypes(df, [is_integer, is_integer, is_float, is_float, is_float, is_float])
        assert get_row(df, 0) == [0, -456, 1.50, -123.45, 123.456, -789.012]

    @number_type_parametrize
    def test_should_handle_high_precision_values_from_literals_for_number_and_synonyms(self, cursor, num_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # When Query "SELECT 12345678901234567890123456789012345678::<type>(38,0),
        # 123456789012345678901234567890123456.78::<type>(38,2),
        # 1234567890123456789012345678.1234567890::<type>(38,10),
        # 0.0000000000000000000000000000000000001::<type>(38,37)" is executed
        df = execute_and_fetch(
            cursor,
            f"SELECT {NUMBER_38_DIGITS_INT}::{num_type}(38,0), "
            f"{NUMBER_38_DIGITS_SCALE2}::{num_type}(38,2), "
            f"{NUMBER_38_DIGITS_SCALE10}::{num_type}(38,10), "
            f"{NUMBER_38_37_MIN_POSITIVE}::{num_type}(38,37)",
        )

        # Then Result should contain [12345678901234567890123456789012345678,
        # 123456789012345678901234567890123456.78,
        # 1234567890123456789012345678.1234567890,
        # 0.0000000000000000000000000000000000001]
        assert_dtypes(df, [is_object, is_decimal, is_decimal, is_float])
        row = get_row(df, 0)
        assert row[0] == NUMBER_38_DIGITS_INT
        assert row[1] == NUMBER_38_DIGITS_SCALE2
        assert row[2] == NUMBER_38_DIGITS_SCALE10
        assert row[3] == pytest.approx(float(NUMBER_38_37_MIN_POSITIVE))

    @number_type_parametrize
    def test_should_handle_scale_and_precision_boundaries_from_literals_for_number_and_synonyms(self, cursor, num_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # When Query "SELECT 999.99::<type>(5,2), -999.99::<type>(5,2),
        # 99999999::<type>(8,0), -99999999::<type>(8,0)" is executed
        df = execute_and_fetch(
            cursor,
            f"SELECT {NUMBER_5_2_MAX}::{num_type}(5,2), {NUMBER_5_2_MIN}::{num_type}(5,2), "
            f"{NUMBER_8_0_MAX}::{num_type}(8,0), {NUMBER_8_0_MIN}::{num_type}(8,0)",
        )

        # Then Result should contain [999.99, -999.99, 99999999, -99999999]
        assert_dtypes(df, [is_float, is_float, is_integer, is_integer])
        assert get_row(df, 0) == [NUMBER_5_2_MAX, NUMBER_5_2_MIN, NUMBER_8_0_MAX, NUMBER_8_0_MIN]

    @number_type_parametrize
    def test_should_handle_high_precision_boundaries_from_literals_for_number_and_synonyms(self, cursor, num_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # When Query "SELECT 99999999999999999999999999999999999999::<type>(38,0),
        # -99999999999999999999999999999999999999::<type>(38,0)" is executed
        df = execute_and_fetch(
            cursor,
            f"SELECT {NUMBER_38_0_MAX}::{num_type}(38,0), {NUMBER_38_0_MIN}::{num_type}(38,0)",
        )

        # Then Result should contain max and min 38-digit integers
        assert_dtypes(df, [is_object, is_object])
        assert get_row(df, 0) == [NUMBER_38_0_MAX, NUMBER_38_0_MIN]

    @number_type_parametrize
    def test_should_handle_null_values_from_literals_for_number_and_synonyms(self, cursor, num_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # When Query "SELECT NULL::<type>(10,0), 42::<type>(10,0), NULL::<type>(10,2), 42.50::<type>(10,2)" is executed
        df = execute_and_fetch(
            cursor,
            f"SELECT NULL::{num_type}(10,0), 42::{num_type}(10,0), NULL::{num_type}(10,2), 42.50::{num_type}(10,2)",
        )

        # Then Result should contain [NULL, 42, NULL, 42.50]
        assert_dtypes(df, [is_float, is_integer, is_float, is_float])
        assert get_row(df, 0) == [None, 42, None, 42.5]

    @number_type_parametrize
    def test_should_download_large_result_set_with_multiple_chunks_from_generator_for_number_and_synonyms(
        self, cursor, num_type
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # When Query "SELECT seq8()::<type>(38,0), (seq8() + 0.12345)::<type>(20,5)
        # FROM TABLE(GENERATOR(ROWCOUNT => 30000)) v" is executed
        combined = execute_and_fetch_multiple_batches(
            cursor,
            f"WITH base AS ("
            f"  SELECT ROW_NUMBER() OVER (ORDER BY seq8()) - 1 as rn "
            f"  FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE}))"
            f") "
            f"SELECT rn::{num_type}(38,0), (rn + 0.12345)::{num_type}(15,5) FROM base "
            f"ORDER BY 1",
        )

        # Then Result should contain 30000 rows with sequential integers in column 1
        # and sequential decimals starting from 0.12345 in column 2
        col0 = get_column(combined, 0)
        col1 = get_column(combined, 1)
        assert_sequential_values(col0, LARGE_RESULT_SET_SIZE)
        assert_sequential_values(
            col1,
            LARGE_RESULT_SET_SIZE,
            transform=lambda i: float(i) + 0.12345,
            compare=floats_equal,
        )


class TestFetchNumpyNumberTable:
    """Table-based scenarios via numpy fetch."""

    @number_type_parametrize
    def test_should_select_numbers_from_table_with_multiple_scales_for_number_and_synonyms(
        self, execute_query, cursor, tmp_schema, num_type
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # And Table with columns (<type>(10,0), <type>(10,2), <type>(15,3), <type>(20,5)) exists
        table_name = f"{tmp_schema}.np_number_table_{num_type.lower()}"
        execute_query(
            f"CREATE OR REPLACE TEMPORARY TABLE {table_name} ("
            f"col_scale0 {num_type}(10,0), "
            f"col_scale2 {num_type}(10,2), "
            f"col_scale3 {num_type}(15,3), "
            f"col_scale5 {num_type}(20,5))"
        )
        # And Row (123, 123.45, 123.456, 12345.67890) is inserted
        execute_query(f"INSERT INTO {table_name} VALUES (123, 123.45, 123.456, 12345.67890)")
        # And Row (-456, -67.89, -789.012, -98765.43210) is inserted
        execute_query(f"INSERT INTO {table_name} VALUES (-456, -67.89, -789.012, -98765.43210)")
        # And Row (0, 0.00, 0.000, 0.00000) is inserted
        execute_query(f"INSERT INTO {table_name} VALUES (0, 0.00, 0.000, 0.00000)")
        # And Row (999999, 999.99, 1000.500, 123456.78901) is inserted
        execute_query(f"INSERT INTO {table_name} VALUES (999999, 999.99, 1000.500, 123456.78901)")

        # When Query "SELECT * FROM <table>" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col_scale0")

        # Then Result should contain 4 rows with expected values
        assert_dtypes(df, [is_integer, is_float, is_float, is_float])
        assert get_row(df, 0) == pytest.approx([-456, -67.89, -789.012, -98765.43210])
        assert get_row(df, 1) == pytest.approx([0, 0.0, 0.0, 0.0])
        assert get_row(df, 2) == pytest.approx([123, 123.45, 123.456, 12345.67890])
        assert get_row(df, 3) == pytest.approx([999999, 999.99, 1000.500, 123456.78901])

    @number_type_parametrize
    def test_should_handle_high_precision_values_from_table_for_number_and_synonyms(
        self, execute_query, cursor, tmp_schema, num_type
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # And Table with columns (<type>(38,0), <type>(38,2), <type>(38,10), <type>(38,37)) exists
        table_name = f"{tmp_schema}.np_precision_table_{num_type.lower()}"
        execute_query(
            f"CREATE OR REPLACE TEMPORARY TABLE {table_name} ("
            f"col_38_0 {num_type}(38,0), "
            f"col_38_2 {num_type}(38,2), "
            f"col_38_10 {num_type}(38,10), "
            f"col_38_37 {num_type}(38,37))"
        )
        # And Row (12345678901234567890123456789012345678,
        # 123456789012345678901234567890123456.78,
        # 1234567890123456789012345678.1234567890,
        # 1.2345678901234567890123456789012345678) is inserted
        execute_query(
            f"INSERT INTO {table_name} VALUES ("
            f"{NUMBER_38_DIGITS_INT}, {NUMBER_38_DIGITS_SCALE2}, "
            f"{NUMBER_38_DIGITS_SCALE10}, {NUMBER_38_DIGITS_SCALE37})"
        )

        # When Query "SELECT * FROM <table>" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name}")

        # Then Result should contain [12345678901234567890123456789012345678,
        # 123456789012345678901234567890123456.78,
        # 1234567890123456789012345678.1234567890,
        # 1.2345678901234567890123456789012345678]
        assert_dtypes(df, [is_object, is_decimal, is_decimal, is_decimal])
        row = get_row(df, 0)
        assert row[0] == NUMBER_38_DIGITS_INT
        assert row[1:] == [NUMBER_38_DIGITS_SCALE2, NUMBER_38_DIGITS_SCALE10, NUMBER_38_DIGITS_SCALE37]

    @number_type_parametrize
    def test_should_handle_scale_and_precision_boundaries_from_table_for_number_and_synonyms(
        self, execute_query, cursor, tmp_schema, num_type
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # And Table with columns (<type>(5,2), <type>(8,0)) exists
        table_name = f"{tmp_schema}.np_boundary_table_{num_type.lower()}"
        execute_query(
            f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col_5_2 {num_type}(5,2), col_8_0 {num_type}(8,0))"
        )
        # And Row (999.99, 99999999) is inserted
        execute_query(f"INSERT INTO {table_name} VALUES ({NUMBER_5_2_MAX}, {NUMBER_8_0_MAX})")
        # And Row (-999.99, -99999999) is inserted
        execute_query(f"INSERT INTO {table_name} VALUES ({NUMBER_5_2_MIN}, {NUMBER_8_0_MIN})")
        # And Row (123.45, 12345678) is inserted
        execute_query(f"INSERT INTO {table_name} VALUES (123.45, 12345678)")
        # And Row (0.01, 0) is inserted
        execute_query(f"INSERT INTO {table_name} VALUES (0.01, 0)")

        # When Query "SELECT * FROM <table>" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col_5_2")

        # Then Result should contain 4 rows with expected boundary values
        assert_dtypes(df, [is_float, is_integer])
        assert get_row(df, 0) == pytest.approx([NUMBER_5_2_MIN, NUMBER_8_0_MIN])
        assert get_row(df, 1) == pytest.approx([0.01, 0])
        assert get_row(df, 2) == pytest.approx([123.45, 12345678])
        assert get_row(df, 3) == pytest.approx([NUMBER_5_2_MAX, NUMBER_8_0_MAX])

    @number_type_parametrize
    def test_should_handle_high_precision_boundaries_from_table_for_number_and_synonyms(
        self, execute_query, cursor, tmp_schema, num_type
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # And Table with columns (<type>(38,0), <type>(38,37)) exists
        table_name = f"{tmp_schema}.np_high_prec_boundary_table_{num_type.lower()}"
        execute_query(
            f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col_38_0 {num_type}(38,0), col_38_37 {num_type}(38,37))"
        )
        # And Row (99999999999999999999999999999999999999, 1.2345678901234567890123456789012345678) is inserted
        execute_query(f"INSERT INTO {table_name} VALUES ({NUMBER_38_0_MAX}, {NUMBER_38_DIGITS_SCALE37})")
        # And Row (-99999999999999999999999999999999999999, -1.2345678901234567890123456789012345678) is inserted
        execute_query(f"INSERT INTO {table_name} VALUES ({NUMBER_38_0_MIN}, {-NUMBER_38_DIGITS_SCALE37})")
        # And Row (12345678901234567890123456789012345678, 0.0000000000000000000000000000000000001) is inserted
        execute_query(f"INSERT INTO {table_name} VALUES ({NUMBER_38_DIGITS_INT}, {NUMBER_38_37_MIN_POSITIVE})")

        # When Query "SELECT * FROM <table>" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col_38_0")

        # Then Result should contain 3 rows with expected high precision boundary values
        assert_dtypes(df, [is_object, is_decimal])
        assert get_row(df, 0)[0] == NUMBER_38_0_MIN
        assert get_row(df, 0)[1] == -NUMBER_38_DIGITS_SCALE37
        assert get_row(df, 1)[0] == NUMBER_38_DIGITS_INT
        assert get_row(df, 1)[1] == NUMBER_38_37_MIN_POSITIVE
        assert get_row(df, 2)[0] == NUMBER_38_0_MAX
        assert get_row(df, 2)[1] == NUMBER_38_DIGITS_SCALE37

    @number_type_parametrize
    def test_should_handle_null_values_from_table_with_multiple_scales_for_number_and_synonyms(
        self, execute_query, cursor, tmp_schema, num_type
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # And Table with columns (<type>(10,0), <type>(10,2), <type>(15,3)) exists
        table_name = f"{tmp_schema}.np_null_table_{num_type.lower()}"
        execute_query(
            f"CREATE OR REPLACE TEMPORARY TABLE {table_name} ("
            f"col_10_0 {num_type}(10,0), "
            f"col_10_2 {num_type}(10,2), "
            f"col_15_3 {num_type}(15,3))"
        )
        # And Row (NULL, NULL, NULL) is inserted
        execute_query(f"INSERT INTO {table_name} VALUES (NULL, NULL, NULL)")
        # And Row (123, 123.45, 123.456) is inserted
        execute_query(f"INSERT INTO {table_name} VALUES (123, 123.45, 123.456)")
        # And Row (NULL, NULL, NULL) is inserted
        execute_query(f"INSERT INTO {table_name} VALUES (NULL, NULL, NULL)")
        # And Row (-456, -67.89, -789.012) is inserted
        execute_query(f"INSERT INTO {table_name} VALUES (-456, -67.89, -789.012)")

        # When Query "SELECT * FROM <table>" is executed
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col_10_0 NULLS LAST")

        # Then Result should contain 4 rows with 2 NULL rows and 2 non-NULL rows with expected values
        assert_dtypes(df, [is_integer, is_float, is_float])
        assert get_row(df, 0) == pytest.approx([-456.0, -67.89, -789.012])
        assert get_row(df, 1) == pytest.approx([123.0, 123.45, 123.456])
        assert get_row(df, 2) == [None, None, None]
        assert get_row(df, 3) == [None, None, None]

    @number_type_parametrize
    def test_should_download_large_result_set_from_table_for_number_and_synonyms(
        self, execute_query, cursor, tmp_schema, num_type
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # And Table with columns (<type>(38,0), <type>(20,5)) exists with 30000 sequential rows,
        # from 0 to 29999 in the first column and from 0.12345 to 29999.12345 in the second column
        table_name = f"{tmp_schema}.np_large_table_{num_type.lower()}"
        execute_query(f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col1 {num_type}(38,0), col2 {num_type}(15,5))")
        execute_query(
            f"INSERT INTO {table_name} "
            f"WITH base AS ("
            f"  SELECT ROW_NUMBER() OVER (ORDER BY seq4()) - 1 as rn "
            f"  FROM TABLE(GENERATOR(ROWCOUNT => {LARGE_RESULT_SET_SIZE}))"
            f") "
            f"SELECT rn::{num_type}(38,0), (rn + 0.12345)::{num_type}(15,5) FROM base"
        )

        # When Query "SELECT * FROM <table>" is executed
        combined = execute_and_fetch_multiple_batches(cursor, f"SELECT * FROM {table_name} ORDER BY 1")

        # Then Result should contain 30000 rows with sequential integers in column 1
        # and sequential decimals starting from 0.12345 in column 2
        col0 = get_column(combined, 0)
        col1 = get_column(combined, 1)
        assert_sequential_values(col0, LARGE_RESULT_SET_SIZE)
        assert_sequential_values(
            col1,
            LARGE_RESULT_SET_SIZE,
            transform=lambda i: float(i) + 0.12345,
            compare=floats_equal,
        )


@with_paramstyle("qmark")
class TestFetchNumpyNumberBinding:
    """Parameter-binding scenarios via numpy fetch."""

    @number_type_parametrize
    def test_should_select_number_using_parameter_binding_for_number_and_synonyms(self, cursor, num_type):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # When Query "SELECT ?::<type>(10,0), ?::<type>(10,0), ?::<type>(10,2),
        # ?::<type>(10,2), ?::<type>(10,0)" is executed with bound values [123, -456, 12.34, -56.78, NULL]
        df = execute_and_fetch(
            cursor,
            f"SELECT ?::{num_type}(10,0), ?::{num_type}(10,0), "
            f"?::{num_type}(10,2), ?::{num_type}(10,2), ?::{num_type}(10,0)",
            params=(123, -456, 12.34, -56.78, None),
        )

        # Then Result should contain [123, -456, 12.34, -56.78, NULL]
        assert_dtypes(df, [is_integer, is_integer, is_float, is_float, is_float])
        assert get_row(df, 0)[:4] == pytest.approx([123, -456, 12.34, -56.78])
        assert get_row(df, 0)[4] is None

    @number_type_parametrize
    def test_should_select_high_precision_number_using_parameter_binding_for_number_and_synonyms(
        self, cursor, num_type
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # When Query "SELECT ?::<type>(38,0), ?::<type>(38,2)" is executed
        # with bound values [12345678901234567890123456789012345678,
        # 123456789012345678901234567890123456.78]
        df = execute_and_fetch(
            cursor,
            f"SELECT ?::{num_type}(38,0), ?::{num_type}(38,2)",
            params=(NUMBER_38_DIGITS_INT, NUMBER_38_DIGITS_SCALE2),
        )

        # Then Result should contain [12345678901234567890123456789012345678,
        # 123456789012345678901234567890123456.78]
        assert_dtypes(df, [is_object, is_decimal])
        assert get_row(df, 0)[0] == NUMBER_38_DIGITS_INT
        assert get_row(df, 0)[1] == NUMBER_38_DIGITS_SCALE2

    @number_type_parametrize
    def test_should_insert_number_using_parameter_binding_for_number_and_synonyms(
        self, execute_query, executemany_insert, cursor, tmp_schema, num_type
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # And Table with columns (<type>(10,0), <type>(10,2)) exists
        table_name = f"{tmp_schema}.np_number_bind_{num_type.lower()}"
        execute_query(
            f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col_int {num_type}(10,0), col_dec {num_type}(10,2))"
        )

        # When Rows (0, 0.00), (123, 123.45), (-456, -67.89), (999999, 999.99), (NULL, NULL) are inserted using binding
        test_data = [
            (0, 0.00),
            (123, 123.45),
            (-456, -67.89),
            (999999, 999.99),
            (None, None),
        ]
        executemany_insert(table_name, f"INSERT INTO {table_name} VALUES (?, ?)", test_data)

        # Then Result should contain 5 rows with expected values
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col_int")
        assert_dtypes(df, [is_integer, is_float])
        assert get_row(df, 0) == pytest.approx([-456.0, -67.89])
        assert get_row(df, 1) == pytest.approx([0.0, 0.0])
        assert get_row(df, 2) == pytest.approx([123.0, 123.45])
        assert get_row(df, 3) == pytest.approx([999999.0, 999.99])
        assert get_row(df, 4) == [None, None]

    @number_type_parametrize
    def test_should_insert_high_precision_number_using_parameter_binding_for_number_and_synonyms(
        self, execute_query, cursor, tmp_schema, num_type
    ):
        # Given Snowflake client is logged in
        assert not cursor.connection.is_closed()

        # And Table with columns (<type>(38,0), <type>(38,2)) exists
        table_name = f"{tmp_schema}.np_high_prec_bind_{num_type.lower()}"
        execute_query(
            f"CREATE OR REPLACE TEMPORARY TABLE {table_name} (col_38_0 {num_type}(38,0), col_38_2 {num_type}(38,2))"
        )

        # When Rows (12345678901234567890123456789012345678,
        # 123456789012345678901234567890123456.78),
        # (99999999999999999999999999999999999999, 0.01),
        # (-99999999999999999999999999999999999999, -0.01) are inserted using binding
        test_data = [
            (NUMBER_38_DIGITS_INT, NUMBER_38_DIGITS_SCALE2),
            (NUMBER_38_0_MAX, Decimal("0.01")),
            (NUMBER_38_0_MIN, Decimal("-0.01")),
        ]
        for row in test_data:
            execute_query(f"INSERT INTO {table_name} VALUES (?, ?)", row)

        # Then Result should contain 3 rows with expected values keeping the precision
        df = execute_and_fetch(cursor, f"SELECT * FROM {table_name} ORDER BY col_38_0")
        assert_dtypes(df, [is_object, is_decimal])
        assert get_row(df, 0)[0] == NUMBER_38_0_MIN
        assert get_row(df, 0)[1] == Decimal("-0.01")
        assert get_row(df, 1)[0] == NUMBER_38_DIGITS_INT
        assert get_row(df, 1)[1] == NUMBER_38_DIGITS_SCALE2
        assert get_row(df, 2)[0] == NUMBER_38_0_MAX
        assert get_row(df, 2)[1] == Decimal("0.01")
