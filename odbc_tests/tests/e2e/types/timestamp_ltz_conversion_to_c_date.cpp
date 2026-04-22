#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"

TEST_CASE("TIMESTAMP_LTZ to SQL_C_TYPE_DATE", "[timestamp_ltz][conversion][c_date]") {
  // Given Snowflake client is logged in with a known session timezone
  Connection conn;
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");

  // When A TIMESTAMP_LTZ with midnight time is fetched as SQL_C_TYPE_DATE
  auto date =
      check_no_truncation<SQL_C_TYPE_DATE>(conn.execute_fetch("SELECT '2024-01-15 00:00:00'::TIMESTAMP_LTZ"), 1);

  // Then Date components are extracted without warning
  CHECK(date.year == 2024);
  CHECK(date.month == 1);
  CHECK(date.day == 15);
}

TEST_CASE("TIMESTAMP_LTZ to SQL_C_TYPE_DATE boundary values", "[timestamp_ltz][conversion][c_date]") {
  // Given Snowflake client is logged in with a known session timezone
  Connection conn;
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");

  // When Boundary TIMESTAMP_LTZ values are fetched as SQL_C_TYPE_DATE
  (void)0;
  // Then Date components match expected values
  {
    INFO("pre-epoch");
    auto date = check_fractional_truncation<SQL_C_TYPE_DATE>(
        conn.execute_fetch("SELECT '1960-06-15 12:00:00'::TIMESTAMP_LTZ"), 1);
    CHECK(date.year == 1960);
    CHECK(date.month == 6);
    CHECK(date.day == 15);
  }

  {
    INFO("leap day");
    auto date =
        check_no_truncation<SQL_C_TYPE_DATE>(conn.execute_fetch("SELECT '2000-02-29 00:00:00'::TIMESTAMP_LTZ"), 1);
    CHECK(date.year == 2000);
    CHECK(date.month == 2);
    CHECK(date.day == 29);
  }
}

TEST_CASE("TIMESTAMP_LTZ to SQL_C_TYPE_DATE with time truncation", "[timestamp_ltz][conversion][c_date][truncation]") {
  // Given Snowflake client is logged in with a known session timezone
  Connection conn;
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");

  // When A TIMESTAMP_LTZ with non-zero time is fetched as SQL_C_TYPE_DATE
  auto date = check_fractional_truncation<SQL_C_TYPE_DATE>(
      conn.execute_fetch("SELECT '2024-01-15 14:30:45'::TIMESTAMP_LTZ"), 1);

  // Then Date components are extracted with SQLSTATE 01S07 warning
  CHECK(date.year == 2024);
  CHECK(date.month == 1);
  CHECK(date.day == 15);
}

TEST_CASE("TIMESTAMP_LTZ NULL to SQL_C_TYPE_DATE", "[timestamp_ltz][conversion][c_date][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL TIMESTAMP_LTZ value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::TIMESTAMP_LTZ");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_TYPE_DATE);
}
