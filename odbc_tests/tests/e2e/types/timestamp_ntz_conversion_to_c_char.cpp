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

TEST_CASE("TIMESTAMP_NTZ to SQL_C_CHAR", "[timestamp_ntz][conversion][c_char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When TIMESTAMP_NTZ values are fetched as SQL_C_CHAR
  (void)0;
  // Then String representation matches expected format
  {
    INFO("basic timestamp without fractional seconds");
    auto result = check_char_success(conn.execute_fetch("SELECT '2024-01-15 14:30:45'::TIMESTAMP_NTZ"), 1);
    CHECK(result == "2024-01-15 14:30:45");
  }

  {
    INFO("timestamp with fractional seconds");
    auto result = check_char_success(conn.execute_fetch("SELECT '2024-01-15 10:30:00.123456789'::TIMESTAMP_NTZ"), 1);
    CHECK(result == "2024-01-15 10:30:00.123456789");
  }

  {
    INFO("epoch");
    auto result = check_char_success(conn.execute_fetch("SELECT '1970-01-01 00:00:00'::TIMESTAMP_NTZ"), 1);
    CHECK(result == "1970-01-01 00:00:00");
  }

  {
    INFO("midnight");
    auto result = check_char_success(conn.execute_fetch("SELECT '2024-06-15 00:00:00'::TIMESTAMP_NTZ"), 1);
    CHECK(result == "2024-06-15 00:00:00");
  }

  {
    INFO("pre-epoch timestamp");
    auto result = check_char_success(conn.execute_fetch("SELECT '1960-06-15 12:00:00'::TIMESTAMP_NTZ"), 1);
    CHECK(result == "1960-06-15 12:00:00");
  }
}

TEST_CASE("TIMESTAMP_NTZ to SQL_C_CHAR fractional truncation", "[timestamp_ntz][conversion][c_char][01004]") {
  SKIP_OLD_DRIVER("BD#32", "Old driver crashes (SIGSEGV) on TIMESTAMP_NTZ to SQL_C_CHAR truncation");
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIMESTAMP_NTZ with fractional seconds is fetched into a 21-byte buffer
  auto stmt = conn.execute_fetch("SELECT '2024-01-15 10:30:00.123456789'::TIMESTAMP_NTZ");
  char buffer[21] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);

  // Then SQL_SUCCESS_WITH_INFO is returned with SQLSTATE 01004 and fractional part truncated
  CHECK(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(indicator == 29);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "01004");
  CHECK(std::string(buffer) == "2024-01-15 10:30:00.");
}

TEST_CASE("TIMESTAMP_NTZ to SQL_C_CHAR buffer too small", "[timestamp_ntz][conversion][c_char][22003]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A TIMESTAMP_NTZ value is fetched into a buffer smaller than 20 bytes
  auto stmt = conn.execute_fetch("SELECT '2024-01-15 14:30:45'::TIMESTAMP_NTZ");
  char buffer[10] = {};
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_CHAR, buffer, sizeof(buffer), &indicator);

  // Then SQL_ERROR is returned with SQLSTATE 22003
  CHECK(ret == SQL_ERROR);
  auto records = get_diag_rec(stmt);
  CHECK(!records.empty());
  CHECK(records[0].sqlState == "22003");
}

TEST_CASE("TIMESTAMP_NTZ NULL to SQL_C_CHAR", "[timestamp_ntz][conversion][c_char][null]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When A NULL TIMESTAMP_NTZ value is queried
  auto stmt = conn.execute_fetch("SELECT NULL::TIMESTAMP_NTZ");

  // Then Indicator returns SQL_NULL_DATA
  check_null_via_get_data(stmt, 1, SQL_C_CHAR);
}
