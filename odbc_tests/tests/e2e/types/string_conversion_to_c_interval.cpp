// String to ODBC interval type conversions tests
// Tests converting Snowflake VARCHAR/STRING type to interval ODBC C types:
// SQL_C_INTERVAL_YEAR, SQL_C_INTERVAL_MONTH, SQL_C_INTERVAL_DAY,
// SQL_C_INTERVAL_HOUR, SQL_C_INTERVAL_MINUTE, SQL_C_INTERVAL_SECOND,
// SQL_C_INTERVAL_YEAR_TO_MONTH, SQL_C_INTERVAL_DAY_TO_HOUR,
// SQL_C_INTERVAL_DAY_TO_MINUTE, SQL_C_INTERVAL_DAY_TO_SECOND,
// SQL_C_INTERVAL_HOUR_TO_MINUTE, SQL_C_INTERVAL_HOUR_TO_SECOND,
// SQL_C_INTERVAL_MINUTE_TO_SECOND
//
// Test cases based on ODBC spec:
// 1. Data value is a valid interval value; no truncation -> Data, Length of data in bytes, n/a
// 2. Data value is a valid interval value; truncation of trailing fields -> Truncated data, Length, 01S07
// 3. Data is valid interval; leading field significant precision is lost -> Undefined, Undefined, 22015
// 4. The data value is not a valid interval value -> Undefined, Undefined, 22018

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <string>
#include <vector>

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

// ============================================================================
// SUCCESSFUL CONVERSIONS - Single-component interval types (no truncation)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should convert string literals to single-component c_type",
                 "[datatype][string][conversion][interval]") {
  // Catch2 needs one test to be present in the suite, so we skip this one.
  SKIP();
  // Given Snowflake client is logged in

  // When Query selecting string literals representing interval values is executed
  auto stmt = conn.execute_fetch(
      "SELECT '5' AS years, '10' AS months, '15' AS days, "
      "'8' AS hours, '30' AS minutes, '45' AS seconds");

  // Then <c_type> conversions should work
  {
    INFO("SQL_C_INTERVAL_YEAR");
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.year_month.year == 5);
  }
  {
    INFO("SQL_C_INTERVAL_MONTH");
    auto interval = check_no_truncation<SQL_C_INTERVAL_MONTH>(stmt, 2);
    CHECK(interval.interval_type == SQL_IS_MONTH);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.year_month.month == 10);
  }
  {
    INFO("SQL_C_INTERVAL_DAY");
    auto interval = check_no_truncation<SQL_C_INTERVAL_DAY>(stmt, 3);
    CHECK(interval.interval_type == SQL_IS_DAY);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.day == 15);
  }
  {
    INFO("SQL_C_INTERVAL_HOUR");
    auto interval = check_no_truncation<SQL_C_INTERVAL_HOUR>(stmt, 4);
    CHECK(interval.interval_type == SQL_IS_HOUR);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.hour == 8);
  }
  {
    INFO("SQL_C_INTERVAL_MINUTE");
    auto interval = check_no_truncation<SQL_C_INTERVAL_MINUTE>(stmt, 5);
    CHECK(interval.interval_type == SQL_IS_MINUTE);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.minute == 30);
  }
  {
    INFO("SQL_C_INTERVAL_SECOND");
    auto interval = check_no_truncation<SQL_C_INTERVAL_SECOND>(stmt, 6);
    CHECK(interval.interval_type == SQL_IS_SECOND);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.second == 45);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should convert negative c_type string literals",
                 "[datatype][string][conversion][interval]") {
  // Given Snowflake client is logged in

  // When Query selecting negative interval values is executed
  auto stmt = conn.execute_fetch("SELECT '-5' AS neg_years, '-10' AS neg_months, '-15' AS neg_days");

  // Then negative <c_type> should be correctly parsed
  {
    INFO("SQL_C_INTERVAL_YEAR");
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.interval_sign == SQL_TRUE);
    CHECK(interval.intval.year_month.year == 5);
  }
  {
    INFO("SQL_C_INTERVAL_MONTH");
    auto interval = check_no_truncation<SQL_C_INTERVAL_MONTH>(stmt, 2);
    CHECK(interval.interval_type == SQL_IS_MONTH);
    CHECK(interval.interval_sign == SQL_TRUE);
    CHECK(interval.intval.year_month.month == 10);
  }
  {
    INFO("SQL_C_INTERVAL_DAY");
    auto interval = check_no_truncation<SQL_C_INTERVAL_DAY>(stmt, 3);
    CHECK(interval.interval_type == SQL_IS_DAY);
    CHECK(interval.interval_sign == SQL_TRUE);
    CHECK(interval.intval.day_second.day == 15);
  }
}

