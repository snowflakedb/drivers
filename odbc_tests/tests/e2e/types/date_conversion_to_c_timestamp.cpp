#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"

TEST_CASE("DATE to SQL_C_TYPE_TIMESTAMP", "[date][conversion][c_timestamp]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A DATE value is fetched as SQL_C_TYPE_TIMESTAMP
  auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '2024-01-15'::DATE"), 1);

  // Then Date fields are populated and time fields are zero
  CHECK(ts.year == 2024);
  CHECK(ts.month == 1);
  CHECK(ts.day == 15);
  CHECK(ts.hour == 0);
  CHECK(ts.minute == 0);
  CHECK(ts.second == 0);
  CHECK(ts.fraction == 0);
}

TEST_CASE("DATE to SQL_C_TYPE_TIMESTAMP boundary values", "[date][conversion][c_timestamp]") {
  // Given Snowflake client is logged in
  Connection conn;

  {
    // When pre-epoch DATE is fetched as SQL_C_TYPE_TIMESTAMP
    auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '1960-06-15'::DATE"), 1);
    // Then Date fields are populated and time fields are zero
    CHECK(ts.year == 1960);
    CHECK(ts.month == 6);
    CHECK(ts.day == 15);
    CHECK(ts.hour == 0);
    CHECK(ts.minute == 0);
    CHECK(ts.second == 0);
    CHECK(ts.fraction == 0);
  }

  {
    // When leap day DATE is fetched as SQL_C_TYPE_TIMESTAMP
    auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '2000-02-29'::DATE"), 1);
    // Then Date fields are populated and time fields are zero
    CHECK(ts.year == 2000);
    CHECK(ts.month == 2);
    CHECK(ts.day == 29);
    CHECK(ts.hour == 0);
    CHECK(ts.minute == 0);
    CHECK(ts.second == 0);
    CHECK(ts.fraction == 0);
  }

  {
    // When epoch DATE is fetched as SQL_C_TYPE_TIMESTAMP
    auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '1970-01-01'::DATE"), 1);
    // Then Date fields are populated and time fields are zero
    CHECK(ts.year == 1970);
    CHECK(ts.month == 1);
    CHECK(ts.day == 1);
    CHECK(ts.hour == 0);
    CHECK(ts.minute == 0);
    CHECK(ts.second == 0);
    CHECK(ts.fraction == 0);
  }

  {
    // When end of year DATE is fetched as SQL_C_TYPE_TIMESTAMP
    auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '1999-12-31'::DATE"), 1);
    // Then Date fields are populated and time fields are zero
    CHECK(ts.year == 1999);
    CHECK(ts.month == 12);
    CHECK(ts.day == 31);
    CHECK(ts.hour == 0);
    CHECK(ts.minute == 0);
    CHECK(ts.second == 0);
    CHECK(ts.fraction == 0);
  }

  {
    // When first day of year DATE is fetched as SQL_C_TYPE_TIMESTAMP
    auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '2025-01-01'::DATE"), 1);
    // Then Date fields are populated and time fields are zero
    CHECK(ts.year == 2025);
    CHECK(ts.month == 1);
    CHECK(ts.day == 1);
    CHECK(ts.hour == 0);
    CHECK(ts.minute == 0);
    CHECK(ts.second == 0);
    CHECK(ts.fraction == 0);
  }
}

TEST_CASE("DATE to SQL_C_TYPE_TIMESTAMP far future", "[date][conversion][c_timestamp][edge]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When The maximum DATE value 9999-12-31 is fetched as SQL_C_TYPE_TIMESTAMP
  auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '9999-12-31'::DATE"), 1);

  // Then Date fields match 9999-12-31 and time fields are zero
  CHECK(ts.year == 9999);
  CHECK(ts.month == 12);
  CHECK(ts.day == 31);
  CHECK(ts.hour == 0);
  CHECK(ts.minute == 0);
  CHECK(ts.second == 0);
  CHECK(ts.fraction == 0);
}

TEST_CASE("DATE to SQL_C_TYPE_TIMESTAMP far past", "[date][conversion][c_timestamp][edge]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When The minimum DATE value 0001-01-01 is fetched as SQL_C_TYPE_TIMESTAMP
  auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '0001-01-01'::DATE"), 1);

  // Then Date fields match 0001-01-01 and time fields are zero
  CHECK(ts.year == 1);
  CHECK(ts.month == 1);
  CHECK(ts.day == 1);
  CHECK(ts.hour == 0);
  CHECK(ts.minute == 0);
  CHECK(ts.second == 0);
  CHECK(ts.fraction == 0);
}

TEST_CASE("DATE NULL to SQL_C_TYPE_TIMESTAMP", "[date][conversion][c_timestamp][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL DATE value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::DATE");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_TYPE_TIMESTAMP);
}
