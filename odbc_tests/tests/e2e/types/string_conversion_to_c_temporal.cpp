// String to ODBC temporal type conversions tests
// Tests converting Snowflake VARCHAR/STRING type to temporal ODBC C types:
// SQL_C_TYPE_DATE, SQL_C_TYPE_TIME, SQL_C_TYPE_TIMESTAMP

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <chrono>
#include <cstring>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "macros.hpp"
#include "test_setup.hpp"

// ============================================================================
// SUCCESSFUL CONVERSIONS - String to Date/Time Types
// ============================================================================

TEST_CASE("should convert string literals to c_type", "[datatype][string][conversion][temporal]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting string literals representing dates and times is executed
  auto stmt = conn.execute_fetch(
      "SELECT '2024-01-15' AS c1, '1999-12-31' AS c2, '2000-01-01' AS c3, "
      "'14:30:45' AS c4, '00:00:00' AS c5, '23:59:59' AS c6, "
      "'2024-01-15 14:30:45' AS c7, '1999-12-31 23:59:59' AS c8, '  2024-01-15 14:30:45  ' AS c9");

  // Then <c_type> conversions should work
  {
    INFO("SQL_C_TYPE_DATE");
    auto date1 = check_no_truncation<SQL_C_TYPE_DATE>(stmt, 1);
    CHECK(date1.year == 2024);
    CHECK(date1.month == 1);
    CHECK(date1.day == 15);

    auto date2 = check_no_truncation<SQL_C_TYPE_DATE>(stmt, 2);
    CHECK(date2.year == 1999);
    CHECK(date2.month == 12);
    CHECK(date2.day == 31);

    auto y2k = check_no_truncation<SQL_C_TYPE_DATE>(stmt, 3);
    CHECK(y2k.year == 2000);
    CHECK(y2k.month == 1);
    CHECK(y2k.day == 1);
  }

  {
    INFO("SQL_C_TYPE_TIME");
    auto time1 = check_no_truncation<SQL_C_TYPE_TIME>(stmt, 4);
    CHECK(time1.hour == 14);
    CHECK(time1.minute == 30);
    CHECK(time1.second == 45);

    auto midnight = check_no_truncation<SQL_C_TYPE_TIME>(stmt, 5);
    CHECK(midnight.hour == 0);
    CHECK(midnight.minute == 0);
    CHECK(midnight.second == 0);

    auto end_of_day = check_no_truncation<SQL_C_TYPE_TIME>(stmt, 6);
    CHECK(end_of_day.hour == 23);
    CHECK(end_of_day.minute == 59);
    CHECK(end_of_day.second == 59);
  }

  {
    INFO("SQL_C_TYPE_TIMESTAMP");
    auto ts1 = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(stmt, 7);
    CHECK(ts1.year == 2024);
    CHECK(ts1.month == 1);
    CHECK(ts1.day == 15);
    CHECK(ts1.hour == 14);
    CHECK(ts1.minute == 30);
    CHECK(ts1.second == 45);

    auto millennium = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(stmt, 8);
    CHECK(millennium.year == 1999);
    CHECK(millennium.month == 12);
    CHECK(millennium.day == 31);
    CHECK(millennium.hour == 23);
    CHECK(millennium.minute == 59);
    CHECK(millennium.second == 59);

    auto ts2 = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(stmt, 9);
    CHECK(ts2.year == 2024);
    CHECK(ts2.month == 1);
    CHECK(ts2.day == 15);
    CHECK(ts2.hour == 14);
    CHECK(ts2.minute == 30);
    CHECK(ts2.second == 45);
  }
}

// ============================================================================
// SUCCESSFUL CONVERSIONS - Date/Time strings to TIMESTAMP
// ============================================================================

TEST_CASE("should convert date-only and time-only strings to SQL_C_TYPE_TIMESTAMP",
          "[datatype][string][conversion][temporal]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting date-only string is executed
  auto stmt = conn.execute_fetch("SELECT '2024-01-15' AS date_only, '14:30:45' AS time_only");

  // And Data is retrieved as SQL_C_TYPE_TIMESTAMP
  auto date_only = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(stmt, 1);
  auto time_only = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(stmt, 2);

  // Then the date components should be correctly parsed
  CHECK(date_only.year == 2024);
  CHECK(date_only.month == 1);
  CHECK(date_only.day == 15);

  // And the time components should default to midnight
  CHECK(date_only.hour == 0);
  CHECK(date_only.minute == 0);
  CHECK(date_only.second == 0);

  // And the date components should default to today's date
  auto now = std::chrono::system_clock::now();
  auto now_c = std::chrono::system_clock::to_time_t(now);
  auto now_tm = *std::localtime(&now_c);

  CHECK(time_only.year == now_tm.tm_year + 1900);
  CHECK(time_only.month == now_tm.tm_mon + 1);
  CHECK(time_only.day == now_tm.tm_mday);

  // And the time components should be correctly parsed
  CHECK(time_only.hour == 14);
  CHECK(time_only.minute == 30);
  CHECK(time_only.second == 45);
}