// ============================================================================
// SUCCESSFUL CONVERSIONS - Multi-component interval types (no truncation)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should convert string literals to year-month interval type",
                 "[datatype][string][conversion][interval]") {
  // Given Snowflake client is logged in

  // When Query selecting year-month interval string is executed
  auto stmt = conn.execute_fetch("SELECT '3-6' AS year_month, '-2-9' AS neg_year_month, '0-11' AS zero_year");

  // Then SQL_C_INTERVAL_YEAR_TO_MONTH conversions should work
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_YEAR_TO_MONTH);
    CHECK(interval.interval_sign == SQL_FALSE);  // Positive
    CHECK(interval.intval.year_month.year == 3);
    CHECK(interval.intval.year_month.month == 6);
  }

  // And negative year-month should be correctly parsed
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 2);
    CHECK(interval.interval_type == SQL_IS_YEAR_TO_MONTH);
    CHECK(interval.interval_sign == SQL_TRUE);  // Negative
    CHECK(interval.intval.year_month.year == 2);
    CHECK(interval.intval.year_month.month == 9);
  }

  // And zero years with months should work
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 3);
    CHECK(interval.interval_type == SQL_IS_YEAR_TO_MONTH);
    CHECK(interval.interval_sign == SQL_FALSE);  // Positive
    CHECK(interval.intval.year_month.year == 0);
    CHECK(interval.intval.year_month.month == 11);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should convert string literals to compound c_type",
                 "[datatype][string][conversion][interval]") {
  // Given Snowflake client is logged in

  // When Query selecting day-time interval strings is executed
  auto stmt = conn.execute_fetch(
      "SELECT '5 10' AS day_hour, '3 14:30' AS day_minute, "
      "'2 08:15:30' AS day_second, '10:45' AS hour_minute, "
      "'12:30:45' AS hour_second, '45:30' AS minute_second");

  // Then <c_type> conversions should work
  {
    INFO("SQL_C_INTERVAL_DAY_TO_HOUR");
    auto interval = check_no_truncation<SQL_C_INTERVAL_DAY_TO_HOUR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_DAY_TO_HOUR);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.day == 5);
    CHECK(interval.intval.day_second.hour == 10);
  }
  {
    INFO("SQL_C_INTERVAL_DAY_TO_MINUTE");
    auto interval = check_no_truncation<SQL_C_INTERVAL_DAY_TO_MINUTE>(stmt, 2);
    CHECK(interval.interval_type == SQL_IS_DAY_TO_MINUTE);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.day == 3);
    CHECK(interval.intval.day_second.hour == 14);
    CHECK(interval.intval.day_second.minute == 30);
  }
  {
    INFO("SQL_C_INTERVAL_DAY_TO_SECOND");
    auto interval = check_no_truncation<SQL_C_INTERVAL_DAY_TO_SECOND>(stmt, 3);
    CHECK(interval.interval_type == SQL_IS_DAY_TO_SECOND);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.day == 2);
    CHECK(interval.intval.day_second.hour == 8);
    CHECK(interval.intval.day_second.minute == 15);
    CHECK(interval.intval.day_second.second == 30);
  }
  {
    INFO("SQL_C_INTERVAL_HOUR_TO_MINUTE");
    auto interval = check_no_truncation<SQL_C_INTERVAL_HOUR_TO_MINUTE>(stmt, 4);
    CHECK(interval.interval_type == SQL_IS_HOUR_TO_MINUTE);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.hour == 10);
    CHECK(interval.intval.day_second.minute == 45);
  }
  {
    INFO("SQL_C_INTERVAL_HOUR_TO_SECOND");
    auto interval = check_no_truncation<SQL_C_INTERVAL_HOUR_TO_SECOND>(stmt, 5);
    CHECK(interval.interval_type == SQL_IS_HOUR_TO_SECOND);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.hour == 12);
    CHECK(interval.intval.day_second.minute == 30);
    CHECK(interval.intval.day_second.second == 45);
  }
  {
    INFO("SQL_C_INTERVAL_MINUTE_TO_SECOND");
    auto interval = check_no_truncation<SQL_C_INTERVAL_MINUTE_TO_SECOND>(stmt, 6);
    CHECK(interval.interval_type == SQL_IS_MINUTE_TO_SECOND);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.minute == 45);
    CHECK(interval.intval.day_second.second == 30);
  }
}

