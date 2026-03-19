#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "sf_odbc.h"

// ============================================================================
// SQL_SF_STMT_ATTR_LAST_QUERY_ID
// ============================================================================

TEST_CASE("SQL_SF_STMT_ATTR_LAST_QUERY_ID returns empty string before any execution.") {
  SKIP_OLD_DRIVER("SNOW-3235556", "Snowflake custom statement attributes are new driver only");

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_SF_STMT_ATTR_LAST_QUERY_ID is queried on a fresh statement
  char query_id[256] = {};
  SQLINTEGER len = 0;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_LAST_QUERY_ID, query_id, sizeof(query_id), &len);

  // Then it should return SQL_SUCCESS and an empty string
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(std::string(query_id).empty());
}

TEST_CASE("SQL_SF_STMT_ATTR_LAST_QUERY_ID set returns HY092.") {
  SKIP_OLD_DRIVER("SNOW-3235556", "Snowflake custom statement attributes are new driver only");

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_SF_STMT_ATTR_LAST_QUERY_ID is set to any value
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_LAST_QUERY_ID, (SQLPOINTER) "some-id", SQL_NTS);

  // Then it should return SQL_ERROR with SQLSTATE HY092
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "HY092");
}

TEST_CASE("SQL_SF_STMT_ATTR_LAST_QUERY_ID returns non-empty query ID after SQLExecDirect.") {
  SKIP_OLD_DRIVER("SNOW-3235556", "Snowflake custom statement attributes are new driver only");

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

  // Then it should return SQL_SUCCESS and a non-empty query ID string
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(!std::string(query_id).empty());
  CHECK(len > 0);
}

TEST_CASE("SQL_SF_STMT_ATTR_LAST_QUERY_ID returns non-empty query ID after SQLPrepare and SQLExecute.") {
  SKIP_OLD_DRIVER("SNOW-3235556", "Snowflake custom statement attributes are new driver only");

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

  // Then it should return SQL_SUCCESS and a non-empty query ID string
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(!std::string(query_id).empty());
  CHECK(len > 0);
}

TEST_CASE("SQL_SF_STMT_ATTR_LAST_QUERY_ID each execution produces a distinct query ID.") {
  SKIP_OLD_DRIVER("SNOW-3235556", "Snowflake custom statement attributes are new driver only");

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLExecDirect is called twice on the same statement
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 1", SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  char first_id[256] = {};
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_LAST_QUERY_ID, first_id, sizeof(first_id), nullptr);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 2", SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  char second_id[256] = {};
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_LAST_QUERY_ID, second_id, sizeof(second_id), nullptr);
  REQUIRE(ret == SQL_SUCCESS);

  // Then each SQL_SF_STMT_ATTR_LAST_QUERY_ID value should be non-empty and different
  CHECK(!std::string(first_id).empty());
  CHECK(!std::string(second_id).empty());
  CHECK(std::string(first_id) != std::string(second_id));
}

// ============================================================================
// SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT
// ============================================================================

TEST_CASE("SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT default value is -1.") {
  SKIP_OLD_DRIVER("SNOW-3235556", "Snowflake custom statement attributes are new driver only");

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT is queried on a fresh statement
  SQLINTEGER value = 0;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT, &value, 0, nullptr);

  // Then it should return SQL_SUCCESS and the value -1
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(value == -1);
}

TEST_CASE("SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT can be set and retrieved.") {
  SKIP_OLD_DRIVER("SNOW-3235556", "Snowflake custom statement attributes are new driver only");

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT is set to 0, then 3, then reset to -1
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT, (SQLPOINTER)0, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then each get should return the value that was set
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

TEST_CASE("SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT invalid value less than -1 returns error.") {
  SKIP_OLD_DRIVER("SNOW-3235556", "Snowflake custom statement attributes are new driver only");

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT is set to -2
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_MULTI_STATEMENT_COUNT, (SQLPOINTER)-2, 0);

  // Then it should return SQL_ERROR
  REQUIRE(ret == SQL_ERROR);
}
