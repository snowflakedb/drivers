// ODBC E2E: TIMESTAMP_TZ -> SQL_C_CHAR / SQL_C_WCHAR always renders the bare
// UTC wall-clock with no offset suffix, even when the session sets a
// `TIMESTAMP_TZ_OUTPUT_FORMAT` that carries a TZH / TZM / TZHTZM token. These
// cases run on both drivers to assert parity. (The default, no-format path is
// covered in `timestamp_tz_conversion_to_c_char.cpp`.)

#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "WideString.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_diag_rec.hpp"

TEST_CASE("TIMESTAMP_TZ to SQL_C_CHAR drops offset even when TIMESTAMP_TZ_OUTPUT_FORMAT is set",
          "[timestamp_tz][conversion][c_char][output_format]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When the session sets an offset-bearing TIMESTAMP_TZ_OUTPUT_FORMAT token
  conn.execute("ALTER SESSION SET TIMESTAMP_TZ_OUTPUT_FORMAT = 'YYYY-MM-DD HH24:MI:SS.FF TZH:TZM'");

  // Then the fetch still renders the bare UTC wall-clock, with no offset suffix

  {
    INFO("positive offset -> bare UTC, no suffix");
    auto result = check_char_success(conn.execute_fetch("SELECT '2024-01-15 14:30:45 +05:30'::TIMESTAMP_TZ"), 1);
    CHECK(result == "2024-01-15 09:00:45");
  }

  {
    INFO("negative offset -> bare UTC, no suffix");
    auto result = check_char_success(conn.execute_fetch("SELECT '2024-01-15 14:30:45 -08:00'::TIMESTAMP_TZ"), 1);
    CHECK(result == "2024-01-15 22:30:45");
  }

  {
    INFO("zero offset -> bare UTC, no suffix");
    auto result = check_char_success(conn.execute_fetch("SELECT '2024-01-15 14:30:45 +00:00'::TIMESTAMP_TZ"), 1);
    CHECK(result == "2024-01-15 14:30:45");
  }

  {
    INFO("fractional seconds -> bare UTC, no suffix");
    auto result =
        check_char_success(conn.execute_fetch("SELECT '2024-01-15 10:30:00.123456789 +07:00'::TIMESTAMP_TZ"), 1);
    CHECK(result == "2024-01-15 03:30:00.123456789");
  }
}

TEST_CASE("TIMESTAMP_TZ to SQL_C_CHAR drops offset even with TZHTZM token",
          "[timestamp_tz][conversion][c_char][output_format]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When the session sets the compact (no-colon) offset token
  conn.execute("ALTER SESSION SET TIMESTAMP_TZ_OUTPUT_FORMAT = 'YYYY-MM-DD HH24:MI:SS.FF TZHTZM'");

  // Then the fetch still renders the bare UTC wall-clock, with no offset suffix
  auto result = check_char_success(conn.execute_fetch("SELECT '2024-01-15 14:30:45 +05:30'::TIMESTAMP_TZ"), 1);
  CHECK(result == "2024-01-15 09:00:45");
}

TEST_CASE("TIMESTAMP_TZ to SQL_C_WCHAR drops offset even when TIMESTAMP_TZ_OUTPUT_FORMAT is set",
          "[timestamp_tz][conversion][c_wchar][output_format]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When the session sets an offset-bearing token, fetched as wide chars
  conn.execute("ALTER SESSION SET TIMESTAMP_TZ_OUTPUT_FORMAT = 'YYYY-MM-DD HH24:MI:SS.FF TZH:TZM'");

  // Then the fetch still renders the bare UTC wall-clock, with no offset suffix.
  // Assertions are made on the *decoded* code-point sequence and on the indicator
  // expressed in DM-side SQLWCHAR units, so the test is portable across UTF-16
  // (unixODBC) and UTF-32 (iODBC) without touching the driver path.
  auto stmt = conn.execute_fetch("SELECT '2024-01-15 14:30:45 +05:30'::TIMESTAMP_TZ");
  SQLWCHAR buffer[64] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, buffer, sizeof(buffer), &indicator);
  CHECK(ret == SQL_SUCCESS);
  std::u32string expected = U"2024-01-15 09:00:45";
  // 19 ASCII chars * sizeof(SQLWCHAR) bytes (excluding NUL).
  CHECK(indicator == static_cast<SQLLEN>(expected.size() * sf::wide::wchar_byte_size()));
  auto actual = sf::wide::decode_wide(buffer, expected.size());
  CHECK(actual == expected);
}
