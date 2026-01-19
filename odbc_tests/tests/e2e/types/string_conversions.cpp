// String to ODBC type conversions tests
// Tests converting Snowflake VARCHAR/STRING type to various ODBC C types
//
// This file tests:
// 1. Successful conversions from string literals representing numbers to numeric ODBC types
// 2. Failing conversions (invalid strings that cannot be converted to target types)
// 3. Edge cases like overflow, underflow, and precision loss

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cmath>
#include <cstring>
#include <limits>
#include <string>
#include <vector>

#include <catch2/catch_approx.hpp>
#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "macros.hpp"
#include "test_setup.hpp"

// Helper to get raw data with error checking for expected failures
template <typename T>
static SQLRETURN get_data_raw(const StatementHandleWrapper& stmt, SQLUSMALLINT col, SQLSMALLINT target_type, T* value,
                              SQLLEN* indicator) {
  return SQLGetData(stmt.getHandle(), col, target_type, value, sizeof(*value), indicator);
}

// Helper to check SQLSTATE from diagnostic records
static std::string get_sqlstate(const StatementHandleWrapper& stmt) {
  auto records = get_diag_rec(stmt);
  if (!records.empty()) {
    return records[0].sqlState;
  }
  return "";
}

// ============================================================================
// SUCCESSFUL CONVERSIONS - String to Integer Types
// ============================================================================

TEST_CASE("should convert string literals to signed integer types", "[datatype][string][conversion]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting string literals representing integers is executed
  auto stmt = conn.execute_fetch(
      "SELECT '123' AS c1, '-456' AS c2, '0' AS c3, "
      "'2147483647' AS c4, '-2147483648' AS c5, "
      "'999' AS c6, '-999' AS c7, "
      "'32767' AS c8, '-32768' AS c9, "
      "'100' AS c10, '-100' AS c11, '127' AS c12, '-128' AS c13, "
      "'50' AS c14, '-50' AS c15, "
      "'9223372036854775807' AS c16, '-9223372036854775808' AS c17, '1234567890123456789' AS c18");

  // Then SQL_C_LONG conversions should work
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 123);
  CHECK(get_data<SQL_C_LONG>(stmt, 2) == -456);
  CHECK(get_data<SQL_C_LONG>(stmt, 3) == 0);
  CHECK(get_data<SQL_C_LONG>(stmt, 4) == 2147483647);
  CHECK(get_data<SQL_C_LONG>(stmt, 5) == -2147483648);

  // And SQL_C_SLONG conversions should work
  CHECK(get_data<SQL_C_SLONG>(stmt, 6) == 999);
  CHECK(get_data<SQL_C_SLONG>(stmt, 7) == -999);

  // And SQL_C_SHORT conversions should work
  CHECK(get_data<SQL_C_SHORT>(stmt, 8) == 32767);
  CHECK(get_data<SQL_C_SHORT>(stmt, 9) == -32768);

  // And SQL_C_TINYINT conversions should work
  CHECK(get_data<SQL_C_TINYINT>(stmt, 10) == 100);
  CHECK(get_data<SQL_C_TINYINT>(stmt, 11) == -100);
  CHECK(get_data<SQL_C_TINYINT>(stmt, 12) == 127);
  CHECK(get_data<SQL_C_TINYINT>(stmt, 13) == -128);

  // And SQL_C_STINYINT conversions should work
  CHECK(get_data<SQL_C_STINYINT>(stmt, 14) == 50);
  CHECK(get_data<SQL_C_STINYINT>(stmt, 15) == -50);

  // And SQL_C_SBIGINT conversions should work
  CHECK(get_data<SQL_C_SBIGINT>(stmt, 16) == 9223372036854775807LL);
  CHECK(get_data<SQL_C_SBIGINT>(stmt, 17) == (-9223372036854775807LL - 1));
  CHECK(get_data<SQL_C_SBIGINT>(stmt, 18) == 1234567890123456789LL);
}

TEST_CASE("should convert string literals to unsigned integer types", "[datatype][string][conversion]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting string literals representing unsigned integers is executed
  auto stmt = conn.execute_fetch(
      "SELECT '123' AS c1, '0' AS c2, '4294967295' AS c3, "
      "'65535' AS c4, "
      "'255' AS c5, "
      "'18446744073709551615' AS c6, '12345678901234567890' AS c7, "
      "'100' AS c8, '200' AS c9");

  // Then SQL_C_ULONG conversions should work
  CHECK(get_data<SQL_C_ULONG>(stmt, 1) == 123);
  CHECK(get_data<SQL_C_ULONG>(stmt, 2) == 0);
  CHECK(get_data<SQL_C_ULONG>(stmt, 3) == 4294967295U);

  // And SQL_C_USHORT conversions should work
  CHECK(get_data<SQL_C_USHORT>(stmt, 4) == 65535);

  // And SQL_C_UTINYINT conversions should work
  CHECK(get_data<SQL_C_UTINYINT>(stmt, 5) == 255);

  // And SQL_C_UBIGINT conversions should work
  CHECK(get_data<SQL_C_UBIGINT>(stmt, 6) == 18446744073709551615ULL);
  CHECK(get_data<SQL_C_UBIGINT>(stmt, 7) == 12345678901234567890ULL);

  // And SQL_C_SSHORT conversions should work
  CHECK(get_data<SQL_C_SSHORT>(stmt, 8) == 100);
  CHECK(get_data<SQL_C_SSHORT>(stmt, 9) == 200);
}

// ============================================================================
// SUCCESSFUL CONVERSIONS - String to Floating Point Types
// ============================================================================

