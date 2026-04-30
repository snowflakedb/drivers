#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_diag_rec.hpp"

TEST_CASE("TIMESTAMP_TZ to SQL_C_CHAR", "[timestamp_tz][conversion][c_char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When TIMESTAMP_TZ values are fetched as SQL_C_CHAR
  // Then The string representation preserves the local wall-clock and the
  // original `+/-HH:MM` offset suffix (matches legacy 3.16.0 driver and the
  // ISO-8601 formatting most BI tools expect for TIMESTAMP_TZ).
  {
    INFO("UTC timestamp keeps +00:00 suffix");
    auto result = check_char_success(conn.execute_fetch("SELECT '2024-01-15 14:30:45 +00:00'::TIMESTAMP_TZ"), 1);
    CHECK(result == "2024-01-15 14:30:45 +00:00");
  }

  {
    INFO("positive offset preserves the local wall-clock and suffix");
    auto result = check_char_success(conn.execute_fetch("SELECT '2024-01-15 14:30:45 +05:30'::TIMESTAMP_TZ"), 1);
    CHECK(result == "2024-01-15 14:30:45 +05:30");
  }

  {
    INFO("negative offset preserves the local wall-clock and suffix");
    auto result = check_char_success(conn.execute_fetch("SELECT '2024-01-15 14:30:45 -08:00'::TIMESTAMP_TZ"), 1);
    CHECK(result == "2024-01-15 14:30:45 -08:00");
  }

  {
    INFO("fractional seconds are preserved alongside the offset");
    auto result =
        check_char_success(conn.execute_fetch("SELECT '2024-01-15 10:30:00.123456789 +00:00'::TIMESTAMP_TZ"), 1);
    CHECK(result == "2024-01-15 10:30:00.123456789 +00:00");
  }

  {
    INFO("pre-epoch timestamp keeps suffix");
    auto result = check_char_success(conn.execute_fetch("SELECT '1960-06-15 12:00:00 +00:00'::TIMESTAMP_TZ"), 1);
    CHECK(result == "1960-06-15 12:00:00 +00:00");
  }

  {
    INFO("midnight UTC keeps suffix");
    auto result = check_char_success(conn.execute_fetch("SELECT '2024-06-15 00:00:00 +00:00'::TIMESTAMP_TZ"), 1);
    CHECK(result == "2024-06-15 00:00:00 +00:00");
  }

  {
    // The local wall-clock stays at 02:00 (the literal the user wrote);
    // SELECT'ing into SQL_C_CHAR is *not* a UTC conversion -- the offset is
    // exposed instead. This is the behavior every TIMESTAMP_TZ-aware
    // database driver (and the legacy Snowflake ODBC driver) provides.
    INFO("offset is preserved verbatim, not re-anchored to UTC");
    auto result = check_char_success(conn.execute_fetch("SELECT '2024-01-15 02:00:00 +05:00'::TIMESTAMP_TZ"), 1);
    CHECK(result == "2024-01-15 02:00:00 +05:00");
  }
}

TEST_CASE("TIMESTAMP_TZ to SQL_C_CHAR fractional truncation", "[timestamp_tz][conversion][c_char][01004]") {
  SKIP_OLD_DRIVER("BD#30", "Old driver crashes (SIGSEGV) on TIMESTAMP to SQL_C_CHAR truncation");
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIMESTAMP_TZ with fractional seconds is fetched into a 28-byte
  // buffer that fits the date/time but cuts off the fractional portion
  // *before* the offset suffix (full string is
  // `2024-01-15 10:30:00.123456789 +00:00` = 35 chars + null).
  auto stmt = conn.execute_fetch("SELECT '2024-01-15 10:30:00.123456789 +00:00'::TIMESTAMP_TZ");
  char buffer[28] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);

  // Then SQL_SUCCESS_WITH_INFO with SQLSTATE 01004; indicator reports the full
  // length the driver tried to write (35), and the buffer holds the prefix.
  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(indicator == 35);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "01004");
  CHECK(std::string(buffer) == "2024-01-15 10:30:00.123456");
}

TEST_CASE("TIMESTAMP_TZ to SQL_C_CHAR buffer too small", "[timestamp_tz][conversion][c_char][22003]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIMESTAMP_TZ value is fetched into a buffer smaller than 27 bytes
  // (the minimum for "YYYY-MM-DD HH:MM:SS +/-HH:MM" + null terminator).
  auto stmt = conn.execute_fetch("SELECT '2024-01-15 14:30:45 +00:00'::TIMESTAMP_TZ");
  char buffer[10] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);

  // Then SQL_ERROR is returned with SQLSTATE 22003
  CHECK(ret == SQL_ERROR);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "22003");
}

TEST_CASE("TIMESTAMP_TZ NULL to SQL_C_CHAR", "[timestamp_tz][conversion][c_char][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL TIMESTAMP_TZ value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::TIMESTAMP_TZ");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_CHAR);
}
