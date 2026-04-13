#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"

TEST_CASE("DATE to SQL_C_TYPE_DATE", "[date][conversion][c_date]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A DATE value is fetched as SQL_C_TYPE_DATE
  auto date = check_no_truncation<SQL_C_TYPE_DATE>(conn.execute_fetch("SELECT '2024-01-15'::DATE"), 1);

  // Then Date components match the source value
  CHECK(date.year == 2024);
  CHECK(date.month == 1);
  CHECK(date.day == 15);
}

TEST_CASE("DATE to SQL_C_TYPE_DATE boundary values", "[date][conversion][c_date]") {
  // Given Snowflake client is logged in
  Connection conn;

  {
    // When pre-epoch DATE is fetched as SQL_C_TYPE_DATE
    auto date = check_no_truncation<SQL_C_TYPE_DATE>(conn.execute_fetch("SELECT '1960-06-15'::DATE"), 1);

    // Then Date components match expected values
    CHECK(date.year == 1960);
    CHECK(date.month == 6);
    CHECK(date.day == 15);
  }

  {
    // When Leap day DATE is fetched as SQL_C_TYPE_DATE
    auto date = check_no_truncation<SQL_C_TYPE_DATE>(conn.execute_fetch("SELECT '2000-02-29'::DATE"), 1);

    // Then Date components match expected values
    CHECK(date.year == 2000);
    CHECK(date.month == 2);
    CHECK(date.day == 29);
  }

  {
    // When End-of-year DATE is fetched as SQL_C_TYPE_DATE
    auto date = check_no_truncation<SQL_C_TYPE_DATE>(conn.execute_fetch("SELECT '1999-12-31'::DATE"), 1);

    // Then Date components match expected values
    CHECK(date.year == 1999);
    CHECK(date.month == 12);
    CHECK(date.day == 31);
  }

  {
    // When Epoch DATE is fetched as SQL_C_TYPE_DATE
    auto date = check_no_truncation<SQL_C_TYPE_DATE>(conn.execute_fetch("SELECT '1970-01-01'::DATE"), 1);

    // Then Date components match expected values
    CHECK(date.year == 1970);
    CHECK(date.month == 1);
    CHECK(date.day == 1);
  }
}

TEST_CASE("DATE NULL to SQL_C_TYPE_DATE", "[date][conversion][c_date][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL DATE value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::DATE");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_TYPE_DATE);
}

// ============================================================================
// SQL_C_DEFAULT
// ============================================================================

TEST_CASE("DATE to SQL_C_DEFAULT", "[date][conversion][c_default]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A DATE value is fetched as SQL_C_DEFAULT
  auto stmt = conn.execute_fetch("SELECT '2024-01-15'::DATE");
  SQL_DATE_STRUCT date = {};
  SQLLEN indicator = -999;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DEFAULT, &date, sizeof(date), &indicator);

  // Then SQL_C_DEFAULT resolves to SQL_C_TYPE_DATE with correct values
  CHECK(ret == SQL_SUCCESS);
  CHECK(indicator == sizeof(SQL_DATE_STRUCT));
  CHECK(date.year == 2024);
  CHECK(date.month == 1);
  CHECK(date.day == 15);
}

TEST_CASE("DATE NULL to SQL_C_DEFAULT", "[date][conversion][c_default][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL DATE value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::DATE");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_DEFAULT);
}