TEST_CASE("should convert string literals to floating point types", "[datatype][string][conversion]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting string literals representing floating point numbers is executed
  auto stmt = conn.execute_fetch(
      "SELECT '123.456' AS c1, '-789.012' AS c2, '0.0' AS c3, "
      "'3.14159' AS c4, '1.5e10' AS c5, "
      "'123.456789012345' AS c6, '-1.7976931348623157e308' AS c7, "
      "'2.2250738585072014e-308' AS c8, "
      "'42' AS c9, '-100' AS c10, '  123.456  ' AS c11, '    -789.012  ' AS c12");

  // Then SQL_C_FLOAT conversions should work
  CHECK(get_data<SQL_C_FLOAT>(stmt, 1) == Catch::Approx(123.456f).epsilon(0.001f));
  CHECK(get_data<SQL_C_FLOAT>(stmt, 2) == Catch::Approx(-789.012f).epsilon(0.001f));
  CHECK(get_data<SQL_C_FLOAT>(stmt, 3) == Catch::Approx(0.0f));
  CHECK(get_data<SQL_C_FLOAT>(stmt, 4) == Catch::Approx(3.14159f).epsilon(0.00001f));
  CHECK(get_data<SQL_C_FLOAT>(stmt, 5) == Catch::Approx(1.5e10f).margin(1e6f));

  // And SQL_C_DOUBLE conversions should work
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 6) == Catch::Approx(123.456789012345).epsilon(1e-12));
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 7) == Catch::Approx(-1.7976931348623157e308));
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 8) == Catch::Approx(2.2250738585072014e-308));

  // And integer strings should convert to floating point
  CHECK(get_data<SQL_C_FLOAT>(stmt, 9) == Catch::Approx(42.0f));
  CHECK(get_data<SQL_C_FLOAT>(stmt, 10) == Catch::Approx(-100.0f));
  CHECK(get_data<SQL_C_FLOAT>(stmt, 11) == Catch::Approx(123.456f).epsilon(0.001f));
  CHECK(get_data<SQL_C_FLOAT>(stmt, 12) == Catch::Approx(-789.012f).epsilon(0.001f));
}

// ============================================================================
// SUCCESSFUL CONVERSIONS - String to BIT Type
// ============================================================================

TEST_CASE("should convert string literals to SQL_C_BIT", "[datatype][string][conversion]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting string literals representing boolean values is executed
  auto stmt = conn.execute_fetch("SELECT '1' AS true_val, '0' AS false_val, ' 1 ' AS c3, ' 0 ' AS c4");

  // Then the string values should be correctly converted to SQL_C_BIT
  CHECK(get_data<SQL_C_BIT>(stmt, 1) == 1);
  CHECK(get_data<SQL_C_BIT>(stmt, 2) == 0);
  CHECK(get_data<SQL_C_BIT>(stmt, 3) == 1);
  CHECK(get_data<SQL_C_BIT>(stmt, 4) == 0);
}

// ============================================================================
// SUCCESSFUL CONVERSIONS - String to Date/Time Types
// ============================================================================

TEST_CASE("should convert string literals to date and time types", "[datatype][string][conversion]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting string literals representing dates and times is executed
  auto stmt = conn.execute_fetch(
      "SELECT '2024-01-15' AS c1, '1999-12-31' AS c2, '2000-01-01' AS c3, "
      "'14:30:45' AS c4, '00:00:00' AS c5, '23:59:59' AS c6, "
      "'2024-01-15 14:30:45' AS c7, '1999-12-31 23:59:59' AS c8, '  2024-01-15 14:30:45  ' AS c9");

  // Then SQL_C_TYPE_DATE conversions should work
  auto date1 = get_data<SQL_C_TYPE_DATE>(stmt, 1);
  CHECK(date1.year == 2024);
  CHECK(date1.month == 1);
  CHECK(date1.day == 15);

  auto date2 = get_data<SQL_C_TYPE_DATE>(stmt, 2);
  CHECK(date2.year == 1999);
  CHECK(date2.month == 12);
  CHECK(date2.day == 31);

  auto y2k = get_data<SQL_C_TYPE_DATE>(stmt, 3);
  CHECK(y2k.year == 2000);
  CHECK(y2k.month == 1);
  CHECK(y2k.day == 1);

  // And SQL_C_TYPE_TIME conversions should work
  auto time1 = get_data<SQL_C_TYPE_TIME>(stmt, 4);
  CHECK(time1.hour == 14);
  CHECK(time1.minute == 30);
  CHECK(time1.second == 45);

  auto midnight = get_data<SQL_C_TYPE_TIME>(stmt, 5);
  CHECK(midnight.hour == 0);
  CHECK(midnight.minute == 0);
  CHECK(midnight.second == 0);

  auto end_of_day = get_data<SQL_C_TYPE_TIME>(stmt, 6);
  CHECK(end_of_day.hour == 23);
  CHECK(end_of_day.minute == 59);
  CHECK(end_of_day.second == 59);

  // And SQL_C_TYPE_TIMESTAMP conversions should work
  auto ts1 = get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 7);
  CHECK(ts1.year == 2024);
  CHECK(ts1.month == 1);
  CHECK(ts1.day == 15);
  CHECK(ts1.hour == 14);
  CHECK(ts1.minute == 30);
  CHECK(ts1.second == 45);

  auto millennium = get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 8);
  CHECK(millennium.year == 1999);
  CHECK(millennium.month == 12);
  CHECK(millennium.day == 31);
  CHECK(millennium.hour == 23);
  CHECK(millennium.minute == 59);
  CHECK(millennium.second == 59);

  auto ts2 = get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 9);
  CHECK(ts2.year == 2024);
  CHECK(ts2.month == 1);
  CHECK(ts2.day == 15);
  CHECK(ts2.hour == 14);
  CHECK(ts2.minute == 30);
  CHECK(ts2.second == 45);
}

// ============================================================================
// SUCCESSFUL CONVERSIONS - Strings with leading/trailing whitespace
// ============================================================================

TEST_CASE("should convert string literals with whitespace to numeric types", "[datatype][string][conversion]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting string literals with leading/trailing whitespace is executed
  auto stmt = conn.execute_fetch("SELECT '  123  ' AS padded, '   456' AS leading, '789   ' AS trailing");

  // Then the string values should be correctly converted, stripping whitespace
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 123);
  CHECK(get_data<SQL_C_LONG>(stmt, 2) == 456);
  CHECK(get_data<SQL_C_LONG>(stmt, 3) == 789);
}

// ============================================================================
// SUCCESSFUL CONVERSIONS - Decimal strings to integer (truncation)
// ============================================================================

