#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("SQLDescribeCol for TIMESTAMP_NTZ", "[timestamp_ntz][describe_col]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIMESTAMP_NTZ column is described via SQLDescribeCol
  auto stmt = conn.execute_fetch("SELECT '2024-01-15 14:30:45'::TIMESTAMP_NTZ");
  SQLSMALLINT data_type = 0;
  SQLULEN column_size = 0;
  SQLSMALLINT decimal_digits = 0;
  SQLRETURN ret =
      SQLDescribeCol(stmt.getHandle(), 1, nullptr, 0, nullptr, &data_type, &column_size, &decimal_digits, nullptr);

  // Then Data type is SQL_TYPE_TIMESTAMP with expected precision and column size
  REQUIRE_ODBC(ret, stmt);
  CHECK(data_type == SQL_TYPE_TIMESTAMP);
  CHECK(column_size == 29);
  CHECK(decimal_digits == 9);
}

TEST_CASE("SQLDescribeCol for TIMESTAMP_LTZ", "[timestamp_ltz][describe_col]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIMESTAMP_LTZ column is described via SQLDescribeCol
  auto stmt = conn.execute_fetch("SELECT '2024-01-15 14:30:45'::TIMESTAMP_LTZ");
  SQLSMALLINT data_type = 0;
  SQLULEN column_size = 0;
  SQLSMALLINT decimal_digits = 0;
  SQLRETURN ret =
      SQLDescribeCol(stmt.getHandle(), 1, nullptr, 0, nullptr, &data_type, &column_size, &decimal_digits, nullptr);

  // Then Data type is SQL_TYPE_TIMESTAMP with expected precision and column size
  REQUIRE_ODBC(ret, stmt);
  CHECK(data_type == SQL_TYPE_TIMESTAMP);
  CHECK(column_size == 29);
  CHECK(decimal_digits == 9);
}

TEST_CASE("SQLDescribeCol for TIMESTAMP_TZ", "[timestamp_tz][describe_col]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIMESTAMP_TZ column is described via SQLDescribeCol
  auto stmt = conn.execute_fetch("SELECT '2024-01-15 14:30:45 +00:00'::TIMESTAMP_TZ");
  SQLSMALLINT data_type = 0;
  SQLULEN column_size = 0;
  SQLSMALLINT decimal_digits = 0;
  SQLRETURN ret =
      SQLDescribeCol(stmt.getHandle(), 1, nullptr, 0, nullptr, &data_type, &column_size, &decimal_digits, nullptr);

  // Then Data type is SQL_TYPE_TIMESTAMP with expected precision and column size
  REQUIRE_ODBC(ret, stmt);
  CHECK(data_type == SQL_TYPE_TIMESTAMP);
  CHECK(column_size == 29);
  CHECK(decimal_digits == 9);
}