// ============================================================================
// SUCCESSFUL CONVERSIONS - Timestamp strings to DATE or TIME
// ============================================================================

TEST_CASE("should convert timestamp string to SQL_C_TYPE_DATE", "[datatype][string][conversion][temporal][.skip]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting timestamp strings is executed
  auto stmt = conn.execute_fetch(
      "SELECT '2024-01-15 14:30:45' AS c1, '1999-12-31 23:59:59' AS c2, '2000-06-15 00:00:00' AS c3");

  // Then SQL_C_TYPE_DATE should extract the date component (time is truncated)
  auto date1 = check_fractional_truncation<SQL_C_TYPE_DATE>(stmt, 1);
  CHECK(date1.year == 2024);
  CHECK(date1.month == 1);
  CHECK(date1.day == 15);

  auto date2 = check_fractional_truncation<SQL_C_TYPE_DATE>(stmt, 2);
  CHECK(date2.year == 1999);
  CHECK(date2.month == 12);
  CHECK(date2.day == 31);

  auto date3 = check_no_truncation<SQL_C_TYPE_DATE>(stmt, 3);
  CHECK(date3.year == 2000);
  CHECK(date3.month == 6);
  CHECK(date3.day == 15);
}

TEST_CASE("should convert timestamp string to SQL_C_TYPE_TIME", "[datatype][string][conversion][temporal][.skip]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting timestamp strings is executed
  auto stmt = conn.execute_fetch(
      "SELECT '2024-01-15 14:30:45' AS c1, '1999-12-31 23:59:59' AS c2, '2000-06-15 00:00:00' AS c3");

  // Then SQL_C_TYPE_TIME should extract the time component (date is truncated)
  auto time1 = check_no_truncation<SQL_C_TYPE_TIME>(stmt, 1);
  CHECK(time1.hour == 14);
  CHECK(time1.minute == 30);
  CHECK(time1.second == 45);

  auto time2 = check_no_truncation<SQL_C_TYPE_TIME>(stmt, 2);
  CHECK(time2.hour == 23);
  CHECK(time2.minute == 59);
  CHECK(time2.second == 59);

  auto time3 = check_no_truncation<SQL_C_TYPE_TIME>(stmt, 3);
  CHECK(time3.hour == 0);
  CHECK(time3.minute == 0);
  CHECK(time3.second == 0);
}

// ============================================================================
// FAILING CONVERSIONS - Invalid date/time format strings
// ============================================================================

TEST_CASE("should fail converting invalid date/time strings", "[datatype][string][conversion][temporal][failure]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting invalid date/time strings is executed
  auto stmt = conn.execute_fetch("SELECT 'not-a-date' AS c1, 'not-a-time' AS c2, 'invalid-timestamp' AS c3");

  // Then invalid date/time strings should fail
  check_invalid_string<SQL_C_TYPE_DATE>(stmt, 1);
  check_invalid_string<SQL_C_TYPE_TIME>(stmt, 2);
  check_invalid_string<SQL_C_TYPE_TIMESTAMP>(stmt, 3);
}

// ============================================================================
// FAILING CONVERSIONS - Impossible date/time values (correct syntax, invalid values)
// ============================================================================

TEST_CASE("should fail converting impossible date values",
          "[datatype][string][conversion][temporal][failure][impossible]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting date strings with correct syntax but impossible values is executed
  auto stmt = conn.execute_fetch(
      "SELECT '2024-13-01' AS month_13, '2024-00-15' AS month_0, '2024-01-32' AS day_32, "
      "'2024-02-30' AS feb_30, '2024-04-31' AS apr_31, '2024-01-00' AS day_0");

  // Then impossible date values should fail with SQLSTATE 22018
  std::vector<std::pair<int, std::string>> impossible_date_columns = {
      {1, "month 13"}, {2, "month 0"}, {3, "day 32"}, {4, "February 30"}, {5, "April 31"}, {6, "day 0"}};

  for (const auto& [column, description] : impossible_date_columns) {
    INFO("Converting impossible date: " + description);
    check_invalid_string<SQL_C_TYPE_DATE>(stmt, column);
  }
}

TEST_CASE("should fail converting impossible time values",
          "[datatype][string][conversion][temporal][failure][impossible][.skip]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting time strings with correct syntax but impossible values is executed
  auto stmt = conn.execute_fetch(
      "SELECT '25:00:00' AS hour_25, '24:00:00' AS hour_24, '14:60:00' AS minute_60, '14:30:60' AS second_60");

  // Then hour 25 should fail
  INFO("Converting impossible time: hour 25");
  check_invalid_string<SQL_C_TYPE_TIME>(stmt, 1);

  // And hour 24 should fail
  INFO("Converting impossible time: hour 24");
  check_invalid_string<SQL_C_TYPE_TIME>(stmt, 2);

  // And minute 60 should fail
  INFO("Converting impossible time: minute 60");
  check_invalid_string<SQL_C_TYPE_TIME>(stmt, 3);

  // And second 60 might behave differently in the old driver
  INFO("Converting impossible time: second 60");
  auto time = check_no_truncation<SQL_C_TYPE_TIME>(stmt, 4);
  CHECK(time.hour == 14);
  CHECK(time.minute == 30);
  CHECK(time.second == 60);
}