/* TODO: This test is failing, but it should pass. */
TEST_CASE("should truncate decimal string literals when converting to integer types",
          "[.][datatype][string][conversion]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting string literals with decimal parts is executed
  auto stmt = conn.execute_fetch(
      "SELECT '123.999' AS round_down, '-456.001' AS neg_round, '0.9' AS less_than_one, "
      "'1.2345678901241242141241241e9' AS scientific_notation");

  // Then the string values should be truncated when converted to SQL_C_LONG
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 123);  // Truncated, not rounded
  CHECK(get_data<SQL_C_LONG>(stmt, 2) == -456);
  CHECK(get_data<SQL_C_LONG>(stmt, 3) == 0);
  CHECK(get_data<SQL_C_LONG>(stmt, 4) == 1234567890);
}

// ============================================================================
// FAILING CONVERSIONS - Non-numeric strings to numeric types
// ============================================================================

TEST_CASE("should fail converting non-numeric strings to numeric types", "[datatype][string][conversion][failure]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting various non-numeric strings is executed
  auto stmt = conn.execute_fetch(
      "SELECT 'hello' AS c1, 'not a number' AS c2, 'abc123' AS c3, "
      "'not_an_integer' AS c4, '' AS c5, '!@#$%' AS c6");

  // Test cases: column, type, description
  struct TestCase {
    int column;
    SQLSMALLINT type;
    const char* description;
  };

  std::vector<TestCase> test_cases = {
      {1, SQL_C_LONG, "text to SQL_C_LONG"},           {2, SQL_C_FLOAT, "text to SQL_C_FLOAT"},
      {3, SQL_C_DOUBLE, "mixed text to SQL_C_DOUBLE"}, {4, SQL_C_SBIGINT, "text to SQL_C_SBIGINT"},
      {5, SQL_C_LONG, "empty string to SQL_C_LONG"},   {6, SQL_C_LONG, "special chars to SQL_C_LONG"},
  };

  // Then all conversions should fail with SQL_ERROR and SQLSTATE 22018
  for (const auto& tc : test_cases) {
    INFO("Converting " << tc.description);

    SQLBIGINT value;
    SQLLEN indicator;
    SQLRETURN ret = get_data_raw(stmt, tc.column, tc.type, &value, &indicator);

    CHECK(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "22018");
  }
}

TEST_CASE("should fail converting Unicode string to numeric types", "[datatype][string][conversion][failure]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting Unicode string is executed
  auto stmt = conn.executew_fetch(u"SELECT '日本語' AS japanese");

  // And Attempt to get data as SQL_C_LONG
  SQLINTEGER value;
  SQLLEN indicator;
  SQLRETURN ret = get_data_raw(stmt, 1, SQL_C_LONG, &value, &indicator);

  // Then the conversion should fail with SQL_ERROR
  CHECK(ret == SQL_ERROR);

  // And the SQLSTATE should indicate invalid character value for cast (22018)
  CHECK(get_sqlstate(stmt) == "22018");
}

// ============================================================================
// FAILING CONVERSIONS - Overflow scenarios
// ============================================================================

TEST_CASE("should fail when string value overflows signed integer types",
          "[datatype][string][conversion][failure][overflow]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting string values that overflow various types is executed
  auto stmt = conn.execute_fetch("SELECT '200' AS c1, '50000' AS c2, '9999999999999' AS c3");

  // Then SQL_C_TINYINT should overflow (max 127)
  {
    INFO("TINYINT overflow");
    SQLSCHAR value;
    SQLLEN indicator;
    SQLRETURN ret = get_data_raw(stmt, 1, SQL_C_TINYINT, &value, &indicator);
    CHECK(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "22003");
  }

  // And SQL_C_SHORT should overflow (max 32767)
  {
    INFO("SHORT overflow");
    SQLSMALLINT value;
    SQLLEN indicator;
    SQLRETURN ret = get_data_raw(stmt, 2, SQL_C_SHORT, &value, &indicator);
    CHECK(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "22003");
  }

  // And SQL_C_LONG should overflow (max 2147483647)
  {
    INFO("LONG overflow");
    SQLINTEGER value;
    SQLLEN indicator;
    SQLRETURN ret = get_data_raw(stmt, 3, SQL_C_LONG, &value, &indicator);
    CHECK(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "22003");
  }
}

TEST_CASE("should fail when negative string value used with unsigned types",
          "[datatype][string][conversion][failure][overflow]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting negative string values is executed
  auto stmt = conn.execute_fetch("SELECT '-100' AS c1, '-1' AS c2, '-50' AS c3");

  // Then SQL_C_ULONG should fail
  {
    INFO("negative to ULONG");
    SQLUINTEGER value;
    SQLLEN indicator;
    SQLRETURN ret = get_data_raw(stmt, 1, SQL_C_ULONG, &value, &indicator);
    CHECK(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "22003");
  }

  // And SQL_C_UTINYINT should fail
  {
    INFO("negative to UTINYINT");
    SQLCHAR value;
    SQLLEN indicator;
    SQLRETURN ret = get_data_raw(stmt, 2, SQL_C_UTINYINT, &value, &indicator);
    CHECK(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "22003");
  }

  // And SQL_C_USHORT should fail
  {
    INFO("negative to USHORT");
    SQLUSMALLINT value;
    SQLLEN indicator;
    SQLRETURN ret = get_data_raw(stmt, 3, SQL_C_USHORT, &value, &indicator);
    CHECK(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "22003");
  }
}

// ============================================================================
// FAILING CONVERSIONS - Invalid date/time format strings
// ============================================================================

