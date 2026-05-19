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
  SKIP_OLD_DRIVER("BD#52", "Old driver does not honor TIMESTAMP_TZ_OUTPUT_FORMAT for TIMESTAMP_TZ -> CHAR/WCHAR fetch");
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
    auto result =
        check_char_success(conn.execute_fetch("SELECT '2024-01-15 10:30:00.123456789 +07:00'::TIMESTAMP_TZ"), 1);
    CHECK(result == "2024-01-15 10:30:00.123456789 +07:00");
  }
}

TEST_CASE("TIMESTAMP_TZ to SQL_C_CHAR honors TIMESTAMP_TZ_OUTPUT_FORMAT with TZHTZM",
          "[timestamp_tz][conversion][c_char][output_format]") {
  SKIP_OLD_DRIVER("BD#52", "Old driver does not honor TIMESTAMP_TZ_OUTPUT_FORMAT for TIMESTAMP_TZ -> CHAR/WCHAR fetch");
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
  SKIP_OLD_DRIVER("BD#52", "Old driver does not honor TIMESTAMP_TZ_OUTPUT_FORMAT for TIMESTAMP_TZ -> CHAR/WCHAR fetch");
  SKIP_IODBC(
      "Test hardcodes UTF-16 SQLWCHAR semantics (`indicator == 52` = 26 chars * 2 bytes, `char16_t*` cast). Under "
      "iODBC SQLWCHAR is 4 bytes, so the indicator is 104 and the buffer holds UTF-32 — a width-aware rewrite "
      "(`sizeof(SQLWCHAR)`, encoding-aware decode) would cover both DMs");
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

TEST_CASE("TIMESTAMP_TZ_OUTPUT_FORMAT changes mid-connection take effect on the next execute",
          "[timestamp_tz][conversion][c_char][output_format][per_execute_reread]") {
  SKIP_OLD_DRIVER("BD#52", "Old driver does not honor TIMESTAMP_TZ_OUTPUT_FORMAT for TIMESTAMP_TZ -> CHAR/WCHAR fetch");
  // The load-bearing claim of the per-execute sequential RPC in
  // `update_numeric_settings` is that `ALTER SESSION SET / UNSET
  // TIMESTAMP_TZ_OUTPUT_FORMAT` mid-connection takes effect on the next
  // statement. Each of the other TEST_CASEs in this file does exactly
  // one ALTER followed by one fetch -- a connection-time-only read
  // would pass them identically, leaving the entire RPC cost
  // unverified. This case toggles the format three times within one
  // connection and asserts each fetch reflects the latest setting,
  // including the UNSET path back to bare UTC. See PR #1068 review on
  // `timestamp_tz_output_format.cpp:36`.
  // Given Snowflake client is logged in
  Connection conn;
  const char* select = "SELECT '2024-01-15 14:30:45 +05:30'::TIMESTAMP_TZ";

  // When TIMESTAMP_TZ_OUTPUT_FORMAT is toggled (set / re-set / unset) within one connection
  // Then each subsequent execute reflects the latest setting

  // Phase 1: no format set -> bare UTC wall-clock, no offset suffix.
  // The `+05:30` of the source literal is converted to the equivalent
  // UTC instant (09:00:45) and rendered without offset.
  {
    INFO("phase 1: no TIMESTAMP_TZ_OUTPUT_FORMAT -> bare UTC");
    auto bare = check_char_success(conn.execute_fetch(select), 1);
    CHECK(bare == "2024-01-15 09:00:45");
  }

  // Phase 2: SET to TZH:TZM -> next fetch must show `+HH:MM`. If
  // `update_numeric_settings` only read at connect-time we'd still see
  // bare UTC here.
  {
    INFO("phase 2: SET TZH:TZM -> +HH:MM suffix on next fetch");
    conn.execute("ALTER SESSION SET TIMESTAMP_TZ_OUTPUT_FORMAT = 'YYYY-MM-DD HH24:MI:SS.FF TZH:TZM'");
    auto colon = check_char_success(conn.execute_fetch(select), 1);
    CHECK(colon == "2024-01-15 14:30:45 +05:30");
  }

  // Phase 3: SET to TZHTZM -> next fetch must flip to `+HHMM`. Pins
  // the per-execute reread again with a *different* token so a
  // connection-time-only cache that happened to land on TZH:TZM
  // wouldn't pass.
  {
    INFO("phase 3: SET TZHTZM -> +HHMM suffix on next fetch");
    conn.execute("ALTER SESSION SET TIMESTAMP_TZ_OUTPUT_FORMAT = 'YYYY-MM-DD HH24:MI:SS.FF TZHTZM'");
    auto no_colon = check_char_success(conn.execute_fetch(select), 1);
    CHECK(no_colon == "2024-01-15 14:30:45 +0530");
  }

  // Phase 4: UNSET -> revert to bare UTC. A driver that cached the
  // last non-empty value and never re-checked would still emit the
  // offset here.
  {
    INFO("phase 4: UNSET -> revert to bare UTC");
    conn.execute("ALTER SESSION UNSET TIMESTAMP_TZ_OUTPUT_FORMAT");
    auto bare_again = check_char_success(conn.execute_fetch(select), 1);
    CHECK(bare_again == "2024-01-15 09:00:45");
  }
}
