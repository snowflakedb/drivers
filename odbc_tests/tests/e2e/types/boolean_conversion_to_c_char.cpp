#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

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
// SQLBindCol — SQL_C_CHAR
// ============================================================================

TEST_CASE("should convert boolean using SQLBindCol for SQL_C_CHAR", "[datatype][boolean][conversion][char]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed with SQLBindCol for SQL_C_CHAR
  const auto stmt = conn.createStatement();
  char true_buf[16] = {};
  char false_buf[16] = {};
  SQLLEN true_ind = 0;
  SQLLEN false_ind = 0;

  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_CHAR, true_buf, sizeof(true_buf), &true_ind);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindCol(stmt.getHandle(), 2, SQL_C_CHAR, false_buf, sizeof(false_buf), &false_ind);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT TRUE::BOOLEAN, FALSE::BOOLEAN"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then Bound buffers should contain "1" for TRUE and "0" for FALSE
  REQUIRE(true_ind == 1);
  REQUIRE(std::string(true_buf, 1) == "1");
  REQUIRE(true_buf[1] == '\0');
  REQUIRE(false_ind == 1);
  REQUIRE(std::string(false_buf, 1) == "0");
  REQUIRE(false_buf[1] == '\0');
}

// ============================================================================
// NULL handling
// ============================================================================

TEST_CASE("should handle NULL boolean with c_type", "[datatype][boolean][conversion][char]") {
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

  // Then <c_type> should return SQL_NULL_DATA indicator
  {
    INFO("SQL_C_CHAR");
    check_null(SQL_C_CHAR);
  }
  {
    INFO("SQL_C_WCHAR");
    check_null(SQL_C_WCHAR);
  }
}
