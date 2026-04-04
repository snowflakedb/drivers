#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"

TEST_CASE("CLIENT_TIMESTAMP_TYPE_MAPPING=TIMESTAMP_NTZ maps untyped timestamp to NTZ", "[timestamp][type_mapping]") {
  // Given Snowflake client is logged in with NTZ timestamp mapping
  Connection conn;
  conn.execute("ALTER SESSION SET CLIENT_TIMESTAMP_TYPE_MAPPING = 'TIMESTAMP_NTZ'");

  // When An unqualified TIMESTAMP column is queried and described
  auto stmt = conn.execute_fetch("SELECT '2024-01-15 14:30:45'::TIMESTAMP");

  // Then The value is fetched as SQL_C_TYPE_TIMESTAMP correctly
  auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(stmt, 1);
  CHECK(ts.year == 2024);
  CHECK(ts.month == 1);
  CHECK(ts.day == 15);
  CHECK(ts.hour == 14);
  CHECK(ts.minute == 30);
  CHECK(ts.second == 45);
}

TEST_CASE("CLIENT_TIMESTAMP_TYPE_MAPPING=TIMESTAMP_LTZ maps untyped timestamp to LTZ", "[timestamp][type_mapping]") {
  // Given Snowflake client is logged in with LTZ timestamp mapping and UTC timezone
  Connection conn;
  conn.execute("ALTER SESSION SET CLIENT_TIMESTAMP_TYPE_MAPPING = 'TIMESTAMP_LTZ'");
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");

  // When An unqualified TIMESTAMP column is queried
  auto stmt = conn.execute_fetch("SELECT '2024-01-15 14:30:45'::TIMESTAMP");

  // Then The value is fetched correctly (server interprets the literal in session timezone)
  auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(stmt, 1);
  CHECK(ts.year == 2024);
  CHECK(ts.month == 1);
  CHECK(ts.day == 15);
  CHECK(ts.hour == 14);
  CHECK(ts.minute == 30);
  CHECK(ts.second == 45);
}

TEST_CASE("TIMESTAMP_TYPE_MAPPING changes column type for TIMESTAMP", "[timestamp][type_mapping]") {
  // Given Snowflake client is logged in with NTZ type mapping
  Connection conn;
  conn.execute("ALTER SESSION SET TIMESTAMP_TYPE_MAPPING = 'TIMESTAMP_NTZ'");

  // When A TIMESTAMP column is described via SQLDescribeCol
  auto stmt = conn.execute_fetch("SELECT '2024-01-15 14:30:45'::TIMESTAMP");
  SQLCHAR col_name[256];
  SQLSMALLINT name_len, data_type, decimal_digits, nullable;
  SQLULEN column_size;
  SQLRETURN ret = SQLDescribeCol(stmt.getHandle(), 1, col_name, sizeof(col_name), &name_len, &data_type, &column_size,
                                 &decimal_digits, &nullable);
  REQUIRE_ODBC(ret, stmt);

  // Then The SQL data type should be SQL_TYPE_TIMESTAMP
  CHECK(data_type == SQL_TYPE_TIMESTAMP);
}
