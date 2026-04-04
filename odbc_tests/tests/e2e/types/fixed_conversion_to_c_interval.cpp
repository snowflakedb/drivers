#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "Schema.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "odbc_matchers.hpp"

// ============================================================================
// SUCCESSFUL CONVERSIONS - Single-component interval types
// ============================================================================

TEST_CASE("NUMBER to single-field interval types", "[datatype][number][interval]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Positive, negative, and zero NUMBER values are fetched as SQL_C_INTERVAL_YEAR
  auto stmt = conn.execute_fetch("SELECT 5::NUMBER(10,0)");

  // Then SQL_C_INTERVAL_YEAR should contain the correct year values with proper signs
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.year_month.year == 5);
  }
  {
    stmt = conn.execute_fetch("SELECT -3::NUMBER(10,0)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.interval_sign == SQL_TRUE);
    CHECK(interval.intval.year_month.year == 3);
  }
  {
    stmt = conn.execute_fetch("SELECT 0::NUMBER(10,0)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.year_month.year == 0);
  }

  // And SQL_C_INTERVAL_MONTH should contain the correct month value
  {
    stmt = conn.execute_fetch("SELECT 10::NUMBER(10,0)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_MONTH>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_MONTH);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.year_month.month == 10);
  }

  // And SQL_C_INTERVAL_DAY should contain the correct day value
  {
    stmt = conn.execute_fetch("SELECT 15::NUMBER(10,0)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_DAY>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_DAY);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.day == 15);
  }

  // And SQL_C_INTERVAL_HOUR should contain the correct hour value
  {
    stmt = conn.execute_fetch("SELECT 8::NUMBER(10,0)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_HOUR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_HOUR);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.hour == 8);
  }

  // And SQL_C_INTERVAL_MINUTE should contain the correct minute value
  {
    stmt = conn.execute_fetch("SELECT 30::NUMBER(10,0)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_MINUTE>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_MINUTE);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.minute == 30);
  }

  // And SQL_C_INTERVAL_SECOND should contain the correct second values including fractional parts
  {
    stmt = conn.execute_fetch("SELECT 45::NUMBER(10,0)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_SECOND>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_SECOND);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.second == 45);
    CHECK(interval.intval.day_second.fraction == 0);
  }
  {
    stmt = conn.execute_fetch("SELECT 45.500::DECIMAL(10,3)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_SECOND>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_SECOND);
    CHECK(interval.intval.day_second.second == 45);
    CHECK(interval.intval.day_second.fraction == 500000);
  }
  {
    stmt = conn.execute_fetch("SELECT -10.25::DECIMAL(10,2)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_SECOND>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_SECOND);
    CHECK(interval.interval_sign == SQL_TRUE);
    CHECK(interval.intval.day_second.second == 10);
    CHECK(interval.intval.day_second.fraction == 250000);
  }
}

// ============================================================================
// TRUNCATION WITH INFO - Fractional truncation (SQLSTATE 01S07)
// ============================================================================

TEST_CASE("NUMBER to interval - fractional truncation returns 01S07", "[datatype][number][interval][01S07]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When DECIMAL value with fractional part is fetched as SQL_C_INTERVAL_YEAR
  auto stmt = conn.execute_fetch("SELECT 5.7::DECIMAL(10,1)");

  // Then SQL_C_INTERVAL_YEAR should truncate the fraction and return 01S07
  {
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_YEAR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.intval.year_month.year == 5);
  }

  // And SQL_C_INTERVAL_MONTH should truncate the fraction and return 01S07
  {
    stmt = conn.execute_fetch("SELECT 10.3::DECIMAL(10,1)");
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_MONTH>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_MONTH);
    CHECK(interval.intval.year_month.month == 10);
  }

  // And SQL_C_INTERVAL_DAY should truncate the fraction and return 01S07
  {
    stmt = conn.execute_fetch("SELECT 15.9::DECIMAL(10,1)");
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_DAY>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_DAY);
    CHECK(interval.intval.day_second.day == 15);
  }

  // And SQL_C_INTERVAL_HOUR should truncate the fraction and return 01S07
  {
    stmt = conn.execute_fetch("SELECT 8.5::DECIMAL(10,1)");
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_HOUR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_HOUR);
    CHECK(interval.intval.day_second.hour == 8);
  }

  // And SQL_C_INTERVAL_MINUTE should truncate the fraction and return 01S07
  {
    stmt = conn.execute_fetch("SELECT 30.1::DECIMAL(10,1)");
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_MINUTE>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_MINUTE);
    CHECK(interval.intval.day_second.minute == 30);
  }
}

// ============================================================================
// TRUNCATION WITH INFO - Sub-microsecond truncation (SQLSTATE 01S07)
// ============================================================================

TEST_CASE("NUMBER to interval - sub-microsecond truncation returns 01S07", "[datatype][number][interval][01S07]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When DECIMAL with 9-digit fractional part is fetched as SQL_C_INTERVAL_SECOND
  auto stmt = conn.execute_fetch("SELECT 45.123456789::DECIMAL(12,9)");

  // Then Sub-microsecond digits should be truncated and 01S07 posted
  {
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_SECOND>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_SECOND);
    CHECK(interval.intval.day_second.second == 45);
    CHECK(interval.intval.day_second.fraction == 123456);
  }

  // And Exact microsecond values should succeed without warning
  {
    stmt = conn.execute_fetch("SELECT 45.123456000::DECIMAL(12,9)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_SECOND>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_SECOND);
    CHECK(interval.intval.day_second.second == 45);
    CHECK(interval.intval.day_second.fraction == 123456);
  }
}

// ============================================================================
// EDGE CASES - Negative zero handling
// ============================================================================

TEST_CASE("NUMBER to interval - no negative zero", "[datatype][number][interval][edge]") {
  SKIP_OLD_DRIVER("BD#17", "Old driver produces negative zero for interval types");
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Negative fractional DECIMAL is fetched as SQL_C_INTERVAL_YEAR
  auto stmt = conn.execute_fetch("SELECT -0.5::DECIMAL(10,1)");

  // Then SQL_C_INTERVAL_YEAR should have positive sign when truncated to zero
  {
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_YEAR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.year_month.year == 0);
  }

  // And SQL_C_INTERVAL_MONTH should have positive sign when truncated to zero
  {
    stmt = conn.execute_fetch("SELECT -0.3::DECIMAL(10,1)");
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_MONTH>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_MONTH);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.year_month.month == 0);
  }

  // And SQL_C_INTERVAL_DAY should have positive sign when truncated to zero
  {
    stmt = conn.execute_fetch("SELECT -0.9::DECIMAL(10,1)");
    auto interval = check_fractional_truncation<SQL_C_INTERVAL_DAY>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_DAY);
    CHECK(interval.interval_sign == SQL_FALSE);
    CHECK(interval.intval.day_second.day == 0);
  }

  // And SQL_C_INTERVAL_SECOND should keep negative sign when fraction is nonzero
  {
    stmt = conn.execute_fetch("SELECT -0.5::DECIMAL(10,1)");
    auto interval = check_no_truncation<SQL_C_INTERVAL_SECOND>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_SECOND);
    CHECK(interval.interval_sign == SQL_TRUE);
    CHECK(interval.intval.day_second.second == 0);
    CHECK(interval.intval.day_second.fraction == 500000);
  }
}

