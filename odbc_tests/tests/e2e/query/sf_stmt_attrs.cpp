#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "get_data.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "sf_odbc.h"

// ============================================================================
// SQL_SF_STMT_ATTR_LAST_QUERY_ID
// ============================================================================

TEST_CASE("SQL_SF_STMT_ATTR_LAST_QUERY_ID returns empty string before any execution.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_SF_STMT_ATTR_LAST_QUERY_ID is queried on a fresh statement
  char query_id[256] = {};
  SQLINTEGER len = 0;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_LAST_QUERY_ID, query_id, sizeof(query_id), &len);

  // Then it should return SQL_SUCCESS and an empty string (BD#56)
  NEW_DRIVER_ONLY("BD#56") {
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(std::string(query_id).empty());
  }
  // Old driver silently accepts the call and returns SQL_SUCCESS
  OLD_DRIVER_ONLY("BD#56") { REQUIRE(ret == SQL_SUCCESS); }
}

TEST_CASE("SQL_SF_STMT_ATTR_LAST_QUERY_ID set returns HY092.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_SF_STMT_ATTR_LAST_QUERY_ID is set to any value
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_LAST_QUERY_ID, (SQLPOINTER) "some-id", SQL_NTS);

  // Under iODBC the SetStmtOption shim (see c_api.rs) makes
  // SQLSetStmtAttr_Internal's default branch forward the call to the
  // driver, so the driver's HY092 path is reachable on every DM.
  NEW_DRIVER_ONLY("BD#56") {
    // Then the driver returns SQL_ERROR with SQLSTATE HY092 (BD#56)
    REQUIRE(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "HY092");
  }
}

TEST_CASE("SQL_SF_STMT_ATTR_LAST_QUERY_ID returns non-empty query ID after SQLExecDirect.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLExecDirect is called to execute a simple SELECT query
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 1", SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // And SQL_SF_STMT_ATTR_LAST_QUERY_ID is queried
  char query_id[256] = {};
  SQLINTEGER len = 0;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_LAST_QUERY_ID, query_id, sizeof(query_id), &len);

  // Then it should return SQL_SUCCESS and a non-empty query ID string (BD#56)
  NEW_DRIVER_ONLY("BD#56") {
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(!std::string(query_id).empty());
    CHECK(len > 0);
  }
  // Old driver silently accepts the call and returns SQL_SUCCESS
  OLD_DRIVER_ONLY("BD#56") { REQUIRE(ret == SQL_SUCCESS); }
}

TEST_CASE("SQL_SF_STMT_ATTR_LAST_QUERY_ID returns non-empty query ID after SQLPrepare and SQLExecute.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLPrepare and SQLExecute are called to execute a simple SELECT query
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"SELECT 1", SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);

  // And SQL_SF_STMT_ATTR_LAST_QUERY_ID is queried
  char query_id[256] = {};
  SQLINTEGER len = 0;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_LAST_QUERY_ID, query_id, sizeof(query_id), &len);

  // Then it should return SQL_SUCCESS and a non-empty query ID string (BD#56)
  NEW_DRIVER_ONLY("BD#56") {
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(!std::string(query_id).empty());
    CHECK(len > 0);
  }
}

