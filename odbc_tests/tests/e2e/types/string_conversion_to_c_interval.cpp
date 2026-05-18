// VARCHAR / STRING -> SQL_C_INTERVAL_* fetch tests (SQLGetData / SQLBindCol).
//
// Snowflake holds interval values as VARCHAR ANSI-literal text; when fetched
// as a SQL_C_INTERVAL_* C type the driver parses the literal per ODBC
// Appendix D ("Converting Data from SQL to C Data Types" - "Character to
// Interval") and writes a SQL_INTERVAL_STRUCT. Appendix D maps four cases to
// return codes:
//
//   1. Valid value, no truncation                 -> SQL_SUCCESS
//   2. Valid value, truncation of trailing fields -> SQL_SUCCESS_WITH_INFO, 01S07
//   3. Valid value, leading-field precision lost  -> SQL_ERROR, 22015
//   4. Not a valid interval value                 -> SQL_ERROR, 22018
//
// The reference (old) driver already implements most of Appendix D for the
// VARCHAR -> SQL_C_INTERVAL_* path, so the majority of these tests are driver
// agnostic. A handful of cases diverge between the two drivers - those keep
// explicit OLD_DRIVER_ONLY / NEW_DRIVER_ONLY branches and are documented
// against BD#55.

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "HandleWrapper.hpp"
#include "SchemaFixtures.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "odbc_matchers.hpp"
#include "test_setup.hpp"

namespace {

constexpr const char* kBd55VarcharIntervalFetch = "BD#55";

}  // namespace

