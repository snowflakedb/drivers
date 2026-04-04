#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"
#include "get_diag_rec.hpp"

TEST_CASE("TIMESTAMP_TZ to SQL_C_BINARY", "[timestamp_tz][conversion][c_binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIMESTAMP_TZ value is fetched as SQL_C_BINARY with sufficient buffer
  auto stmt = conn.execute_fetch("SELECT '2024-01-15 14:30:45 +00:00'::TIMESTAMP_TZ");
  char buffer[100] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);

  // Then SQL_SUCCESS is returned and indicator equals sizeof(SQL_TIMESTAMP_STRUCT)
  CHECK(ret == SQL_SUCCESS);
  CHECK(indicator == sizeof(SQL_TIMESTAMP_STRUCT));
  SQL_TIMESTAMP_STRUCT ts;
  std::memcpy(&ts, buffer, sizeof(SQL_TIMESTAMP_STRUCT));
  CHECK(ts.year == 2024);
  CHECK(ts.month == 1);
  CHECK(ts.day == 15);
  CHECK(ts.hour == 14);
  CHECK(ts.minute == 30);
  CHECK(ts.second == 45);
  CHECK(ts.fraction == 0);
}

TEST_CASE("TIMESTAMP_TZ to SQL_C_BINARY with fractional seconds", "[timestamp_tz][conversion][c_binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIMESTAMP_TZ with fractional seconds is fetched as SQL_C_BINARY
  auto stmt = conn.execute_fetch("SELECT '2024-01-15 10:30:00.123456789 +00:00'::TIMESTAMP_TZ");
  char buffer[100] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);

  // Then SQL_SUCCESS is returned and fraction field is preserved
  CHECK(ret == SQL_SUCCESS);
  CHECK(indicator == sizeof(SQL_TIMESTAMP_STRUCT));
  SQL_TIMESTAMP_STRUCT ts_frac;
  std::memcpy(&ts_frac, buffer, sizeof(SQL_TIMESTAMP_STRUCT));
  CHECK(ts_frac.fraction == 123456789);
}

TEST_CASE("TIMESTAMP_TZ to SQL_C_BINARY pre-epoch", "[timestamp_tz][conversion][c_binary]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A pre-epoch TIMESTAMP_TZ value is fetched as SQL_C_BINARY
  auto stmt = conn.execute_fetch("SELECT '1960-06-15 12:00:00 +00:00'::TIMESTAMP_TZ");
  char buffer[100] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);

  // Then SQL_SUCCESS is returned with correct pre-epoch date components
  CHECK(ret == SQL_SUCCESS);
  CHECK(indicator == sizeof(SQL_TIMESTAMP_STRUCT));
  SQL_TIMESTAMP_STRUCT ts;
  std::memcpy(&ts, buffer, sizeof(SQL_TIMESTAMP_STRUCT));
  CHECK(ts.year == 1960);
  CHECK(ts.month == 6);
  CHECK(ts.day == 15);
  CHECK(ts.hour == 12);
  CHECK(ts.minute == 0);
  CHECK(ts.second == 0);
  CHECK(ts.fraction == 0);
}

TEST_CASE("TIMESTAMP_TZ to SQL_C_BINARY buffer too small", "[timestamp_tz][conversion][c_binary][22003]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIMESTAMP_TZ value is fetched into a buffer smaller than sizeof(SQL_TIMESTAMP_STRUCT)
  auto stmt = conn.execute_fetch("SELECT '2024-01-15 14:30:45 +00:00'::TIMESTAMP_TZ");
  char buffer[4] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);

  // Then SQL_ERROR is returned with SQLSTATE 22003
  CHECK(ret == SQL_ERROR);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "22003");
}

TEST_CASE("TIMESTAMP_TZ NULL to SQL_C_BINARY", "[timestamp_tz][conversion][c_binary][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL TIMESTAMP_TZ value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::TIMESTAMP_TZ");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_BINARY);
}