// ============================================================================
// TRUNCATION WITH INFO - Trailing field truncation (SQLSTATE 01S07)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should truncate trailing fields when converting interval strings",
                 "[datatype][string][conversion][interval][truncation]") {
  // Given Snowflake client is logged in

  // When Query selecting interval strings with more precision than target type is executed
  auto stmt = conn.execute_fetch(
      "SELECT '3-6' AS year_month_to_year, '5 10:30:45' AS day_second_to_day, "
      "'12:30:45' AS hour_second_to_hour, '45:30' AS minute_second_to_minute");

  // Then year-month to year should truncate month field
  {
    auto interval = check_interval_trailing_truncation<SQL_C_INTERVAL_YEAR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.intval.year_month.year == 3);
    // Month field is truncated
  }

  // And day-second to day should truncate time fields
  {
    auto interval = check_interval_trailing_truncation<SQL_C_INTERVAL_DAY>(stmt, 2);
    CHECK(interval.interval_type == SQL_IS_DAY);
    CHECK(interval.intval.day_second.day == 5);
    // Hour, minute, second fields are truncated
  }

  // And hour-second to hour should truncate minute and second
  {
    auto interval = check_interval_trailing_truncation<SQL_C_INTERVAL_HOUR>(stmt, 3);
    CHECK(interval.interval_type == SQL_IS_HOUR);
    CHECK(interval.intval.day_second.hour == 12);
    // Minute and second fields are truncated
  }

  // And minute-second to minute: the new driver parses 2-component non-fractional strings as H:M,
  // so '45:30' becomes hour=45/minute=30. Reading as IntervalMinute keeps minute=30 and warns 01S07
  // for the dropped hour component. (See BehaviorDifferences for divergence from old driver, which
  // returned 22015 here.)
  {
    auto interval = check_interval_trailing_truncation<SQL_C_INTERVAL_MINUTE>(stmt, 4);
    CHECK(interval.interval_type == SQL_IS_MINUTE);
    CHECK(interval.intval.day_second.minute == 30);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should truncate trailing fields in day-time intervals",
                 "[datatype][string][conversion][interval][truncation]") {
  // Given Snowflake client is logged in

  // When Query selecting day-time interval strings with more precision is executed
  auto stmt = conn.execute_fetch(
      "SELECT '2 08:15:30' AS day_second_to_day_hour, '2 08:15:30' AS day_second_to_day_minute, "
      "'12:30:45' AS hour_second_to_hour_minute");

  // Then day-second to day-hour should truncate minute and second
  {
    auto interval = check_interval_trailing_truncation<SQL_C_INTERVAL_DAY_TO_HOUR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_DAY_TO_HOUR);
    CHECK(interval.intval.day_second.day == 2);
    CHECK(interval.intval.day_second.hour == 8);
    // Minute and second fields are truncated
  }

  // And day-second to day-minute should truncate second
  {
    auto interval = check_interval_trailing_truncation<SQL_C_INTERVAL_DAY_TO_MINUTE>(stmt, 2);
    CHECK(interval.interval_type == SQL_IS_DAY_TO_MINUTE);
    CHECK(interval.intval.day_second.day == 2);
    CHECK(interval.intval.day_second.hour == 8);
    CHECK(interval.intval.day_second.minute == 15);
    // Second field is truncated
  }

  // And hour-second to hour-minute should truncate second
  {
    auto interval = check_interval_trailing_truncation<SQL_C_INTERVAL_HOUR_TO_MINUTE>(stmt, 3);
    CHECK(interval.interval_type == SQL_IS_HOUR_TO_MINUTE);
    CHECK(interval.intval.day_second.hour == 12);
    CHECK(interval.intval.day_second.minute == 30);
    // Second field is truncated
  }
}

// ============================================================================
// LEADING FIELD PRECISION LOSS - (SQLSTATE 22015)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should fail when leading field precision is lost for year intervals",
                 "[datatype][string][conversion][interval][precision]") {
  // Given Snowflake client is logged in

  // Default leading precision is typically 2 digits for intervals
  // When Query selecting interval values with leading field exceeding precision is executed
  auto stmt = conn.execute_fetch("SELECT '10000' AS large_year, '99999' AS very_large_year");

  // Then values exceeding leading field precision should fail with 22015
  check_interval_precision_lost<SQL_C_INTERVAL_YEAR>(stmt, 1);
  check_interval_precision_lost<SQL_C_INTERVAL_YEAR>(stmt, 2);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should fail when leading field precision is lost for month intervals",
                 "[datatype][string][conversion][interval][precision]") {
  // Given Snowflake client is logged in

  // When Query selecting interval values with leading field exceeding precision is executed
  auto stmt = conn.execute_fetch("SELECT '10000' AS large_month, '99999' AS very_large_month");

  // Then values exceeding leading field precision should fail with 22015
  check_interval_precision_lost<SQL_C_INTERVAL_MONTH>(stmt, 1);
  check_interval_precision_lost<SQL_C_INTERVAL_MONTH>(stmt, 2);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should fail when leading field precision is lost for day intervals",
                 "[datatype][string][conversion][interval][precision]") {
  // Given Snowflake client is logged in

  // When Query selecting interval values with leading field exceeding precision is executed
  auto stmt = conn.execute_fetch("SELECT '10000' AS large_day, '99999' AS very_large_day");

  // Then values exceeding leading field precision should fail with 22015
  check_interval_precision_lost<SQL_C_INTERVAL_DAY>(stmt, 1);
  check_interval_precision_lost<SQL_C_INTERVAL_DAY>(stmt, 2);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should fail when leading field precision is lost for hour intervals",
                 "[datatype][string][conversion][interval][precision]") {
  // Given Snowflake client is logged in

  // When Query selecting interval values with leading field exceeding precision is executed
  auto stmt = conn.execute_fetch("SELECT '10000' AS large_hour, '99999' AS very_large_hour");

  // Then values exceeding leading field precision should fail with 22015
  check_interval_precision_lost<SQL_C_INTERVAL_HOUR>(stmt, 1);
  check_interval_precision_lost<SQL_C_INTERVAL_HOUR>(stmt, 2);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should fail when leading field precision is lost for compound intervals",
                 "[datatype][string][conversion][interval][precision]") {
  // Given Snowflake client is logged in

  // When Query selecting compound interval values with leading field exceeding precision is executed
  auto stmt = conn.execute_fetch("SELECT '10000-6' AS large_year_month, '99999 10:30:45' AS large_day_second");

  // Then values exceeding leading field precision should fail with 22015
  check_interval_precision_lost<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 1);
  check_interval_precision_lost<SQL_C_INTERVAL_DAY_TO_SECOND>(stmt, 2);
}

