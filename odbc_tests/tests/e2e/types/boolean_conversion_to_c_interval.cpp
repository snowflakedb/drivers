#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "odbc_matchers.hpp"

// ============================================================================
// SUCCESSFUL CONVERSIONS - Single-field interval types
// ============================================================================

TEST_CASE("BOOLEAN to single-field interval types", "[datatype][boolean][conversion][c_interval]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When TRUE and FALSE BOOLEAN values are fetched as interval types
  (void)0;
  // Then Each single-field interval type returns 1 for TRUE and 0 for FALSE
  {
    INFO("SQL_C_INTERVAL_YEAR TRUE");
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(conn.execute_fetch("SELECT TRUE::BOOLEAN"), 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.year_month.year == 1);
  }
  {
    INFO("SQL_C_INTERVAL_YEAR FALSE");
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(conn.execute_fetch("SELECT FALSE::BOOLEAN"), 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.year_month.year == 0);
  }
  {
    INFO("SQL_C_INTERVAL_MONTH TRUE");
    auto interval = check_no_truncation<SQL_C_INTERVAL_MONTH>(conn.execute_fetch("SELECT TRUE::BOOLEAN"), 1);
    CHECK(interval.interval_type == SQL_IS_MONTH);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.year_month.month == 1);
  }
  {
    INFO("SQL_C_INTERVAL_DAY TRUE");
    auto interval = check_no_truncation<SQL_C_INTERVAL_DAY>(conn.execute_fetch("SELECT TRUE::BOOLEAN"), 1);
    CHECK(interval.interval_type == SQL_IS_DAY);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.day == 1);
  }
  {
    INFO("SQL_C_INTERVAL_HOUR TRUE");
    auto interval = check_no_truncation<SQL_C_INTERVAL_HOUR>(conn.execute_fetch("SELECT TRUE::BOOLEAN"), 1);
    CHECK(interval.interval_type == SQL_IS_HOUR);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.hour == 1);
  }
  {
    INFO("SQL_C_INTERVAL_MINUTE TRUE");
    auto interval = check_no_truncation<SQL_C_INTERVAL_MINUTE>(conn.execute_fetch("SELECT TRUE::BOOLEAN"), 1);
    CHECK(interval.interval_type == SQL_IS_MINUTE);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.minute == 1);
  }
  {
    INFO("SQL_C_INTERVAL_SECOND TRUE");
    auto interval = check_no_truncation<SQL_C_INTERVAL_SECOND>(conn.execute_fetch("SELECT TRUE::BOOLEAN"), 1);
    CHECK(interval.interval_type == SQL_IS_SECOND);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.second == 1);
    CHECK(interval.intval.day_second.fraction == 0);
  }
  {
    INFO("SQL_C_INTERVAL_SECOND FALSE");
    auto interval = check_no_truncation<SQL_C_INTERVAL_SECOND>(conn.execute_fetch("SELECT FALSE::BOOLEAN"), 1);
    CHECK(interval.interval_type == SQL_IS_SECOND);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.second == 0);
    CHECK(interval.intval.day_second.fraction == 0);
  }
}

// ============================================================================
// MULTI-FIELD INTERVAL TYPES (SQLSTATE 22015)
// ============================================================================

TEST_CASE("BOOLEAN to multi-field interval returns 22015", "[datatype][boolean][conversion][c_interval][22015]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A BOOLEAN value is fetched as multi-field interval types
  (void)0;
  // Then All multi-field interval conversions fail with SQLSTATE 22015
  check_interval_precision_lost<SQL_C_INTERVAL_YEAR_TO_MONTH>(conn.execute_fetch("SELECT TRUE::BOOLEAN"), 1);
  check_interval_precision_lost<SQL_C_INTERVAL_DAY_TO_HOUR>(conn.execute_fetch("SELECT TRUE::BOOLEAN"), 1);
  check_interval_precision_lost<SQL_C_INTERVAL_DAY_TO_MINUTE>(conn.execute_fetch("SELECT TRUE::BOOLEAN"), 1);
  check_interval_precision_lost<SQL_C_INTERVAL_DAY_TO_SECOND>(conn.execute_fetch("SELECT TRUE::BOOLEAN"), 1);
  check_interval_precision_lost<SQL_C_INTERVAL_HOUR_TO_MINUTE>(conn.execute_fetch("SELECT TRUE::BOOLEAN"), 1);
  check_interval_precision_lost<SQL_C_INTERVAL_HOUR_TO_SECOND>(conn.execute_fetch("SELECT TRUE::BOOLEAN"), 1);
  check_interval_precision_lost<SQL_C_INTERVAL_MINUTE_TO_SECOND>(conn.execute_fetch("SELECT TRUE::BOOLEAN"), 1);
}

// ============================================================================
// NULL handling
// ============================================================================

TEST_CASE("BOOLEAN NULL to interval C types", "[datatype][boolean][conversion][c_interval][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL BOOLEAN value is queried
  (void)0;
  // Then Indicator returns SQL_NULL_DATA for all single-field interval types
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::BOOLEAN"), 1, SQL_C_INTERVAL_YEAR);
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::BOOLEAN"), 1, SQL_C_INTERVAL_MONTH);
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::BOOLEAN"), 1, SQL_C_INTERVAL_DAY);
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::BOOLEAN"), 1, SQL_C_INTERVAL_HOUR);
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::BOOLEAN"), 1, SQL_C_INTERVAL_MINUTE);
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::BOOLEAN"), 1, SQL_C_INTERVAL_SECOND);
}
