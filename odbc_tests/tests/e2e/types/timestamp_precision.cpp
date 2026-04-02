#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"

TEST_CASE("TIMESTAMP_NTZ precision 0 truncates fractional seconds", "[timestamp][precision][ntz]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When TIMESTAMP_NTZ(0) is queried with a fractional-second value
  auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
      conn.execute_fetch("SELECT '2024-01-15 14:30:45.987654321'::TIMESTAMP_NTZ(0)"), 1);

  // Then Fractional part is zero because precision 0 truncates sub-second digits
  CHECK(ts.hour == 14);
  CHECK(ts.minute == 30);
  CHECK(ts.second == 45);
  CHECK(ts.fraction == 0);
}

TEST_CASE("TIMESTAMP_NTZ precision 3 keeps milliseconds", "[timestamp][precision][ntz]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When TIMESTAMP_NTZ(3) is queried with nanosecond-precision input
  auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
      conn.execute_fetch("SELECT '2024-01-15 14:30:45.987654321'::TIMESTAMP_NTZ(3)"), 1);

  // Then Only millisecond precision is preserved (3 decimal digits, truncated)
  CHECK(ts.second == 45);
  CHECK(ts.fraction == 987000000);
}

TEST_CASE("TIMESTAMP_NTZ precision 6 keeps microseconds", "[timestamp][precision][ntz]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When TIMESTAMP_NTZ(6) is queried with nanosecond-precision input
  auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
      conn.execute_fetch("SELECT '2024-01-15 14:30:45.987654321'::TIMESTAMP_NTZ(6)"), 1);

  // Then Only microsecond precision is preserved (6 decimal digits)
  CHECK(ts.second == 45);
  CHECK(ts.fraction == 987654000);
}

TEST_CASE("TIMESTAMP_NTZ precision 9 keeps nanoseconds", "[timestamp][precision][ntz]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When TIMESTAMP_NTZ(9) is queried with full nanosecond-precision input
  auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
      conn.execute_fetch("SELECT '2024-01-15 14:30:45.987654321'::TIMESTAMP_NTZ(9)"), 1);

  // Then Full nanosecond precision is preserved
  CHECK(ts.second == 45);
  CHECK(ts.fraction == 987654321);
}

TEST_CASE("TIMESTAMP_LTZ precision variants", "[timestamp][precision][ltz]") {
  // Given Snowflake client is logged in with UTC timezone
  Connection conn;
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");

  // When TIMESTAMP_LTZ values at different precisions are fetched
  (void)0;
  // Then Fractional seconds are truncated according to declared precision
  {
    INFO("precision 0");
    auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
        conn.execute_fetch("SELECT '2024-01-15 14:30:45.987654321'::TIMESTAMP_LTZ(0)"), 1);
    CHECK(ts.second == 45);
    CHECK(ts.fraction == 0);
  }

  {
    INFO("precision 3");
    auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
        conn.execute_fetch("SELECT '2024-01-15 14:30:45.987654321'::TIMESTAMP_LTZ(3)"), 1);
    CHECK(ts.second == 45);
    CHECK(ts.fraction == 987000000);
  }

  {
    INFO("precision 6");
    auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
        conn.execute_fetch("SELECT '2024-01-15 14:30:45.987654321'::TIMESTAMP_LTZ(6)"), 1);
    CHECK(ts.second == 45);
    CHECK(ts.fraction == 987654000);
  }

  {
    INFO("precision 9");
    auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
        conn.execute_fetch("SELECT '2024-01-15 14:30:45.987654321'::TIMESTAMP_LTZ(9)"), 1);
    CHECK(ts.second == 45);
    CHECK(ts.fraction == 987654321);
  }
}

TEST_CASE("TIMESTAMP_TZ precision variants", "[timestamp][precision][tz]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When TIMESTAMP_TZ values at different precisions are fetched
  (void)0;
  // Then Fractional seconds are truncated according to declared precision
  {
    INFO("precision 0");
    auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
        conn.execute_fetch("SELECT '2024-01-15 14:30:45.987654321 +00:00'::TIMESTAMP_TZ(0)"), 1);
    CHECK(ts.second == 45);
    CHECK(ts.fraction == 0);
  }

  {
    INFO("precision 3");
    auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
        conn.execute_fetch("SELECT '2024-01-15 14:30:45.987654321 +00:00'::TIMESTAMP_TZ(3)"), 1);
    CHECK(ts.second == 45);
    CHECK(ts.fraction == 987000000);
  }

  {
    INFO("precision 6");
    auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
        conn.execute_fetch("SELECT '2024-01-15 14:30:45.987654321 +00:00'::TIMESTAMP_TZ(6)"), 1);
    CHECK(ts.second == 45);
    CHECK(ts.fraction == 987654000);
  }

  {
    INFO("precision 9");
    auto ts = check_no_truncation<SQL_C_TYPE_TIMESTAMP>(
        conn.execute_fetch("SELECT '2024-01-15 14:30:45.987654321 +00:00'::TIMESTAMP_TZ(9)"), 1);
    CHECK(ts.second == 45);
    CHECK(ts.fraction == 987654321);
  }
}

TEST_CASE("TIMESTAMP_NTZ precision affects SQL_C_CHAR output", "[timestamp][precision][c_char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When TIMESTAMP_NTZ at various precisions is fetched as SQL_C_CHAR
  (void)0;
  // Then String representation reflects the declared precision
  {
    INFO("precision 0 omits fractional part");
    auto result = check_char_success(conn.execute_fetch("SELECT '2024-01-15 14:30:45.987654321'::TIMESTAMP_NTZ(0)"), 1);
    CHECK(result == "2024-01-15 14:30:45");
  }

  {
    INFO("precision 3 shows milliseconds");
    auto result = check_char_success(conn.execute_fetch("SELECT '2024-01-15 14:30:45.987654321'::TIMESTAMP_NTZ(3)"), 1);
    CHECK(result == "2024-01-15 14:30:45.987");
  }

  {
    INFO("precision 9 shows nanoseconds");
    auto result = check_char_success(conn.execute_fetch("SELECT '2024-01-15 14:30:45.123456789'::TIMESTAMP_NTZ(9)"), 1);
    CHECK(result == "2024-01-15 14:30:45.123456789");
  }
}
