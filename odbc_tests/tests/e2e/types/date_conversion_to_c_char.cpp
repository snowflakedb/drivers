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
  Connection conn;

  {
    INFO("basic date");
    auto result = check_char_success(conn.execute_fetch("SELECT '2024-01-15'::DATE"), 1);
    CHECK(result == "2024-01-15");
  }

  {
    INFO("pre-epoch date");
    auto result = check_char_success(conn.execute_fetch("SELECT '1960-06-15'::DATE"), 1);
    CHECK(result == "1960-06-15");
  }

  {
    INFO("leap day");
    auto result = check_char_success(conn.execute_fetch("SELECT '2000-02-29'::DATE"), 1);
    CHECK(result == "2000-02-29");
  }

  {
    INFO("epoch");
    auto result = check_char_success(conn.execute_fetch("SELECT '1970-01-01'::DATE"), 1);
    CHECK(result == "1970-01-01");
  }

  {
    INFO("end of year");
    auto result = check_char_success(conn.execute_fetch("SELECT '1999-12-31'::DATE"), 1);
    CHECK(result == "1999-12-31");
  }

  {
    INFO("first day of year");
    auto result = check_char_success(conn.execute_fetch("SELECT '2025-01-01'::DATE"), 1);
    CHECK(result == "2025-01-01");
  }

  {
    INFO("leap year non-leap day (Feb 28)");
    auto result = check_char_success(conn.execute_fetch("SELECT '2024-02-28'::DATE"), 1);
    CHECK(result == "2024-02-28");
  }

  {
    INFO("non-leap year Feb 28");
    auto result = check_char_success(conn.execute_fetch("SELECT '2023-02-28'::DATE"), 1);
    CHECK(result == "2023-02-28");
  }
}

TEST_CASE("DATE to SQL_C_CHAR exact buffer fit", "[date][conversion][c_char]") {
  Connection conn;

  // "yyyy-mm-dd" = 10 chars + null terminator = 11 bytes exactly
  auto stmt = conn.execute_fetch("SELECT '2024-01-15'::DATE");
  char buffer[11] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);

  CHECK(ret == SQL_SUCCESS);
  CHECK(indicator == 10);
  CHECK(std::string(buffer) == "2024-01-15");
}

TEST_CASE("DATE to SQL_C_CHAR truncation", "[date][conversion][c_char][01004]") {
  SKIP_OLD_DRIVER("BD#41", "old driver returns error instead of 01004 truncation");
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT '2024-01-15'::DATE");
  char buffer[8] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);

  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(indicator == 10);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "01004");
  CHECK(std::string(buffer) == "2024-01");
}

TEST_CASE("DATE NULL to SQL_C_CHAR", "[date][conversion][c_char][null]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT NULL::DATE");

  check_null_via_get_data(stmt, 1, SQL_C_CHAR);
}

// ============================================================================
// SQL_C_WCHAR
// ============================================================================

TEST_CASE("DATE to SQL_C_WCHAR", "[date][conversion][c_char]") {
  Connection conn;

  {
    INFO("basic date");
    auto result = check_wchar_success(conn.execute_fetch("SELECT '2024-01-15'::DATE"), 1);
    CHECK(result == u"2024-01-15");
  }

  {
    INFO("pre-epoch date");
    auto result = check_wchar_success(conn.execute_fetch("SELECT '1960-06-15'::DATE"), 1);
    CHECK(result == u"1960-06-15");
  }

  {
    INFO("leap day");
    auto result = check_wchar_success(conn.execute_fetch("SELECT '2000-02-29'::DATE"), 1);
    CHECK(result == u"2000-02-29");
  }

  {
    INFO("epoch");
    auto result = check_wchar_success(conn.execute_fetch("SELECT '1970-01-01'::DATE"), 1);
    CHECK(result == u"1970-01-01");
  }
}

TEST_CASE("DATE to SQL_C_WCHAR truncation", "[date][conversion][c_char][01004]") {
  SKIP_OLD_DRIVER("BD#41", "old driver returns error instead of 01004 truncation");
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT '2024-01-15'::DATE");
  char16_t buffer[6] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, buffer, sizeof(buffer), &indicator);

  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "01004");
}

TEST_CASE("DATE NULL to SQL_C_WCHAR", "[date][conversion][c_char][null]") {
  Connection conn;

  auto stmt = conn.execute_fetch("SELECT NULL::DATE");

  check_null_via_get_data(stmt, 1, SQL_C_WCHAR);
}
