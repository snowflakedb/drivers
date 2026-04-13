#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_diag_rec.hpp"

// ============================================================================
// SQL_C_CHAR
// ============================================================================

TEST_CASE("TIME to SQL_C_CHAR", "[time][conversion][c_char]") {
  // Given Snowflake client is logged in
  Connection conn;

  {
    // When A basic TIME is fetched as SQL_C_CHAR
    auto result = check_char_success(conn.execute_fetch("SELECT '14:30:45'::TIME"), 1);

    // Then String representation matches expected format
    CHECK(result == "14:30:45");
  }

  {
    // When A TIME with fractional seconds is fetched as SQL_C_CHAR
    auto result = check_char_success(conn.execute_fetch("SELECT '10:30:00.123456789'::TIME"), 1);
    // Then String includes fractional seconds
    CHECK(result == "10:30:00.123456789");
  }

  {
    // When Midnight TIME is fetched as SQL_C_CHAR
    auto result = check_char_success(conn.execute_fetch("SELECT '00:00:00'::TIME"), 1);
    // Then String representation is all zeros
    CHECK(result == "00:00:00");
  }

  {
    // When End-of-day TIME is fetched as SQL_C_CHAR
    auto result = check_char_success(conn.execute_fetch("SELECT '23:59:59'::TIME"), 1);
    // Then String representation matches
    CHECK(result == "23:59:59");
  }
}

TEST_CASE("TIME to SQL_C_CHAR fractional truncation", "[time][conversion][c_char][01004]") {
  SKIP_OLD_DRIVER("BD#38", "Old driver returns 22003 instead of 01004 for TIME partial truncation");
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIME with fractional seconds is fetched into a 9-byte buffer
  auto stmt = conn.execute_fetch("SELECT '10:30:00.123456789'::TIME");
  char buffer[9] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);

  // Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004 and fractional part truncated
  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(indicator == 18);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "01004");
  CHECK(std::string(buffer) == "10:30:00");
}

TEST_CASE("TIME to SQL_C_CHAR buffer too small", "[time][conversion][c_char][22003]") {
  SKIP_OLD_DRIVER("BD#38", "Old driver returns SQL_SUCCESS instead of SQL_ERROR for TIME buffer too small");
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIME value is fetched into a buffer smaller than the time string
  auto stmt = conn.execute_fetch("SELECT '14:30:45'::TIME");
  char buffer[5] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);

  // Then SQL_ERROR is returned with SQLSTATE 22003
  CHECK(ret == SQL_ERROR);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "22003");
}

TEST_CASE("TIME NULL to SQL_C_CHAR", "[time][conversion][c_char][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL TIME value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::TIME");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_CHAR);
}

// ============================================================================
// SQL_C_WCHAR
// ============================================================================

TEST_CASE("TIME to SQL_C_WCHAR", "[time][conversion][c_wchar]") {
  // Given Snowflake client is logged in
  Connection conn;

  {
    // When A basic TIME is fetched as SQL_C_WCHAR
    auto result = check_wchar_success(conn.execute_fetch("SELECT '14:30:45'::TIME"), 1);

    // Then Wide string representation matches expected format
    CHECK(result == u"14:30:45");
  }

  {
    // When A TIME with fractional seconds is fetched as SQL_C_WCHAR
    auto result = check_wchar_success(conn.execute_fetch("SELECT '10:30:00.123456789'::TIME"), 1);

    // Then Wide string includes fractional seconds
    CHECK(result == u"10:30:00.123456789");
  }

  {
    // When Midnight TIME is fetched as SQL_C_WCHAR
    auto result = check_wchar_success(conn.execute_fetch("SELECT '00:00:00'::TIME"), 1);

    // Then Wide string representation is all zeros
    CHECK(result == u"00:00:00");
  }
}

TEST_CASE("TIME to SQL_C_WCHAR fractional truncation", "[time][conversion][c_wchar][01004]") {
  SKIP_OLD_DRIVER("BD#38", "Old driver returns 22003 instead of 01004 for TIME partial truncation");
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIME with fractional seconds is fetched into a WCHAR buffer of 9 characters
  auto stmt = conn.execute_fetch("SELECT '10:30:00.123456789'::TIME");
  char16_t buffer[9] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, buffer, sizeof(buffer), &indicator);

  // Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004
  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "01004");
}

TEST_CASE("TIME to SQL_C_WCHAR buffer too small", "[time][conversion][c_wchar][22003]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIME value is fetched into a WCHAR buffer smaller than the time string
  auto stmt = conn.execute_fetch("SELECT '14:30:45'::TIME");
  char16_t buffer[5] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, buffer, sizeof(buffer), &indicator);

  // Then SQL_ERROR is returned with SQLSTATE 22003
  CHECK(ret == SQL_ERROR);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "22003");
}

TEST_CASE("TIME NULL to SQL_C_WCHAR", "[time][conversion][c_wchar][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL TIME value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::TIME");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_WCHAR);
}
