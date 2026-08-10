#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "WideString.hpp"
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

TEST_CASE("TIME to SQL_C_CHAR exact buffer fit", "[time][conversion][c_char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIME value is fetched into a 9-byte buffer
  auto stmt = conn.execute_fetch("SELECT '14:30:45'::TIME");
  char buffer[9] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);

  // Then SQL_SUCCESS is returned with indicator 8
  CHECK(ret == SQL_SUCCESS);
  CHECK(indicator == 8);
  CHECK(std::string(buffer) == "14:30:45");
}

TEST_CASE("TIME to SQL_C_CHAR chunked retrieval", "[time][conversion][c_char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIME with fractional seconds is fetched via two sequential SQLGetData calls with a 10-byte buffer
  auto stmt = conn.execute_fetch("SELECT '10:30:00.123456789'::TIME");

  char buf1[10] = {};
  SQLLEN ind1 = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buf1, sizeof(buf1), &ind1);

  // Then Both drivers report the same total length, but write different partial values
  CHECK(ind1 == 18);

  char buf2[10] = {};
  SQLLEN ind2 = 0;

  // BD#38: the old driver stops at the seconds boundary even though a 9th byte was free
  OLD_DRIVER_ONLY("BD#38") {
    CHECK(ret == SQL_SUCCESS);
    CHECK(std::string(buf1) == "10:30:00");
    CHECK(SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buf2, sizeof(buf2), &ind2) == SQL_NO_DATA);
  }
  NEW_DRIVER_ONLY("BD#38") {
    CHECK(ret == SQL_SUCCESS_WITH_INFO);
    CHECK(std::string(buf1) == "10:30:00.");
    CHECK(SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buf2, sizeof(buf2), &ind2) == SQL_SUCCESS);
    CHECK(ind2 == 9);
    CHECK(std::string(buf2) == "123456789");
  }
}

TEST_CASE("TIME to SQL_C_CHAR fractional truncation", "[time][conversion][c_char][01004]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIME with fractional seconds is fetched into a 9-byte buffer
  auto stmt = conn.execute_fetch("SELECT '10:30:00.123456789'::TIME");
  char buffer[9] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);

  OLD_DRIVER_ONLY("BD#38") {
    CHECK(ret == SQL_ERROR);
    auto records = get_diag_rec(stmt);
    CHECK(!records.empty());
    CHECK(records[0].sqlState == "22003");
  }
  NEW_DRIVER_ONLY("BD#38") {
    // Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004 and fractional part truncated
    CHECK(ret == SQL_SUCCESS_WITH_INFO);
    CHECK(indicator == 18);
    auto records = get_diag_rec(stmt);
    CHECK(!records.empty());
    CHECK(records[0].sqlState == "01004");
    CHECK(std::string(buffer) == "10:30:00");
  }
}

TEST_CASE("TIME to SQL_C_CHAR buffer too small", "[time][conversion][c_char][22003]") {
  SKIP_OLD_DRIVER("BD#38", "old driver has undefined behavior on undersized CHAR buffers (SIGABRT)");
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
    CHECK(result == U"14:30:45");
  }

  {
    // When A TIME with fractional seconds is fetched as SQL_C_WCHAR
    auto result = check_wchar_success(conn.execute_fetch("SELECT '10:30:00.123456789'::TIME"), 1);

    // Then Wide string includes fractional seconds
    CHECK(result == U"10:30:00.123456789");
  }

  {
    // When Midnight TIME is fetched as SQL_C_WCHAR
    auto result = check_wchar_success(conn.execute_fetch("SELECT '00:00:00'::TIME"), 1);

    // Then Wide string representation is all zeros
    CHECK(result == U"00:00:00");
  }
}

TEST_CASE("TIME to SQL_C_WCHAR exact buffer fit", "[time][conversion][c_wchar]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIME value is fetched into a WCHAR buffer of exactly 9 characters
  auto stmt = conn.execute_fetch("SELECT '14:30:45'::TIME");
  SQLWCHAR buffer[9] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, buffer, sizeof(buffer), &indicator);

  // Then SQL_SUCCESS is returned with the correct wide string
  CHECK(ret == SQL_SUCCESS);
  CHECK(sf::wide::decode_wide_cstr(buffer) == U"14:30:45");
}

TEST_CASE("TIME to SQL_C_WCHAR fractional truncation", "[time][conversion][c_wchar][01004]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIME with fractional seconds is fetched into a WCHAR buffer of 9 characters
  auto stmt = conn.execute_fetch("SELECT '10:30:00.123456789'::TIME");
  SQLWCHAR buffer[9] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, buffer, sizeof(buffer), &indicator);

  OLD_DRIVER_ONLY("BD#38") {
    CHECK(ret == SQL_ERROR);
    auto records = get_diag_rec(stmt);
    CHECK(!records.empty());
    CHECK(records[0].sqlState == "22003");
  }
  NEW_DRIVER_ONLY("BD#38") {
    // Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004
    CHECK(ret == SQL_SUCCESS_WITH_INFO);
    auto records = get_diag_rec(stmt);
    CHECK(!records.empty());
    CHECK(records[0].sqlState == "01004");
    CHECK(sf::wide::decode_wide_cstr(buffer) == U"10:30:00");
  }
}

TEST_CASE("TIME to SQL_C_WCHAR buffer too small", "[time][conversion][c_wchar][22003]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIME value is fetched into a WCHAR buffer that cannot hold a
  // single character plus a NUL terminator. A 1-unit buffer is too small
  // under both UTF-16 (2 bytes) and UTF-32 (4 bytes), so the driver's
  // "buffer too small" branch fires regardless of `sizeof(SQLWCHAR)`.
  auto stmt = conn.execute_fetch("SELECT '14:30:45'::TIME");
  SQLWCHAR buffer[1] = {};
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
