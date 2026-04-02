#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "conversion_checks.hpp"
#include "get_diag_rec.hpp"

TEST_CASE("TIMESTAMP_LTZ to SQL_C_WCHAR", "[timestamp_ltz][conversion][c_wchar]") {
  // Given Snowflake client is logged in with a known session timezone
  Connection conn;
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");

  // When TIMESTAMP_LTZ values are fetched as SQL_C_WCHAR
  (void)0;
  // Then Wide string representation matches expected format
  {
    INFO("basic timestamp");
    auto result = check_wchar_success(conn.execute_fetch("SELECT '2024-01-15 14:30:45'::TIMESTAMP_LTZ"), 1);
    CHECK(result == u"2024-01-15 14:30:45");
  }

  {
    INFO("timestamp with fractional seconds");
    auto result = check_wchar_success(conn.execute_fetch("SELECT '2024-01-15 10:30:00.123456789'::TIMESTAMP_LTZ"), 1);
    CHECK(result == u"2024-01-15 10:30:00.123456789");
  }

  {
    INFO("pre-epoch timestamp");
    auto result = check_wchar_success(conn.execute_fetch("SELECT '1960-06-15 12:00:00'::TIMESTAMP_LTZ"), 1);
    CHECK(result == u"1960-06-15 12:00:00");
  }

  {
    INFO("midnight");
    auto result = check_wchar_success(conn.execute_fetch("SELECT '2024-06-15 00:00:00'::TIMESTAMP_LTZ"), 1);
    CHECK(result == u"2024-06-15 00:00:00");
  }
}

TEST_CASE("TIMESTAMP_LTZ to SQL_C_WCHAR buffer too small", "[timestamp_ltz][conversion][c_wchar][22003]") {
  // Given Snowflake client is logged in with a known session timezone
  Connection conn;
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");

  // When A TIMESTAMP_LTZ value is fetched into a WCHAR buffer smaller than 20 characters
  auto stmt = conn.execute_fetch("SELECT '2024-01-15 14:30:45'::TIMESTAMP_LTZ");
  char16_t buffer[5] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, buffer, sizeof(buffer), &indicator);

  // Then SQL_ERROR is returned with SQLSTATE 22003
  CHECK(ret == SQL_ERROR);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "22003");
}

TEST_CASE("TIMESTAMP_LTZ to SQL_C_WCHAR truncation", "[timestamp_ltz][conversion][c_wchar][01004]") {
  SKIP_OLD_DRIVER("BD#32", "Old driver crashes on TIMESTAMP to SQL_C_WCHAR truncation");
  // Given Snowflake client is logged in with a known session timezone
  Connection conn;
  conn.execute("ALTER SESSION SET TIMEZONE = 'UTC'");

  // When A TIMESTAMP_LTZ with fractional seconds is fetched into a WCHAR buffer of 21 characters
  auto stmt = conn.execute_fetch("SELECT '2024-01-15 10:30:00.123456789'::TIMESTAMP_LTZ");
  char16_t buffer[21] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_WCHAR, buffer, sizeof(buffer), &indicator);

  // Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004
  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "01004");
}

TEST_CASE("TIMESTAMP_LTZ NULL to SQL_C_WCHAR", "[timestamp_ltz][conversion][c_wchar][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL TIMESTAMP_LTZ value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::TIMESTAMP_LTZ");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_WCHAR);
}
