#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <limits>
#include <optional>
#include <sstream>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "TestTable.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "odbc_matchers.hpp"

template <int SQL_C_TYPE>
void test_at_limits(Connection& conn) {
  std::stringstream queryBuilder;
  queryBuilder << "SELECT ";
  queryBuilder << +std::numeric_limits<typename MetaOfSqlCType<SQL_C_TYPE>::type>::max() << " AS max, ";
  queryBuilder << +std::numeric_limits<typename MetaOfSqlCType<SQL_C_TYPE>::type>::min() << " AS min";
  auto query = queryBuilder.str();
  INFO("Executing query: " << query);
  auto stmt = conn.execute_fetch(query);
  CHECK(check_no_truncation<SQL_C_TYPE>(stmt, 1) ==
        std::numeric_limits<typename MetaOfSqlCType<SQL_C_TYPE>::type>::max());
  CHECK(check_no_truncation<SQL_C_TYPE>(stmt, 2) ==
        std::numeric_limits<typename MetaOfSqlCType<SQL_C_TYPE>::type>::min());
}

TEST_CASE("Test decimal to integer conversion", "[fixed][conversion][c_integer]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When A table with various DECIMAL/NUMBER/INT columns is queried
  TestTable table(
      conn, "test_decimal_int",
      "num0 NUMBER, num10 NUMBER(10,1), dec20 DECIMAL(20,2), numeric30 NUMERIC(30,3), int1 INT, int2 INTEGER",
      "(123, 123.4, 123.45, 123.456, 123, 123)");

  auto stmt = conn.execute_fetch("SELECT * FROM " + table.name());

  std::vector<int> exact_cols = {1, 5, 6};
  std::vector<int> truncated_cols = {2, 3, 4};
  typename MetaOfSqlCType<SQL_C_LONG>::type expected = 123;

  // Then All integer C types return 123 (exact or with fractional truncation)
  check_integer_columns<SQL_C_LONG>(stmt, exact_cols, truncated_cols, expected);
  check_integer_columns<SQL_C_SLONG>(stmt, exact_cols, truncated_cols, expected);
  check_integer_columns<SQL_C_ULONG>(stmt, exact_cols, truncated_cols, expected);
  check_integer_columns<SQL_C_SHORT>(stmt, exact_cols, truncated_cols, expected);
  check_integer_columns<SQL_C_SSHORT>(stmt, exact_cols, truncated_cols, expected);
  check_integer_columns<SQL_C_USHORT>(stmt, exact_cols, truncated_cols, expected);
  check_integer_columns<SQL_C_TINYINT>(stmt, exact_cols, truncated_cols, expected);
  check_integer_columns<SQL_C_STINYINT>(stmt, exact_cols, truncated_cols, expected);
  check_integer_columns<SQL_C_UTINYINT>(stmt, exact_cols, truncated_cols, expected);
  check_integer_columns<SQL_C_SBIGINT>(stmt, exact_cols, truncated_cols, expected);
  check_integer_columns<SQL_C_UBIGINT>(stmt, exact_cols, truncated_cols, expected);
}

TEST_CASE("Test integer at limits", "[fixed][conversion][c_integer][limits]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Max and min values for each integer C type are queried
  test_at_limits<SQL_C_LONG>(conn);
  test_at_limits<SQL_C_SLONG>(conn);
  test_at_limits<SQL_C_ULONG>(conn);
  test_at_limits<SQL_C_SHORT>(conn);
  test_at_limits<SQL_C_SSHORT>(conn);
  test_at_limits<SQL_C_USHORT>(conn);
  test_at_limits<SQL_C_TINYINT>(conn);
  test_at_limits<SQL_C_STINYINT>(conn);
  test_at_limits<SQL_C_UTINYINT>(conn);
  test_at_limits<SQL_C_SBIGINT>(conn);
  test_at_limits<SQL_C_UBIGINT>(conn);

  // Then Each type returns its exact limit values
  (void)0;  // assertions are inside test_at_limits
}