TEST_CASE("should fail converting invalid date/time strings", "[datatype][string][conversion][failure]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting invalid date/time strings is executed
  auto stmt = conn.execute_fetch("SELECT 'not-a-date' AS c1, 'not-a-time' AS c2, 'invalid-timestamp' AS c3");

  // Then SQL_C_TYPE_DATE should fail
  {
    INFO("invalid date string");
    SQL_DATE_STRUCT value;
    SQLLEN indicator;
    SQLRETURN ret = get_data_raw(stmt, 1, SQL_C_TYPE_DATE, &value, &indicator);
    CHECK(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "22018");
  }

  // And SQL_C_TYPE_TIME should fail
  {
    INFO("invalid time string");
    SQL_TIME_STRUCT value;
    SQLLEN indicator;
    SQLRETURN ret = get_data_raw(stmt, 2, SQL_C_TYPE_TIME, &value, &indicator);
    CHECK(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "22018");
  }

  // And SQL_C_TYPE_TIMESTAMP should fail
  {
    INFO("invalid timestamp string");
    SQL_TIMESTAMP_STRUCT value;
    SQLLEN indicator;
    SQLRETURN ret = get_data_raw(stmt, 3, SQL_C_TYPE_TIMESTAMP, &value, &indicator);
    CHECK(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "22018");
  }
}

// ============================================================================
// FAILING CONVERSIONS - Alternative date/time serialization formats
// ============================================================================

TEST_CASE("should fail converting alternative date formats to SQL_C_TYPE_DATE",
          "[datatype][string][conversion][failure][date_format]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting multiple date strings in alternative formats is executed
  auto stmt = conn.execute_fetch(
      "SELECT '01/15/2024' AS us_format, '15.01.2024' AS european_format, '2024/01/15' AS slash_format, 'January 15, "
      "2024' AS spelled_month, '15-Jan-2024' AS abbreviated_month, '15-01-2024' AS reversed_format, '24-01-15' AS "
      "two_digit_year, '2024-1-5' AS single_digit");

  // Test various invalid date formats that should all fail conversion
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

    // And Attempt to get data as SQL_C_TYPE_DATE
    SQL_DATE_STRUCT value;
    SQLLEN indicator;
    SQLRETURN ret = get_data_raw(stmt, column, SQL_C_TYPE_DATE, &value, &indicator);

    // Then the conversion should fail with SQL_ERROR
    CHECK(ret == SQL_ERROR);

    // And the SQLSTATE should indicate invalid character value for cast (22018)
    CHECK(get_sqlstate(stmt) == "22018");
  }
}

TEST_CASE("should fail converting alternative time formats to SQL_C_TYPE_TIME",
          "[datatype][string][conversion][failure][time_format]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting multiple time strings in alternative formats is executed
  auto stmt = conn.execute_fetch(
      "SELECT '2:30:45 PM' AS twelve_hour_format, '14:30' AS no_seconds, '14.30.45' AS dot_separator, '9:5:3' AS "
      "single_digit");

  // Test various invalid time formats that should all fail conversion
  std::vector<std::pair<int, std::string>> invalid_time_columns = {{1, "12-hour format with AM/PM"},
                                                                   {2, "time without seconds"},
                                                                   {3, "dot separator"},
                                                                   {4, "single-digit components"}};

  for (const auto& [column, description] : invalid_time_columns) {
    INFO("Converting " + description);

    // And Attempt to get data as SQL_C_TYPE_TIME
    SQL_TIME_STRUCT value;
    SQLLEN indicator;
    SQLRETURN ret = get_data_raw(stmt, column, SQL_C_TYPE_TIME, &value, &indicator);

    // Then the conversion should fail with SQL_ERROR
    CHECK(ret == SQL_ERROR);

    // And the SQLSTATE should indicate invalid character value for cast (22018)
    CHECK(get_sqlstate(stmt) == "22018");
  }
}

TEST_CASE("should fail converting alternative timestamp formats to SQL_C_TYPE_TIMESTAMP",
          "[datatype][string][conversion][failure][timestamp_format]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting multiple timestamp strings in alternative formats is executed
  auto stmt = conn.execute_fetch(
      "SELECT '2024-01-15T14:30:45' AS iso_t_separator, '2024-01-15 14:30:45+05:00' AS timezone_offset, "
      "'2024-01-15T14:30:45Z' AS utc_suffix, '01/15/2024 14:30:45' AS us_format");

  // Test various invalid timestamp formats that should all fail conversion
  std::vector<std::pair<int, std::string>> invalid_timestamp_columns = {
      {1, "ISO 8601 with T separator"}, {2, "timezone offset"}, {3, "Z (UTC) suffix"}, {4, "US format (MM/DD/YYYY)"}};

  for (const auto& [column, description] : invalid_timestamp_columns) {
    INFO("Converting " + description);

    // And Attempt to get data as SQL_C_TYPE_TIMESTAMP
    SQL_TIMESTAMP_STRUCT value;
    SQLLEN indicator;
    SQLRETURN ret = get_data_raw(stmt, column, SQL_C_TYPE_TIMESTAMP, &value, &indicator);

    // Then the conversion should fail with SQL_ERROR
    CHECK(ret == SQL_ERROR);

    // And the SQLSTATE should indicate invalid character value for cast (22018)
    CHECK(get_sqlstate(stmt) == "22018");
  }
}

TEST_CASE("should convert date-only and time-only strings to SQL_C_TYPE_TIMESTAMP",
          "[datatype][string][conversion][timestamp_format]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting date-only string is executed
  auto stmt = conn.execute_fetch("SELECT '2024-01-15' AS date_only, '14:30:45' AS time_only");

  // And Data is retrieved as SQL_C_TYPE_TIMESTAMP
  auto date_only = get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 1);
  auto time_only = get_data<SQL_C_TYPE_TIMESTAMP>(stmt, 2);

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
// EDGE CASES - Special floating point strings
// ============================================================================