// ============================================================================
// ILLEGAL CONVERSIONS - Multi-field interval types (SQLSTATE 22015)
// ============================================================================

TEST_CASE("NUMBER to multi-field interval returns 22015", "[datatype][number][interval][22015]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query "SELECT 42::NUMBER(10,0)" is executed and fetched as SQL_C_INTERVAL_YEAR_TO_MONTH
  auto stmt = conn.execute_fetch("SELECT 42::NUMBER(10,0)");

  // Then SQL_C_INTERVAL_YEAR_TO_MONTH conversion should fail with 22015
  check_interval_precision_lost<SQL_C_INTERVAL_YEAR_TO_MONTH>(stmt, 1);

  // And SQL_C_INTERVAL_DAY_TO_HOUR conversion should fail with 22015
  check_interval_precision_lost<SQL_C_INTERVAL_DAY_TO_HOUR>(conn.execute_fetch("SELECT 42::NUMBER(10,0)"), 1);

  // And SQL_C_INTERVAL_DAY_TO_MINUTE conversion should fail with 22015
  check_interval_precision_lost<SQL_C_INTERVAL_DAY_TO_MINUTE>(conn.execute_fetch("SELECT 42::NUMBER(10,0)"), 1);

  // And SQL_C_INTERVAL_DAY_TO_SECOND conversion should fail with 22015
  check_interval_precision_lost<SQL_C_INTERVAL_DAY_TO_SECOND>(conn.execute_fetch("SELECT 42::NUMBER(10,0)"), 1);

  // And SQL_C_INTERVAL_HOUR_TO_MINUTE conversion should fail with 22015
  check_interval_precision_lost<SQL_C_INTERVAL_HOUR_TO_MINUTE>(conn.execute_fetch("SELECT 42::NUMBER(10,0)"), 1);

  // And SQL_C_INTERVAL_HOUR_TO_SECOND conversion should fail with 22015
  check_interval_precision_lost<SQL_C_INTERVAL_HOUR_TO_SECOND>(conn.execute_fetch("SELECT 42::NUMBER(10,0)"), 1);

  // And SQL_C_INTERVAL_MINUTE_TO_SECOND conversion should fail with 22015
  check_interval_precision_lost<SQL_C_INTERVAL_MINUTE_TO_SECOND>(conn.execute_fetch("SELECT 42::NUMBER(10,0)"), 1);
}

