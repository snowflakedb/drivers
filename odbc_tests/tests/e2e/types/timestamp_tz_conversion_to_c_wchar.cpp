#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_diag_rec.hpp"

TEST_CASE("TIMESTAMP_TZ to SQL_C_WCHAR", "[timestamp_tz][conversion][c_wchar]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When TIMESTAMP_TZ values are fetched as SQL_C_WCHAR
  // Then The wide-string representation preserves the local wall-clock and
  // the original `+/-HH:MM` offset suffix (the WCHAR path mirrors the CHAR
  // path; only the encoding differs).
  {
    INFO("UTC timestamp keeps +00:00 suffix");
    auto result = check_wchar_success(conn.execute_fetch("SELECT '2024-01-15 14:30:45 +00:00'::TIMESTAMP_TZ"), 1);
    CHECK(result == u"2024-01-15 14:30:45 +00:00");
  }

  {
    INFO("positive offset preserves the local wall-clock and suffix");
    auto result = check_wchar_success(conn.execute_fetch("SELECT '2024-01-15 14:30:45 +05:30'::TIMESTAMP_TZ"), 1);
    CHECK(result == u"2024-01-15 14:30:45 +05:30");
  }

  {
    INFO("fractional seconds are preserved alongside the offset");
    auto result =
        check_wchar_success(conn.execute_fetch("SELECT '2024-01-15 10:30:00.123456789 +00:00'::TIMESTAMP_TZ"), 1);
    CHECK(result == u"2024-01-15 10:30:00.123456789 +00:00");
  }

  {
    INFO("pre-epoch timestamp keeps suffix");
    auto result = check_wchar_success(conn.execute_fetch("SELECT '1960-06-15 12:00:00 +00:00'::TIMESTAMP_TZ"), 1);
    CHECK(result == u"1960-06-15 12:00:00 +00:00");
  }

  {
    INFO("midnight UTC keeps suffix");
    auto result = check_wchar_success(conn.execute_fetch("SELECT '2024-06-15 00:00:00 +00:00'::TIMESTAMP_TZ"), 1);
    CHECK(result == u"2024-06-15 00:00:00 +00:00");
  }

  {
    // Same as the CHAR test: offset is preserved, not re-anchored to UTC.
    INFO("offset is preserved verbatim, not re-anchored to UTC");
    auto result = check_wchar_success(conn.execute_fetch("SELECT '2024-01-15 02:00:00 +05:00'::TIMESTAMP_TZ"), 1);
    CHECK(result == u"2024-01-15 02:00:00 +05:00");
  }
}

TEST_CASE("TIMESTAMP_TZ to SQL_C_WCHAR buffer too small", "[timestamp_tz][conversion][c_wchar][22003]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIMESTAMP_TZ value is fetched into a WCHAR buffer smaller than the
  // 27-char minimum ("YYYY-MM-DD HH:MM:SS +/-HH:MM" + null terminator, 27
  // chars * 2 bytes each = 54 bytes). The 5-char buffer fails on size.
  auto stmt = conn.execute_fetch("SELECT '2024-01-15 14:30:45 +00:00'::TIMESTAMP_TZ");
  char16_t buffer[5] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, buffer, sizeof(buffer), &indicator);

  // Then SQL_ERROR is returned with SQLSTATE 22003
  CHECK(ret == SQL_ERROR);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "22003");
}

TEST_CASE("TIMESTAMP_TZ to SQL_C_WCHAR truncation", "[timestamp_tz][conversion][c_wchar][01004]") {
  SKIP_OLD_DRIVER("BD#30", "Old driver crashes on TIMESTAMP to SQL_C_WCHAR truncation");
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIMESTAMP_TZ with fractional seconds is fetched into a 28-char
  // WCHAR buffer. Full string is `2024-01-15 10:30:00.123456789 +00:00` =
  // 35 chars + null terminator; the 28-char buffer truncates partway through
  // the fractional digits.
  auto stmt = conn.execute_fetch("SELECT '2024-01-15 10:30:00.123456789 +00:00'::TIMESTAMP_TZ");
  char16_t buffer[28] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, buffer, sizeof(buffer), &indicator);

  // Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004
  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "01004");
}

TEST_CASE("TIMESTAMP_TZ NULL to SQL_C_WCHAR", "[timestamp_tz][conversion][c_wchar][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL TIMESTAMP_TZ value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::TIMESTAMP_TZ");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_WCHAR);
}
