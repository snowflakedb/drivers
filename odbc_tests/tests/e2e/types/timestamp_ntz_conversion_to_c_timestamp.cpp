#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"

TEST_CASE("TIMESTAMP_NTZ to SQL_C_TYPE_TIMESTAMP basic values", "[timestamp_ntz][conversion][c_timestamp]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When TIMESTAMP_NTZ values are fetched as SQL_C_TYPE_TIMESTAMP
  (void)0;
  // Then SQL_TIMESTAMP_STRUCT fields match expected date and time components
  {
    INFO("basic timestamp");
    auto ts =
        check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '2024-01-15 14:30:45'::TIMESTAMP_NTZ"), 1);
    CHECK(ts.year == 2024);
    CHECK(ts.month == 1);
    CHECK(ts.day == 15);
    CHECK(ts.hour == 14);
    CHECK(ts.minute == 30);
    CHECK(ts.second == 45);
    CHECK(ts.fraction == 0);
  }

  {
    INFO("end of year");
    auto ts =
        check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '1999-12-31 23:59:59'::TIMESTAMP_NTZ"), 1);
    CHECK(ts.year == 1999);
    CHECK(ts.month == 12);
    CHECK(ts.day == 31);
    CHECK(ts.hour == 23);
    CHECK(ts.minute == 59);
    CHECK(ts.second == 59);
    CHECK(ts.fraction == 0);
  }

  {
    INFO("epoch");
    auto ts =
        check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '1970-01-01 00:00:00'::TIMESTAMP_NTZ"), 1);
    CHECK(ts.year == 1970);
    CHECK(ts.month == 1);
    CHECK(ts.day == 1);
    CHECK(ts.hour == 0);
    CHECK(ts.minute == 0);
    CHECK(ts.second == 0);
    CHECK(ts.fraction == 0);
  }

  {
    INFO("midnight");
    auto ts =
        check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '2024-06-15 00:00:00'::TIMESTAMP_NTZ"), 1);
    CHECK(ts.year == 2024);
    CHECK(ts.month == 6);
    CHECK(ts.day == 15);
    CHECK(ts.hour == 0);
    CHECK(ts.minute == 0);
    CHECK(ts.second == 0);
    CHECK(ts.fraction == 0);
  }

  {
    INFO("fractional seconds");
    auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
        conn.execute_fetch("SELECT '2024-01-15 10:30:00.123456789'::TIMESTAMP_NTZ"), 1);
    CHECK(ts.year == 2024);
    CHECK(ts.month == 1);
    CHECK(ts.day == 15);
    CHECK(ts.hour == 10);
    CHECK(ts.minute == 30);
    CHECK(ts.second == 0);
    CHECK(ts.fraction == 123456789);
  }

  {
    INFO("pre-epoch timestamp");
    auto ts =
        check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '1960-06-15 12:00:00'::TIMESTAMP_NTZ"), 1);
    CHECK(ts.year == 1960);
    CHECK(ts.month == 6);
    CHECK(ts.day == 15);
    CHECK(ts.hour == 12);
    CHECK(ts.minute == 0);
    CHECK(ts.second == 0);
    CHECK(ts.fraction == 0);
  }

  {
    INFO("leap day");
    auto ts =
        check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '2000-02-29 23:59:59'::TIMESTAMP_NTZ"), 1);
    CHECK(ts.year == 2000);
    CHECK(ts.month == 2);
    CHECK(ts.day == 29);
    CHECK(ts.hour == 23);
    CHECK(ts.minute == 59);
    CHECK(ts.second == 59);
    CHECK(ts.fraction == 0);
  }

  {
    INFO("single-digit fractional seconds");
    auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
        conn.execute_fetch("SELECT '2024-01-15 10:30:00.5'::TIMESTAMP_NTZ"), 1);
    CHECK(ts.year == 2024);
    CHECK(ts.fraction == 500000000);
  }
}

TEST_CASE("TIMESTAMP_NTZ NULL to SQL_C_TYPE_TIMESTAMP", "[timestamp_ntz][conversion][c_timestamp][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL TIMESTAMP_NTZ value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::TIMESTAMP_NTZ");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_TYPE_TIMESTAMP);
}