// ============================================================================
// INVALID INTERVAL VALUES - (SQLSTATE 22018)
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should fail converting invalid interval string formats",
                 "[datatype][string][conversion][interval][failure]") {
  // Given Snowflake client is logged in

  // When Query selecting invalid interval strings is executed
  auto stmt = conn.execute_fetch(
      "SELECT 'not-an-interval' AS invalid1, 'abc' AS invalid2, "
      "'12.34.56' AS invalid3, '' AS empty");

  // Then invalid interval strings should fail with SQLSTATE 22018
  check_invalid_string<SQL_C_INTERVAL_YEAR>(stmt, 1);
  check_invalid_string<SQL_C_INTERVAL_MONTH>(stmt, 2);
  check_invalid_string<SQL_C_INTERVAL_DAY>(stmt, 3);
  check_invalid_string<SQL_C_INTERVAL_HOUR>(stmt, 4);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should fail converting malformed interval strings for year-month type",
                 "[datatype][string][conversion][interval][failure]") {
  // Given Snowflake client is logged in

  // When Query selecting malformed year-month interval strings is executed
  auto stmt = conn.execute_fetch(
      "SELECT '3/6' AS wrong_separator, '3.6' AS decimal_separator, "
      "'year-month' AS text, '3 6' AS space_separator");

  // Then malformed year-month strings should fail with SQLSTATE 22018
  check_invalid_string<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 1);
  check_invalid_string<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 2);
  check_invalid_string<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 3);
  check_invalid_string<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 4);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should fail converting malformed interval strings for day-time types",
                 "[datatype][string][conversion][interval][failure]") {
  // Given Snowflake client is logged in

  // When Query selecting malformed day-time interval strings is executed
  auto stmt = conn.execute_fetch(
      "SELECT '5-10' AS wrong_separator, 'day hour' AS text_values, "
      "'5:10:30:45' AS too_many_components, '::' AS empty_components");

  // Then malformed day-time strings should fail with SQLSTATE 22018
  check_invalid_string<SQL_C_INTERVAL_DAY_TO_HOUR>(stmt, 1);
  check_invalid_string<SQL_C_INTERVAL_DAY_TO_SECOND>(stmt, 2);
  check_invalid_string<SQL_C_INTERVAL_HOUR_TO_SECOND>(stmt, 3);
  check_invalid_string<SQL_C_INTERVAL_MINUTE_TO_SECOND>(stmt, 4);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject out-of-range trailing component values with 22015",
                 "[datatype][string][conversion][interval][failure]") {
  // Given Snowflake client is logged in
  //
  // Per the Microsoft ODBC spec ("Interval Data Types": "Trailing fields must follow the usual
  // constraints of the Gregorian calendar"), composite interval qualifiers reject out-of-range
  // trailing fields with SQLSTATE 22015. The leading field is unconstrained (subject to its own
  // precision check), so e.g. hour=25 is fine in HOUR_TO_MINUTE -- the rejection is on minute=61.

  // When Query selecting interval strings with invalid trailing component ranges is executed
  auto stmt = conn.execute_fetch(
      "SELECT '3-13' AS invalid_month, '25:61' AS invalid_minute_in_hm, "
      "'10:61' AS invalid_minute, '30:61' AS invalid_second");

  // Then '3-13' as YEAR_TO_MONTH fails: trailing month=13 > 11 -> 22015
  check_interval_precision_lost<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 1);

  // And '25:61' as HOUR_TO_MINUTE fails: leading hour=25 is allowed (precision permits 2 digits),
  // but trailing minute=61 > 59 -> 22015
  check_interval_precision_lost<SQL_C_INTERVAL_HOUR_TO_MINUTE>(stmt, 2);

  // And '10:61' as HOUR_TO_MINUTE fails: trailing minute=61 > 59 -> 22015
  check_interval_precision_lost<SQL_C_INTERVAL_HOUR_TO_MINUTE>(stmt, 3);

  // And '30:61' as MINUTE_TO_SECOND fails: leading minute=30 allowed, trailing second=61 -> 22015
  check_interval_precision_lost<SQL_C_INTERVAL_MINUTE_TO_SECOND>(stmt, 4);
}

