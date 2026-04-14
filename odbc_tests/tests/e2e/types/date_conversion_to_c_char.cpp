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

TEST_CASE("DATE to SQL_C_CHAR", "[date][conversion][c_char]") {
  // Given Snowflake client is logged in
  Connection conn;

  {
    // When basic DATE is fetched as SQL_C_CHAR
    auto result = check_char_success(conn.execute_fetch("SELECT '2024-01-15'::DATE"), 1);
    // Then String representation matches "yyyy-mm-dd" format
    CHECK(result == "2024-01-15");
  }

  {
    // When pre-epoch DATE is fetched as SQL_C_CHAR
    auto result = check_char_success(conn.execute_fetch("SELECT '1960-06-15'::DATE"), 1);
    // Then String representation matches "yyyy-mm-dd" format
    CHECK(result == "1960-06-15");
  }

  {
    // When leap day DATE is fetched as SQL_C_CHAR
    auto result = check_char_success(conn.execute_fetch("SELECT '2000-02-29'::DATE"), 1);
    // Then String representation matches "yyyy-mm-dd" format
    CHECK(result == "2000-02-29");
  }

  {
    // When epoch DATE is fetched as SQL_C_CHAR
    auto result = check_char_success(conn.execute_fetch("SELECT '1970-01-01'::DATE"), 1);
    // Then String representation matches "yyyy-mm-dd" format
    CHECK(result == "1970-01-01");
  }

  {
    // When end of year DATE is fetched as SQL_C_CHAR
    auto result = check_char_success(conn.execute_fetch("SELECT '1999-12-31'::DATE"), 1);
    // Then String representation matches "yyyy-mm-dd" format
    CHECK(result == "1999-12-31");
  }

  {
    // When first day of year DATE is fetched as SQL_C_CHAR
    auto result = check_char_success(conn.execute_fetch("SELECT '2025-01-01'::DATE"), 1);
    // Then String representation matches "yyyy-mm-dd" format
    CHECK(result == "2025-01-01");
  }

  {
    // When leap year non-leap day DATE is fetched as SQL_C_CHAR
    auto result = check_char_success(conn.execute_fetch("SELECT '2024-02-28'::DATE"), 1);
    // Then String representation matches "yyyy-mm-dd" format
    CHECK(result == "2024-02-28");
  }

  {
    // When non-leap year Feb 28 DATE is fetched as SQL_C_CHAR
    auto result = check_char_success(conn.execute_fetch("SELECT '2023-02-28'::DATE"), 1);
    // Then String representation matches "yyyy-mm-dd" format
    CHECK(result == "2023-02-28");
  }
}

TEST_CASE("DATE to SQL_C_CHAR exact buffer fit", "[date][conversion][c_char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A DATE value is fetched into an 11-byte buffer
  auto stmt = conn.execute_fetch("SELECT '2024-01-15'::DATE");
  char buffer[11] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);

  // Then SQL_SUCCESS is returned with indicator 10
  CHECK(ret == SQL_SUCCESS);
  CHECK(indicator == 10);
  CHECK(std::string(buffer) == "2024-01-15");
}

TEST_CASE("DATE to SQL_C_CHAR truncation", "[date][conversion][c_char][01004]") {
  SKIP_OLD_DRIVER("BD#41", "old driver returns error instead of 01004 truncation");
  // Given Snowflake client is logged in
  Connection conn;

  // When A DATE value is fetched into a buffer smaller than 11 bytes
  auto stmt = conn.execute_fetch("SELECT '2024-01-15'::DATE");
  char buffer[8] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);

  // Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004
  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(indicator == 10);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "01004");
  CHECK(std::string(buffer) == "2024-01");
}