TEST_CASE("SQL_SF_STMT_ATTR_LAST_QUERY_ID each execution produces a distinct query ID.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLExecDirect is called twice on the same statement
  // And SQL_SF_STMT_ATTR_LAST_QUERY_ID is queried after each execution
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 1", SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  char first_id[256] = {};
  SQLRETURN get_ret_1 =
      SQLGetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_LAST_QUERY_ID, first_id, sizeof(first_id), nullptr);
  ret = SQLFreeStmt(stmt.getHandle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 2", SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  char second_id[256] = {};
  SQLRETURN get_ret_2 =
      SQLGetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_LAST_QUERY_ID, second_id, sizeof(second_id), nullptr);

  NEW_DRIVER_ONLY("BD#56") {
    NON_IODBC {
      // Then each query-ID value is non-empty and distinct (BD#56)
      REQUIRE(get_ret_1 == SQL_SUCCESS);
      REQUIRE(get_ret_2 == SQL_SUCCESS);
      CHECK(!std::string(first_id).empty());
      CHECK(!std::string(second_id).empty());
      CHECK(std::string(first_id) != std::string(second_id));
    }
    IODBC_ONLY {
      // Then the SQLGetStmtAttr call succeeds (SQL_SUCCESS) but
      //   the UUID is not preserved into the narrow ANSI buffer:
      //   SQL_SF_STMT_ATTR_LAST_QUERY_ID is routed through the wide path and
      //   collapses to a placeholder (e.g. "0"). Only the return code is
      //   asserted because the buffer contents are not reliable.
      //   TODO: revisit once SQLGetStmtAttrW handling of vendor string
      //   attributes is fixed end-to-end.
      CHECK(get_ret_1 == SQL_SUCCESS);
      CHECK(get_ret_2 == SQL_SUCCESS);
    }
  }
}

// ============================================================================
// SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT
// ============================================================================

TEST_CASE("SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT default value is -1.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT is queried on a fresh statement
  SQLINTEGER value = 0;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT, &value, 0, nullptr);

  // Then it should return SQL_SUCCESS and the value -1 (BD#56)
  NEW_DRIVER_ONLY("BD#56") {
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(value == -1);
  }
}

TEST_CASE("SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT can be set and retrieved.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT is set to 0, then 3, then reset to -1
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT, (SQLPOINTER)0, 0);

  NEW_DRIVER_ONLY("BD#56") {
    // Then each get returns the value that was last set (BD#56)
    REQUIRE(ret == SQL_SUCCESS);
    SQLINTEGER val = -99;
    ret = SQLGetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT, &val, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(val == 0);

    ret = SQLSetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT, (SQLPOINTER)3, 0);
    REQUIRE(ret == SQL_SUCCESS);
    val = -99;
    ret = SQLGetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT, &val, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(val == 3);

    ret = SQLSetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT, (SQLPOINTER)-1, 0);
    REQUIRE(ret == SQL_SUCCESS);
    val = 0;
    ret = SQLGetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT, &val, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(val == -1);
  }
}

TEST_CASE("SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT invalid value less than -1 returns HY024.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT is set to -2
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT, (SQLPOINTER)-2, 0);

  // The SetStmtOption shim makes the driver's HY024 validation path
  // reachable on every DM.
  NEW_DRIVER_ONLY("BD#56") {
    // Then the driver returns SQL_ERROR with SQLSTATE HY024 (BD#56)
    REQUIRE(ret == SQL_ERROR);
    CHECK(get_sqlstate(stmt) == "HY024");
  }
}

TEST_CASE("SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT forwards count to server and executes all statements.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When MULTI_STATEMENT_COUNT is set to 2 and a two-SELECT batch is executed
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT, (SQLPOINTER)2, 0);

  NEW_DRIVER_ONLY("BD#56") {
    // The SetStmtOption shim routes the call to the driver under iODBC, so
    // the multistatement batch dispatches identically on every DM.
    // Then the count is forwarded to the server, both result sets are produced in
    //   order, and a third SQLMoreResults reports SQL_NO_DATA (BD#56)
    REQUIRE(ret == SQL_SUCCESS);

    ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT 1 AS a; SELECT 2 AS b"), SQL_NTS);
    REQUIRE_ODBC(ret, stmt);

    // And first result set contains 1
    ret = SQLFetch(stmt.getHandle());
    REQUIRE_ODBC(ret, stmt);
    CHECK(get_data<SQL_C_LONG>(stmt, 1) == 1);

    // And second result set contains 2
    ret = SQLMoreResults(stmt.getHandle());
    REQUIRE_ODBC(ret, stmt);
    ret = SQLFetch(stmt.getHandle());
    REQUIRE_ODBC(ret, stmt);
    CHECK(get_data<SQL_C_LONG>(stmt, 1) == 2);

    // And no more result sets are produced
    ret = SQLMoreResults(stmt.getHandle());
    CHECK(ret == SQL_NO_DATA);
  }
}