// ============================================================================
// EDGE CASES - Whitespace and special formatting
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should handle whitespace in interval strings",
                 "[datatype][string][conversion][interval][edge]") {
  // Given Snowflake client is logged in

  // When Query selecting interval strings with leading/trailing whitespace is executed
  auto stmt = conn.execute_fetch(
      "SELECT '  5  ' AS padded_years, '  10  ' AS padded_months, "
      "'  3-6  ' AS padded_year_month");

  // Then whitespace should be trimmed and values parsed correctly
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.intval.year_month.year == 5);
  }

  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_MONTH>(stmt, 2);
    CHECK(interval.interval_type == SQL_IS_MONTH);
    CHECK(interval.intval.year_month.month == 10);
  }

  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 3);
    CHECK(interval.interval_type == SQL_IS_YEAR_TO_MONTH);
    CHECK(interval.intval.year_month.year == 3);
    CHECK(interval.intval.year_month.month == 6);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should handle zero values in interval strings",
                 "[datatype][string][conversion][interval][edge]") {
  // Given Snowflake client is logged in

  // When Query selecting zero interval values is executed
  auto stmt = conn.execute_fetch(
      "SELECT '0' AS zero_years, '0' AS zero_days, '0-0' AS zero_year_month, "
      "'0 00:00:00' AS zero_day_second, '00:00:00' AS zero_time");

  // Then zero values should be correctly parsed
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.intval.year_month.year == 0);
  }

  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_DAY>(stmt, 2);
    CHECK(interval.interval_type == SQL_IS_DAY);
    CHECK(interval.intval.day_second.day == 0);
  }

  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 3);
    CHECK(interval.interval_type == SQL_IS_YEAR_TO_MONTH);
    CHECK(interval.intval.year_month.year == 0);
    CHECK(interval.intval.year_month.month == 0);
  }

  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_DAY_TO_SECOND>(stmt, 4);
    CHECK(interval.interval_type == SQL_IS_DAY_TO_SECOND);
    CHECK(interval.intval.day_second.day == 0);
    CHECK(interval.intval.day_second.hour == 0);
    CHECK(interval.intval.day_second.minute == 0);
    CHECK(interval.intval.day_second.second == 0);
  }

  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_HOUR_TO_SECOND>(stmt, 5);
    CHECK(interval.interval_type == SQL_IS_HOUR_TO_SECOND);
    CHECK(interval.intval.day_second.hour == 0);
    CHECK(interval.intval.day_second.minute == 0);
    CHECK(interval.intval.day_second.second == 0);
  }
}

// ============================================================================
// NULL VALUE HANDLING
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should handle NULL string when converting to interval types",
                 "[datatype][string][conversion][interval][null]") {
  // Given Snowflake client is logged in

  // When Query selecting NULL is executed
  auto stmt = conn.execute_fetch("SELECT NULL::STRING AS null_interval");

  // Then NULL should return SQL_NULL_DATA indicator
  {
    SQL_INTERVAL_STRUCT interval;
    SQLLEN indicator;
    SQLRETURN ret = get_data_raw(stmt, 1, SQL_C_INTERVAL_YEAR, &interval, &indicator);
    REQUIRE_ODBC(ret, stmt);
    CHECK(indicator == SQL_NULL_DATA);
  }
}

// ============================================================================
// CONVERSION WITH SQLBindCol - Interval types
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should convert strings to interval types using SQLBindCol",
                 "[datatype][string][conversion][interval]") {
  // Given Snowflake client is logged in

  // When Query selecting interval value is executed with SQLBindCol for SQL_C_INTERVAL_YEAR
  {
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT '5' AS interval_year", SQL_NTS);
    REQUIRE_ODBC(ret, stmt);

    SQL_INTERVAL_STRUCT interval;
    SQLLEN indicator;
    ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_INTERVAL_YEAR, &interval, sizeof(interval), &indicator);
    REQUIRE_ODBC(ret, stmt);

    ret = SQLFetch(stmt.getHandle());
    REQUIRE_ODBC(ret, stmt);

    // Then the bound interval value should match the string representation
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.intval.year_month.year == 5);
    CHECK(indicator == sizeof(SQL_INTERVAL_STRUCT));
  }

  // And invalid interval string should fail binding with SQLSTATE 22018
  {
    auto stmt = conn.createStatement();
    SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 'not_an_interval' AS str_val", SQL_NTS);
    REQUIRE_ODBC(ret, stmt);

    SQL_INTERVAL_STRUCT interval;
    SQLLEN indicator;
    ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_INTERVAL_YEAR, &interval, sizeof(interval), &indicator);
    REQUIRE_ODBC(ret, stmt);

    ret = SQLFetch(stmt.getHandle());
    CHECK(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "22018");
  }
}

