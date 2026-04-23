#include <sql.h>
#include <sqlext.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "sf_odbc.h"

// =============================================================================
// Tests for multi-statement query execution via SQLMoreResults
// =============================================================================

TEST_CASE("should execute multiple SELECT statements", "[query]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When Multistatement query with 3 SELECTs is executed
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT, (SQLPOINTER)3, 0);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT 1 AS a; SELECT 2 AS b; SELECT 3 AS c"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then 3 result sets are returned
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  // And each result set contains correct data
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 1);

  // Second result set
  ret = SQLMoreResults(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 2);

  // Third result set
  ret = SQLMoreResults(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 3);

  // No more result sets
  ret = SQLMoreResults(stmt.getHandle());
  CHECK(ret == SQL_NO_DATA);
}

TEST_CASE("should execute multiple DML statements", "[query][multistatement]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When Multistatement query with CREATE TABLE, INSERT, and DROP is executed
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT, (SQLPOINTER)3, 0);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(),
                      sqlchar("CREATE OR REPLACE TEMPORARY TABLE ms_odbc_dml_test(id INT);"
                              " INSERT INTO ms_odbc_dml_test VALUES (1),(2),(3);"
                              " DROP TABLE ms_odbc_dml_test"),
                      SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then 3 result sets are returned
  ret = SQLMoreResults(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  ret = SQLMoreResults(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // No more result sets
  ret = SQLMoreResults(stmt.getHandle());
  CHECK(ret == SQL_NO_DATA);
}

TEST_CASE("should execute mixed statement types", "[query][multistatement]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When Multistatement query with various types is executed
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT, (SQLPOINTER)5, 0);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(),
                      sqlchar("ALTER SESSION SET TIMEZONE='UTC';"
                              " CREATE OR REPLACE TEMPORARY TABLE ms_odbc_mix_test(val TEXT);"
                              " INSERT INTO ms_odbc_mix_test VALUES ('hello');"
                              " SELECT val FROM ms_odbc_mix_test;"
                              " DROP TABLE ms_odbc_mix_test"),
                      SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then 5 result sets are returned
  ret = SQLMoreResults(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  ret = SQLMoreResults(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  ret = SQLMoreResults(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // And the SELECT result contains expected data
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_CHAR>(stmt, 1) == "hello");

  // 5th: DROP TABLE
  ret = SQLMoreResults(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // No more result sets
  ret = SQLMoreResults(stmt.getHandle());
  CHECK(ret == SQL_NO_DATA);
}

TEST_CASE("should fail when multistatement SQL is sent without multi_statement_count", "[query][multistatement]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When Multistatement SQL is executed without configuring multi_statement_count
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT 1; SELECT 2; SELECT 3"), SQL_NTS);

  // Then an error is returned indicating multi-statement is not enabled
  CHECK(ret == SQL_ERROR);
}

TEST_CASE("should fail when multi_statement_count does not match actual statement count", "[query][multistatement]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When Single SELECT is executed with multi_statement_count set to 3
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT, (SQLPOINTER)3, 0);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT 1"), SQL_NTS);

  // Then an error is returned indicating statement count mismatch
  CHECK(ret == SQL_ERROR);
}