TEST_CASE("should handle special floating point string conversions", "[datatype][string][conversion][edge]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting special float strings is executed
  auto stmt = conn.execute_fetch("SELECT 'inf' AS pos_inf, '-inf' AS neg_inf, 'NaN' AS nan_val");

  // Then inf conversion either succeeds with infinity or fails
  {
    SQLDOUBLE pos_value;
    SQLLEN pos_indicator;
    SQLRETURN pos_ret = get_data_raw(stmt, 1, SQL_C_DOUBLE, &pos_value, &pos_indicator);

    if (pos_ret == SQL_SUCCESS || pos_ret == SQL_SUCCESS_WITH_INFO) {
      CHECK(std::isinf(pos_value));
      CHECK(pos_value > 0);
    } else {
      CHECK(pos_ret == SQL_ERROR);
    }
  }

  // And -inf conversion either succeeds or fails
  {
    SQLDOUBLE neg_value;
    SQLLEN neg_indicator;
    SQLRETURN neg_ret = get_data_raw(stmt, 2, SQL_C_DOUBLE, &neg_value, &neg_indicator);

    if (neg_ret == SQL_SUCCESS || neg_ret == SQL_SUCCESS_WITH_INFO) {
      CHECK(std::isinf(neg_value));
      CHECK(neg_value < 0);
    } else {
      CHECK(neg_ret == SQL_ERROR);
    }
  }

  // And NaN conversion either succeeds with NaN or fails
  {
    SQLDOUBLE nan_value;
    SQLLEN nan_indicator;
    SQLRETURN nan_ret = get_data_raw(stmt, 3, SQL_C_DOUBLE, &nan_value, &nan_indicator);

    if (nan_ret == SQL_SUCCESS || nan_ret == SQL_SUCCESS_WITH_INFO) {
      CHECK(std::isnan(nan_value));
    } else {
      CHECK(nan_ret == SQL_ERROR);
    }
  }
}

// ============================================================================
// NULL VALUE HANDLING
// ============================================================================

TEST_CASE("should handle NULL string when converting to numeric and floating point types",
          "[datatype][string][conversion][null]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting NULL is executed
  auto stmt = conn.execute_fetch("SELECT NULL::STRING AS null_int, NULL::STRING AS null_double");

  // And Attempt to get data as SQL_C_LONG
  {
    SQLINTEGER value = 999;  // Initialize to non-zero to verify it's unchanged
    SQLLEN indicator;
    SQLRETURN ret = get_data_raw(stmt, 1, SQL_C_LONG, &value, &indicator);
    CHECK_ODBC(ret, stmt);
    CHECK(indicator == SQL_NULL_DATA);
  }

  // And Attempt to get data as SQL_C_DOUBLE
  {
    SQLDOUBLE value = 999.0;
    SQLLEN indicator;
    SQLRETURN ret = get_data_raw(stmt, 2, SQL_C_DOUBLE, &value, &indicator);
    CHECK_ODBC(ret, stmt);
    CHECK(indicator == SQL_NULL_DATA);
  }
}

// ============================================================================
// CONVERSION VIA TABLE - String column to numeric types
// ============================================================================

TEST_CASE("should convert string column values to numeric types", "[datatype][string][conversion]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // And A table with VARCHAR column containing numeric strings is created
  conn.execute("DROP TABLE IF EXISTS test_string_conv");
  conn.execute("CREATE TABLE test_string_conv (id INT, int_val VARCHAR(100), float_val VARCHAR(100))");
  conn.execute("INSERT INTO test_string_conv VALUES (1, '100', '3.14159')");
  conn.execute("INSERT INTO test_string_conv VALUES (2, '-200', '-2.71828')");
  conn.execute("INSERT INTO test_string_conv VALUES (3, '0', '0.0')");

  // When Query selecting from the table is executed
  auto stmt = conn.createStatement();
  SQLRETURN ret =
      SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT int_val, float_val FROM test_string_conv ORDER BY id", SQL_NTS);
  CHECK_ODBC(ret, stmt);

  // Then the string values should be correctly converted
  std::vector<SQLINTEGER> expected_ints = {100, -200, 0};
  std::vector<double> expected_floats = {3.14159, -2.71828, 0.0};
  int row = 0;
  while (true) {
    ret = SQLFetch(stmt.getHandle());
    if (ret == SQL_NO_DATA) break;
    CHECK_ODBC(ret, stmt);
    CHECK(get_data<SQL_C_LONG>(stmt, 1) == expected_ints[row]);
    CHECK(get_data<SQL_C_DOUBLE>(stmt, 2) == Catch::Approx(expected_floats[row]).epsilon(0.00001));
    row++;
  }
  CHECK(row == 3);
}

// ============================================================================
// CONVERSION WITH SQLBindCol
// ============================================================================

TEST_CASE("should convert strings using SQLBindCol", "[datatype][string][conversion]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // Test successful SQL_C_LONG binding
  {
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT '12345' AS str_num", SQL_NTS);
    CHECK_ODBC(ret, stmt);

    SQLINTEGER value;
    SQLLEN indicator;
    ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
    CHECK_ODBC(ret, stmt);

    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);

    CHECK(value == 12345);
    CHECK(indicator == sizeof(SQLINTEGER));
  }

  // Test successful SQL_C_DOUBLE binding
  {
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT '987.654' AS str_num", SQL_NTS);
    CHECK_ODBC(ret, stmt);

    SQLDOUBLE value;
    SQLLEN indicator;
    ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_DOUBLE, &value, sizeof(value), &indicator);
    CHECK_ODBC(ret, stmt);

    ret = SQLFetch(stmt.getHandle());
    CHECK_ODBC(ret, stmt);

    CHECK(value == Catch::Approx(987.654).epsilon(0.001));
    CHECK(indicator == sizeof(SQLDOUBLE));
  }

  // Test failed binding for invalid string
  {
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 'not_a_number' AS str_val", SQL_NTS);
    CHECK_ODBC(ret, stmt);

    SQLINTEGER value;
    SQLLEN indicator;
    ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, &value, sizeof(value), &indicator);
    CHECK_ODBC(ret, stmt);

    ret = SQLFetch(stmt.getHandle());
    CHECK(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "22018");
  }
}

// ============================================================================
// SUCCESSFUL CONVERSIONS - String to SQL_C_NUMERIC
// ============================================================================

// Helper to convert SQL_NUMERIC_STRUCT val array to integer for comparison
static long long numeric_val_to_int(const SQL_NUMERIC_STRUCT& num) {
  long long result = 0;
  long long multiplier = 1;
  for (int i = 0; i < SQL_MAX_NUMERIC_LEN; i++) {
    result += static_cast<long long>(num.val[i]) * multiplier;
    multiplier *= 256;
  }
  return result;
}

