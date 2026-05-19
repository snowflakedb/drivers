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

// =============================================================================
// Multi-statement queries with positional parameter bindings
// =============================================================================

TEST_CASE("should execute multistatement DML with positional parameters", "[query][multistatement][parameters]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // And A temporary table with column (id NUMBER) exists
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(),
                                sqlchar("CREATE OR REPLACE TEMPORARY TABLE ms_odbc_bind_dml(id NUMBER)"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // When Multistatement INSERT chain is executed with 3 positional parameters
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT, (SQLPOINTER)2, 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLPrepare(stmt.getHandle(),
                   sqlchar("INSERT INTO ms_odbc_bind_dml VALUES(?);"
                           " INSERT INTO ms_odbc_bind_dml VALUES(?),(?)"),
                   SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLINTEGER p1 = 10, p2 = 20, p3 = 30;
  SQLLEN p1_len = sizeof(p1), p2_len = sizeof(p2), p3_len = sizeof(p3);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_LONG, SQL_INTEGER, 0, 0, &p1, sizeof(p1), &p1_len);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_LONG, SQL_INTEGER, 0, 0, &p2, sizeof(p2), &p2_len);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 3, SQL_PARAM_INPUT, SQL_C_LONG, SQL_INTEGER, 0, 0, &p3, sizeof(p3), &p3_len);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then 2 result sets are returned
  SQLLEN row_count = 0;

  // And the first result set reports update count 1
  ret = SQLRowCount(stmt.getHandle(), &row_count);
  REQUIRE_ODBC(ret, stmt);
  CHECK(row_count == 1);

  // And the second result set reports update count 2
  ret = SQLMoreResults(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLRowCount(stmt.getHandle(), &row_count);
  REQUIRE_ODBC(ret, stmt);
  CHECK(row_count == 2);
  ret = SQLMoreResults(stmt.getHandle());
  CHECK(ret == SQL_NO_DATA);

  // And the table contains rows [10, 20, 30]
  auto verify = conn.createStatement();
  ret = SQLExecDirect(verify.getHandle(), sqlchar("SELECT id FROM ms_odbc_bind_dml ORDER BY id"), SQL_NTS);
  REQUIRE_ODBC(ret, verify);
  ret = SQLFetch(verify.getHandle());
  REQUIRE_ODBC(ret, verify);
  CHECK(get_data<SQL_C_LONG>(verify, 1) == 10);
  ret = SQLFetch(verify.getHandle());
  REQUIRE_ODBC(ret, verify);
  CHECK(get_data<SQL_C_LONG>(verify, 1) == 20);
  ret = SQLFetch(verify.getHandle());
  REQUIRE_ODBC(ret, verify);
  CHECK(get_data<SQL_C_LONG>(verify, 1) == 30);
  ret = SQLFetch(verify.getHandle());
  CHECK(ret == SQL_NO_DATA);
}

TEST_CASE("should execute multistatement SELECT with positional parameters", "[query][multistatement][parameters]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When Multistatement SELECT chain is executed with 6 positional parameters
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT, (SQLPOINTER)3, 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT ?; SELECT ?, ?; SELECT ?, ?, ?"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLINTEGER params[6] = {10, 20, 30, 40, 50, 60};
  SQLLEN param_lens[6] = {sizeof(SQLINTEGER), sizeof(SQLINTEGER), sizeof(SQLINTEGER),
                          sizeof(SQLINTEGER), sizeof(SQLINTEGER), sizeof(SQLINTEGER)};
  for (SQLUSMALLINT i = 0; i < 6; ++i) {
    ret = SQLBindParameter(stmt.getHandle(), i + 1, SQL_PARAM_INPUT, SQL_C_LONG, SQL_INTEGER, 0, 0, &params[i],
                           sizeof(SQLINTEGER), &param_lens[i]);
    REQUIRE_ODBC(ret, stmt);
  }
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then 3 result sets are returned
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // And the first result set contains row [10]
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 10);

  // And the second result set contains row [20, 30]
  ret = SQLMoreResults(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 20);
  CHECK(get_data<SQL_C_LONG>(stmt, 2) == 30);

  // And the third result set contains row [40, 50, 60]
  ret = SQLMoreResults(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);
  CHECK(get_data<SQL_C_LONG>(stmt, 1) == 40);
  CHECK(get_data<SQL_C_LONG>(stmt, 2) == 50);
  CHECK(get_data<SQL_C_LONG>(stmt, 3) == 60);
  ret = SQLMoreResults(stmt.getHandle());
  CHECK(ret == SQL_NO_DATA);
}

TEST_CASE("should fail when multistatement query has too few parameters", "[query][multistatement][parameters]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When Multistatement SELECT requires 3 parameters but only 1 is bound
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT, (SQLPOINTER)2, 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT ?; SELECT ?, ?"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLINTEGER p1 = 10;
  SQLLEN p1_len = sizeof(p1);
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_LONG, SQL_INTEGER, 0, 0, &p1, sizeof(p1), &p1_len);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then an error is returned indicating parameter count mismatch
  CHECK(ret == SQL_ERROR);
}

TEST_CASE("should fail when NULL positional parameters are used in multistatement query",
          "[query][multistatement][parameters]") {
  // Snowflake's SYSTEM$MULTISTMT server-side dispatcher rejects NULL bindings
  // with "Bind variable ? not set" — confirmed against legacy snowflake-jdbc
  // and legacy snowflake-odbc; the universal-driver inherits the same behavior.

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When Multistatement SELECT is executed with NULL positional parameters
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT, (SQLPOINTER)2, 0);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT ?; SELECT ?, ?"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  SQLINTEGER p1_buf = 0, p2_buf = 10, p3_buf = 0;
  SQLLEN p1_len = SQL_NULL_DATA;
  SQLLEN p2_len = sizeof(p2_buf);
  SQLLEN p3_len = SQL_NULL_DATA;
  ret = SQLBindParameter(stmt.getHandle(), 1, SQL_PARAM_INPUT, SQL_C_LONG, SQL_INTEGER, 0, 0, &p1_buf, sizeof(p1_buf),
                         &p1_len);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 2, SQL_PARAM_INPUT, SQL_C_LONG, SQL_INTEGER, 0, 0, &p2_buf, sizeof(p2_buf),
                         &p2_len);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLBindParameter(stmt.getHandle(), 3, SQL_PARAM_INPUT, SQL_C_LONG, SQL_INTEGER, 0, 0, &p3_buf, sizeof(p3_buf),
                         &p3_len);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());

  // Then an error is returned indicating NULL bindings are not supported
  CHECK(ret == SQL_ERROR);
}