TEST_CASE_METHOD(ConnSchemaFixture, "should fetch VARCHAR as single-field SQL_C_INTERVAL_*",
                 "[datatype][string][conversion][interval]") {
  // Given Snowflake client is logged in
  // When A VARCHAR row carrying bare integer values for each interval field is fetched
  auto stmt = conn.execute_fetch(
      "SELECT '5' AS years, '10' AS months, '15' AS days, "
      "'8' AS hours, '30' AS minutes, '45' AS seconds");

  // Then SQL_C_INTERVAL_YEAR reads year = 5
  {
    INFO("SQL_C_INTERVAL_YEAR");
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.year_month.year == 5);
  }
  // And SQL_C_INTERVAL_MONTH reads month = 10
  {
    INFO("SQL_C_INTERVAL_MONTH");
    auto interval = check_no_truncation<SQL_C_INTERVAL_MONTH>(stmt, 2);
    CHECK(interval.interval_type == SQL_IS_MONTH);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.year_month.month == 10);
  }
  // And SQL_C_INTERVAL_DAY reads day = 15
  {
    INFO("SQL_C_INTERVAL_DAY");
    auto interval = check_no_truncation<SQL_C_INTERVAL_DAY>(stmt, 3);
    CHECK(interval.interval_type == SQL_IS_DAY);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.day == 15);
  }
  // And SQL_C_INTERVAL_HOUR reads hour = 8
  {
    INFO("SQL_C_INTERVAL_HOUR");
    auto interval = check_no_truncation<SQL_C_INTERVAL_HOUR>(stmt, 4);
    CHECK(interval.interval_type == SQL_IS_HOUR);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.hour == 8);
  }
  // And SQL_C_INTERVAL_MINUTE reads minute = 30
  {
    INFO("SQL_C_INTERVAL_MINUTE");
    auto interval = check_no_truncation<SQL_C_INTERVAL_MINUTE>(stmt, 5);
    CHECK(interval.interval_type == SQL_IS_MINUTE);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.minute == 30);
  }
  // And SQL_C_INTERVAL_SECOND reads second = 45 with fraction = 0
  {
    INFO("SQL_C_INTERVAL_SECOND");
    auto interval = check_no_truncation<SQL_C_INTERVAL_SECOND>(stmt, 6);
    CHECK(interval.interval_type == SQL_IS_SECOND);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.second == 45);
    CHECK(interval.intval.day_second.fraction == 0);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should preserve negative sign across SQL_C_INTERVAL_* targets",
                 "[datatype][string][conversion][interval]") {
  // Given Snowflake client is logged in
  // When A VARCHAR row carrying negative interval literals is fetched
  auto stmt = conn.execute_fetch(
      "SELECT '-5' AS neg_years, '-10' AS neg_months, '-15' AS neg_days, "
      "'-3-6' AS neg_year_month, '-5 10' AS neg_day_hour");

  // Then SQL_C_INTERVAL_YEAR has interval_sign = SQL_TRUE and year = 5
  {
    INFO("SQL_C_INTERVAL_YEAR (negative)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(stmt, 1);
    CHECK(interval.interval_sign == SQL_TRUE);
    CHECK(interval.intval.year_month.year == 5);
  }
  // And SQL_C_INTERVAL_MONTH has interval_sign = SQL_TRUE and month = 10
  {
    INFO("SQL_C_INTERVAL_MONTH (negative)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_MONTH>(stmt, 2);
    CHECK(interval.interval_sign == SQL_TRUE);
    CHECK(interval.intval.year_month.month == 10);
  }
  // And SQL_C_INTERVAL_DAY has interval_sign = SQL_TRUE and day = 15
  {
    INFO("SQL_C_INTERVAL_DAY (negative)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_DAY>(stmt, 3);
    CHECK(interval.interval_sign == SQL_TRUE);
    CHECK(interval.intval.day_second.day == 15);
  }
  // And SQL_C_INTERVAL_YEAR_TO_MONTH has interval_sign = SQL_TRUE, year = 3, month = 6
  {
    INFO("SQL_C_INTERVAL_YEAR_TO_MONTH (negative)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 4);
    CHECK(interval.interval_sign == SQL_TRUE);
    CHECK(interval.intval.year_month.year == 3);
    CHECK(interval.intval.year_month.month == 6);
  }
  // And SQL_C_INTERVAL_DAY_TO_HOUR has interval_sign = SQL_TRUE, day = 5, hour = 10
  {
    INFO("SQL_C_INTERVAL_DAY_TO_HOUR (negative)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_DAY_TO_HOUR>(stmt, 5);
    CHECK(interval.interval_sign == SQL_TRUE);
    CHECK(interval.intval.day_second.day == 5);
    CHECK(interval.intval.day_second.hour == 10);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should fetch zero VARCHAR as SQL_C_INTERVAL_* with unset sign",
                 "[datatype][string][conversion][interval]") {
  // Given Snowflake client is logged in
  // When A VARCHAR row carrying zero interval values is fetched
  auto stmt = conn.execute_fetch(
      "SELECT '0' AS zero_year, '0-0' AS zero_year_month, "
      "'-0' AS neg_zero_year, '-0-0' AS neg_zero_year_month, "
      "'0 00:00:00' AS zero_day_second");

  // Then SQL_C_INTERVAL_YEAR has year = 0 and interval_sign = SQL_FALSE
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(stmt, 1);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.year_month.year == 0);
  }
  // And SQL_C_INTERVAL_YEAR_TO_MONTH has both fields zero and interval_sign = SQL_FALSE
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 2);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.year_month.year == 0);
    CHECK(interval.intval.year_month.month == 0);
  }
  // And '-0' fetched as YEAR has year = 0; the sign bit follows BD#55 below
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(stmt, 3);
    // BD#55 (zero-magnitude sign): the reference driver preserves the
    // literal minus and reports interval_sign = SQL_TRUE; the universal
    // driver clears it because the magnitude is zero.
    OLD_DRIVER_ONLY(kBd55VarcharIntervalFetch) { CHECK(interval.interval_sign == SQL_TRUE); }
    NEW_DRIVER_ONLY(kBd55VarcharIntervalFetch) { CHECK(interval.interval_sign == SQL_FALSE); }
    CHECK(interval.intval.year_month.year == 0);
  }
  // And '-0-0' fetched as YEAR_TO_MONTH has both fields zero; the sign bit follows BD#55
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 4);
    OLD_DRIVER_ONLY(kBd55VarcharIntervalFetch) { CHECK(interval.interval_sign == SQL_TRUE); }
    NEW_DRIVER_ONLY(kBd55VarcharIntervalFetch) { CHECK(interval.interval_sign == SQL_FALSE); }
    CHECK(interval.intval.year_month.year == 0);
    CHECK(interval.intval.year_month.month == 0);
  }
  // And '0 00:00:00' fetched as DAY_TO_SECOND has all fields zero
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_DAY_TO_SECOND>(stmt, 5);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.day == 0);
    CHECK(interval.intval.day_second.hour == 0);
    CHECK(interval.intval.day_second.minute == 0);
    CHECK(interval.intval.day_second.second == 0);
    CHECK(interval.intval.day_second.fraction == 0);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should fetch VARCHAR as composite SQL_C_INTERVAL_YEAR_TO_MONTH",
                 "[datatype][string][conversion][interval]") {
  // Given Snowflake client is logged in
  // When A VARCHAR row carrying year-month interval literals is fetched
  auto stmt =
      conn.execute_fetch("SELECT '3-6' AS y_m, '0-11' AS zero_year_eleven_month, '12-0' AS twelve_year_zero_month");

  // Then '3-6' produces year = 3, month = 6
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_YEAR_TO_MONTH);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.year_month.year == 3);
    CHECK(interval.intval.year_month.month == 6);
  }
  // And '0-11' produces year = 0, month = 11
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 2);
    CHECK(interval.intval.year_month.year == 0);
    CHECK(interval.intval.year_month.month == 11);
  }
  // And '12-0' produces year = 12, month = 0
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 3);
    CHECK(interval.intval.year_month.year == 12);
    CHECK(interval.intval.year_month.month == 0);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should fetch VARCHAR as composite day-time SQL_C_INTERVAL_*",
                 "[datatype][string][conversion][interval]") {
  // Given Snowflake client is logged in
  // When A VARCHAR row carrying day-time interval literals is fetched
  auto stmt = conn.execute_fetch(
      "SELECT '5 10' AS d_h, '3 14:30' AS d_m, "
      "'2 08:15:30' AS d_s, '10:45' AS h_m, "
      "'12:30:45' AS h_s, '45:30' AS m_s");

  // Then SQL_C_INTERVAL_DAY_TO_HOUR populates day and hour
  {
    INFO("SQL_C_INTERVAL_DAY_TO_HOUR");
    auto interval = check_no_truncation<SQL_C_INTERVAL_DAY_TO_HOUR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_DAY_TO_HOUR);
    CHECK(interval.intval.day_second.day == 5);
    CHECK(interval.intval.day_second.hour == 10);
  }
  // And SQL_C_INTERVAL_DAY_TO_MINUTE populates day, hour, minute
  {
    INFO("SQL_C_INTERVAL_DAY_TO_MINUTE");
    auto interval = check_no_truncation<SQL_C_INTERVAL_DAY_TO_MINUTE>(stmt, 2);
    CHECK(interval.interval_type == SQL_IS_DAY_TO_MINUTE);
    CHECK(interval.intval.day_second.day == 3);
    CHECK(interval.intval.day_second.hour == 14);
    CHECK(interval.intval.day_second.minute == 30);
  }
  // And SQL_C_INTERVAL_DAY_TO_SECOND populates day, hour, minute, second
  {
    INFO("SQL_C_INTERVAL_DAY_TO_SECOND");
    auto interval = check_no_truncation<SQL_C_INTERVAL_DAY_TO_SECOND>(stmt, 3);
    CHECK(interval.interval_type == SQL_IS_DAY_TO_SECOND);
    CHECK(interval.intval.day_second.day == 2);
    CHECK(interval.intval.day_second.hour == 8);
    CHECK(interval.intval.day_second.minute == 15);
    CHECK(interval.intval.day_second.second == 30);
    CHECK(interval.intval.day_second.fraction == 0);
  }
  // And SQL_C_INTERVAL_HOUR_TO_MINUTE populates hour and minute
  {
    INFO("SQL_C_INTERVAL_HOUR_TO_MINUTE");
    auto interval = check_no_truncation<SQL_C_INTERVAL_HOUR_TO_MINUTE>(stmt, 4);
    CHECK(interval.interval_type == SQL_IS_HOUR_TO_MINUTE);
    CHECK(interval.intval.day_second.hour == 10);
    CHECK(interval.intval.day_second.minute == 45);
  }
  // And SQL_C_INTERVAL_HOUR_TO_SECOND populates hour, minute, second
  {
    INFO("SQL_C_INTERVAL_HOUR_TO_SECOND");
    auto interval = check_no_truncation<SQL_C_INTERVAL_HOUR_TO_SECOND>(stmt, 5);
    CHECK(interval.interval_type == SQL_IS_HOUR_TO_SECOND);
    CHECK(interval.intval.day_second.hour == 12);
    CHECK(interval.intval.day_second.minute == 30);
    CHECK(interval.intval.day_second.second == 45);
  }
  // And SQL_C_INTERVAL_MINUTE_TO_SECOND populates minute and second
  {
    INFO("SQL_C_INTERVAL_MINUTE_TO_SECOND");
    auto interval = check_no_truncation<SQL_C_INTERVAL_MINUTE_TO_SECOND>(stmt, 6);
    CHECK(interval.interval_type == SQL_IS_MINUTE_TO_SECOND);
    CHECK(interval.intval.day_second.minute == 45);
    CHECK(interval.intval.day_second.second == 30);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should fetch VARCHAR with fractional seconds as SQL_C_INTERVAL_*",
                 "[datatype][string][conversion][interval][fractional]") {
  // Given Snowflake client is logged in
  // When A VARCHAR row carrying fractional-second interval literals is fetched
  auto stmt = conn.execute_fetch(
      "SELECT '12.500000' AS sec_frac, '45:30.125' AS m_s_frac, "
      "'12:30:45.999' AS h_s_frac, '2 08:15:30.500' AS d_s_frac");

  // Then SQL_C_INTERVAL_SECOND parses '12.500000' as second = 12, fraction = 500000 microseconds
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_SECOND>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_SECOND);
    CHECK(interval.intval.day_second.second == 12);
    CHECK(interval.intval.day_second.fraction == 500000);
  }
  // And SQL_C_INTERVAL_MINUTE_TO_SECOND parses '45:30.125' with fraction = 125000 microseconds
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_MINUTE_TO_SECOND>(stmt, 2);
    CHECK(interval.intval.day_second.minute == 45);
    CHECK(interval.intval.day_second.second == 30);
    CHECK(interval.intval.day_second.fraction == 125000);
  }
  // And SQL_C_INTERVAL_HOUR_TO_SECOND parses '12:30:45.999' with fraction = 999000 microseconds
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_HOUR_TO_SECOND>(stmt, 3);
    CHECK(interval.intval.day_second.hour == 12);
    CHECK(interval.intval.day_second.minute == 30);
    CHECK(interval.intval.day_second.second == 45);
    CHECK(interval.intval.day_second.fraction == 999000);
  }
  // And SQL_C_INTERVAL_DAY_TO_SECOND parses '2 08:15:30.500' with fraction = 500000 microseconds
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_DAY_TO_SECOND>(stmt, 4);
    CHECK(interval.intval.day_second.day == 2);
    CHECK(interval.intval.day_second.hour == 8);
    CHECK(interval.intval.day_second.minute == 15);
    CHECK(interval.intval.day_second.second == 30);
    CHECK(interval.intval.day_second.fraction == 500000);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should trim whitespace in VARCHAR -> SQL_C_INTERVAL_*",
                 "[datatype][string][conversion][interval][edge]") {
  // Given Snowflake client is logged in
  // When A VARCHAR row carrying interval literals padded with whitespace is fetched
  auto stmt = conn.execute_fetch("SELECT '  5  ' AS pad_year, '  3-6  ' AS pad_year_month");

  // Then leading/trailing whitespace is ignored and SQL_C_INTERVAL_YEAR parses year = 5
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(stmt, 1);
    CHECK(interval.intval.year_month.year == 5);
  }
  // And SQL_C_INTERVAL_YEAR_TO_MONTH parses year = 3, month = 6
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 2);
    CHECK(interval.intval.year_month.year == 3);
    CHECK(interval.intval.year_month.month == 6);
  }
}