static unsigned int to_unsigned_int(char c) { return static_cast<unsigned int>(c); }

/* TODO: This test is failing, but it should pass. */
TEST_CASE("should convert string literals to SQL_C_NUMERIC", "[.][datatype][string][conversion][numeric]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting various numeric string formats is executed
  auto stmt = conn.execute_fetch(
      "SELECT '12345' AS c1, '-67890' AS c2, '0' AS c3, "
      "'123.456' AS c4, '  999  ' AS c5, '+42' AS c6, "
      "'00123' AS c7, '1.5e3' AS c8, '123456789012345678901234567890' AS c9, NULL::STRING AS c10");

  // Then positive integer '12345' should convert correctly
  {
    auto num = get_data<SQL_C_NUMERIC>(stmt, 1);
    CHECK(num.precision == 38);
    CHECK(num.scale == 0);
    CHECK(num.sign == 1);  // Positive
    CHECK(numeric_val_to_int(num) == 12345);
  }

  // And negative integer '-67890' should convert correctly
  {
    auto num = get_data<SQL_C_NUMERIC>(stmt, 2);
    CHECK(num.precision == 38);
    CHECK(num.scale == 0);
    CHECK(num.sign == 0);  // Negative
    CHECK(numeric_val_to_int(num) == 67890);
  }

  // And zero '0' should convert correctly
  {
    auto num = get_data<SQL_C_NUMERIC>(stmt, 3);
    CHECK(num.precision == 38);
    CHECK(num.scale == 0);
    CHECK(num.sign == 1);  // Zero is positive
    CHECK(numeric_val_to_int(num) == 0);
  }

  // And decimal '123.456' should convert correctly with appropriate scale
  {
    // NOTE: This is the behavior of the old ODBC driver
    auto num = get_data<SQL_C_NUMERIC>(stmt, 4);
    CHECK(num.precision == 38);
    CHECK(num.scale == 0);
    CHECK(num.sign == 1);  // Positive
    CHECK(numeric_val_to_int(num) == 123);
  }

  // And whitespace '  999  ' should be stripped
  {
    auto num = get_data<SQL_C_NUMERIC>(stmt, 5);
    CHECK(num.precision == 38);
    CHECK(num.scale == 0);
    CHECK(num.sign == 1);  // Positive
    CHECK(numeric_val_to_int(num) == 999);
  }

  // And explicit plus sign '+42' should be handled
  {
    auto num = get_data<SQL_C_NUMERIC>(stmt, 6);
    CHECK(num.precision == 38);
    CHECK(num.scale == 0);
    CHECK(num.sign == 1);  // Positive
    CHECK(numeric_val_to_int(num) == 42);
  }

  // And leading zeros '00123' should be handled
  {
    auto num = get_data<SQL_C_NUMERIC>(stmt, 7);
    CHECK(num.precision == 38);
    CHECK(num.scale == 0);
    CHECK(num.sign == 1);  // Positive
    CHECK(numeric_val_to_int(num) == 123);
  }

  // And scientific notation '1.5e3' should convert correctly (1.5e3 = 1500)
  {
    auto num = get_data<SQL_C_NUMERIC>(stmt, 8);
    CHECK(num.precision == 38);
    CHECK(num.scale == 0);
    CHECK(num.sign == 1);  // Positive
    CHECK(numeric_val_to_int(num) == 1500);
  }

  // And very large integer '123456789012345678901234567890' should convert correctly to 18EE90FF6C373E0EE4E3F0AD2
  {
    auto num = get_data<SQL_C_NUMERIC>(stmt, 9);
    CHECK(num.precision == 38);
    CHECK(num.scale == 0);
    CHECK(num.sign == 1);  // Positive
    CHECK(to_unsigned_int(num.val[0]) == 0xD2);
    CHECK(to_unsigned_int(num.val[1]) == 0x0A);
    CHECK(to_unsigned_int(num.val[2]) == 0x3F);
    CHECK(to_unsigned_int(num.val[3]) == 0x4E);
    CHECK(to_unsigned_int(num.val[4]) == 0xEE);
    CHECK(to_unsigned_int(num.val[5]) == 0xE0);
    CHECK(to_unsigned_int(num.val[6]) == 0x73);
    CHECK(to_unsigned_int(num.val[7]) == 0xC3);
    CHECK(to_unsigned_int(num.val[8]) == 0xF6);
    CHECK(to_unsigned_int(num.val[9]) == 0x0F);
    CHECK(to_unsigned_int(num.val[10]) == 0xE9);
    CHECK(to_unsigned_int(num.val[11]) == 0x8E);
    CHECK(to_unsigned_int(num.val[12]) == 0x01);
    CHECK(to_unsigned_int(num.val[13]) == 0x00);
    CHECK(to_unsigned_int(num.val[14]) == 0x00);
    CHECK(to_unsigned_int(num.val[15]) == 0x00);
  }

  // And NULL should return SQL_NULL_DATA indicator
  {
    SQL_NUMERIC_STRUCT num;
    SQLLEN indicator;
    SQLRETURN ret = get_data_raw(stmt, 10, SQL_C_NUMERIC, &num, &indicator);
    CHECK_ODBC(ret, stmt);
    CHECK(indicator == SQL_NULL_DATA);
  }
}

// ============================================================================
// FAILING CONVERSIONS - String to SQL_C_NUMERIC
// ============================================================================