TEST_CASE("SQL_DECIMAL explicit integer conversions truncate", "[fixed][conversion][c_integer][truncation]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When A fractional DECIMAL value 123.789 is fetched as each integer C type
  const std::string query = "SELECT 123.789::DECIMAL(10,3)";

  // Then All integer C types return 123 with 01S07 truncation warning
  CHECK(check_fractional_truncation<SQL_C_LONG>(conn.execute_fetch(query), 1) == 123);
  CHECK(check_fractional_truncation<SQL_C_SLONG>(conn.execute_fetch(query), 1) == 123);
  CHECK(check_fractional_truncation<SQL_C_ULONG>(conn.execute_fetch(query), 1) == 123);
  CHECK(check_fractional_truncation<SQL_C_SHORT>(conn.execute_fetch(query), 1) == 123);
  CHECK(check_fractional_truncation<SQL_C_SSHORT>(conn.execute_fetch(query), 1) == 123);
  CHECK(check_fractional_truncation<SQL_C_USHORT>(conn.execute_fetch(query), 1) == 123);
  CHECK(check_fractional_truncation<SQL_C_TINYINT>(conn.execute_fetch(query), 1) == 123);
  CHECK(check_fractional_truncation<SQL_C_STINYINT>(conn.execute_fetch(query), 1) == 123);
  CHECK(check_fractional_truncation<SQL_C_UTINYINT>(conn.execute_fetch(query), 1) == 123);
  CHECK(check_fractional_truncation<SQL_C_SBIGINT>(conn.execute_fetch(query), 1) == 123);
  CHECK(check_fractional_truncation<SQL_C_UBIGINT>(conn.execute_fetch(query), 1) == 123);
}

TEST_CASE("SQL_DECIMAL truncation and scale", "[fixed][conversion][c_integer][truncation]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When NUMBER values with various scales are fetched as SQL_C_LONG
  (void)0;
  // Then Values truncate toward zero and scale is handled correctly
  {
    INFO("zero with high scale to SQL_C_LONG");
    auto stmt = conn.execute_fetch("SELECT 0::NUMBER(38,10), 0::NUMBER(38,37), 0::NUMBER(20,15)");

    CHECK(check_no_truncation<SQL_C_LONG>(stmt, 1) == 0);
    CHECK(check_no_truncation<SQL_C_LONG>(stmt, 2) == 0);
    CHECK(check_no_truncation<SQL_C_LONG>(stmt, 3) == 0);
  }

  {
    INFO("nonzero with high scale to SQL_C_LONG");
    CHECK(check_no_truncation<SQL_C_LONG>(conn.execute_fetch("SELECT 5.0000000000::NUMBER(38,10)"), 1) == 5);
    CHECK(check_no_truncation<SQL_C_LONG>(conn.execute_fetch("SELECT -3.0000000000::NUMBER(38,10)"), 1) == -3);
  }

  {
    INFO("fractional values truncate toward zero");
    auto stmt = conn.execute_fetch(
        "SELECT 0.9::DECIMAL(3,1), -0.9::DECIMAL(3,1), "
        "0.1::DECIMAL(3,1), -0.1::DECIMAL(3,1), "
        "0.5::DECIMAL(3,1), -0.5::DECIMAL(3,1)");

    for (int i = 1; i <= 6; ++i) {
      INFO("Column " << i);
      CHECK(check_fractional_truncation<SQL_C_LONG>(stmt, i) == 0);
    }
  }

  {
    INFO("values just below type boundary");
    auto stmt = conn.execute_fetch(
        "SELECT 1.99::DECIMAL(5,2), -1.99::DECIMAL(5,2), "
        "127.99::DECIMAL(5,2), -128.99::DECIMAL(6,2)");

    CHECK(check_fractional_truncation<SQL_C_LONG>(stmt, 1) == 1);
    CHECK(check_fractional_truncation<SQL_C_LONG>(stmt, 2) == -1);
    CHECK(check_fractional_truncation<SQL_C_STINYINT>(stmt, 3) == 127);
    CHECK(check_fractional_truncation<SQL_C_STINYINT>(stmt, 4) == -128);
  }

  {
    INFO("exact scale division - no fractional remainder");
    auto stmt = conn.execute_fetch(
        "SELECT 100.00::DECIMAL(10,2), 0.50::DECIMAL(10,2), "
        "-25.00::DECIMAL(10,2), 1.00::DECIMAL(10,2)");

    CHECK(check_no_truncation<SQL_C_LONG>(stmt, 1) == 100);
    CHECK(check_fractional_truncation<SQL_C_LONG>(stmt, 2) == 0);
    CHECK(check_no_truncation<SQL_C_LONG>(stmt, 3) == -25);
    CHECK(check_no_truncation<SQL_C_LONG>(stmt, 4) == 1);
  }
}

