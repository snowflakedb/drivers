#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "get_data.hpp"
#include "odbc_cast.hpp"

// =============================================================================
// Tests for SQLMoreResults based on ODBC specification:
// https://learn.microsoft.com/en-us/sql/odbc/reference/syntax/sqlmoreresults-function
// =============================================================================

TEST_CASE("should return SQL_NO_DATA when there are no more result sets", "[query]") {
  // Given A query is executed and its result set is fetched
  Connection conn;
  auto stmt = conn.createStatement();

  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT 1 AS value"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 1);

  // When SQLMoreResults is called
  ret = SQLMoreResults(stmt.getHandle());

  // Then it should return SQL_NO_DATA (no additional result sets)
  CHECK(ret == SQL_NO_DATA);
}

TEST_CASE("should close cursor so re-execution succeeds without explicit cursor close", "[query]") {
  // Given A prepared statement is executed and fetched
  Connection conn;
  auto stmt = conn.createStatement();

  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT 42 AS value"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 42);

  // And SQLMoreResults is called (which should close the cursor)
  ret = SQLMoreResults(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);

  // When the same prepared statement is re-executed without explicit SQLCloseCursor / SQLFreeStmt(SQL_CLOSE)
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the result should be returned correctly
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 42);
}