TEST_CASE("should fail converting invalid strings to SQL_C_NUMERIC",
          "[datatype][string][conversion][numeric][failure]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting invalid numeric strings is executed
  auto stmt = conn.execute_fetch("SELECT 'hello' AS c1, '' AS c2, '123abc' AS c3, '123.456.789' AS c4");

  // Then text should fail with 22018
  {
    INFO("text to SQL_C_NUMERIC");
    SQL_NUMERIC_STRUCT value;
    SQLLEN indicator;
    SQLRETURN ret = get_data_raw(stmt, 1, SQL_C_NUMERIC, &value, &indicator);
    CHECK(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "22018");
  }

  // And empty string should fail
  {
    INFO("empty string to SQL_C_NUMERIC");
    SQL_NUMERIC_STRUCT value;
    SQLLEN indicator;
    SQLRETURN ret = get_data_raw(stmt, 2, SQL_C_NUMERIC, &value, &indicator);
    CHECK(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "22018");
  }

  // And trailing text should fail
  {
    INFO("trailing text");
    SQL_NUMERIC_STRUCT value;
    SQLLEN indicator;
    SQLRETURN ret = get_data_raw(stmt, 3, SQL_C_NUMERIC, &value, &indicator);
    CHECK(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "22018");
  }

  // And multiple decimal points should fail
  {
    INFO("multiple decimal points");
    SQL_NUMERIC_STRUCT value;
    SQLLEN indicator;
    SQLRETURN ret = get_data_raw(stmt, 4, SQL_C_NUMERIC, &value, &indicator);
    CHECK(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "22018");
  }
}

// ============================================================================
// SUCCESSFUL CONVERSIONS - String to SQL_C_BINARY
// ============================================================================

TEST_CASE("should convert string literals to SQL_C_BINARY", "[datatype][string][conversion][binary]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting various string literals is executed
  auto stmt = conn.execute_fetch("SELECT 'hello' AS c1, '' AS c2, 'ABC123!@#' AS c3, NULL::STRING AS c4");

  // Then ASCII string 'hello' should convert to raw bytes
  {
    SQLCHAR buffer[100];
    SQLLEN indicator;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
    CHECK_ODBC(ret, stmt);
    CHECK(indicator == 5);
    CHECK(buffer[0] == 'h');
    CHECK(buffer[1] == 'e');
    CHECK(buffer[2] == 'l');
    CHECK(buffer[3] == 'l');
    CHECK(buffer[4] == 'o');
  }

  // And empty string should return 0 bytes
  {
    SQLCHAR buffer[100];
    SQLLEN indicator;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 2, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
    CHECK_ODBC(ret, stmt);
    CHECK(indicator == 0);
  }

  // And mixed ASCII with special characters should convert correctly
  {
    SQLCHAR buffer[100];
    SQLLEN indicator;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 3, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
    CHECK_ODBC(ret, stmt);
    CHECK(indicator == 9);
    CHECK(buffer[0] == 'A');
    CHECK(buffer[1] == 'B');
    CHECK(buffer[2] == 'C');
    CHECK(buffer[3] == '1');
    CHECK(buffer[4] == '2');
    CHECK(buffer[5] == '3');
    CHECK(buffer[6] == '!');
    CHECK(buffer[7] == '@');
    CHECK(buffer[8] == '#');
  }

  // And NULL should return SQL_NULL_DATA
  {
    SQLCHAR buffer[100];
    SQLLEN indicator;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 4, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
    CHECK_ODBC(ret, stmt);
    CHECK(indicator == SQL_NULL_DATA);
  }
}

TEST_CASE("should convert UTF-8 string literals to SQL_C_BINARY", "[datatype][string][conversion][binary][utf8]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting UTF-8 string literals is executed
  auto stmt = conn.executew_fetch(
      u"SELECT '日本語' AS japanese, 'Привет' AS russian, '你好' AS chinese, "
      u"'émoji: 😀' AS emoji, 'café' AS french, 'Ñoño' AS spanish, '𝄞' AS clef");

  // Then Japanese '日本語' should convert to UTF-8 bytes (3 chars × 3 bytes each = 9 bytes)
  {
    SQLCHAR buffer[100];
    SQLLEN indicator;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
    CHECK_ODBC(ret, stmt);
    CHECK(indicator == 9);  // 3 Japanese characters × 3 bytes each
    // '日' = E6 97 A5
    CHECK(buffer[0] == 0xE6);
    CHECK(buffer[1] == 0x97);
    CHECK(buffer[2] == 0xA5);
    // '本' = E6 9C AC
    CHECK(buffer[3] == 0xE6);
    CHECK(buffer[4] == 0x9C);
    CHECK(buffer[5] == 0xAC);
    // '語' = E8 AA 9E
    CHECK(buffer[6] == 0xE8);
    CHECK(buffer[7] == 0xAA);
    CHECK(buffer[8] == 0x9E);
  }

  // And Russian 'Привет' should convert to UTF-8 bytes (6 chars × 2 bytes each = 12 bytes)
  {
    SQLCHAR buffer[100];
    SQLLEN indicator;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 2, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
    CHECK_ODBC(ret, stmt);
    CHECK(indicator == 12);  // 6 Cyrillic characters × 2 bytes each
    // 'П' = D0 9F
    CHECK(buffer[0] == 0xD0);
    CHECK(buffer[1] == 0x9F);
  }

  // And Chinese '你好' should convert to UTF-8 bytes (2 chars × 3 bytes each = 6 bytes)
  {
    SQLCHAR buffer[100];
    SQLLEN indicator;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 3, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
    CHECK_ODBC(ret, stmt);
    CHECK(indicator == 6);  // 2 Chinese characters × 3 bytes each
    // '你' = E4 BD A0
    CHECK(buffer[0] == 0xE4);
    CHECK(buffer[1] == 0xBD);
    CHECK(buffer[2] == 0xA0);
    // '好' = E5 A5 BD
    CHECK(buffer[3] == 0xE5);
    CHECK(buffer[4] == 0xA5);
    CHECK(buffer[5] == 0xBD);
  }

  // And emoji string 'émoji: 😀' should include 4-byte emoji
  {
    SQLCHAR buffer[100];
    SQLLEN indicator;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 4, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
    CHECK_ODBC(ret, stmt);
    // 'é' (2 bytes) + 'm' + 'o' + 'j' + 'i' + ':' + ' ' (6 bytes) + '😀' (4 bytes) = 12 bytes
    CHECK(indicator == 12);
    // 'é' = C3 A9
    CHECK(buffer[0] == 0xC3);
    CHECK(buffer[1] == 0xA9);
    // '😀' = F0 9F 98 80 (at end)
    CHECK(buffer[8] == 0xF0);
    CHECK(buffer[9] == 0x9F);
    CHECK(buffer[10] == 0x98);
    CHECK(buffer[11] == 0x80);
  }

  // And French 'café' should convert correctly (4 chars, 5 bytes due to 'é')
  {
    SQLCHAR buffer[100];
    SQLLEN indicator;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 5, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
    CHECK_ODBC(ret, stmt);
    CHECK(indicator == 5);  // 'c' + 'a' + 'f' + 'é' (2 bytes)
    CHECK(buffer[0] == 'c');
    CHECK(buffer[1] == 'a');
    CHECK(buffer[2] == 'f');
    // 'é' = C3 A9
    CHECK(buffer[3] == 0xC3);
    CHECK(buffer[4] == 0xA9);
  }

  // And Spanish 'Ñoño' should convert correctly
  {
    SQLCHAR buffer[100];
    SQLLEN indicator;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 6, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
    CHECK_ODBC(ret, stmt);
    CHECK(indicator == 6);  // 'Ñ' (2 bytes) + 'o' + 'ñ' (2 bytes) + 'o'
    // 'Ñ' = C3 91
    CHECK(buffer[0] == 0xC3);
    CHECK(buffer[1] == 0x91);
    CHECK(buffer[2] == 'o');
    // 'ñ' = C3 B1
    CHECK(buffer[3] == 0xC3);
    CHECK(buffer[4] == 0xB1);
    CHECK(buffer[5] == 'o');
  }

  // And musical symbol '𝄞' should convert correctly
  {
    SQLCHAR buffer[100];
    SQLLEN indicator;
    SQLRETURN ret = SQLGetData(stmt.getHandle(), 7, SQL_C_BINARY, buffer, sizeof(buffer), &indicator);
    CHECK_ODBC(ret, stmt);
    CHECK(indicator == 4);
    // UTF-8 encoding of '𝄞' is F0 9D 84 9E
    CHECK(to_unsigned_int(buffer[0]) == 0xF0);
    CHECK(to_unsigned_int(buffer[1]) == 0x9D);
    CHECK(to_unsigned_int(buffer[2]) == 0x84);
    CHECK(to_unsigned_int(buffer[3]) == 0x9E);
  }
}