TEST_CASE("should fail converting impossible timestamp values",
          "[datatype][string][conversion][temporal][failure][impossible]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting timestamp strings with correct syntax but impossible values is executed
  auto stmt = conn.execute_fetch(
      "SELECT '2024-13-01 14:30:45' AS month_13, '2024-01-15 25:00:00' AS hour_25, "
      "'2024-02-30 12:00:00' AS feb_30, '2024-01-15 14:60:00' AS minute_60");

  // Then impossible timestamp values should fail with SQLSTATE 22018
  std::vector<std::pair<int, std::string>> impossible_timestamp_columns = {
      {1, "month 13"}, {2, "hour 25"}, {3, "February 30"}, {4, "minute 60"}};

  for (const auto& [column, description] : impossible_timestamp_columns) {
    INFO("Converting impossible timestamp: " + description);
    check_invalid_string<SQL_C_TYPE_TIMESTAMP>(stmt, column);
  }
}

// ============================================================================
// FAILING CONVERSIONS - Alternative date serialization formats
// ============================================================================

TEST_CASE("should fail converting alternative date formats to SQL_C_TYPE_DATE",
          "[datatype][string][conversion][temporal][failure][date_format]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting multiple date strings in alternative formats is executed
  auto stmt = conn.execute_fetch(
      "SELECT '01/15/2024' AS us_format, '15.01.2024' AS european_format, '2024/01/15' AS slash_format, 'January 15, "
      "2024' AS spelled_month, '15-Jan-2024' AS abbreviated_month, '15-01-2024' AS reversed_format, '24-01-15' AS "
      "two_digit_year, '2024-1-5' AS single_digit");

  // Then all alternative date formats should fail with SQLSTATE 22018
  std::vector<std::pair<int, std::string>> invalid_date_columns = {{1, "US format (MM/DD/YYYY)"},
                                                                   {2, "European format (DD.MM.YYYY)"},
                                                                   {3, "slash separators (YYYY/MM/DD)"},
                                                                   {4, "spelled out month"},
                                                                   {5, "abbreviated month"},
                                                                   {6, "reversed format (DD-MM-YYYY)"},
                                                                   {7, "two-digit year"},
                                                                   {8, "single-digit month and day"}};

  for (const auto& [column, description] : invalid_date_columns) {
    INFO("Converting " + description);
    check_invalid_string<SQL_C_TYPE_DATE>(stmt, column);
  }
}

// ============================================================================
// FAILING CONVERSIONS - Alternative time serialization formats
// ============================================================================

TEST_CASE("should fail converting alternative time formats to SQL_C_TYPE_TIME",
          "[datatype][string][conversion][temporal][failure][time_format]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting multiple time strings in alternative formats is executed
  auto stmt = conn.execute_fetch(
      "SELECT '2:30:45 PM' AS twelve_hour_format, '14:30' AS no_seconds, '14.30.45' AS dot_separator, '9:5:3' AS "
      "single_digit");

  // Then all alternative time formats should fail with SQLSTATE 22018
  std::vector<std::pair<int, std::string>> invalid_time_columns = {{1, "12-hour format with AM/PM"},
                                                                   {2, "time without seconds"},
                                                                   {3, "dot separator"},
                                                                   {4, "single-digit components"}};

  for (const auto& [column, description] : invalid_time_columns) {
    INFO("Converting " + description);
    check_invalid_string<SQL_C_TYPE_TIMESTAMP>(stmt, column);
  }
}

// ============================================================================
// FAILING CONVERSIONS - Alternative timestamp serialization formats
// ============================================================================

TEST_CASE("should fail converting alternative timestamp formats to SQL_C_TYPE_TIMESTAMP",
          "[datatype][string][conversion][temporal][failure][timestamp_format]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting multiple timestamp strings in alternative formats is executed
  auto stmt = conn.execute_fetch(
      "SELECT '2024-01-15T14:30:45' AS iso_t_separator, '2024-01-15 14:30:45+05:00' AS timezone_offset, "
      "'2024-01-15T14:30:45Z' AS utc_suffix, '01/15/2024 14:30:45' AS us_format");

  // Then all alternative timestamp formats should fail with SQLSTATE 22018
  std::vector<std::pair<int, std::string>> invalid_timestamp_columns = {
      {1, "ISO 8601 with T separator"}, {2, "timezone offset"}, {3, "Z (UTC) suffix"}, {4, "US format (MM/DD/YYYY)"}};

  for (const auto& [column, description] : invalid_timestamp_columns) {
    INFO("Converting " + description);
    check_invalid_string<SQL_C_TYPE_TIMESTAMP>(stmt, column);
  }
}
