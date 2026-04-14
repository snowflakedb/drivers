#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <ctime>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_diag_rec.hpp"

static void get_local_date(int& year, int& month, int& day) {
  std::time_t now = std::time(nullptr);
  std::tm* local = std::localtime(&now);
  year = local->tm_year + 1900;
  month = local->tm_mon + 1;
  day = local->tm_mday;
}

TEST_CASE("TIME to SQL_C_TYPE_TIMESTAMP", "[time][conversion][c_timestamp]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIME value is fetched as SQL_C_TYPE_TIMESTAMP
  int today_y, today_m, today_d;
  get_local_date(today_y, today_m, today_d);
  auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '14:30:45'::TIME"), 1);

  // Then Time fields are populated and date fields are set to current date
  CHECK(ts.year == today_y);
  CHECK(ts.month == today_m);
  CHECK(ts.day == today_d);
  CHECK(ts.hour == 14);
  CHECK(ts.minute == 30);
  CHECK(ts.second == 45);
  CHECK(ts.fraction == 0);
}

TEST_CASE("TIME to SQL_C_TYPE_TIMESTAMP midnight", "[time][conversion][c_timestamp]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Midnight TIME is fetched as SQL_C_TYPE_TIMESTAMP
  int today_y, today_m, today_d;
  get_local_date(today_y, today_m, today_d);
  auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '00:00:00'::TIME"), 1);

  // Then All time components are zero and date is current date
  CHECK(ts.year == today_y);
  CHECK(ts.month == today_m);
  CHECK(ts.day == today_d);
  CHECK(ts.hour == 0);
  CHECK(ts.minute == 0);
  CHECK(ts.second == 0);
  CHECK(ts.fraction == 0);
}

TEST_CASE("TIME to SQL_C_TYPE_TIMESTAMP end of day", "[time][conversion][c_timestamp]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When End-of-day TIME is fetched as SQL_C_TYPE_TIMESTAMP
  int today_y, today_m, today_d;
  get_local_date(today_y, today_m, today_d);
  auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '23:59:59'::TIME"), 1);

  // Then Time components match 23:59:59 and date is current date
  CHECK(ts.year == today_y);
  CHECK(ts.month == today_m);
  CHECK(ts.day == today_d);
  CHECK(ts.hour == 23);
  CHECK(ts.minute == 59);
  CHECK(ts.second == 59);
  CHECK(ts.fraction == 0);
}

TEST_CASE("TIME to SQL_C_TYPE_TIMESTAMP with fractional truncation", "[time][conversion][c_timestamp][truncation]") {
  SKIP_OLD_DRIVER("BD#42", "old driver does not report 01S07 for fractional seconds");
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIME with non-zero fractional seconds is fetched as SQL_C_TYPE_TIMESTAMP
  auto ts = check_fractional_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '14:30:45.123'::TIME"), 1);

  // Then Time components are extracted with SQLSTATE 01S07 warning and fraction is zero
  CHECK(ts.hour == 14);
  CHECK(ts.minute == 30);
  CHECK(ts.second == 45);
  CHECK(ts.fraction == 0);
}

TEST_CASE("TIME to SQL_C_TYPE_TIMESTAMP with high-precision fractional truncation",
          "[time][conversion][c_timestamp][truncation]") {
  SKIP_OLD_DRIVER("BD#42", "old driver does not report 01S07 for fractional seconds");
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIME with high-precision fractional seconds is fetched as SQL_C_TYPE_TIMESTAMP
  auto ts =
      check_fractional_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '10:30:00.123456789'::TIME"), 1);

  // Then Time components are extracted with SQLSTATE 01S07 warning and fraction is zero
  CHECK(ts.hour == 10);
  CHECK(ts.minute == 30);
  CHECK(ts.second == 0);
  CHECK(ts.fraction == 0);
}

TEST_CASE("TIME to SQL_C_TYPE_TIMESTAMP with zero fractional seconds", "[time][conversion][c_timestamp]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIME with ".000" fractional seconds is fetched as SQL_C_TYPE_TIMESTAMP
  auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '14:30:45.000'::TIME"), 1);

  // Then No truncation warning is returned and fraction is zero
  CHECK(ts.hour == 14);
  CHECK(ts.minute == 30);
  CHECK(ts.second == 45);
  CHECK(ts.fraction == 0);
}

TEST_CASE("TIME to SQL_C_TYPE_TIMESTAMP single-digit components", "[time][conversion][c_timestamp]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIME with single-digit hour, minute, second is fetched as SQL_C_TYPE_TIMESTAMP
  int today_y, today_m, today_d;
  get_local_date(today_y, today_m, today_d);
  auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '01:02:03'::TIME"), 1);

  // Then Time components match and date is current date
  CHECK(ts.year == today_y);
  CHECK(ts.month == today_m);
  CHECK(ts.day == today_d);
  CHECK(ts.hour == 1);
  CHECK(ts.minute == 2);
  CHECK(ts.second == 3);
  CHECK(ts.fraction == 0);
}

TEST_CASE("TIME NULL to SQL_C_TYPE_TIMESTAMP", "[time][conversion][c_timestamp][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL TIME value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::TIME");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_TYPE_TIMESTAMP);
}