// ============================================================================
// FRACTIONAL SECONDS HANDLING
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should handle fractional seconds in interval strings",
                 "[datatype][string][conversion][interval][fractional]") {
  // Given Snowflake client is logged in

  // When Query selecting interval strings with fractional seconds is executed
  auto stmt = conn.execute_fetch(
      "SELECT '45.123' AS seconds_fractional, '12:30:45.999' AS hour_second_fractional, "
      "'2 08:15:30.500' AS day_second_fractional");

  // Then fractional seconds should be parsed correctly
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_SECOND>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_SECOND);
    CHECK(interval.intval.day_second.second == 45);
    CHECK(interval.intval.day_second.fraction == 123000);  // Microseconds
  }

  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_HOUR_TO_SECOND>(stmt, 2);
    CHECK(interval.interval_type == SQL_IS_HOUR_TO_SECOND);
    CHECK(interval.intval.day_second.hour == 12);
    CHECK(interval.intval.day_second.minute == 30);
    CHECK(interval.intval.day_second.second == 45);
    CHECK(interval.intval.day_second.fraction == 999000);  // Microseconds
  }

  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_DAY_TO_SECOND>(stmt, 3);
    CHECK(interval.interval_type == SQL_IS_DAY_TO_SECOND);
    CHECK(interval.intval.day_second.day == 2);
    CHECK(interval.intval.day_second.hour == 8);
    CHECK(interval.intval.day_second.minute == 15);
    CHECK(interval.intval.day_second.second == 30);
    CHECK(interval.intval.day_second.fraction == 500000);  // Microseconds
  }
}

// ============================================================================
// END OF SPEC SCAFFOLDING (legacy `[.skip]`-tagged tests above)
//
// The active tests below exercise the
// `crate::conversion::interval_str::varchar_to_interval` path that was
// added alongside this file and intentionally use a small, focused set
// of inputs so the matrix-coverage cells (VARCHAR × SQL_C_INTERVAL_*)
// can flip from RED ("not implemented") to GREEN. The legacy `[.skip]`
// scaffolding above documents the full ODBC Appendix D behavior and
// will be progressively un-skipped in follow-up PRs as we widen
// coverage of truncation / precision-loss / format-error paths.
// ============================================================================

