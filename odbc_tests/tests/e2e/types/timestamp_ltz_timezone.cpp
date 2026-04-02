#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"

TEST_CASE("TIMESTAMP_LTZ with America/New_York timezone", "[timestamp_ltz][conversion][timezone]") {
  // Given Snowflake client is logged in with America/New_York timezone
  Connection conn;
  conn.execute("ALTER SESSION SET TIMEZONE = 'America/New_York'");

  // When A known UTC instant is fetched as TIMESTAMP_LTZ via SQL_C_TYPE_TIMESTAMP
  (void)0;
  // Then SQL_TIMESTAMP_STRUCT contains UTC time (struct has no timezone field)
  {
    INFO("winter (EST = UTC-5) - struct returns UTC hour 17, not local hour 12");
    auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
        conn.execute_fetch("SELECT '2024-01-15 17:00:00 +00:00'::TIMESTAMP_LTZ"), 1);
    CHECK(ts.year == 2024);
    CHECK(ts.month == 1);
    CHECK(ts.day == 15);
    CHECK(ts.hour == 17);
    CHECK(ts.minute == 0);
    CHECK(ts.second == 0);
  }

  {
    INFO("summer (EDT = UTC-4) - struct still returns UTC hour 17");
    auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
        conn.execute_fetch("SELECT '2024-07-15 17:00:00 +00:00'::TIMESTAMP_LTZ"), 1);
    CHECK(ts.year == 2024);
    CHECK(ts.month == 7);
    CHECK(ts.day == 15);
    CHECK(ts.hour == 17);
    CHECK(ts.minute == 0);
    CHECK(ts.second == 0);
  }
}

TEST_CASE("TIMESTAMP_LTZ with Asia/Kolkata timezone", "[timestamp_ltz][conversion][timezone]") {
  // Given Snowflake client is logged in with Asia/Kolkata timezone (UTC+5:30)
  Connection conn;
  conn.execute("ALTER SESSION SET TIMEZONE = 'Asia/Kolkata'");

  // When A known UTC instant is fetched as TIMESTAMP_LTZ via SQL_C_TYPE_TIMESTAMP
  auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
      conn.execute_fetch("SELECT '2024-01-15 18:30:00 +00:00'::TIMESTAMP_LTZ"), 1);

  // Then SQL_TIMESTAMP_STRUCT contains UTC time (hour 18, not IST hour 0 next day)
  CHECK(ts.year == 2024);
  CHECK(ts.month == 1);
  CHECK(ts.day == 15);
  CHECK(ts.hour == 18);
  CHECK(ts.minute == 30);
  CHECK(ts.second == 0);
}

TEST_CASE("TIMESTAMP_LTZ with Pacific/Auckland timezone", "[timestamp_ltz][conversion][timezone]") {
  // Given Snowflake client is logged in with Pacific/Auckland timezone (UTC+12 or +13 DST)
  Connection conn;
  conn.execute("ALTER SESSION SET TIMEZONE = 'Pacific/Auckland'");

  // When A known UTC instant is fetched as TIMESTAMP_LTZ via SQL_C_TYPE_TIMESTAMP
  auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
      conn.execute_fetch("SELECT '2024-01-15 10:00:00 +00:00'::TIMESTAMP_LTZ"), 1);

  // Then SQL_TIMESTAMP_STRUCT contains UTC time (hour 10, not NZDT hour 23)
  CHECK(ts.year == 2024);
  CHECK(ts.month == 1);
  CHECK(ts.day == 15);
  CHECK(ts.hour == 10);
  CHECK(ts.minute == 0);
  CHECK(ts.second == 0);
}

TEST_CASE("TIMESTAMP_LTZ timezone does not affect SQL_C_TYPE_TIMESTAMP struct",
          "[timestamp_ltz][conversion][timezone]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When The same UTC instant is fetched with different session timezones
  (void)0;
  // Then SQL_TIMESTAMP_STRUCT always contains the same UTC values
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");
  auto ts_utc = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
      conn.execute_fetch("SELECT '2024-03-10 07:00:00 +00:00'::TIMESTAMP_LTZ"), 1);

  conn.execute("ALTER SESSION SET TIMEZONE = 'America/New_York'");
  auto ts_ny = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
      conn.execute_fetch("SELECT '2024-03-10 07:00:00 +00:00'::TIMESTAMP_LTZ"), 1);

  conn.execute("ALTER SESSION SET TIMEZONE = 'Asia/Kolkata'");
  auto ts_ist = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
      conn.execute_fetch("SELECT '2024-03-10 07:00:00 +00:00'::TIMESTAMP_LTZ"), 1);

  CHECK(ts_utc.year == ts_ny.year);
  CHECK(ts_utc.month == ts_ny.month);
  CHECK(ts_utc.day == ts_ny.day);
  CHECK(ts_utc.hour == ts_ny.hour);
  CHECK(ts_utc.minute == ts_ny.minute);
  CHECK(ts_utc.second == ts_ny.second);

  CHECK(ts_utc.year == ts_ist.year);
  CHECK(ts_utc.month == ts_ist.month);
  CHECK(ts_utc.day == ts_ist.day);
  CHECK(ts_utc.hour == ts_ist.hour);
  CHECK(ts_utc.minute == ts_ist.minute);
  CHECK(ts_utc.second == ts_ist.second);
}

TEST_CASE("TIMESTAMP_LTZ timezone does not affect SQL_C_CHAR output", "[timestamp_ltz][conversion][timezone][c_char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When The same UTC instant is fetched as SQL_C_CHAR with different session timezones
  (void)0;
  // Then String output is the same regardless of session timezone
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");
  auto str_utc = check_char_success(conn.execute_fetch("SELECT '2024-01-15 17:00:00 +00:00'::TIMESTAMP_LTZ"), 1);

  conn.execute("ALTER SESSION SET TIMEZONE = 'America/New_York'");
  auto str_ny = check_char_success(conn.execute_fetch("SELECT '2024-01-15 17:00:00 +00:00'::TIMESTAMP_LTZ"), 1);

  CHECK(str_utc == str_ny);
}

TEST_CASE("TIMESTAMP_LTZ implicit timezone from literal without offset", "[timestamp_ltz][conversion][timezone]") {
  // Given Snowflake client is logged in with two different timezones
  Connection conn;

  // When A TIMESTAMP_LTZ literal without explicit offset is cast
  (void)0;
  // Then The session timezone determines the UTC instant, so struct values differ
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");
  auto ts_utc =
      check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '2024-01-15 12:00:00'::TIMESTAMP_LTZ"), 1);

  conn.execute("ALTER SESSION SET TIMEZONE = 'America/New_York'");
  auto ts_ny =
      check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '2024-01-15 12:00:00'::TIMESTAMP_LTZ"), 1);

  CHECK(ts_utc.hour == 12);
  CHECK(ts_ny.hour == 17);
}