// ============================================================================
// Appendix D: trailing fields truncated (SQL_SUCCESS_WITH_INFO, SQLSTATE 01S07).
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should truncate trailing fields with SQLSTATE 01S07",
                 "[datatype][string][conversion][interval][truncation]") {
  // Given Snowflake client is logged in
  // When A VARCHAR row carrying literals wider than the target qualifier is fetched
  auto stmt = conn.execute_fetch(
      "SELECT '3-6' AS y_m_to_year, '5 10:30:45' AS d_s_to_day, "
      "'12:30:45' AS h_s_to_hour");

  // Then '3-6' fetched as SQL_C_INTERVAL_YEAR keeps year = 3 and warns 01S07 for the dropped month
  {
    auto interval = check_interval_trailing_truncation<SQL_C_INTERVAL_YEAR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.intval.year_month.year == 3);
  }
  // And '5 10:30:45' fetched as SQL_C_INTERVAL_DAY keeps day = 5 and warns 01S07
  {
    auto interval = check_interval_trailing_truncation<SQL_C_INTERVAL_DAY>(stmt, 2);
    CHECK(interval.interval_type == SQL_IS_DAY);
    CHECK(interval.intval.day_second.day == 5);
  }
  // And '12:30:45' fetched as SQL_C_INTERVAL_HOUR keeps hour = 12 and warns 01S07
  {
    auto interval = check_interval_trailing_truncation<SQL_C_INTERVAL_HOUR>(stmt, 3);
    CHECK(interval.interval_type == SQL_IS_HOUR);
    CHECK(interval.intval.day_second.hour == 12);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should truncate trailing fields in compound day-time intervals",
                 "[datatype][string][conversion][interval][truncation]") {
  // Given Snowflake client is logged in
  // When A VARCHAR row carrying broader day-time literals than the target qualifier is fetched
  auto stmt = conn.execute_fetch(
      "SELECT '2 08:15:30' AS d_s_to_d_h, '2 08:15:30' AS d_s_to_d_m, "
      "'12:30:45' AS h_s_to_h_m");

  // Then '2 08:15:30' fetched as SQL_C_INTERVAL_DAY_TO_HOUR keeps day = 2, hour = 8 with 01S07
  {
    auto interval = check_interval_trailing_truncation<SQL_C_INTERVAL_DAY_TO_HOUR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_DAY_TO_HOUR);
    CHECK(interval.intval.day_second.day == 2);
    CHECK(interval.intval.day_second.hour == 8);
  }
  // And '2 08:15:30' fetched as SQL_C_INTERVAL_DAY_TO_MINUTE keeps day, hour, minute with 01S07
  {
    auto interval = check_interval_trailing_truncation<SQL_C_INTERVAL_DAY_TO_MINUTE>(stmt, 2);
    CHECK(interval.interval_type == SQL_IS_DAY_TO_MINUTE);
    CHECK(interval.intval.day_second.day == 2);
    CHECK(interval.intval.day_second.hour == 8);
    CHECK(interval.intval.day_second.minute == 15);
  }
  // And '12:30:45' fetched as SQL_C_INTERVAL_HOUR_TO_MINUTE keeps hour, minute with 01S07
  {
    auto interval = check_interval_trailing_truncation<SQL_C_INTERVAL_HOUR_TO_MINUTE>(stmt, 3);
    CHECK(interval.interval_type == SQL_IS_HOUR_TO_MINUTE);
    CHECK(interval.intval.day_second.hour == 12);
    CHECK(interval.intval.day_second.minute == 30);
  }
}

