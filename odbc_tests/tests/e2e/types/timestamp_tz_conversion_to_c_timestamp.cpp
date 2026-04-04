#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"

TEST_CASE("TIMESTAMP_TZ to SQL_C_TYPE_TIMESTAMP basic values", "[timestamp_tz][conversion][c_timestamp]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When TIMESTAMP_TZ values are fetched as SQL_C_TYPE_TIMESTAMP
  (void)0;
  // Then SQL_TIMESTAMP_STRUCT fields match the UTC representation of the timestamp
  {
    INFO("UTC timestamp");
    auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
        conn.execute_fetch("SELECT '2024-01-15 14:30:45 +00:00'::TIMESTAMP_TZ"), 1);
    CHECK(ts.year == 2024);
    CHECK(ts.month == 1);
    CHECK(ts.day == 15);
    CHECK(ts.hour == 14);
    CHECK(ts.minute == 30);
    CHECK(ts.second == 45);
    CHECK(ts.fraction == 0);
  }

  {
    INFO("positive timezone offset returns UTC time");
    auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
        conn.execute_fetch("SELECT '2024-01-15 14:30:45 +05:30'::TIMESTAMP_TZ"), 1);
    CHECK(ts.year == 2024);
    CHECK(ts.month == 1);
    CHECK(ts.day == 15);
    CHECK(ts.hour == 9);
    CHECK(ts.minute == 0);
    CHECK(ts.second == 45);
    CHECK(ts.fraction == 0);
  }

  {
    INFO("negative timezone offset returns UTC time");
    auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
        conn.execute_fetch("SELECT '2024-01-15 14:30:45 -08:00'::TIMESTAMP_TZ"), 1);
    CHECK(ts.year == 2024);
    CHECK(ts.month == 1);
    CHECK(ts.day == 15);
    CHECK(ts.hour == 22);
    CHECK(ts.minute == 30);
    CHECK(ts.second == 45);
    CHECK(ts.fraction == 0);
  }

  {
    INFO("fractional seconds with timezone");
    auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
        conn.execute_fetch("SELECT '2024-01-15 10:30:00.123456789 +00:00'::TIMESTAMP_TZ"), 1);
    CHECK(ts.year == 2024);
    CHECK(ts.month == 1);
    CHECK(ts.day == 15);
    CHECK(ts.hour == 10);
    CHECK(ts.minute == 30);
    CHECK(ts.second == 0);
    CHECK(ts.fraction == 123456789);
  }

  {
    INFO("pre-epoch timestamp");
    auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
        conn.execute_fetch("SELECT '1960-06-15 12:00:00 +00:00'::TIMESTAMP_TZ"), 1);
    CHECK(ts.year == 1960);
    CHECK(ts.month == 6);
    CHECK(ts.day == 15);
    CHECK(ts.hour == 12);
    CHECK(ts.minute == 0);
    CHECK(ts.second == 0);
    CHECK(ts.fraction == 0);
  }

  {
    INFO("midnight UTC");
    auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
        conn.execute_fetch("SELECT '2024-06-15 00:00:00 +00:00'::TIMESTAMP_TZ"), 1);
    CHECK(ts.year == 2024);
    CHECK(ts.month == 6);
    CHECK(ts.day == 15);
    CHECK(ts.hour == 0);
    CHECK(ts.minute == 0);
    CHECK(ts.second == 0);
    CHECK(ts.fraction == 0);
  }

  {
    INFO("timezone offset crosses date boundary to previous day");
    auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
        conn.execute_fetch("SELECT '2024-01-15 02:00:00 +05:00'::TIMESTAMP_TZ"), 1);
    CHECK(ts.year == 2024);
    CHECK(ts.month == 1);
    CHECK(ts.day == 14);
    CHECK(ts.hour == 21);
    CHECK(ts.minute == 0);
    CHECK(ts.second == 0);
    CHECK(ts.fraction == 0);
  }

  {
    INFO("timezone offset crosses date boundary to next day");
    auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
        conn.execute_fetch("SELECT '2024-01-15 23:00:00 -05:00'::TIMESTAMP_TZ"), 1);
    CHECK(ts.year == 2024);
    CHECK(ts.month == 1);
    CHECK(ts.day == 16);
    CHECK(ts.hour == 4);
    CHECK(ts.minute == 0);
    CHECK(ts.second == 0);
    CHECK(ts.fraction == 0);
  }
}

TEST_CASE("TIMESTAMP_TZ NULL to SQL_C_TYPE_TIMESTAMP", "[timestamp_tz][conversion][c_timestamp][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL TIMESTAMP_TZ value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::TIMESTAMP_TZ");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_TYPE_TIMESTAMP);
}