TEST_CASE("DATE to SQL_C_CHAR chunked retrieval", "[date][conversion][c_char]") {
  SKIP_OLD_DRIVER("BD#41", "old driver returns error instead of 01004 truncation");
  // Given Snowflake client is logged in
  Connection conn;

  // When A DATE value is fetched via two sequential SQLGetData calls with a 6-byte buffer
  auto stmt = conn.execute_fetch("SELECT '2024-01-15'::DATE");

  char buf1[6] = {};
  SQLLEN ind1 = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buf1, sizeof(buf1), &ind1);

  // Then The first call returns partial data with 01004 and the second call returns the remainder
  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(ind1 == 10);
  CHECK(std::string(buf1) == "2024-");

  char buf2[6] = {};
  SQLLEN ind2 = 0;
  ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buf2, sizeof(buf2), &ind2);
  CHECK(ret == SQL_SUCCESS);
  CHECK(ind2 == 5);
  CHECK(std::string(buf2) == "01-15");
}

TEST_CASE("DATE to SQL_C_CHAR far future", "[date][conversion][c_char][edge]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When The maximum DATE value 9999-12-31 is fetched as SQL_C_CHAR
  auto result = check_char_success(conn.execute_fetch("SELECT '9999-12-31'::DATE"), 1);

  // Then String representation is "9999-12-31"
  CHECK(result == "9999-12-31");
}

TEST_CASE("DATE to SQL_C_CHAR far past", "[date][conversion][c_char][edge]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When The minimum DATE value 0001-01-01 is fetched as SQL_C_CHAR
  auto result = check_char_success(conn.execute_fetch("SELECT '0001-01-01'::DATE"), 1);

  // Then String representation is "0001-01-01"
  CHECK(result == "0001-01-01");
}

TEST_CASE("DATE NULL to SQL_C_CHAR", "[date][conversion][c_char][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL DATE value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::DATE");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_CHAR);
}

// ============================================================================
// SQL_C_WCHAR
// ============================================================================

TEST_CASE("DATE to SQL_C_WCHAR", "[date][conversion][c_wchar]") {
  // Given Snowflake client is logged in
  Connection conn;

  {
    // When basic DATE is fetched as SQL_C_WCHAR
    auto result = check_wchar_success(conn.execute_fetch("SELECT '2024-01-15'::DATE"), 1);
    // Then Wide string representation matches "yyyy-mm-dd" format
    CHECK(result == u"2024-01-15");
  }

  {
    // When pre-epoch DATE is fetched as SQL_C_WCHAR
    auto result = check_wchar_success(conn.execute_fetch("SELECT '1960-06-15'::DATE"), 1);
    // Then Wide string representation matches "yyyy-mm-dd" format
    CHECK(result == u"1960-06-15");
  }

  {
    // When leap day DATE is fetched as SQL_C_WCHAR
    auto result = check_wchar_success(conn.execute_fetch("SELECT '2000-02-29'::DATE"), 1);
    // Then Wide string representation matches "yyyy-mm-dd" format
    CHECK(result == u"2000-02-29");
  }

  {
    // When epoch DATE is fetched as SQL_C_WCHAR
    auto result = check_wchar_success(conn.execute_fetch("SELECT '1970-01-01'::DATE"), 1);
    // Then Wide string representation matches "yyyy-mm-dd" format
    CHECK(result == u"1970-01-01");
  }
}

TEST_CASE("DATE to SQL_C_WCHAR exact buffer fit", "[date][conversion][c_wchar]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A DATE value is fetched into a WCHAR buffer of exactly 11 characters
  auto stmt = conn.execute_fetch("SELECT '2024-01-15'::DATE");
  char16_t buffer[11] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, buffer, sizeof(buffer), &indicator);

  // Then SQL_SUCCESS is returned with the correct wide string
  CHECK(ret == SQL_SUCCESS);
  CHECK(std::u16string(buffer) == u"2024-01-15");
}

TEST_CASE("DATE to SQL_C_WCHAR truncation", "[date][conversion][c_wchar][01004]") {
  SKIP_OLD_DRIVER("BD#41", "old driver returns error instead of 01004 truncation");
  // Given Snowflake client is logged in
  Connection conn;

  // When A DATE value is fetched into a WCHAR buffer smaller than the date string
  auto stmt = conn.execute_fetch("SELECT '2024-01-15'::DATE");
  char16_t buffer[6] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, buffer, sizeof(buffer), &indicator);

  // Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004
  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "01004");
}

TEST_CASE("DATE NULL to SQL_C_WCHAR", "[date][conversion][c_wchar][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL DATE value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::DATE");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_WCHAR);
}