// ============================================================================
// EDGE CASES - Numeric strings with special formatting
// ============================================================================

TEST_CASE("should handle edge case numeric string formats", "[datatype][string][conversion][edge]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting strings with special formatting is executed
  auto stmt = conn.execute_fetch(
      "SELECT '00123' AS c1, '007' AS c2, '+123' AS c3, '+456.789' AS c4, "
      "'0.00000001' AS c5, '1e-10' AS c6, '1.5E10' AS c7, '2.5E-5' AS c8");

  // Then leading zeros should be handled correctly
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 123);
  CHECK(get_data<SQL_C_LONG>(stmt, 2) == 7);

  // And explicit plus sign should be handled
  CHECK(get_data<SQL_C_LONG>(stmt, 3) == 123);
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 4) == Catch::Approx(456.789).epsilon(0.001));

  // And very small decimal values should convert
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 5) == Catch::Approx(0.00000001).epsilon(1e-12));
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 6) == Catch::Approx(1e-10).epsilon(1e-15));

  // And uppercase E in scientific notation should work
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 7) == Catch::Approx(1.5e10).margin(1e6));
  CHECK(get_data<SQL_C_DOUBLE>(stmt, 8) == Catch::Approx(2.5e-5).epsilon(1e-9));
}

// ============================================================================
// FAILING CONVERSIONS - Partial numeric strings
// ============================================================================

TEST_CASE("should fail converting partial or malformed numeric strings", "[datatype][string][conversion][failure]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting various malformed numeric strings is executed
  auto stmt = conn.execute_fetch("SELECT '123abc' AS c1, 'abc123' AS c2, '123.456.789' AS c3, '123,456' AS c4");

  // Then trailing text should fail for SQL_C_LONG
  {
    INFO("trailing text");
    SQLINTEGER value;
    SQLLEN indicator;
    SQLRETURN ret = get_data_raw(stmt, 1, SQL_C_LONG, &value, &indicator);
    CHECK(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "22018");
  }

  // And leading text should fail for SQL_C_LONG
  {
    INFO("leading text");
    SQLINTEGER value;
    SQLLEN indicator;
    SQLRETURN ret = get_data_raw(stmt, 2, SQL_C_LONG, &value, &indicator);
    CHECK(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "22018");
  }

  // And multiple decimal points should fail for SQL_C_DOUBLE
  {
    INFO("multiple decimal points");
    SQLDOUBLE value;
    SQLLEN indicator;
    SQLRETURN ret = get_data_raw(stmt, 3, SQL_C_DOUBLE, &value, &indicator);
    CHECK(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "22018");
  }

  // And comma as decimal separator should fail for SQL_C_DOUBLE
  {
    INFO("comma as decimal separator");
    SQLDOUBLE value;
    SQLLEN indicator;
    SQLRETURN ret = get_data_raw(stmt, 4, SQL_C_DOUBLE, &value, &indicator);
    CHECK(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "22018");
  }
}

// ============================================================================
// FAILING CONVERSIONS - BIT type edge cases
// ============================================================================

TEST_CASE("should fail converting invalid values to SQL_C_BIT", "[datatype][string][conversion][failure]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query selecting invalid BIT values is executed
  auto stmt = conn.execute_fetch("SELECT 'true' AS c1, '2' AS c2");

  // Then non-boolean text should fail with 22018
  {
    INFO("non-boolean text");
    SQLCHAR value;
    SQLLEN indicator;
    SQLRETURN ret = get_data_raw(stmt, 1, SQL_C_BIT, &value, &indicator);
    CHECK(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "22018");
  }

  // And value > 1 should fail with 22003
  {
    INFO("value greater than 1");
    SQLCHAR value;
    SQLLEN indicator;
    SQLRETURN ret = get_data_raw(stmt, 2, SQL_C_BIT, &value, &indicator);
    // CHECK_ODBC(ret, stmt);
    CHECK(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "22003");
  }
}