TEST_CASE("NUMBER scale=0 - INT and INTEGER types", "[fixed][conversion][c_integer][scale0]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When A table with INT/INTEGER/BIGINT/SMALLINT/TINYINT columns is queried
  TestTable table(conn, "test_int_types", "a INT, b INTEGER, c BIGINT, d SMALLINT, e TINYINT",
                  "(100, -200, 9223372036854775807, -32000, 120)");

  auto stmt = conn.execute_fetch("SELECT * FROM " + table.name());

  // Then Both integer conversions and check_char_success return correct values
  CHECK(check_no_truncation<SQL_C_LONG>(stmt, 1) == 100);
  CHECK(check_no_truncation<SQL_C_LONG>(stmt, 2) == -200);
  CHECK(check_no_truncation<SQL_C_SBIGINT>(stmt, 3) == 9223372036854775807LL);
  CHECK(check_no_truncation<SQL_C_SHORT>(stmt, 4) == -32000);
  CHECK(check_no_truncation<SQL_C_TINYINT>(stmt, 5) == 120);

  CHECK(check_char_success(stmt, 1) == "100");
  CHECK(check_char_success(stmt, 2) == "-200");
  CHECK(check_char_success(stmt, 3) == "9223372036854775807");
  CHECK(check_char_success(stmt, 4) == "-32000");
  CHECK(check_char_success(stmt, 5) == "120");
}

TEST_CASE("SQL_DECIMAL fractional truncation returns 01S07", "[fixed][conversion][c_integer][01S07]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Fractional DECIMAL values are fetched as integer C types
  (void)0;
  // Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01S07
  {
    INFO("SQL_C_LONG");
    auto stmt = conn.execute_fetch("SELECT 123.45::DECIMAL(10,2)");

    CHECK(check_fractional_truncation<SQL_C_LONG>(stmt, 1) == 123);
  }

  {
    INFO("SQL_C_SHORT");
    auto stmt = conn.execute_fetch("SELECT 9.99::DECIMAL(5,2)");

    CHECK(check_fractional_truncation<SQL_C_SHORT>(stmt, 1) == 9);
  }

  {
    INFO("SQL_C_STINYINT");
    auto stmt = conn.execute_fetch("SELECT 1.5::DECIMAL(3,1)");

    CHECK(check_fractional_truncation<SQL_C_STINYINT>(stmt, 1) == 1);
  }

  {
    INFO("SQL_C_SBIGINT");
    auto stmt = conn.execute_fetch("SELECT 999.001::DECIMAL(10,3)");

    CHECK(check_fractional_truncation<SQL_C_SBIGINT>(stmt, 1) == 999);
  }

  {
    INFO("exact integer does NOT produce 01S07");
    auto stmt = conn.execute_fetch("SELECT 100.00::DECIMAL(10,2)");

    CHECK(check_no_truncation<SQL_C_LONG>(stmt, 1) == 100);
  }
}

TEST_CASE("SQL_DECIMAL overflow returns 22003", "[fixed][conversion][c_integer][22003]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Out-of-range NUMBER values are fetched as narrow integer C types
  (void)0;
  // Then SQL_ERROR is returned with SQLSTATE 22003
  {
    INFO("SQL_C_LONG above i32 max");
    check_numeric_out_of_range<SQL_C_LONG>(conn.execute_fetch("SELECT 2147483648::NUMBER(10,0)"), 1);
  }

  {
    INFO("SQL_C_LONG below i32 min");
    check_numeric_out_of_range<SQL_C_LONG>(conn.execute_fetch("SELECT -2147483649::NUMBER(10,0)"), 1);
  }

  {
    INFO("SQL_C_SHORT above i16 max");
    check_numeric_out_of_range<SQL_C_SHORT>(conn.execute_fetch("SELECT 32768::NUMBER(5,0)"), 1);
  }

  {
    INFO("SQL_C_STINYINT above i8 max");
    check_numeric_out_of_range<SQL_C_STINYINT>(conn.execute_fetch("SELECT 128::NUMBER(3,0)"), 1);
  }

  {
    INFO("SQL_C_STINYINT below i8 min");
    check_numeric_out_of_range<SQL_C_STINYINT>(conn.execute_fetch("SELECT -129::NUMBER(3,0)"), 1);
  }

  {
    INFO("SQL_C_UTINYINT negative");
    check_numeric_out_of_range<SQL_C_UTINYINT>(conn.execute_fetch("SELECT -1::NUMBER(1,0)"), 1);
  }

  {
    INFO("SQL_C_UTINYINT above u8 max");
    check_numeric_out_of_range<SQL_C_UTINYINT>(conn.execute_fetch("SELECT 256::NUMBER(3,0)"), 1);
  }

  {
    INFO("SQL_C_SHORT below i16 min");
    check_numeric_out_of_range<SQL_C_SHORT>(conn.execute_fetch("SELECT -32769::NUMBER(5,0)"), 1);
  }

  {
    INFO("SQL_C_USHORT negative");
    check_numeric_out_of_range<SQL_C_USHORT>(conn.execute_fetch("SELECT -1::NUMBER(1,0)"), 1);
  }

  {
    INFO("SQL_C_USHORT above u16 max");
    check_numeric_out_of_range<SQL_C_USHORT>(conn.execute_fetch("SELECT 65536::NUMBER(5,0)"), 1);
  }

  {
    INFO("SQL_C_ULONG negative");
    check_numeric_out_of_range<SQL_C_ULONG>(conn.execute_fetch("SELECT -1::NUMBER(1,0)"), 1);
  }

  {
    INFO("SQL_C_ULONG above u32 max");
    check_numeric_out_of_range<SQL_C_ULONG>(conn.execute_fetch("SELECT 4294967296::NUMBER(10,0)"), 1);
  }

  {
    INFO("SQL_C_SBIGINT above i64 max");
    check_numeric_out_of_range<SQL_C_SBIGINT>(conn.execute_fetch("SELECT 9223372036854775808::NUMBER(19,0)"), 1);
  }

  {
    INFO("SQL_C_SBIGINT below i64 min");
    check_numeric_out_of_range<SQL_C_SBIGINT>(conn.execute_fetch("SELECT -9223372036854775809::NUMBER(19,0)"), 1);
  }

  {
    INFO("SQL_C_UBIGINT negative");
    check_numeric_out_of_range<SQL_C_UBIGINT>(conn.execute_fetch("SELECT -1::NUMBER(1,0)"), 1);
  }
}

