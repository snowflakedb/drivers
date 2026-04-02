#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"
#include "get_data.hpp"

TEST_CASE("Connection-level TIMEZONE parameter via DSN", "[timestamp][connection][timezone]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Session timezone is verified via SHOW PARAMETERS
  auto stmt = conn.execute_fetch("SHOW PARAMETERS LIKE 'TIMEZONE'");
  auto tz_value = check_char_success(stmt, 2);

  // Then A valid timezone string is returned
  CHECK(!tz_value.empty());
}

TEST_CASE("TIMESTAMP_LTZ literal interpretation changes with session timezone",
          "[timestamp][connection][timezone][ltz]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A bare literal is cast to TIMESTAMP_LTZ under different session timezones
  (void)0;
  // Then The resulting UTC epoch differs because the literal is interpreted in the session timezone
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");
  auto ts_utc =
      check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '2024-06-15 12:00:00'::TIMESTAMP_LTZ"), 1);

  conn.execute("ALTER SESSION SET TIMEZONE = 'America/Los_Angeles'");
  auto ts_la =
      check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '2024-06-15 12:00:00'::TIMESTAMP_LTZ"), 1);

  CHECK(ts_utc.hour == 12);
  CHECK(ts_la.hour == 19);
}

TEST_CASE("TIMESTAMP_NTZ is unaffected by session timezone changes", "[timestamp][connection][timezone][ntz]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When The same NTZ literal is fetched with different session timezones
  (void)0;
  // Then The struct values are identical because NTZ has no timezone semantics
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");
  auto ts_utc =
      check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '2024-06-15 12:00:00'::TIMESTAMP_NTZ"), 1);

  conn.execute("ALTER SESSION SET TIMEZONE = 'Asia/Tokyo'");
  auto ts_tokyo =
      check_no_truncation<SQL_C_TYPE_TIMESTAMP>(conn.execute_fetch("SELECT '2024-06-15 12:00:00'::TIMESTAMP_NTZ"), 1);

  CHECK(ts_utc.year == ts_tokyo.year);
  CHECK(ts_utc.month == ts_tokyo.month);
  CHECK(ts_utc.day == ts_tokyo.day);
  CHECK(ts_utc.hour == ts_tokyo.hour);
  CHECK(ts_utc.minute == ts_tokyo.minute);
  CHECK(ts_utc.second == ts_tokyo.second);
}