// ============================================================================
// NULL VALUE HANDLING
// ============================================================================

TEST_CASE("NUMBER to interval - NULL returns SQL_NULL_DATA", "[datatype][number][interval][null]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Query "SELECT NULL::NUMBER(10,0)" is executed and fetched as SQL_C_INTERVAL_YEAR
  auto stmt = conn.execute_fetch("SELECT NULL::NUMBER(10,0)");

  // Then SQL_C_INTERVAL_YEAR should return SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_INTERVAL_YEAR);

  // And SQL_C_INTERVAL_MONTH should return SQL_NULL_DATA
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::NUMBER(10,0)"), 1, SQL_C_INTERVAL_MONTH);

  // And SQL_C_INTERVAL_DAY should return SQL_NULL_DATA
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::NUMBER(10,0)"), 1, SQL_C_INTERVAL_DAY);

  // And SQL_C_INTERVAL_HOUR should return SQL_NULL_DATA
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::NUMBER(10,0)"), 1, SQL_C_INTERVAL_HOUR);

  // And SQL_C_INTERVAL_MINUTE should return SQL_NULL_DATA
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::NUMBER(10,0)"), 1, SQL_C_INTERVAL_MINUTE);

  // And SQL_C_INTERVAL_SECOND should return SQL_NULL_DATA
  check_null_via_get_data(conn.execute_fetch("SELECT NULL::NUMBER(10,0)"), 1, SQL_C_INTERVAL_SECOND);
}

// ============================================================================
// LEADING FIELD PRECISION - Default precision (SQLSTATE 22015)
// ============================================================================

TEST_CASE("NUMBER to interval - default precision rejects values >= 100", "[datatype][number][interval][precision]") {
  SKIP_OLD_DRIVER("BD#18", "Old driver does not enforce interval leading precision");
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When Value 99 is fetched as SQL_C_INTERVAL_YEAR with default precision
  auto stmt = conn.execute_fetch("SELECT 99::NUMBER(10,0)");

  // Then Value 99 should succeed for SQL_C_INTERVAL_YEAR with default precision
  {
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.intval.year_month.year == 99);
  }

  // And Value 100 should fail with 22015 for SQL_C_INTERVAL_YEAR
  {
    stmt = conn.execute_fetch("SELECT 100::NUMBER(10,0)");
    check_interval_precision_lost<SQL_C_INTERVAL_YEAR>(stmt, 1);
  }

  // And Value -100 should fail with 22015 for SQL_C_INTERVAL_DAY
  {
    stmt = conn.execute_fetch("SELECT -100::NUMBER(10,0)");
    check_interval_precision_lost<SQL_C_INTERVAL_DAY>(stmt, 1);
  }

  // And Value 100 should fail with 22015 for SQL_C_INTERVAL_SECOND
  {
    stmt = conn.execute_fetch("SELECT 100::NUMBER(10,0)");
    check_interval_precision_lost<SQL_C_INTERVAL_SECOND>(stmt, 1);
  }
}