// BD#55 (fractional digits dropped): the reference driver treats any non-zero
// fraction targeted at an integer-only qualifier as an invalid literal (22018);
// the universal driver follows Appendix D and surfaces the truncated value
// with SQLSTATE 01S07.
TEST_CASE_METHOD(ConnSchemaFixture, "should warn 01S07 when fractional digits are dropped",
                 "[datatype][string][conversion][interval][truncation][fractional]") {
  // Given Snowflake client is logged in
  // When A VARCHAR row carrying fractional literals targeted at integer-only qualifiers is fetched
  auto stmt = conn.execute_fetch("SELECT '5.5' AS half_year");

  OLD_DRIVER_ONLY(kBd55VarcharIntervalFetch) {
    // Reference driver rejects '5.5' -> INTERVAL_YEAR as an invalid literal.
    check_invalid_string<SQL_C_INTERVAL_YEAR>(stmt, 1);
    return;
  }

  NEW_DRIVER_ONLY(kBd55VarcharIntervalFetch) {
    // Then SQL_C_INTERVAL_YEAR keeps year = 5 and warns 01S07 for the dropped fraction
    auto interval = check_interval_trailing_truncation<SQL_C_INTERVAL_YEAR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.intval.year_month.year == 5);
  }
}

// BD#55 (zero-magnitude fraction): the reference driver rejects '5.0' as
// invalid (22018); the universal driver treats the zero fraction as carrying
// no information and returns SQL_SUCCESS.
TEST_CASE_METHOD(ConnSchemaFixture, "should not warn when fractional component is exactly zero",
                 "[datatype][string][conversion][interval][truncation][fractional]") {
  // Regression test for the audit fix: a literal like '5.0' carries a
  // syntactic fraction but its magnitude is zero, so no information is lost
  // when fetched as an integer-only qualifier. The universal driver returns
  // SQL_SUCCESS (not SQL_SUCCESS_WITH_INFO + 01S07).

  // Given Snowflake client is logged in
  // When A VARCHAR row carrying a zero-magnitude fraction is fetched as SQL_C_INTERVAL_YEAR
  auto stmt = conn.execute_fetch("SELECT '5.0' AS year_dot_zero");

  OLD_DRIVER_ONLY(kBd55VarcharIntervalFetch) {
    // Reference driver rejects '5.0' as an invalid literal regardless of magnitude.
    check_invalid_string<SQL_C_INTERVAL_YEAR>(stmt, 1);
    return;
  }

  NEW_DRIVER_ONLY(kBd55VarcharIntervalFetch) {
    // Then SQL_C_INTERVAL_YEAR returns year = 5 with no truncation warning
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.intval.year_month.year == 5);
  }
}

// ============================================================================
// Appendix D: leading field precision exceeded (SQL_ERROR, SQLSTATE 22015).
//
// The default `SQL_DESC_DATETIME_INTERVAL_PRECISION` is 2, so any leading-field
// magnitude >= 100 overflows. We exercise that default here.
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should fail with 22015 when leading-field precision is exceeded",
                 "[datatype][string][conversion][interval][precision]") {
  // Given Snowflake client is logged in
  // When A VARCHAR row carrying leading-field magnitudes wider than precision = 2 is fetched
  auto stmt = conn.execute_fetch(
      "SELECT '10000' AS big_year, '10000' AS big_month, '10000' AS big_day, "
      "'10000' AS big_hour, '10000' AS big_second");

  // Then SQL_C_INTERVAL_YEAR returns SQL_ERROR with SQLSTATE 22015
  check_interval_precision_lost<SQL_C_INTERVAL_YEAR>(stmt, 1);
  // And SQL_C_INTERVAL_MONTH returns SQL_ERROR with SQLSTATE 22015
  check_interval_precision_lost<SQL_C_INTERVAL_MONTH>(stmt, 2);
  // And SQL_C_INTERVAL_DAY returns SQL_ERROR with SQLSTATE 22015
  check_interval_precision_lost<SQL_C_INTERVAL_DAY>(stmt, 3);
  // And SQL_C_INTERVAL_HOUR returns SQL_ERROR with SQLSTATE 22015
  check_interval_precision_lost<SQL_C_INTERVAL_HOUR>(stmt, 4);
  // And SQL_C_INTERVAL_SECOND returns SQL_ERROR with SQLSTATE 22015
  check_interval_precision_lost<SQL_C_INTERVAL_SECOND>(stmt, 5);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should fail with 22015 when composite leading field exceeds precision",
                 "[datatype][string][conversion][interval][precision]") {
  // Given Snowflake client is logged in
  // When A VARCHAR row carrying composite literals with overflowed leading fields is fetched
  auto stmt = conn.execute_fetch(
      "SELECT '10000-6' AS big_y_m, '10000 10:30:45' AS big_d_s, "
      "'10000:30' AS big_h_m");

  // Then SQL_C_INTERVAL_YEAR_TO_MONTH returns SQL_ERROR with SQLSTATE 22015
  check_interval_precision_lost<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 1);
  // And SQL_C_INTERVAL_DAY_TO_SECOND returns SQL_ERROR with SQLSTATE 22015
  check_interval_precision_lost<SQL_C_INTERVAL_DAY_TO_SECOND>(stmt, 2);
  // And SQL_C_INTERVAL_HOUR_TO_MINUTE returns SQL_ERROR with SQLSTATE 22015
  check_interval_precision_lost<SQL_C_INTERVAL_HOUR_TO_MINUTE>(stmt, 3);
}

