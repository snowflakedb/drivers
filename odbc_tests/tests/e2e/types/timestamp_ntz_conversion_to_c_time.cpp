#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"

TEST_CASE("TIMESTAMP_NTZ to SQL_C_TYPE_TIME", "[timestamp_ntz][conversion][c_time]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIMESTAMP_NTZ with zero fractional seconds is fetched as SQL_C_TYPE_TIME
  auto time =
      check_no_truncation<SQL_C_TYPE_TIME>(conn.execute_fetch("SELECT '2024-01-15 14:30:45'::TIMESTAMP_NTZ"), 1);

  // Then Time components are extracted without warning
  CHECK(time.hour == 14);
  CHECK(time.minute == 30);
  CHECK(time.second == 45);
}

TEST_CASE("TIMESTAMP_NTZ to SQL_C_TYPE_TIME midnight", "[timestamp_ntz][conversion][c_time]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIMESTAMP_NTZ with midnight time is fetched as SQL_C_TYPE_TIME
  auto time =
      check_no_truncation<SQL_C_TYPE_TIME>(conn.execute_fetch("SELECT '2024-01-15 00:00:00'::TIMESTAMP_NTZ"), 1);

  // Then All time components are zero
  CHECK(time.hour == 0);
  CHECK(time.minute == 0);
  CHECK(time.second == 0);
}

TEST_CASE("TIMESTAMP_NTZ to SQL_C_TYPE_TIME with fractional truncation",
          "[timestamp_ntz][conversion][c_time][truncation]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIMESTAMP_NTZ with non-zero fractional seconds is fetched as SQL_C_TYPE_TIME
  auto time = check_fractional_truncation<SQL_C_TYPE_TIME>(
      conn.execute_fetch("SELECT '2024-01-15 14:30:45.123'::TIMESTAMP_NTZ"), 1);

  // Then Time components are extracted with SQLSTATE 01S07 warning
  CHECK(time.hour == 14);
  CHECK(time.minute == 30);
  CHECK(time.second == 45);
}

TEST_CASE("TIMESTAMP_NTZ NULL to SQL_C_TYPE_TIME", "[timestamp_ntz][conversion][c_time][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL TIMESTAMP_NTZ value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::TIMESTAMP_NTZ");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_TYPE_TIME);
}
