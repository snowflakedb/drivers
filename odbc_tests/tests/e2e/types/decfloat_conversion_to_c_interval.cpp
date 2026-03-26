#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"

// ============================================================================
// SUCCESSFUL CONVERSIONS - Single-component interval types
// ============================================================================

TEST_CASE("DECFLOAT to single-field interval types", "[decfloat][conversion][c_interval]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;

  // When Positive, negative, and zero DECFLOAT values are fetched as interval types
  (void)0;
  // Then Each single-field interval type returns the correct value and sign
  {
    INFO("SQL_C_INTERVAL_YEAR positive");
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(conn.execute_fetch("SELECT 5::DECFLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.year_month.year == 5);
  }
  {
    INFO("SQL_C_INTERVAL_YEAR negative");
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(conn.execute_fetch("SELECT '-3'::DECFLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.interval_sign == SQL_TRUE);
    CHECK(interval.intval.year_month.year == 3);
  }
  {
    INFO("SQL_C_INTERVAL_YEAR zero");
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(conn.execute_fetch("SELECT 0::DECFLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.year_month.year == 0);
  }
  {
    INFO("SQL_C_INTERVAL_MONTH");
    auto interval = check_no_truncation<SQL_C_INTERVAL_MONTH>(conn.execute_fetch("SELECT 10::DECFLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_MONTH);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.year_month.month == 10);
  }
  {
    INFO("SQL_C_INTERVAL_DAY");
    auto interval = check_no_truncation<SQL_C_INTERVAL_DAY>(conn.execute_fetch("SELECT 15::DECFLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_DAY);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.day == 15);
  }
  {
    INFO("SQL_C_INTERVAL_HOUR");
    auto interval = check_no_truncation<SQL_C_INTERVAL_HOUR>(conn.execute_fetch("SELECT 8::DECFLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_HOUR);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.hour == 8);
  }
  {
    INFO("SQL_C_INTERVAL_MINUTE");
    auto interval = check_no_truncation<SQL_C_INTERVAL_MINUTE>(conn.execute_fetch("SELECT 30::DECFLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_MINUTE);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.minute == 30);
  }
  {
    INFO("SQL_C_INTERVAL_SECOND integer");
    auto interval = check_no_truncation<SQL_C_INTERVAL_SECOND>(conn.execute_fetch("SELECT 45::DECFLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_SECOND);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.second == 45);
    CHECK(interval.intval.day_second.fraction == 0);
  }
}

// ============================================================================
// FRACTIONAL TRUNCATION (SQLSTATE 01S07)
// ============================================================================

TEST_CASE("DECFLOAT fractional truncation to interval types", "[decfloat][conversion][c_interval][01S07]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;

  // When Fractional DECFLOAT values are fetched as non-second interval types
  (void)0;
  // Then The fractional part is truncated and SQLSTATE 01S07 is returned
  {
    INFO("SQL_C_INTERVAL_YEAR truncates fraction");
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_YEAR>(conn.execute_fetch("SELECT 5.7::DECFLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.intval.year_month.year == 5);
  }
  {
    INFO("SQL_C_INTERVAL_MONTH truncates fraction");
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_MONTH>(conn.execute_fetch("SELECT 10.3::DECFLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_MONTH);
    CHECK(interval.intval.year_month.month == 10);
  }
  {
    INFO("SQL_C_INTERVAL_DAY truncates fraction");
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_DAY>(conn.execute_fetch("SELECT 15.9::DECFLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_DAY);
    CHECK(interval.intval.day_second.day == 15);
  }
  {
    INFO("SQL_C_INTERVAL_HOUR truncates fraction");
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_HOUR>(conn.execute_fetch("SELECT 8.5::DECFLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_HOUR);
    CHECK(interval.intval.day_second.hour == 8);
  }
  {
    INFO("SQL_C_INTERVAL_MINUTE truncates fraction");
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_MINUTE>(conn.execute_fetch("SELECT 30.1::DECFLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_MINUTE);
    CHECK(interval.intval.day_second.minute == 30);
  }
}

// ============================================================================
// MULTI-FIELD INTERVAL TYPES (SQLSTATE 22015)
// ============================================================================

TEST_CASE("DECFLOAT to multi-field interval returns 22015", "[decfloat][conversion][c_interval][22015]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;

  // When A DECFLOAT value is fetched as multi-field interval types
  (void)0;
  // Then All multi-field interval conversions fail with SQLSTATE 22015
  check_interval_precision_lost<SQL_C_INTERVAL_YEAR_TO_MONTH>(conn.execute_fetch("SELECT 42::DECFLOAT"), 1);
  check_interval_precision_lost<SQL_C_INTERVAL_DAY_TO_HOUR>(conn.execute_fetch("SELECT 42::DECFLOAT"), 1);
  check_interval_precision_lost<SQL_C_INTERVAL_DAY_TO_MINUTE>(conn.execute_fetch("SELECT 42::DECFLOAT"), 1);
  check_interval_precision_lost<SQL_C_INTERVAL_DAY_TO_SECOND>(conn.execute_fetch("SELECT 42::DECFLOAT"), 1);
  check_interval_precision_lost<SQL_C_INTERVAL_HOUR_TO_MINUTE>(conn.execute_fetch("SELECT 42::DECFLOAT"), 1);
  check_interval_precision_lost<SQL_C_INTERVAL_HOUR_TO_SECOND>(conn.execute_fetch("SELECT 42::DECFLOAT"), 1);
  check_interval_precision_lost<SQL_C_INTERVAL_MINUTE_TO_SECOND>(conn.execute_fetch("SELECT 42::DECFLOAT"), 1);
}

// ============================================================================
// NULL handling
// ============================================================================

TEST_CASE("DECFLOAT NULL to interval C types", "[decfloat][conversion][c_interval][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL DECFLOAT value is queried
  (void)0;
  // Then Indicator returns SQL_NULL_DATA for all single-field interval types
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::DECFLOAT"), 1, SQL_C_INTERVAL_YEAR);
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::DECFLOAT"), 1, SQL_C_INTERVAL_MONTH);
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::DECFLOAT"), 1, SQL_C_INTERVAL_DAY);
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::DECFLOAT"), 1, SQL_C_INTERVAL_HOUR);
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::DECFLOAT"), 1, SQL_C_INTERVAL_MINUTE);
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::DECFLOAT"), 1, SQL_C_INTERVAL_SECOND);
}