TEST_CASE_METHOD(ConnSchemaFixture, "should fetch VARCHAR as single-field SQL_C_INTERVAL_*",
                 "[datatype][string][conversion][interval]") {
  // Given a VARCHAR row carrying bare integer interval values
  auto stmt = conn.execute_fetch(
      "SELECT '5' AS years, '10' AS months, '15' AS days, "
      "'8' AS hours, '30' AS minutes, '45' AS seconds");

  // When each column is fetched as the matching single-field SQL_C_INTERVAL_*
  // Then SQL_C_INTERVAL_YEAR reads year=5
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.year_month.year == 5);
  }
  // And SQL_C_INTERVAL_MONTH reads month=10
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_MONTH>(stmt, 2);
    CHECK(interval.interval_type == SQL_IS_MONTH);
    CHECK(interval.intval.year_month.month == 10);
  }
  // And SQL_C_INTERVAL_DAY reads day=15
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_DAY>(stmt, 3);
    CHECK(interval.interval_type == SQL_IS_DAY);
    CHECK(interval.intval.day_second.day == 15);
  }
  // And SQL_C_INTERVAL_HOUR reads hour=8
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_HOUR>(stmt, 4);
    CHECK(interval.interval_type == SQL_IS_HOUR);
    CHECK(interval.intval.day_second.hour == 8);
  }
  // And SQL_C_INTERVAL_MINUTE reads minute=30
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_MINUTE>(stmt, 5);
    CHECK(interval.interval_type == SQL_IS_MINUTE);
    CHECK(interval.intval.day_second.minute == 30);
  }
  // And SQL_C_INTERVAL_SECOND reads second=45
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_SECOND>(stmt, 6);
    CHECK(interval.interval_type == SQL_IS_SECOND);
    CHECK(interval.intval.day_second.second == 45);
    CHECK(interval.intval.day_second.fraction == 0);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should fetch VARCHAR as composite SQL_C_INTERVAL_YEAR_TO_MONTH",
                 "[datatype][string][conversion][interval]") {
  // Given a VARCHAR carrying a year-month interval literal
  auto stmt = conn.execute_fetch("SELECT '3-6' AS year_month");

  // When the column is fetched as SQL_C_INTERVAL_YEAR_TO_MONTH
  // Then both year and month components are populated
  auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 1);
  CHECK(interval.interval_type == SQL_IS_YEAR_TO_MONTH);
  CHECK(interval.interval_sign == SQL_FALSE);
  CHECK(interval.intval.year_month.year == 3);
  CHECK(interval.intval.year_month.month == 6);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should fetch VARCHAR as composite day-time SQL_C_INTERVAL_*",
                 "[datatype][string][conversion][interval]") {
  // Given VARCHARs carrying day-time interval literals
  auto stmt = conn.execute_fetch(
      "SELECT '5 10' AS day_hour, '3 14:30' AS day_minute, "
      "'2 08:15:30' AS day_second, '10:45' AS hour_minute, "
      "'12:30:45' AS hour_second, '45:30' AS minute_second");

  // When each column is fetched as the matching composite SQL_C_INTERVAL_*
  // Then every leading-to-trailing component is populated correctly
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_DAY_TO_HOUR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_DAY_TO_HOUR);
    CHECK(interval.intval.day_second.day == 5);
    CHECK(interval.intval.day_second.hour == 10);
  }
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_DAY_TO_MINUTE>(stmt, 2);
    CHECK(interval.interval_type == SQL_IS_DAY_TO_MINUTE);
    CHECK(interval.intval.day_second.day == 3);
    CHECK(interval.intval.day_second.hour == 14);
    CHECK(interval.intval.day_second.minute == 30);
  }
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_DAY_TO_SECOND>(stmt, 3);
    CHECK(interval.interval_type == SQL_IS_DAY_TO_SECOND);
    CHECK(interval.intval.day_second.day == 2);
    CHECK(interval.intval.day_second.hour == 8);
    CHECK(interval.intval.day_second.minute == 15);
    CHECK(interval.intval.day_second.second == 30);
  }
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_HOUR_TO_MINUTE>(stmt, 4);
    CHECK(interval.interval_type == SQL_IS_HOUR_TO_MINUTE);
    CHECK(interval.intval.day_second.hour == 10);
    CHECK(interval.intval.day_second.minute == 45);
  }
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_HOUR_TO_SECOND>(stmt, 5);
    CHECK(interval.interval_type == SQL_IS_HOUR_TO_SECOND);
    CHECK(interval.intval.day_second.hour == 12);
    CHECK(interval.intval.day_second.minute == 30);
    CHECK(interval.intval.day_second.second == 45);
  }
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_MINUTE_TO_SECOND>(stmt, 6);
    CHECK(interval.interval_type == SQL_IS_MINUTE_TO_SECOND);
    CHECK(interval.intval.day_second.minute == 45);
    CHECK(interval.intval.day_second.second == 30);
  }
}

TEST_CASE_METHOD(ConnSchemaFixture, "should fetch negative VARCHAR as SQL_C_INTERVAL_DAY",
                 "[datatype][string][conversion][interval]") {
  // Given a VARCHAR carrying a negative day count
  auto stmt = conn.execute_fetch("SELECT '-15' AS neg_days");

  // When the column is fetched as SQL_C_INTERVAL_DAY
  // Then SQL_C_INTERVAL_DAY preserves the negative sign
  auto interval = check_no_truncation<SQL_C_INTERVAL_DAY>(stmt, 1);
  CHECK(interval.interval_type == SQL_IS_DAY);
  CHECK(interval.interval_sign == SQL_TRUE);
  CHECK(interval.intval.day_second.day == 15);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should fetch fractional VARCHAR as SQL_C_INTERVAL_SECOND",
                 "[datatype][string][conversion][interval]") {
  // Given a VARCHAR carrying a sub-second value
  auto stmt = conn.execute_fetch("SELECT '12.500000' AS sec");

  // When the column is fetched as SQL_C_INTERVAL_SECOND
  // Then the second component is 12 and the fraction is 500_000 microseconds
  auto interval = check_no_truncation<SQL_C_INTERVAL_SECOND>(stmt, 1);
  CHECK(interval.interval_type == SQL_IS_SECOND);
  CHECK(interval.intval.day_second.second == 12);
  CHECK(interval.intval.day_second.fraction == 500000);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject malformed VARCHAR with SQLSTATE 22018 for SQL_C_INTERVAL_YEAR",
                 "[datatype][string][conversion][interval][negative]") {
  // Given a VARCHAR that is not a valid interval literal
  auto stmt = conn.execute_fetch("SELECT 'not-an-interval' AS bad");

  // When the column is fetched as SQL_C_INTERVAL_YEAR
  // Then SQLGetData fails with SQLSTATE 22018 (Invalid character value for cast)
  SQL_INTERVAL_STRUCT interval = {};
  SQLLEN indicator = -999;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_INTERVAL_YEAR, &interval, sizeof(interval), &indicator);
  CHECK(ret == SQL_ERROR);
  auto records = get_diag_rec(stmt);
  REQUIRE(!records.empty());
  CHECK(records[0].sqlState == "22018");
}