// BD#55 (SQL_DESC_DATETIME_INTERVAL_PRECISION override): the reference driver
// ignores the ARD precision attribute and always applies its built-in default;
// the universal driver honours the override and shifts the 22015 boundary
// accordingly.
TEST_CASE_METHOD(ConnSchemaFixture, "should respect SQL_DESC_DATETIME_INTERVAL_PRECISION override on the ARD",
                 "[datatype][string][conversion][interval][precision][descriptor]") {
  // The default leading precision is 2 (so values >= 100 overflow). Setting
  // SQL_DESC_DATETIME_INTERVAL_PRECISION on the ARD must be honoured by the
  // VARCHAR -> SQL_C_INTERVAL_* parser; we exercise both an enlargement
  // (precision 5 admits values up to 99_999) and a tightening (precision 1
  // rejects 10). The reference driver does not honour the override.

  // Given Snowflake client is logged in
  // When SQL_DESC_DATETIME_INTERVAL_PRECISION is set to 5 on the ARD
  {
    auto stmt = conn.execute_fetch("SELECT '99999' AS big_year");
    SQLHDESC ard = SQL_NULL_HDESC;
    SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetDescField(ard, 1, SQL_DESC_DATETIME_INTERVAL_PRECISION, (SQLPOINTER)5, 0);
    REQUIRE(ret == SQL_SUCCESS);

    OLD_DRIVER_ONLY(kBd55VarcharIntervalFetch) {
      // Reference driver ignores precision = 5 and still rejects 99999 against the default precision = 2.
      check_interval_precision_lost<SQL_C_INTERVAL_YEAR>(stmt, 1);
    }
    NEW_DRIVER_ONLY(kBd55VarcharIntervalFetch) {
      // Then Precision 5 admits value 99999 for SQL_C_INTERVAL_YEAR
      auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(stmt, 1);
      CHECK(interval.interval_type == SQL_IS_YEAR);
      CHECK(interval.intval.year_month.year == 99999);
    }
  }

  // And Precision 5 still rejects value 100000 for SQL_C_INTERVAL_YEAR
  {
    auto stmt = conn.execute_fetch("SELECT '100000' AS just_over");
    SQLHDESC ard = SQL_NULL_HDESC;
    SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetDescField(ard, 1, SQL_DESC_DATETIME_INTERVAL_PRECISION, (SQLPOINTER)5, 0);
    REQUIRE(ret == SQL_SUCCESS);

    // Both drivers reject 100000 - the universal driver because precision = 5
    // overflows; the reference driver because it ignores the override and the
    // default precision = 2 already overflows.
    check_interval_precision_lost<SQL_C_INTERVAL_YEAR>(stmt, 1);
  }

  // And Precision 1 admits value 9 for SQL_C_INTERVAL_HOUR
  {
    auto stmt = conn.execute_fetch("SELECT '9' AS small_hour");
    SQLHDESC ard = SQL_NULL_HDESC;
    SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetDescField(ard, 1, SQL_DESC_DATETIME_INTERVAL_PRECISION, (SQLPOINTER)1, 0);
    REQUIRE(ret == SQL_SUCCESS);

    // Both drivers accept 9 - precision = 1 is enough for the universal
    // driver, and the reference driver ignores the override but the value
    // already fits inside the default precision = 2.
    auto interval = check_no_truncation<SQL_C_INTERVAL_HOUR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_HOUR);
    CHECK(interval.intval.day_second.hour == 9);
  }

  // And Precision 1 rejects value 10 for SQL_C_INTERVAL_HOUR
  {
    auto stmt = conn.execute_fetch("SELECT '10' AS too_big");
    SQLHDESC ard = SQL_NULL_HDESC;
    SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetDescField(ard, 1, SQL_DESC_DATETIME_INTERVAL_PRECISION, (SQLPOINTER)1, 0);
    REQUIRE(ret == SQL_SUCCESS);

    OLD_DRIVER_ONLY(kBd55VarcharIntervalFetch) {
      // Reference driver ignores precision = 1 and accepts 10 under default precision = 2.
      auto interval = check_no_truncation<SQL_C_INTERVAL_HOUR>(stmt, 1);
      CHECK(interval.interval_type == SQL_IS_HOUR);
      CHECK(interval.intval.day_second.hour == 10);
    }
    NEW_DRIVER_ONLY(kBd55VarcharIntervalFetch) { check_interval_precision_lost<SQL_C_INTERVAL_HOUR>(stmt, 1); }
  }
}

// ============================================================================
// Appendix D: invalid interval literal (SQL_ERROR, SQLSTATE 22018).
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should reject malformed VARCHAR with SQLSTATE 22018",
                 "[datatype][string][conversion][interval][failure]") {
  // Given Snowflake client is logged in
  // When A VARCHAR row carrying inputs that aren't valid interval literals is fetched
  auto stmt = conn.execute_fetch(
      "SELECT 'not-an-interval' AS bad1, 'abc' AS bad2, "
      "'12.34.56' AS bad3, '' AS empty");

  // Then every interval target returns SQL_ERROR with SQLSTATE 22018
  check_invalid_string<SQL_C_INTERVAL_YEAR>(stmt, 1);
  check_invalid_string<SQL_C_INTERVAL_MONTH>(stmt, 2);
  check_invalid_string<SQL_C_INTERVAL_DAY>(stmt, 3);
  check_invalid_string<SQL_C_INTERVAL_HOUR>(stmt, 4);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject malformed year-month VARCHAR with SQLSTATE 22018",
                 "[datatype][string][conversion][interval][failure]") {
  // Given Snowflake client is logged in
  // When A VARCHAR row carrying malformed year-month literals is fetched
  auto stmt = conn.execute_fetch(
      "SELECT '3/6' AS wrong_sep, '3.6' AS dot_sep, "
      "'year-month' AS text, '3 6' AS space_sep");

  // Then every malformed year-month literal returns SQL_ERROR with SQLSTATE 22018
  check_invalid_string<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 1);
  check_invalid_string<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 2);
  check_invalid_string<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 3);
  check_invalid_string<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 4);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject malformed day-time VARCHAR with SQLSTATE 22018",
                 "[datatype][string][conversion][interval][failure]") {
  // Given Snowflake client is logged in
  // When A VARCHAR row carrying malformed day-time literals is fetched
  auto stmt = conn.execute_fetch(
      "SELECT '5-10' AS wrong_sep, 'day hour' AS text_values, "
      "'5:10:30:45' AS too_many, '::' AS empty_parts");

  // Then every malformed day-time literal returns SQL_ERROR with SQLSTATE 22018
  check_invalid_string<SQL_C_INTERVAL_DAY_TO_HOUR>(stmt, 1);
  check_invalid_string<SQL_C_INTERVAL_DAY_TO_SECOND>(stmt, 2);
  check_invalid_string<SQL_C_INTERVAL_HOUR_TO_SECOND>(stmt, 3);
  check_invalid_string<SQL_C_INTERVAL_MINUTE_TO_SECOND>(stmt, 4);
}

