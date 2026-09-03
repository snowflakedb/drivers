// ODBC E2E: DATE -> SQL_C_CHAR / SQL_C_WCHAR always renders the ISO
// `YYYY-MM-DD` wall date, ignoring any session `DATE_OUTPUT_FORMAT` token.

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

TEST_CASE("DATE to SQL_C_CHAR ignores DATE_OUTPUT_FORMAT", "[date][conversion][c_char][output_format]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When the session sets a DATE_OUTPUT_FORMAT token that, if honored, would
  // reorder the fields, swap the separators, name the month, or drop century
  // digits -- rendering 2024-01-15 as something other than the ISO wall date.
  // Then the fetch still renders the ISO YYYY-MM-DD wall date.
  auto expect_iso = [&](const char* format, const char* iso_date) {
    INFO("DATE_OUTPUT_FORMAT = " << format);
    conn.execute(std::string("ALTER SESSION SET DATE_OUTPUT_FORMAT = '") + format + "'");
    auto result = check_char_success(conn.execute_fetch(std::string("SELECT '") + iso_date + "'::DATE"), 1);
    CHECK(result == iso_date);
  };

  expect_iso("DD-MON-YYYY", "2024-01-15");    // would be 15-Jan-2024
  expect_iso("MM/DD/YYYY", "2024-01-15");     // would be 01/15/2024
  expect_iso("DD.MM.YYYY", "2024-01-15");     // would be 15.01.2024
  expect_iso("YY-MM-DD", "2024-01-15");       // would be 24-01-15
  expect_iso("MMMM DD, YYYY", "2024-01-15");  // would be January 15, 2024
}

TEST_CASE("DATE to SQL_C_CHAR ignores DATE_OUTPUT_FORMAT across boundary dates",
          "[date][conversion][c_char][output_format]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When a non-default token is in effect for the whole session
  conn.execute("ALTER SESSION SET DATE_OUTPUT_FORMAT = 'DD-MON-YYYY'");

  // Then every date -- leap day, the epoch, and the representable extremes --
  // still renders as the ISO wall date, so the token is ignored value-for-value.
  {
    INFO("leap day");
    CHECK(check_char_success(conn.execute_fetch("SELECT '2000-02-29'::DATE"), 1) == "2000-02-29");
  }
  {
    INFO("unix epoch");
    CHECK(check_char_success(conn.execute_fetch("SELECT '1970-01-01'::DATE"), 1) == "1970-01-01");
  }
  {
    INFO("far past");
    CHECK(check_char_success(conn.execute_fetch("SELECT '0001-01-01'::DATE"), 1) == "0001-01-01");
  }
  {
    INFO("far future");
    CHECK(check_char_success(conn.execute_fetch("SELECT '9999-12-31'::DATE"), 1) == "9999-12-31");
  }
}

TEST_CASE("DATE to SQL_C_WCHAR ignores DATE_OUTPUT_FORMAT", "[date][conversion][c_wchar][output_format]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When a non-default field order is in effect, fetched as wide chars
  conn.execute("ALTER SESSION SET DATE_OUTPUT_FORMAT = 'DD-MON-YYYY'");

  // Then the fetch still renders the ISO wall date. Assertions are made on the
  // *decoded* code-point sequence and on the indicator expressed in DM-side
  // SQLWCHAR units, so the test is portable across UTF-16 (unixODBC) and UTF-32
  // (iODBC) without touching the driver path.
  auto stmt = conn.execute_fetch("SELECT '2024-01-15'::DATE");
  SQLWCHAR buffer[64] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, buffer, sizeof(buffer), &indicator);
  CHECK(ret == SQL_SUCCESS);
  std::u32string expected = U"2024-01-15";
  // 10 ASCII chars * sizeof(SQLWCHAR) bytes (excluding NUL).
  CHECK(indicator == static_cast<SQLLEN>(expected.size() * sf::wide::wchar_byte_size()));
  auto actual = sf::wide::decode_wide(buffer, expected.size());
  CHECK(actual == expected);
}
