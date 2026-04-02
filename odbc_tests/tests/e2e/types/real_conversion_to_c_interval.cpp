#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "odbc_matchers.hpp"

// ============================================================================
// SUCCESSFUL CONVERSIONS - Single-component interval types
// ============================================================================

TEST_CASE("FLOAT to single-field interval types", "[datatype][float][conversion][c_interval]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Positive, negative, and zero FLOAT values are fetched as interval types
  (void)0;
  // Then Each single-field interval type returns the correct value and sign
  {
    INFO("SQL_C_INTERVAL_YEAR positive");
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(conn.execute_fetch("SELECT 5::FLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.year_month.year == 5);
  }
  {
    INFO("SQL_C_INTERVAL_YEAR negative");
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(conn.execute_fetch("SELECT (-3)::FLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.interval_sign == SQL_TRUE);
    CHECK(interval.intval.year_month.year == 3);
  }
  {
    INFO("SQL_C_INTERVAL_YEAR zero");
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(conn.execute_fetch("SELECT 0::FLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.year_month.year == 0);
  }
  {
    INFO("SQL_C_INTERVAL_MONTH");
    auto interval = check_no_truncation<SQL_C_INTERVAL_MONTH>(conn.execute_fetch("SELECT 10::FLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_MONTH);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.year_month.month == 10);
  }
  {
    INFO("SQL_C_INTERVAL_DAY");
    auto interval = check_no_truncation<SQL_C_INTERVAL_DAY>(conn.execute_fetch("SELECT 15::FLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_DAY);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.day == 15);
  }
  {
    INFO("SQL_C_INTERVAL_HOUR");
    auto interval = check_no_truncation<SQL_C_INTERVAL_HOUR>(conn.execute_fetch("SELECT 8::FLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_HOUR);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.hour == 8);
  }
  {
    INFO("SQL_C_INTERVAL_MINUTE");
    auto interval = check_no_truncation<SQL_C_INTERVAL_MINUTE>(conn.execute_fetch("SELECT 30::FLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_MINUTE);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.minute == 30);
  }
  {
    INFO("SQL_C_INTERVAL_SECOND integer");
    auto interval = check_no_truncation<SQL_C_INTERVAL_SECOND>(conn.execute_fetch("SELECT 45::FLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_SECOND);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.second == 45);
    CHECK(interval.intval.day_second.fraction == 0);
  }
  {
    INFO("SQL_C_INTERVAL_SECOND with fraction");
    auto interval = check_no_truncation<SQL_C_INTERVAL_SECOND>(conn.execute_fetch("SELECT 45.5::FLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_SECOND);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.second == 45);
    CHECK(interval.intval.day_second.fraction == 500000);
  }
  {
    INFO("SQL_C_INTERVAL_SECOND negative with fraction");
    auto interval = check_no_truncation<SQL_C_INTERVAL_SECOND>(conn.execute_fetch("SELECT (-10.25)::FLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_SECOND);
    CHECK(interval.interval_sign == SQL_TRUE);
    CHECK(interval.intval.day_second.second == 10);
    CHECK(interval.intval.day_second.fraction == 250000);
  }
  {
    INFO("SQL_C_INTERVAL_SECOND exact microseconds (0.125 = 125000)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_SECOND>(conn.execute_fetch("SELECT 45.125::FLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_SECOND);
    CHECK(interval.intval.day_second.second == 45);
    CHECK(interval.intval.day_second.fraction == 125000);
  }
}

// ============================================================================
// FRACTIONAL TRUNCATION (SQLSTATE 01S07)
// ============================================================================

TEST_CASE("FLOAT fractional truncation to interval types", "[datatype][float][conversion][c_interval][01S07]") {
  SKIP_OLD_DRIVER("BD#16", "Old driver does not report 01S07 for FLOAT-to-interval fractional truncation");
  // Given Snowflake client is logged in
  Connection conn;

  // When Fractional FLOAT values are fetched as non-second interval types
  (void)0;
  // Then The fractional part is truncated and SQLSTATE 01S07 is returned
  {
    INFO("SQL_C_INTERVAL_YEAR truncates fraction");
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_YEAR>(conn.execute_fetch("SELECT 5.7::FLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.intval.year_month.year == 5);
  }
  {
    INFO("SQL_C_INTERVAL_MONTH truncates fraction");
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_MONTH>(conn.execute_fetch("SELECT 10.3::FLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_MONTH);
    CHECK(interval.intval.year_month.month == 10);
  }
  {
    INFO("SQL_C_INTERVAL_DAY truncates fraction");
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_DAY>(conn.execute_fetch("SELECT 15.9::FLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_DAY);
    CHECK(interval.intval.day_second.day == 15);
  }
  {
    INFO("SQL_C_INTERVAL_HOUR truncates fraction");
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_HOUR>(conn.execute_fetch("SELECT 8.5::FLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_HOUR);
    CHECK(interval.intval.day_second.hour == 8);
  }
  {
    INFO("SQL_C_INTERVAL_MINUTE truncates fraction");
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_MINUTE>(conn.execute_fetch("SELECT 30.1::FLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_MINUTE);
    CHECK(interval.intval.day_second.minute == 30);
  }
}

// ============================================================================
// MULTI-FIELD INTERVAL TYPES (SQLSTATE 22015)
// ============================================================================

TEST_CASE("FLOAT to multi-field interval returns 22015", "[datatype][float][conversion][c_interval][22015]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A FLOAT value is fetched as multi-field interval types
  (void)0;
  // Then All multi-field interval conversions fail with SQLSTATE 22015
  check_interval_precision_lost<SQL_C_INTERVAL_YEAR_TO_MONTH>(conn.execute_fetch("SELECT 42::FLOAT"), 1);
  check_interval_precision_lost<SQL_C_INTERVAL_DAY_TO_HOUR>(conn.execute_fetch("SELECT 42::FLOAT"), 1);
  check_interval_precision_lost<SQL_C_INTERVAL_DAY_TO_MINUTE>(conn.execute_fetch("SELECT 42::FLOAT"), 1);
  check_interval_precision_lost<SQL_C_INTERVAL_DAY_TO_SECOND>(conn.execute_fetch("SELECT 42::FLOAT"), 1);
  check_interval_precision_lost<SQL_C_INTERVAL_HOUR_TO_MINUTE>(conn.execute_fetch("SELECT 42::FLOAT"), 1);
  check_interval_precision_lost<SQL_C_INTERVAL_HOUR_TO_SECOND>(conn.execute_fetch("SELECT 42::FLOAT"), 1);
  check_interval_precision_lost<SQL_C_INTERVAL_MINUTE_TO_SECOND>(conn.execute_fetch("SELECT 42::FLOAT"), 1);
}

// ============================================================================
// SUB-MICROSECOND TRUNCATION (SQLSTATE 01S07)
// ============================================================================

TEST_CASE("FLOAT sub-microsecond truncation to interval second", "[datatype][float][conversion][c_interval][01S07]") {
  SKIP_OLD_DRIVER("BD#16", "Old driver does not report 01S07 for FLOAT-to-interval fractional truncation");
  // Given Snowflake client is logged in
  Connection conn;

  // When FLOAT values with sub-microsecond precision are fetched as SQL_C_INTERVAL_SECOND
  (void)0;
  // Then Sub-microsecond digits are truncated and SQLSTATE 01S07 is returned
  {
    INFO("Sub-microsecond fraction truncated");
    auto interval =
        check_fractional_truncation<SQL_C_INTERVAL_SECOND>(conn.execute_fetch("SELECT 45.1234567::FLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_SECOND);
    CHECK(interval.intval.day_second.second == 45);
    CHECK(interval.intval.day_second.fraction == 123456);
  }
}

// ============================================================================
// EDGE CASES - No negative zero
// ============================================================================

TEST_CASE("FLOAT to interval - no negative zero", "[datatype][float][conversion][c_interval][edge]") {
  SKIP_OLD_DRIVER("BD#16", "Old driver does not report 01S07 for FLOAT-to-interval fractional truncation");
  // Given Snowflake client is logged in
  Connection conn;

  // When Negative fractional FLOAT values truncate to zero for non-second intervals
  (void)0;
  // Then Interval sign is positive when the integer part truncates to zero
  {
    INFO("SQL_C_INTERVAL_YEAR -0.5 truncates to +0");
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_YEAR>(conn.execute_fetch("SELECT (-0.5)::FLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.year_month.year == 0);
  }
  {
    INFO("SQL_C_INTERVAL_MONTH -0.3 truncates to +0");
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_MONTH>(conn.execute_fetch("SELECT (-0.3)::FLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_MONTH);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.year_month.month == 0);
  }
  {
    INFO("SQL_C_INTERVAL_DAY -0.9 truncates to +0");
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_DAY>(conn.execute_fetch("SELECT (-0.9)::FLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_DAY);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.day == 0);
  }
  {
    INFO("SQL_C_INTERVAL_SECOND -0.5 keeps negative (fraction nonzero)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_SECOND>(conn.execute_fetch("SELECT (-0.5)::FLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_SECOND);
    CHECK(interval.interval_sign == SQL_TRUE);
    CHECK(interval.intval.day_second.second == 0);
    CHECK(interval.intval.day_second.fraction == 500000);
  }
}

// ============================================================================
// LEADING FIELD PRECISION - Default precision (SQLSTATE 22015)
// ============================================================================

TEST_CASE("FLOAT to interval - default precision rejects values >= 100",
          "[datatype][float][conversion][c_interval][precision]") {
  SKIP_OLD_DRIVER("BD#18", "Old driver does not enforce interval leading precision");
  // Given Snowflake client is logged in
  Connection conn;

  // When FLOAT values at and beyond the default 2-digit precision are fetched as intervals
  (void)0;
  // Then Value 99 succeeds and value 100 fails with SQLSTATE 22015
  {
    INFO("99 fits in default precision 2");
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(conn.execute_fetch("SELECT 99::FLOAT"), 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.intval.year_month.year == 99);
  }
  {
    INFO("100 exceeds default precision for YEAR");
    check_interval_precision_lost<SQL_C_INTERVAL_YEAR>(conn.execute_fetch("SELECT 100::FLOAT"), 1);
  }
  {
    INFO("-100 exceeds default precision for DAY");
    check_interval_precision_lost<SQL_C_INTERVAL_DAY>(conn.execute_fetch("SELECT (-100)::FLOAT"), 1);
  }
  {
    INFO("100 exceeds default precision for SECOND");
    check_interval_precision_lost<SQL_C_INTERVAL_SECOND>(conn.execute_fetch("SELECT 100::FLOAT"), 1);
  }
}

// ============================================================================
// LEADING FIELD PRECISION - Custom precision via SQLSetDescField
// ============================================================================

TEST_CASE("FLOAT to interval - custom precision via SQLSetDescField",
          "[datatype][float][conversion][c_interval][precision][descriptor]") {
  SKIP_OLD_DRIVER("BD#18", "Old driver does not support SQL_DESC_DATETIME_INTERVAL_PRECISION");
  // Given Snowflake client is logged in
  Connection conn;

  // When SQL_DESC_DATETIME_INTERVAL_PRECISION is set on the ARD
  (void)0;
  // Then Values within custom precision succeed and values beyond it fail
  {
    INFO("Precision 5 allows 99999 for YEAR");
    auto stmt = conn.execute_fetch("SELECT 99999::FLOAT");
    SQLHDESC ard = SQL_NULL_HDESC;
    SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
    REQUIRE_ODBC(ret, stmt);
    ret = SQLSetDescField(ard, 1, SQL_DESC_DATETIME_INTERVAL_PRECISION, (SQLPOINTER)5, 0);
    REQUIRE(ret == SQL_SUCCESS);

    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.intval.year_month.year == 99999);
  }
  {
    INFO("Precision 5 rejects 100000 for YEAR");
    auto stmt = conn.execute_fetch("SELECT 100000::FLOAT");
    SQLHDESC ard = SQL_NULL_HDESC;
    SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
    REQUIRE_ODBC(ret, stmt);
    ret = SQLSetDescField(ard, 1, SQL_DESC_DATETIME_INTERVAL_PRECISION, (SQLPOINTER)5, 0);
    REQUIRE(ret == SQL_SUCCESS);
    check_interval_precision_lost<SQL_C_INTERVAL_YEAR>(stmt, 1);
  }
  {
    INFO("Precision 1 allows 9 for HOUR");
    auto stmt = conn.execute_fetch("SELECT 9::FLOAT");
    SQLHDESC ard = SQL_NULL_HDESC;
    SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
    REQUIRE_ODBC(ret, stmt);
    ret = SQLSetDescField(ard, 1, SQL_DESC_DATETIME_INTERVAL_PRECISION, (SQLPOINTER)1, 0);
    REQUIRE(ret == SQL_SUCCESS);
    auto interval = check_no_truncation<SQL_C_INTERVAL_HOUR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_HOUR);
    CHECK(interval.intval.day_second.hour == 9);
  }
  {
    INFO("Precision 1 rejects 10 for HOUR");
    auto stmt = conn.execute_fetch("SELECT 10::FLOAT");
    SQLHDESC ard = SQL_NULL_HDESC;
    SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
    REQUIRE_ODBC(ret, stmt);
    ret = SQLSetDescField(ard, 1, SQL_DESC_DATETIME_INTERVAL_PRECISION, (SQLPOINTER)1, 0);
    REQUIRE(ret == SQL_SUCCESS);
    check_interval_precision_lost<SQL_C_INTERVAL_HOUR>(stmt, 1);
  }
  {
    INFO("Precision 9 allows 999999999 for SECOND");
    auto stmt = conn.execute_fetch("SELECT 999999999::FLOAT");
    SQLHDESC ard = SQL_NULL_HDESC;
    SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
    REQUIRE_ODBC(ret, stmt);
    ret = SQLSetDescField(ard, 1, SQL_DESC_DATETIME_INTERVAL_PRECISION, (SQLPOINTER)9, 0);
    REQUIRE(ret == SQL_SUCCESS);
    auto interval = check_no_truncation<SQL_C_INTERVAL_SECOND>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_SECOND);
    CHECK(interval.intval.day_second.second == 999999999);
    CHECK(interval.intval.day_second.fraction == 0);
  }
}

// ============================================================================
// NULL handling
// ============================================================================

TEST_CASE("FLOAT NULL to interval C types", "[datatype][float][conversion][c_interval][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL FLOAT value is queried
  (void)0;
  // Then Indicator returns SQL_NULL_DATA for all single-field interval types
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::FLOAT"), 1, SQL_C_INTERVAL_YEAR);
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::FLOAT"), 1, SQL_C_INTERVAL_MONTH);
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::FLOAT"), 1, SQL_C_INTERVAL_DAY);
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::FLOAT"), 1, SQL_C_INTERVAL_HOUR);
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::FLOAT"), 1, SQL_C_INTERVAL_MINUTE);
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::FLOAT"), 1, SQL_C_INTERVAL_SECOND);
}

// ============================================================================
// NaN/Infinity edge cases
// ============================================================================

TEST_CASE("FLOAT NaN to interval returns error", "[datatype][float][conversion][c_interval][edge]") {
  SKIP_OLD_DRIVER("BD#16", "Old driver silently converts NaN to 0 instead of returning 22003");
  // Given Snowflake client is logged in
  Connection conn;

  // When A NaN FLOAT value is fetched as interval types
  (void)0;
  // Then NaN conversion fails with numeric out of range error
  check_numeric_out_of_range<SQL_C_INTERVAL_YEAR>(conn.execute_fetch("SELECT 'NaN'::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_INTERVAL_SECOND>(conn.execute_fetch("SELECT 'NaN'::FLOAT"), 1);
}

TEST_CASE("FLOAT Infinity to interval returns error", "[datatype][float][conversion][c_interval][edge]") {
  SKIP_OLD_DRIVER("BD#16", "Old driver silently converts Infinity instead of returning 22003");
  // Given Snowflake client is logged in
  Connection conn;

  // When Positive and negative Infinity FLOAT values are fetched as interval types
  (void)0;
  // Then Infinity conversion fails with numeric out of range error
  check_numeric_out_of_range<SQL_C_INTERVAL_YEAR>(conn.execute_fetch("SELECT 'Infinity'::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_INTERVAL_SECOND>(conn.execute_fetch("SELECT 'Infinity'::FLOAT"), 1);

  // And Negative infinity also fails
  check_numeric_out_of_range<SQL_C_INTERVAL_YEAR>(conn.execute_fetch("SELECT '-Infinity'::FLOAT"), 1);
  check_numeric_out_of_range<SQL_C_INTERVAL_SECOND>(conn.execute_fetch("SELECT '-Infinity'::FLOAT"), 1);
}
