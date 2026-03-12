#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"
#include "macros.hpp"

// ============================================================================
// SQL_C_CHAR
// ============================================================================

TEST_CASE("should convert boolean to SQL_C_CHAR", "[datatype][boolean][conversion][char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
  const auto stmt = conn.execute_fetch("SELECT TRUE::BOOLEAN, FALSE::BOOLEAN");

  // Then SQL_C_CHAR should return "1" for TRUE and "0" for FALSE
  REQUIRE(check_char_success(stmt, 1) == "1");
  REQUIRE(check_char_success(stmt, 2) == "0");
}

// ============================================================================
// SQL_C_WCHAR
// ============================================================================

TEST_CASE("should convert boolean to SQL_C_WCHAR", "[datatype][boolean][conversion][char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
  const auto stmt = conn.execute_fetch("SELECT TRUE::BOOLEAN, FALSE::BOOLEAN");

  // Then SQL_C_WCHAR should return "1" for TRUE and "0" for FALSE
  REQUIRE(check_wchar_success(stmt, 1) == u"1");
  REQUIRE(check_wchar_success(stmt, 2) == u"0");
}

// ============================================================================
// NULL handling
// ============================================================================

TEST_CASE("should handle NULL boolean with character C types", "[datatype][boolean][conversion][char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT NULL::BOOLEAN" is executed
  auto check_null = [&](SQLSMALLINT c_type) {
    const auto stmt = conn.execute_fetch("SELECT NULL::BOOLEAN");
    char buffer[100] = {};
    SQLLEN indicator = 0;
    const SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, c_type, buffer, sizeof(buffer), &indicator);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(indicator == SQL_NULL_DATA);
  };

  // Then SQL_C_CHAR should return SQL_NULL_DATA indicator
  check_null(SQL_C_CHAR);
  // And SQL_C_WCHAR should return SQL_NULL_DATA indicator
  check_null(SQL_C_WCHAR);
}
