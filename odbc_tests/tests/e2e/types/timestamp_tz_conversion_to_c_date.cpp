#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"

TEST_CASE("TIMESTAMP_TZ to SQL_C_TYPE_DATE", "[timestamp_tz][conversion][c_date]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIMESTAMP_TZ with midnight UTC time is fetched as SQL_C_TYPE_DATE
  auto date =
      check_no_truncation<SQL_C_TYPE_DATE>(conn.execute_fetch("SELECT '2024-01-15 00:00:00 +00:00'::TIMESTAMP_TZ"), 1);

  // Then Date components are extracted from UTC without warning
  CHECK(date.year == 2024);
  CHECK(date.month == 1);
  CHECK(date.day == 15);
}

TEST_CASE("TIMESTAMP_TZ to SQL_C_TYPE_DATE boundary values", "[timestamp_tz][conversion][c_date]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Boundary TIMESTAMP_TZ values are fetched as SQL_C_TYPE_DATE
  (void)0;
  // Then Date components match expected UTC values
  {
    INFO("pre-epoch");
    auto date = check_fractional_truncation<SQL_C_TYPE_DATE>(
        conn.execute_fetch("SELECT '1960-06-15 12:00:00 +00:00'::TIMESTAMP_TZ"), 1);
    CHECK(date.year == 1960);
    CHECK(date.month == 6);
    CHECK(date.day == 15);
  }

  {
    INFO("leap day");
    auto date = check_no_truncation<SQL_C_TYPE_DATE>(
        conn.execute_fetch("SELECT '2000-02-29 00:00:00 +00:00'::TIMESTAMP_TZ"), 1);
    CHECK(date.year == 2000);
    CHECK(date.month == 2);
    CHECK(date.day == 29);
  }

  {
    INFO("timezone offset crosses date boundary");
    auto date = check_fractional_truncation<SQL_C_TYPE_DATE>(
        conn.execute_fetch("SELECT '2024-01-15 02:00:00 +05:00'::TIMESTAMP_TZ"), 1);
    CHECK(date.year == 2024);
    CHECK(date.month == 1);
    CHECK(date.day == 14);
  }
}

TEST_CASE("TIMESTAMP_TZ to SQL_C_TYPE_DATE with time truncation", "[timestamp_tz][conversion][c_date][truncation]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIMESTAMP_TZ with non-zero UTC time is fetched as SQL_C_TYPE_DATE
  auto date = check_fractional_truncation<SQL_C_TYPE_DATE>(
      conn.execute_fetch("SELECT '2024-01-15 14:30:45 +00:00'::TIMESTAMP_TZ"), 1);

  // Then Date components are extracted from UTC with SQLSTATE 01S07 warning
  CHECK(date.year == 2024);
  CHECK(date.month == 1);
  CHECK(date.day == 15);
}

TEST_CASE("TIMESTAMP_TZ NULL to SQL_C_TYPE_DATE", "[timestamp_tz][conversion][c_date][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL TIMESTAMP_TZ value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::TIMESTAMP_TZ");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_TYPE_DATE);
}