// BD#55 (bare integer for composite target): the universal driver rejects a
// bare integer as a malformed composite literal (22018); the reference driver
// accepts it for YEAR_TO_MONTH (treating it as the leading year field) and
// reports SQL_NO_DATA on subsequent SQLGetData calls because the column data
// is already consumed.
TEST_CASE_METHOD(ConnSchemaFixture, "should reject bare integer for every composite SQL_C_INTERVAL_* target",
                 "[datatype][string][conversion][interval][failure]") {
  // Given Snowflake client is logged in
  // When A VARCHAR carrying a bare integer is fetched as each composite target
  auto stmt = conn.execute_fetch("SELECT '5' AS bare");

  OLD_DRIVER_ONLY(kBd55VarcharIntervalFetch) {
    // Reference driver accepts the bare integer as a YEAR_TO_MONTH leading field;
    // subsequent SQLGetData calls on the same column return SQL_NO_DATA.
    SQL_INTERVAL_STRUCT value{};
    SQLLEN indicator = -999;
    SQLRETURN ret = get_data_raw(stmt, 1, SQL_C_INTERVAL_YEAR_TO_MONTH, &value, &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    return;
  }

  NEW_DRIVER_ONLY(kBd55VarcharIntervalFetch) {
    // Then every composite target returns SQL_ERROR with SQLSTATE 22018
    check_invalid_string<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 1);
    check_invalid_string<SQL_C_INTERVAL_DAY_TO_HOUR>(stmt, 1);
    check_invalid_string<SQL_C_INTERVAL_DAY_TO_MINUTE>(stmt, 1);
    check_invalid_string<SQL_C_INTERVAL_DAY_TO_SECOND>(stmt, 1);
    check_invalid_string<SQL_C_INTERVAL_HOUR_TO_MINUTE>(stmt, 1);
    check_invalid_string<SQL_C_INTERVAL_HOUR_TO_SECOND>(stmt, 1);
    check_invalid_string<SQL_C_INTERVAL_MINUTE_TO_SECOND>(stmt, 1);
  }
}

// BD#55 (out-of-range trailing fields): the universal driver enforces canonical
// ANSI SQL ranges for the trailing field of a composite interval and rejects
// out-of-range values with 22015 ("Interval field overflow"); the reference
// driver accepts the literal as long as the leading field fits inside its
// default precision. Both drivers reject YEAR_TO_MONTH month >= 12, the
// reference driver with 22018 and the universal driver with 22015.
TEST_CASE_METHOD(ConnSchemaFixture, "should reject out-of-range field magnitudes with SQLSTATE 22015",
                 "[datatype][string][conversion][interval][failure]") {
  // Per the SQLGetData diagnostics table, SQLSTATE 22015 ("Interval field
  // overflow") fires when "there was no representation of the value of the
  // SQL type in the interval C type". An interval C structure cannot
  // represent a trailing field outside its Gregorian range (per the
  // "Constraints of the Gregorian Calendar" appendix), so the universal
  // driver surfaces 22015 for:
  //   YEAR_TO_MONTH   trailing MONTH  : 0..=11
  //   *_TO_HOUR       trailing HOUR   : 0..=23
  //   *_TO_MINUTE     trailing MINUTE : 0..=59
  //   *_TO_SECOND     trailing SECOND : 0..=59
  // The leading slot of each composite is precision-driven (also 22015 on
  // overflow) and is intentionally NOT range-checked.
  //
  // Given Snowflake client is logged in
  // When A VARCHAR row carrying out-of-canonical-range trailing fields is fetched
  auto stmt = conn.execute_fetch(
      "SELECT '25:61' AS h_m, '30:61' AS m_s, "
      "'5 24:0:0' AS d_s, '3-12' AS y_m");

  OLD_DRIVER_ONLY(kBd55VarcharIntervalFetch) {
    // Reference driver does not enforce canonical ranges on trailing
    // day-time fields - it stores the parsed value as-is. It still rejects
    // month >= 12 for YEAR_TO_MONTH, but uses 22018 ("invalid literal")
    // for that case rather than the universal driver's 22015.
    SQL_INTERVAL_STRUCT value{};
    SQLLEN indicator = -999;
    REQUIRE(get_data_raw(stmt, 1, SQL_C_INTERVAL_HOUR_TO_MINUTE, &value, &indicator) == SQL_SUCCESS);
    REQUIRE(get_data_raw(stmt, 2, SQL_C_INTERVAL_MINUTE_TO_SECOND, &value, &indicator) == SQL_SUCCESS);
    REQUIRE(get_data_raw(stmt, 3, SQL_C_INTERVAL_DAY_TO_SECOND, &value, &indicator) == SQL_SUCCESS);
    check_invalid_string<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 4);
    return;
  }

  NEW_DRIVER_ONLY(kBd55VarcharIntervalFetch) {
    // Then SQL_C_INTERVAL_HOUR_TO_MINUTE rejects minute=61 with SQLSTATE 22015
    check_interval_field_overflow<SQL_C_INTERVAL_HOUR_TO_MINUTE>(stmt, 1);
    // And SQL_C_INTERVAL_MINUTE_TO_SECOND rejects second=61 with SQLSTATE 22015
    check_interval_field_overflow<SQL_C_INTERVAL_MINUTE_TO_SECOND>(stmt, 2);
    // And SQL_C_INTERVAL_DAY_TO_SECOND rejects hour=24 with SQLSTATE 22015
    check_interval_field_overflow<SQL_C_INTERVAL_DAY_TO_SECOND>(stmt, 3);
    // And SQL_C_INTERVAL_YEAR_TO_MONTH rejects month=12 with SQLSTATE 22015
    check_interval_field_overflow<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 4);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should accept boundary field magnitudes",
                 "[datatype][string][conversion][interval][edge]") {
  // The ANSI ceiling enforced by the driver is exclusive: 24/60/12 are
  // rejected (see the 22015 case above) while the inclusive max of
  // 23/59/11 must round-trip cleanly. Pins the boundary behavior for
  // every range-checked trailing slot.
  //
  // Given Snowflake client is logged in
  // When A VARCHAR row carrying inclusive-max trailing fields is fetched
  auto stmt = conn.execute_fetch(
      "SELECT '23:59' AS h_m, '23:59:59' AS h_s, "
      "'45:59' AS m_s, '3-11' AS y_m");

  // Then SQL_C_INTERVAL_HOUR_TO_MINUTE accepts hour=23, minute=59
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_HOUR_TO_MINUTE>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_HOUR_TO_MINUTE);
    CHECK(interval.intval.day_second.hour == 23);
    CHECK(interval.intval.day_second.minute == 59);
  }
  // And SQL_C_INTERVAL_HOUR_TO_SECOND accepts hour=23, minute=59, second=59
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_HOUR_TO_SECOND>(stmt, 2);
    CHECK(interval.interval_type == SQL_IS_HOUR_TO_SECOND);
    CHECK(interval.intval.day_second.hour == 23);
    CHECK(interval.intval.day_second.minute == 59);
    CHECK(interval.intval.day_second.second == 59);
  }
  // And SQL_C_INTERVAL_MINUTE_TO_SECOND accepts minute=45, second=59
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_MINUTE_TO_SECOND>(stmt, 3);
    CHECK(interval.interval_type == SQL_IS_MINUTE_TO_SECOND);
    CHECK(interval.intval.day_second.minute == 45);
    CHECK(interval.intval.day_second.second == 59);
  }
  // And SQL_C_INTERVAL_YEAR_TO_MONTH accepts year=3, month=11
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 4);
    CHECK(interval.interval_type == SQL_IS_YEAR_TO_MONTH);
    CHECK(interval.intval.year_month.year == 3);
    CHECK(interval.intval.year_month.month == 11);
  }
}

