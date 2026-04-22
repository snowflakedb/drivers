#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"

TEST_CASE("TIMESTAMP_NTZ historical era year 1600", "[timestamp][historical][ntz]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIMESTAMP_NTZ from year 1600 is fetched
  auto ts =
      check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '1600-01-01 00:00:00'::TIMESTAMP_NTZ"), 1);

  // Then SQL_TIMESTAMP_STRUCT contains the correct historical date
  CHECK(ts.year == 1600);
  CHECK(ts.month == 1);
  CHECK(ts.day == 1);
  CHECK(ts.hour == 0);
  CHECK(ts.minute == 0);
  CHECK(ts.second == 0);
}

TEST_CASE("TIMESTAMP_NTZ far future year 3017", "[timestamp][historical][ntz]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIMESTAMP_NTZ from year 3017 is fetched
  auto ts =
      check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '3017-06-15 12:30:45'::TIMESTAMP_NTZ"), 1);

  // Then SQL_TIMESTAMP_STRUCT contains the correct far-future date
  CHECK(ts.year == 3017);
  CHECK(ts.month == 6);
  CHECK(ts.day == 15);
  CHECK(ts.hour == 12);
  CHECK(ts.minute == 30);
  CHECK(ts.second == 45);
}

TEST_CASE("TIMESTAMP_NTZ year 0001 minimum representable date", "[timestamp][historical][ntz]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When The earliest possible Snowflake timestamp is fetched
  auto ts =
      check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '0001-01-01 00:00:00'::TIMESTAMP_NTZ"), 1);

  // Then SQL_TIMESTAMP_STRUCT contains year 1
  CHECK(ts.year == 1);
  CHECK(ts.month == 1);
  CHECK(ts.day == 1);
}

TEST_CASE("TIMESTAMP_NTZ year 9999 maximum representable date", "[timestamp][historical][ntz]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When The latest possible Snowflake timestamp is fetched
  auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
      conn.execute_fetch("SELECT '9999-12-31 23:59:59.999999999'::TIMESTAMP_NTZ"), 1);

  // Then SQL_TIMESTAMP_STRUCT contains the maximum date
  CHECK(ts.year == 9999);
  CHECK(ts.month == 12);
  CHECK(ts.day == 31);
  CHECK(ts.hour == 23);
  CHECK(ts.minute == 59);
  CHECK(ts.second == 59);
  CHECK(ts.fraction == 999999999);
}

TEST_CASE("TIMESTAMP_LTZ historical era year 1600", "[timestamp][historical][ltz]") {
  // Given Snowflake client is logged in with UTC timezone
  Connection conn;
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");

  // When A TIMESTAMP_LTZ from year 1600 is fetched
  auto ts =
      check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '1600-01-01 00:00:00'::TIMESTAMP_LTZ"), 1);

  // Then SQL_TIMESTAMP_STRUCT contains the correct historical date
  CHECK(ts.year == 1600);
  CHECK(ts.month == 1);
  CHECK(ts.day == 1);
}

TEST_CASE("TIMESTAMP_LTZ far future year 3017", "[timestamp][historical][ltz]") {
  // Given Snowflake client is logged in with UTC timezone
  Connection conn;
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");

  // When A TIMESTAMP_LTZ from year 3017 is fetched
  auto ts =
      check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '3017-06-15 12:30:45'::TIMESTAMP_LTZ"), 1);

  // Then SQL_TIMESTAMP_STRUCT contains the correct far-future date
  CHECK(ts.year == 3017);
  CHECK(ts.month == 6);
  CHECK(ts.day == 15);
}

TEST_CASE("TIMESTAMP_TZ historical era year 1600 with offset crossing date boundary", "[timestamp][historical][tz]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A pre-epoch TIMESTAMP_TZ with offset crossing a date boundary is fetched
  auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
      conn.execute_fetch("SELECT '1600-01-01 02:00:00 +05:00'::TIMESTAMP_TZ"), 1);

  // Then SQL_TIMESTAMP_STRUCT contains the UTC-converted date (crosses back to previous day)
  CHECK(ts.year == 1599);
  CHECK(ts.month == 12);
  CHECK(ts.day == 31);
  CHECK(ts.hour == 21);
  CHECK(ts.minute == 0);
  CHECK(ts.second == 0);
}

TEST_CASE("TIMESTAMP_TZ far future year 3017", "[timestamp][historical][tz]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIMESTAMP_TZ from year 3017 is fetched
  auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
      conn.execute_fetch("SELECT '3017-06-15 12:30:45 -08:00'::TIMESTAMP_TZ"), 1);

  // Then SQL_TIMESTAMP_STRUCT contains the UTC-converted far-future date
  CHECK(ts.year == 3017);
  CHECK(ts.month == 6);
  CHECK(ts.day == 15);
  CHECK(ts.hour == 20);
  CHECK(ts.minute == 30);
  CHECK(ts.second == 45);
}

TEST_CASE("TIMESTAMP_TZ year 0001 with positive offset", "[timestamp][historical][tz]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When The earliest representable TIMESTAMP_TZ with a positive offset is fetched
  auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
      conn.execute_fetch("SELECT '0001-01-01 12:00:00 +05:30'::TIMESTAMP_TZ"), 1);

  // Then SQL_TIMESTAMP_STRUCT contains the UTC-converted value
  CHECK(ts.year == 1);
  CHECK(ts.month == 1);
  CHECK(ts.day == 1);
  CHECK(ts.hour == 6);
  CHECK(ts.minute == 30);
  CHECK(ts.second == 0);
}

TEST_CASE("TIMESTAMP_NTZ historical dates as SQL_C_CHAR", "[timestamp][historical][c_char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Historical and far-future timestamps are fetched as SQL_C_CHAR
  (void)0;
  // Then String representations are correctly formatted
  {
    INFO("year 1600");
    auto result = check_char_success(conn.execute_fetch("SELECT '1600-01-01 00:00:00'::TIMESTAMP_NTZ"), 1);
    CHECK(result == "1600-01-01 00:00:00");
  }

  {
    INFO("year 3017");
    auto result = check_char_success(conn.execute_fetch("SELECT '3017-06-15 12:30:45'::TIMESTAMP_NTZ"), 1);
    CHECK(result == "3017-06-15 12:30:45");
  }

  {
    INFO("year 0001");
    auto result = check_char_success(conn.execute_fetch("SELECT '0001-01-01 00:00:00'::TIMESTAMP_NTZ"), 1);
    CHECK(result == "0001-01-01 00:00:00");
  }
}
