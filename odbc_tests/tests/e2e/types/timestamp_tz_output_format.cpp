// ODBC E2E: TIMESTAMP_TZ -> SQL_C_CHAR / SQL_C_WCHAR with the session's
// `TIMESTAMP_TZ_OUTPUT_FORMAT` opted-in to a TZH/TZM/TZHTZM token.
//
// The default fetch path drops the offset and renders the bare UTC
// wall-clock (covered in `timestamp_tz_conversion_to_c_char.cpp`). When
// the customer sets `TIMESTAMP_TZ_OUTPUT_FORMAT` to a value containing
// `TZH:TZM`, `TZHTZM`, or `TZH`, the universal driver mirrors the legacy
// 3.16.0 driver's behaviour and appends `+/-HH[:]MM` (or `+/-HH`) so the
// original observer's offset survives the fetch — see
// `parse_tz_offset_format` and `format_timestamp_tz_string_into` in
// `odbc/src/conversion/timestamp.rs`.

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

TEST_CASE("TIMESTAMP_TZ to SQL_C_CHAR honors TIMESTAMP_TZ_OUTPUT_FORMAT with TZH:TZM",
          "[timestamp_tz][conversion][c_char][output_format]") {
  SKIP_OLD_DRIVER("BD#000",
                  "New driver opts into legacy `TZH:TZM`-aware TZ rendering only when the session "
                  "format is set; legacy driver's behaviour for the same format is what we mirror");
  // Given Snowflake client is logged in
  Connection conn;

  // When the session opts into the verbose Snowflake offset token
  conn.execute("ALTER SESSION SET TIMESTAMP_TZ_OUTPUT_FORMAT = 'YYYY-MM-DD HH24:MI:SS.FF TZH:TZM'");

  // Then a positive-offset TIMESTAMP_TZ round-trips as `<local_wall_clock> +HH:MM`
  {
    INFO("positive offset rendered as +05:30");
    auto result = check_char_success(conn.execute_fetch("SELECT '2024-01-15 14:30:45 +05:30'::TIMESTAMP_TZ"), 1);
    CHECK(result == "2024-01-15 14:30:45 +05:30");
  }

  // And negative offsets carry the `-` sign
  {
    INFO("negative offset rendered as -08:00");
    auto result = check_char_success(conn.execute_fetch("SELECT '2024-01-15 14:30:45 -08:00'::TIMESTAMP_TZ"), 1);
    CHECK(result == "2024-01-15 14:30:45 -08:00");
  }

  // And zero-offset (UTC) keeps `+00:00` (not `-00:00`)
  {
    INFO("zero offset rendered as +00:00");
    auto result = check_char_success(conn.execute_fetch("SELECT '2024-01-15 14:30:45 +00:00'::TIMESTAMP_TZ"), 1);
    CHECK(result == "2024-01-15 14:30:45 +00:00");
  }

  // And fractional seconds are preserved alongside the offset
  {
    INFO("fractional seconds + offset");
    auto result = check_char_success(
        conn.execute_fetch("SELECT '2024-01-15 10:30:00.123456789 +07:00'::TIMESTAMP_TZ"), 1);
    CHECK(result == "2024-01-15 10:30:00.123456789 +07:00");
  }
}

TEST_CASE("TIMESTAMP_TZ to SQL_C_CHAR honors TIMESTAMP_TZ_OUTPUT_FORMAT with TZHTZM",
          "[timestamp_tz][conversion][c_char][output_format]") {
  SKIP_OLD_DRIVER("BD#000", "New driver opts into TZHTZM-aware TZ rendering only when the session format is set");
  // Given Snowflake client is logged in
  Connection conn;

  // When the session opts into the compact Snowflake offset token (no colon)
  conn.execute("ALTER SESSION SET TIMESTAMP_TZ_OUTPUT_FORMAT = 'YYYY-MM-DD HH24:MI:SS.FF TZHTZM'");

  // Then a positive-offset TIMESTAMP_TZ round-trips as `<local_wall_clock> +HHMM`
  auto result = check_char_success(conn.execute_fetch("SELECT '2024-01-15 14:30:45 +05:30'::TIMESTAMP_TZ"), 1);
  CHECK(result == "2024-01-15 14:30:45 +0530");
}

TEST_CASE("TIMESTAMP_TZ to SQL_C_WCHAR honors TIMESTAMP_TZ_OUTPUT_FORMAT with TZH:TZM",
          "[timestamp_tz][conversion][c_wchar][output_format]") {
  SKIP_OLD_DRIVER("BD#000", "New driver opts into TZH:TZM-aware TZ rendering only when the session format is set");
  // Given Snowflake client is logged in
  Connection conn;

  // When the session opts into the verbose Snowflake offset token, fetched as wide chars
  conn.execute("ALTER SESSION SET TIMESTAMP_TZ_OUTPUT_FORMAT = 'YYYY-MM-DD HH24:MI:SS.FF TZH:TZM'");

  // Then a positive-offset TIMESTAMP_TZ round-trips as the same UTF-16 string with `+05:30`
  auto stmt = conn.execute_fetch("SELECT '2024-01-15 14:30:45 +05:30'::TIMESTAMP_TZ");
  SQLWCHAR buffer[64] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, buffer, sizeof(buffer), &indicator);
  CHECK(ret == SQL_SUCCESS);
  // 26 ASCII chars * 2 bytes = 52 bytes (excluding NUL).
  CHECK(indicator == 52);
  std::u16string expected = u"2024-01-15 14:30:45 +05:30";
  std::u16string actual(reinterpret_cast<const char16_t*>(buffer), expected.size());
  CHECK(actual == expected);
}