TEST_CASE("NUMBER NULL to SQL_C_INTEGER types", "[fixed][conversion][c_integer][null]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When A NULL NUMBER value is queried
  const auto query = "SELECT NULL::NUMBER(10,0)";
  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(conn.execute_fetch(query), 1, SQL_C_LONG);
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::DECIMAL(10,2)"), 1, SQL_C_LONG);
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::NUMERIC(20,5)"), 1, SQL_C_LONG);
  check_null_via_get_data(conn.execute_fetch(query), 1, SQL_C_SHORT);
  check_null_via_get_data(conn.execute_fetch(query), 1, SQL_C_SBIGINT);
  check_null_via_get_data(conn.execute_fetch(query), 1, SQL_C_STINYINT);
  check_null_via_get_data(conn.execute_fetch(query), 1, SQL_C_ULONG);
  check_null_via_get_data(conn.execute_fetch(query), 1, SQL_C_USHORT);
  check_null_via_get_data(conn.execute_fetch(query), 1, SQL_C_UBIGINT);
  check_null_via_get_data(conn.execute_fetch(query), 1, SQL_C_UTINYINT);
}

TEST_CASE("NUMBER NULL mixed with non-NULL in multiple rows", "[fixed][conversion][c_integer][null]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When A table with mixed NULL and non-NULL rows is queried
  TestTable table(conn, "test_fixed_null", "val NUMBER(10,0)", "(42), (NULL), (-7), (NULL), (0)");
  auto stmt = conn.execute_fetch("SELECT * FROM " + table.name());

  // Then NULLs return SQL_NULL_DATA and non-NULLs return correct values
  std::vector<std::optional<SQLINTEGER>> expected = {42, std::nullopt, -7, std::nullopt, 0};
  SQLINTEGER value = 0;
  SQLLEN indicator = 0;

  for (size_t row = 0; row < expected.size(); ++row) {
    if (row > 0) {
      SQLRETURN ret = SQLFetch(stmt.getHandle());
      CHECK(ret == SQL_SUCCESS);
    }
    INFO("Row " << row);
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
    CHECK(ret == SQL_SUCCESS);
    if (expected[row].has_value()) {
      CHECK(indicator != SQL_NULL_DATA);
      CHECK(value == expected[row].value());
    } else {
      CHECK(indicator == SQL_NULL_DATA);
    }
  }
}

TEST_CASE("SQL_DECIMAL SQLGetData NULL without indicator returns 22002",
          "[fixed][conversion][c_integer][null][22002]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When A NULL value is fetched without providing an indicator pointer
  auto stmt = conn.execute_fetch("SELECT NULL::DECIMAL(10,2)");
  SQLINTEGER value = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), nullptr);

  // Then SQL_ERROR is returned with SQLSTATE 22002
  CHECK(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "22002");
}
