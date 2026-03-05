#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "conversion_checks.hpp"
#include "macros.hpp"
#include "odbc_cast.hpp"

// ============================================================================
// SQL_C_BIT
// ============================================================================

TEST_CASE("should convert boolean to SQL_C_BIT", "[datatype][boolean][conversion][bit]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
  const auto stmt = conn.execute_fetch("SELECT TRUE::BOOLEAN, FALSE::BOOLEAN");

  // Then SQL_C_BIT should return 1 for TRUE and 0 for FALSE
  REQUIRE(check_no_truncation<SQL_C_BIT>(stmt, 1) == 1);
  REQUIRE(check_no_truncation<SQL_C_BIT>(stmt, 2) == 0);
}

// ============================================================================
// SQL_C_DEFAULT
// ============================================================================

TEST_CASE("should convert boolean to SQL_C_DEFAULT", "[datatype][boolean][conversion][bit]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed
  const auto stmt = conn.execute_fetch("SELECT TRUE::BOOLEAN, FALSE::BOOLEAN");

  // Then SQL_C_DEFAULT should return the same values as SQL_C_BIT
  SQLCHAR true_val = 0xFF;
  SQLLEN indicator = 0;
  SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_DEFAULT, &true_val, sizeof(true_val), &indicator);
  CHECK_ODBC(ret, stmt);
  REQUIRE(true_val == 1);
  REQUIRE(indicator == sizeof(SQLCHAR));

  SQLCHAR false_val = 0xFF;
  indicator = -999;
  ret = SQLGetData(stmt.getHandle(), 2, SQL_C_DEFAULT, &false_val, sizeof(false_val), &indicator);
  CHECK_ODBC(ret, stmt);
  REQUIRE(false_val == 0);
  REQUIRE(indicator == sizeof(SQLCHAR));
}

// ============================================================================
// NULL handling
// ============================================================================

TEST_CASE("should handle NULL boolean with SQL_C_BIT", "[datatype][boolean][conversion][bit]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT NULL::BOOLEAN" is executed
  const auto stmt = conn.execute_fetch("SELECT NULL::BOOLEAN");

  // Then SQL_C_BIT should return SQL_NULL_DATA indicator
  char buffer[100] = {};
  SQLLEN indicator = 0;
  const SQLRETURN ret = SQLGetData(stmt.getHandle(), 1, SQL_C_BIT, buffer, sizeof(buffer), &indicator);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(indicator == SQL_NULL_DATA);
}

// ============================================================================
// SQLBindCol
// ============================================================================

TEST_CASE("should convert boolean using SQLBindCol for SQL_C_BIT", "[datatype][boolean][conversion][bit]") {
  // Given Snowflake client is logged in
  Connection conn;

  // When Query "SELECT TRUE::BOOLEAN, FALSE::BOOLEAN" is executed with SQLBindCol for SQL_C_BIT
  const auto stmt = conn.createStatement();
  SQLCHAR true_val = 0xFF;
  SQLCHAR false_val = 0xFF;
  SQLLEN true_ind = 0;
  SQLLEN false_ind = 0;

  SQLRETURN ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_BIT, &true_val, sizeof(true_val), &true_ind);
  CHECK_ODBC(ret, stmt);
  ret = SQLBindCol(stmt.getHandle(), 2, SQL_C_BIT, &false_val, sizeof(false_val), &false_ind);
  CHECK_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT TRUE::BOOLEAN, FALSE::BOOLEAN"), SQL_NTS);
  CHECK_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  CHECK_ODBC(ret, stmt);

  // Then the bound values should be 1 and 0
  REQUIRE(true_val == 1);
  REQUIRE(false_val == 0);
  REQUIRE(true_ind == sizeof(SQLCHAR));
  REQUIRE(false_ind == sizeof(SQLCHAR));
}