// ============================================================================
// NULL handling - SQL_NULL_DATA indicator regardless of target.
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should return SQL_NULL_DATA when VARCHAR is NULL",
                 "[datatype][string][conversion][interval][null]") {
  // Given Snowflake client is logged in
  // When A NULL VARCHAR is fetched as SQL_C_INTERVAL_YEAR
  auto stmt = conn.execute_fetch("SELECT NULL::STRING AS null_interval");
  SQL_INTERVAL_STRUCT interval = {};
  SQLLEN indicator = -999;
  SQLRETURN ret = get_data_raw(stmt, 1, SQL_C_INTERVAL_YEAR, &interval, &indicator);

  // Then the call returns SQL_SUCCESS with indicator = SQL_NULL_DATA
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(indicator == SQL_NULL_DATA);
}

// ============================================================================
// SQLBindCol path - exercises the same conversion through SQLBindCol/SQLFetch
// instead of SQLGetData. Confirms parity between the two binding entry points.
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should fetch VARCHAR as SQL_C_INTERVAL_YEAR via SQLBindCol",
                 "[datatype][string][conversion][interval][bindcol]") {
  // Given Snowflake client is logged in
  // When SQLBindCol binds a SQL_INTERVAL_STRUCT to the result of a VARCHAR query
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT '5' AS interval_year", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQL_INTERVAL_STRUCT interval = {};
  SQLLEN indicator = -999;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_INTERVAL_YEAR, &interval, sizeof(interval), &indicator);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the bound struct holds year = 5 with indicator = sizeof(SQL_INTERVAL_STRUCT)
  CHECK(interval.interval_type == SQL_IS_YEAR);
  CHECK(interval.intval.year_month.year == 5);
  CHECK(indicator == sizeof(SQL_INTERVAL_STRUCT));
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject malformed VARCHAR via SQLBindCol with SQLSTATE 22018",
                 "[datatype][string][conversion][interval][bindcol][failure]") {
  // Given Snowflake client is logged in
  // When SQLBindCol binds an interval struct against a malformed VARCHAR
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 'not_an_interval' AS str_val", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQL_INTERVAL_STRUCT interval = {};
  SQLLEN indicator = -999;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_INTERVAL_YEAR, &interval, sizeof(interval), &indicator);
  REQUIRE_ODBC(ret, stmt);

  // Then SQLFetch returns SQL_ERROR with a SQLSTATE 22018 diagnostic record
  ret = SQLFetch(stmt.getHandle());
  CHECK(ret == SQL_ERROR);
  auto records = get_diag_rec(stmt);
  REQUIRE(!records.empty());
  CHECK(records[0].sqlState == "22018");
}

// BD#55 (out-of-range trailing fields, SQLBindCol surface): mirrors the
// SQLGetData "should reject out-of-range field magnitudes with SQLSTATE 22015"
// test above. The diagnostic flows through a separate code path
// (SQLFetch + SQLBindCol vs. SQLGetData), so we pin both surfaces to confirm
// they emit the same SQLSTATE.
TEST_CASE_METHOD(ConnSchemaFixture, "should report 22015 on out-of-range trailing field via SQLBindCol",
                 "[datatype][string][conversion][interval][bindcol][failure]") {
  // Given Snowflake client is logged in
  // When SQLBindCol binds SQL_C_INTERVAL_HOUR_TO_MINUTE against '25:61'
  // (canonical-shape literal whose trailing minute=61 is outside ANSI 0..=59)
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT '25:61' AS oor_h_m", SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  SQL_INTERVAL_STRUCT interval = {};
  SQLLEN indicator = -999;
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_INTERVAL_HOUR_TO_MINUTE, &interval, sizeof(interval), &indicator);
  REQUIRE_ODBC(ret, stmt);

  OLD_DRIVER_ONLY(kBd55VarcharIntervalFetch) {
    // Reference driver does not enforce canonical ranges on trailing day-time
    // fields - it stores the parsed value as-is. SQLFetch therefore returns
    // SQL_SUCCESS even though minute=61 is outside the canonical ANSI range.
    REQUIRE(SQLFetch(stmt.getHandle()) == SQL_SUCCESS);
    return;
  }

  NEW_DRIVER_ONLY(kBd55VarcharIntervalFetch) {
    // Then SQLFetch returns SQL_ERROR with a SQLSTATE 22015 diagnostic record
    ret = SQLFetch(stmt.getHandle());
    CHECK(ret == SQL_ERROR);
    auto records = get_diag_rec(stmt);
    REQUIRE(!records.empty());
    CHECK(records[0].sqlState == "22015");
  }
}