// ============================================================================
// EXTENDED GAP COVERAGE - Negative composites, sub-microsecond truncation,
// fractional-only sign, explicit zero fraction, wider malformed matrix.
//
// These exercise edge cases of `varchar_to_interval` that aren't reachable
// through the legacy scaffolding above. Pure-Rust equivalents already live in
// `odbc/src/conversion/interval_str_tests.rs`; the e2e variants here also
// confirm the dispatch from `SnowflakeVarchar::write_odbc` and the wire
// representation of the produced `SQL_INTERVAL_STRUCT`.
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

TEST_CASE_METHOD(ConnSchemaFixture, "should treat -0.0 SQL_C_INTERVAL_SECOND as positive zero",
                 "[datatype][string][conversion][interval][edge][fractional]") {
  // Given a VARCHAR carrying an explicit "negative zero" with an explicit zero fraction
  auto stmt = conn.execute_fetch("SELECT '-0.0' AS neg_zero_second");

  // When the column is fetched as SQL_C_INTERVAL_SECOND
  auto interval = check_no_truncation<SQL_C_INTERVAL_SECOND>(stmt, 1);

  // Then the sign bit is FALSE because the magnitude is exactly zero.
  CHECK(interval.interval_type == SQL_IS_SECOND);
  CHECK(interval.interval_sign == SQL_FALSE);
  CHECK(interval.intval.day_second.second == 0);
  CHECK(interval.intval.day_second.fraction == 0);
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

TEST_CASE_METHOD(ConnSchemaFixture, "should not warn when extra fractional digits are zero padding",
                 "[datatype][string][conversion][interval][fractional]") {
  // Given a VARCHAR whose extra fractional digits are all zeros (no actual precision lost)
  auto stmt = conn.execute_fetch("SELECT '12:30:45.1234560000' AS padded");

  // When fetched as SQL_C_INTERVAL_HOUR_TO_SECOND
  auto interval = check_no_truncation<SQL_C_INTERVAL_HOUR_TO_SECOND>(stmt, 1);

  // Then no truncation warning is emitted because the dropped digits are insignificant zeros.
  CHECK(interval.intval.day_second.fraction == 123456);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should treat explicit zero fraction in H:M as no fractional part",
                 "[datatype][string][conversion][interval][fractional][edge]") {
  // Given a VARCHAR formatted as H:M with an explicit ".0" fractional appendix
  auto stmt = conn.execute_fetch("SELECT '5:10.0' AS hm_zero_frac");

  // When the column is fetched as SQL_C_INTERVAL_HOUR_TO_MINUTE
  auto interval = check_no_truncation<SQL_C_INTERVAL_HOUR_TO_MINUTE>(stmt, 1);

  // Then the parser keeps the H:M shape and the explicit zero fraction is ignored
  CHECK(interval.interval_type == SQL_IS_HOUR_TO_MINUTE);
  CHECK(interval.intval.day_second.hour == 5);
  CHECK(interval.intval.day_second.minute == 10);
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject H:M with non-zero fraction for SQL_C_INTERVAL_HOUR_TO_MINUTE",
                 "[datatype][string][conversion][interval][negative][fractional]") {
  // Given a VARCHAR with a non-zero fraction ("5:10.125") that the parser must read as M:S.fraction
  auto stmt = conn.execute_fetch("SELECT '5:10.125' AS unambiguous_ms");

  // When the column is fetched as SQL_C_INTERVAL_HOUR_TO_MINUTE (which has no fraction field)
  SQL_INTERVAL_STRUCT interval = {};
  SQLLEN indicator = -999;
  SQLRETURN ret =
      SQLGetData(stmt.getHandle(), 1, SQL_C_INTERVAL_HOUR_TO_MINUTE, &interval, sizeof(interval), &indicator);

  // Then SQLGetData fails with SQLSTATE 22018 because the input is unambiguously M:S, not H:M
  CHECK(ret == SQL_ERROR);
  auto records = get_diag_rec(stmt);
  REQUIRE(!records.empty());
  CHECK(records[0].sqlState == "22018");
}

TEST_CASE_METHOD(ConnSchemaFixture, "should reject malformed VARCHAR for every composite SQL_C_INTERVAL_*",
                 "[datatype][string][conversion][interval][negative][failure]") {
  // Given a VARCHAR carrying a string that looks like an interval but uses wrong separators
  auto stmt = conn.execute_fetch("SELECT '12.34.56' AS dotted, '5/10' AS slashed, 'abc' AS letters");

  // When fetched as a variety of composite SQL_C_INTERVAL_* targets
  // Then SQLGetData fails with SQLSTATE 22018 in each case
  check_invalid_string<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 1);
  check_invalid_string<SQL_C_INTERVAL_DAY_TO_SECOND>(stmt, 2);
  check_invalid_string<SQL_C_INTERVAL_HOUR_TO_MINUTE>(stmt, 3);
}
