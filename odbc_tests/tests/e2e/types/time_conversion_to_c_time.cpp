#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"
#include "get_diag_rec.hpp"

TEST_CASE("TIME to SQL_C_TYPE_TIME", "[time][conversion][c_time]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIME with zero fractional seconds is fetched as SQL_C_TYPE_TIME
  auto time = check_no_truncation<SQL_C_TYPE_TIME>(conn.execute_fetch("SELECT '14:30:45'::TIME"), 1);

  // Then Time components are extracted without warning
  CHECK(time.hour == 14);
  CHECK(time.minute == 30);
  CHECK(time.second == 45);
}

TEST_CASE("TIME to SQL_C_TYPE_TIME midnight", "[time][conversion][c_time]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIME with midnight value is fetched as SQL_C_TYPE_TIME
  auto time = check_no_truncation<SQL_C_TYPE_TIME>(conn.execute_fetch("SELECT '00:00:00'::TIME"), 1);

  // Then All time components are zero
  CHECK(time.hour == 0);
  CHECK(time.minute == 0);
  CHECK(time.second == 0);
}

TEST_CASE("TIME to SQL_C_TYPE_TIME end of day", "[time][conversion][c_time]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIME near end of day is fetched as SQL_C_TYPE_TIME
  auto time = check_no_truncation<SQL_C_TYPE_TIME>(conn.execute_fetch("SELECT '23:59:59'::TIME"), 1);

  // Then Time components match end of day values
  CHECK(time.hour == 23);
  CHECK(time.minute == 59);
  CHECK(time.second == 59);
}

TEST_CASE("TIME to SQL_C_TYPE_TIME with fractional truncation", "[time][conversion][c_time][truncation]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIME with non-zero fractional seconds is fetched as SQL_C_TYPE_TIME
  auto time = check_fractional_truncation<SQL_C_TYPE_TIME>(conn.execute_fetch("SELECT '14:30:45.123'::TIME"), 1);

  // Then Time components are extracted with SQLSTATE 01S07 warning
  CHECK(time.hour == 14);
  CHECK(time.minute == 30);
  CHECK(time.second == 45);
}

TEST_CASE("TIME NULL to SQL_C_TYPE_TIME", "[time][conversion][c_time][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL TIME value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::TIME");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_TYPE_TIME);
}

// ============================================================================
// SQL_C_DEFAULT
// ============================================================================

TEST_CASE("TIME to SQL_C_DEFAULT", "[time][conversion][c_default]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIME with zero fractional seconds is fetched as SQL_C_DEFAULT
  auto stmt = conn.execute_fetch("SELECT '14:30:45'::TIME");
  SQL_TIME_STRUCT time = {};
  SQLLEN indicator = -999;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DEFAULT, &time, sizeof(time), &indicator);

  // Then SQL_C_DEFAULT resolves to SQL_C_TYPE_TIME with correct values
  CHECK(ret == SQL_SUCCESS);
  CHECK(indicator == sizeof(SQL_TIME_STRUCT));
  CHECK(time.hour == 14);
  CHECK(time.minute == 30);
  CHECK(time.second == 45);
}

TEST_CASE("TIME to SQL_C_DEFAULT with fractional truncation", "[time][conversion][c_default][truncation]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIME with non-zero fractional seconds is fetched as SQL_C_DEFAULT
  auto stmt = conn.execute_fetch("SELECT '14:30:45.123'::TIME");
  SQL_TIME_STRUCT time = {};
  SQLLEN indicator = -999;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DEFAULT, &time, sizeof(time), &indicator);

  // Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01S07
  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "01S07");
  CHECK(time.hour == 14);
  CHECK(time.minute == 30);
  CHECK(time.second == 45);
}

TEST_CASE("TIME NULL to SQL_C_DEFAULT", "[time][conversion][c_default][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL TIME value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::TIME");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_DEFAULT);
}