// ============================================================================
// LEADING FIELD PRECISION - Custom precision via SQLSetDescField
// ============================================================================

TEST_CASE("NUMBER to interval - custom precision via SQLSetDescField",
          "[datatype][number][interval][precision][descriptor]") {
  SKIP_OLD_DRIVER("BD#18", "Old driver does not support SQL_DESC_DATETIME_INTERVAL_PRECISION");
  // Given Snowflake client is logged in
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When SQL_DESC_DATETIME_INTERVAL_PRECISION is set to 5 on the ARD
  {
    auto stmt = conn.execute_fetch("SELECT 99999::NUMBER(10,0)");
    SQLHDESC ard = SQL_NULL_HDESC;
    SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetDescField(ard, 1, SQL_DESC_DATETIME_INTERVAL_PRECISION, (SQLPOINTER)5, 0);
    REQUIRE(ret == SQL_SUCCESS);

    // Then Precision 5 should allow value 99999 for SQL_C_INTERVAL_YEAR
    auto interval = check_no_truncation<SQL_C_INTERVAL_YEAR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_YEAR);
    CHECK(interval.intval.year_month.year == 99999);
  }

  // And Precision 5 should reject value 100000 for SQL_C_INTERVAL_YEAR
  {
    auto stmt = conn.execute_fetch("SELECT 100000::NUMBER(10,0)");
    SQLHDESC ard = SQL_NULL_HDESC;
    SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetDescField(ard, 1, SQL_DESC_DATETIME_INTERVAL_PRECISION, (SQLPOINTER)5, 0);
    REQUIRE(ret == SQL_SUCCESS);
    check_interval_precision_lost<SQL_C_INTERVAL_YEAR>(stmt, 1);
  }

  // And Precision 1 should allow value 9 for SQL_C_INTERVAL_HOUR
  {
    auto stmt = conn.execute_fetch("SELECT 9::NUMBER(10,0)");
    SQLHDESC ard = SQL_NULL_HDESC;
    SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetDescField(ard, 1, SQL_DESC_DATETIME_INTERVAL_PRECISION, (SQLPOINTER)1, 0);
    REQUIRE(ret == SQL_SUCCESS);
    auto interval = check_no_truncation<SQL_C_INTERVAL_HOUR>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_HOUR);
    CHECK(interval.intval.day_second.hour == 9);
  }

  // And Precision 1 should reject value 10 for SQL_C_INTERVAL_HOUR
  {
    auto stmt = conn.execute_fetch("SELECT 10::NUMBER(10,0)");
    SQLHDESC ard = SQL_NULL_HDESC;
    SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetDescField(ard, 1, SQL_DESC_DATETIME_INTERVAL_PRECISION, (SQLPOINTER)1, 0);
    REQUIRE(ret == SQL_SUCCESS);
    check_interval_precision_lost<SQL_C_INTERVAL_HOUR>(stmt, 1);
  }

  // And Precision 9 should allow value 999999999 for SQL_C_INTERVAL_SECOND
  {
    auto stmt = conn.execute_fetch("SELECT 999999999::NUMBER(10,0)");
    SQLHDESC ard = SQL_NULL_HDESC;
    SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, NULL);
    REQUIRE(ret == SQL_SUCCESS);
    ret = SQLSetDescField(ard, 1, SQL_DESC_DATETIME_INTERVAL_PRECISION, (SQLPOINTER)9, 0);
    REQUIRE(ret == SQL_SUCCESS);
    auto interval = check_no_truncation<SQL_C_INTERVAL_SECOND>(stmt, 1);
    CHECK(interval.interval_type == SQL_IS_SECOND);
    CHECK(interval.intval.day_second.second == 999999999);
    CHECK(interval.intval.day_second.fraction == 0);
  }
}

TEST_CASE("NUMBER NULL to SQL_C_INTERVAL types", "[fixed][conversion][c_interval][null]") {
  // Given A Snowflake connection is established
  Connection conn;
  auto random_schema = Schema::use_random_schema(conn);

  // When A NULL NUMBER value is queried
  const auto query = "SELECT NULL::NUMBER(10,0)";
  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(conn.execute_fetch(query), 1, SQL_C_INTERVAL_YEAR);
  check_null_via_get_data(conn.execute_fetch(query), 1, SQL_C_INTERVAL_MONTH);
}