// ============================================================================
// Additional edge-case coverage: negative composite day-time intervals,
// fractional-only negative seconds, sub-microsecond truncation, and
// composite malformed-input rejection. The assertions are intentionally
// driver-agnostic; BD#55-divergent edge cases (zero-magnitude minus
// sign, zero-padded fractional digits, H:M with explicit fraction) are
// covered separately so they can carry the proper OLD_DRIVER_ONLY /
// NEW_DRIVER_ONLY branches.
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should fetch negative composite VARCHAR as SQL_C_INTERVAL_DAY_TO_SECOND",
                 "[datatype][string][conversion][interval][negative]") {
  // Given a VARCHAR carrying a negative day-to-second interval literal
  auto stmt = conn.execute_fetch("SELECT '-2 08:15:30.250000' AS neg_day_second");

  // When the column is fetched as SQL_C_INTERVAL_DAY_TO_SECOND
  auto interval = check_no_truncation<SQL_C_INTERVAL_DAY_TO_SECOND>(stmt, 1);

  // Then every component is preserved and the sign bit reflects the negative magnitude
  CHECK(interval.interval_type == SQL_IS_DAY_TO_SECOND);
  CHECK(interval.interval_sign == SQL_TRUE);
  CHECK(interval.intval.day_second.day == 2);
  CHECK(interval.intval.day_second.hour == 8);
  CHECK(interval.intval.day_second.minute == 15);
  CHECK(interval.intval.day_second.second == 30);
  CHECK(interval.intval.day_second.fraction == 250000);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should fetch negative composite VARCHAR as SQL_C_INTERVAL_HOUR_TO_SECOND",
                 "[datatype][string][conversion][interval][negative]") {
  // Given a VARCHAR carrying a negative hour-to-second interval literal
  auto stmt = conn.execute_fetch("SELECT '-12:30:45' AS neg_hour_second");

  // When the column is fetched as SQL_C_INTERVAL_HOUR_TO_SECOND
  auto interval = check_no_truncation<SQL_C_INTERVAL_HOUR_TO_SECOND>(stmt, 1);

  // Then the hour/minute/second components are populated and the sign bit is set
  CHECK(interval.interval_type == SQL_IS_HOUR_TO_SECOND);
  CHECK(interval.interval_sign == SQL_TRUE);
  CHECK(interval.intval.day_second.hour == 12);
  CHECK(interval.intval.day_second.minute == 30);
  CHECK(interval.intval.day_second.second == 45);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should fetch negative composite VARCHAR as SQL_C_INTERVAL_MINUTE_TO_SECOND",
                 "[datatype][string][conversion][interval][negative]") {
  // Given a VARCHAR carrying a negative minute-to-second interval literal with fractional seconds
  auto stmt = conn.execute_fetch("SELECT '-45:30.750000' AS neg_minute_second");

  // When the column is fetched as SQL_C_INTERVAL_MINUTE_TO_SECOND
  auto interval = check_no_truncation<SQL_C_INTERVAL_MINUTE_TO_SECOND>(stmt, 1);

  // Then minute, second, fraction are populated and the sign bit is set
  CHECK(interval.interval_type == SQL_IS_MINUTE_TO_SECOND);
  CHECK(interval.interval_sign == SQL_TRUE);
  CHECK(interval.intval.day_second.minute == 45);
  CHECK(interval.intval.day_second.second == 30);
  CHECK(interval.intval.day_second.fraction == 750000);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should set sign bit on negative fractional-only SQL_C_INTERVAL_SECOND",
                 "[datatype][string][conversion][interval][negative][fractional]") {
  // Given a VARCHAR whose magnitude lives entirely in the fractional part
  auto stmt = conn.execute_fetch("SELECT '-0.5' AS neg_half_second");

  // When the column is fetched as SQL_C_INTERVAL_SECOND
  auto interval = check_no_truncation<SQL_C_INTERVAL_SECOND>(stmt, 1);

  // Then the sign bit is TRUE even though the integer part is zero (Microsoft ODBC spec: sign
  // reflects the overall interval value, not just the leading field).
  CHECK(interval.interval_type == SQL_IS_SECOND);
  CHECK(interval.interval_sign == SQL_TRUE);
  CHECK(interval.intval.day_second.second == 0);
  CHECK(interval.intval.day_second.fraction == 500000);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should warn 01S07 on sub-microsecond fractional truncation",
                 "[datatype][string][conversion][interval][truncation][fractional]") {
  // Given VARCHARs whose fractional parts carry more than 6 significant digits
  auto stmt = conn.execute_fetch(
      "SELECT '12:30:45.1234567' AS hour_second_extra, "
      "'2 08:15:30.7654321' AS day_second_extra, "
      "'45:30.1234567' AS minute_second_extra");

  // When fetched as composite SQL_C_INTERVAL_* with second precision
  // Then the value is truncated to microseconds and SQLSTATE 01S07 is reported
  {
    auto interval = check_interval_trailing_truncation<SQL_C_INTERVAL_HOUR_TO_SECOND>(stmt, 1);
    CHECK(interval.intval.day_second.hour == 12);
    CHECK(interval.intval.day_second.minute == 30);
    CHECK(interval.intval.day_second.second == 45);
    CHECK(interval.intval.day_second.fraction == 123456);
  }
  {
    auto interval = check_interval_trailing_truncation<SQL_C_INTERVAL_DAY_TO_SECOND>(stmt, 2);
    CHECK(interval.intval.day_second.day == 2);
    CHECK(interval.intval.day_second.hour == 8);
    CHECK(interval.intval.day_second.minute == 15);
    CHECK(interval.intval.day_second.second == 30);
    CHECK(interval.intval.day_second.fraction == 765432);
  }
  {
    auto interval = check_interval_trailing_truncation<SQL_C_INTERVAL_MINUTE_TO_SECOND>(stmt, 3);
    CHECK(interval.intval.day_second.minute == 45);
    CHECK(interval.intval.day_second.second == 30);
    CHECK(interval.intval.day_second.fraction == 123456);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture,
                 "should reject malformed VARCHAR for representative composite SQL_C_INTERVAL_* targets",
                 "[datatype][string][conversion][interval][negative][failure]") {
  // The four day-time-only composite targets (DAY_TO_HOUR / DAY_TO_SECOND /
  // HOUR_TO_SECOND / MINUTE_TO_SECOND) are already covered by
  // `should reject malformed day-time VARCHAR with SQLSTATE 22018` above; this
  // test fills in the remaining family members (YEAR_TO_MONTH, an additional
  // DAY_TO_SECOND probe with the dotted shape, HOUR_TO_MINUTE) without
  // duplicating coverage.
  //
  // Given a VARCHAR carrying a string that looks like an interval but uses wrong separators
  auto stmt = conn.execute_fetch("SELECT '12.34.56' AS dotted, '5/10' AS slashed, 'abc' AS letters");

  // When fetched as a representative subset of composite SQL_C_INTERVAL_* targets
  // Then SQLGetData fails with SQLSTATE 22018 in each case
  check_invalid_string<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 1);
  check_invalid_string<SQL_C_INTERVAL_DAY_TO_SECOND>(stmt, 2);
  check_invalid_string<SQL_C_INTERVAL_HOUR_TO_MINUTE>(stmt, 3);
}
