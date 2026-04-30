#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "odbc_matchers.hpp"
#include "snowflake_odbc_constants.hpp"

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

  // Then Data type is the Snowflake vendor code SQL_SF_TIMESTAMP_NTZ (2002)
  // with scale-aware column size matching the wire string format.
  REQUIRE_ODBC(ret, stmt);
  CHECK(data_type == SQL_SF_TIMESTAMP_NTZ);
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

  // Then Data type is the Snowflake vendor code SQL_SF_TIMESTAMP_LTZ (2000).
  // LTZ uses the same wire string layout as NTZ, so column size is identical.
  REQUIRE_ODBC(ret, stmt);
  CHECK(data_type == SQL_SF_TIMESTAMP_LTZ);
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

  // Then Data type is the Snowflake vendor code SQL_SF_TIMESTAMP_TZ (2001).
  // Column size is fixed at 35 to accommodate the `±HH:MM` offset suffix.
  REQUIRE_ODBC(ret, stmt);
  CHECK(data_type == SQL_SF_TIMESTAMP_TZ);
  CHECK(column_size == SQL_SF_TIMESTAMP_TZ_COLUMN_SIZE);
  CHECK(decimal_digits == 9);
}

TEST_CASE("SQLColAttribute SQL_DESC_TYPE matches SQLDescribeCol vendor codes for TIMESTAMP variants",
          "[timestamp][col_attribute]") {
  // SQLColAttribute is the second metadata API for column type discovery; the
  // ODBC spec requires it to return the same value as SQLDescribeCol for
  // SQL_DESC_TYPE / SQL_DESC_CONCISE_TYPE. This test guards against regressions
  // where the two APIs diverge for the Snowflake vendor TIMESTAMP codes.
  // Doc: https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlcolattribute-function

  // Given Snowflake client is logged in
  Connection conn;

  struct Variant {
    const char* sql;
    SQLSMALLINT expected_type;
  };
  const Variant variants[] = {
      {"SELECT '2024-01-15 14:30:45'::TIMESTAMP_NTZ", SQL_SF_TIMESTAMP_NTZ},
      {"SELECT '2024-01-15 14:30:45'::TIMESTAMP_LTZ", SQL_SF_TIMESTAMP_LTZ},
      {"SELECT '2024-01-15 14:30:45 +00:00'::TIMESTAMP_TZ", SQL_SF_TIMESTAMP_TZ},
  };

  for (const auto& v : variants) {
    // When SQLColAttribute is called with SQL_DESC_TYPE on each variant
    auto stmt = conn.execute_fetch(v.sql);
    SQLLEN col_type = 0;
    SQLRETURN ret = SQLColAttribute(stmt.getHandle(), 1, SQL_DESC_TYPE, nullptr, 0, nullptr, &col_type);

    // Then the vendor code matches what SQLDescribeCol would return
    REQUIRE_ODBC(ret, stmt);
    CHECK(static_cast<SQLSMALLINT>(col_type) == v.expected_type);
  }
}
