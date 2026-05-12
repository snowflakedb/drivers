#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <cstring>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "sf_odbc.h"

static std::string get_last_query_id(StatementHandleWrapper& stmt) {
  char buf[64] = {};
  SQLINTEGER len = 0;
  SQLRETURN ret = SQLGetStmtAttrW(stmt.getHandle(), SQL_SF_STMT_ATTR_LAST_QUERY_ID, buf, sizeof(buf), &len);
  REQUIRE((ret == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO));
  return std::string(buf, len);
}

static bool is_valid_query_id(const std::string& id) {
  if (id.length() != 36) return false;
  for (size_t i = 0; i < id.length(); ++i) {
    if (i == 8 || i == 13 || i == 18 || i == 23) {
      if (id[i] != '-') return false;
    } else {
      if (!std::isxdigit(static_cast<unsigned char>(id[i]))) return false;
    }
  }
  return true;
}

// =============================================================================
// SUCCESSFUL QUERIES
// =============================================================================

TEST_CASE("Last query ID is empty before any query is executed.", "[query][last_query_id]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLGetStmtAttr is called for SQL_SF_STMT_ATTR_LAST_QUERY_ID
  auto id = get_last_query_id(stmt);

  // Then the returned query ID should be empty
  CHECK(id.empty());
}

TEST_CASE("Last query ID is set after successful SQLExecDirect.", "[query][last_query_id]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLExecDirect is called with a valid query
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 1", SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // Then the last query ID should be a valid UUID
  auto id = get_last_query_id(stmt);
  CHECK(is_valid_query_id(id));
}

TEST_CASE("Last query ID is set after successful SQLPrepare + SQLExecute.", "[query][last_query_id]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLPrepare and SQLExecute are called with a valid query
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), (SQLCHAR*)"SELECT 1", SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);

  // Then the last query ID should be a valid UUID
  auto id = get_last_query_id(stmt);
  CHECK(is_valid_query_id(id));
}

TEST_CASE("Successive queries produce different last query IDs.", "[query][last_query_id]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When two different queries are executed sequentially
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 1", SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  auto id1 = get_last_query_id(stmt);

  ret = SQLCloseCursor(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 2", SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  auto id2 = get_last_query_id(stmt);

  // Then the last query IDs should differ
  CHECK(is_valid_query_id(id1));
  CHECK(is_valid_query_id(id2));
  CHECK(id1 != id2);
}

// =============================================================================
// FAILED QUERIES
// =============================================================================

TEST_CASE("Last query ID is set after failed query with syntax error.", "[query][last_query_id]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLExecDirect is called with a syntax error
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT failed query syntax", SQL_NTS);
  REQUIRE(ret == SQL_ERROR);

  // Then the last query ID should be a valid UUID
  auto id = get_last_query_id(stmt);
  CHECK(is_valid_query_id(id));
}

TEST_CASE("Last query ID is set after failed query referencing nonexistent table.", "[query][last_query_id]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLExecDirect is called referencing a nonexistent table
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT col FROM non_existent_table_xyz_12345", SQL_NTS);
  REQUIRE(ret == SQL_ERROR);

  // Then the last query ID should be a valid UUID
  auto id = get_last_query_id(stmt);
  CHECK(is_valid_query_id(id));
}

TEST_CASE("Last query ID changes between successive failed queries.", "[query][last_query_id]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When two different invalid queries are executed sequentially
  SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT failed query 1", SQL_NTS);
  auto id1 = get_last_query_id(stmt);

  SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT failed query 2", SQL_NTS);
  auto id2 = get_last_query_id(stmt);

  // Then the last query IDs should differ
  CHECK(is_valid_query_id(id1));
  CHECK(is_valid_query_id(id2));
  CHECK(id1 != id2);
}

TEST_CASE("Last query ID updates from successful to failed query.", "[query][last_query_id]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a successful query is followed by a failed query
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 1", SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  auto id_success = get_last_query_id(stmt);

  ret = SQLCloseCursor(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT failed query", SQL_NTS);
  REQUIRE(ret == SQL_ERROR);
  auto id_fail = get_last_query_id(stmt);

  // Then the last query IDs should differ
  CHECK(is_valid_query_id(id_success));
  CHECK(is_valid_query_id(id_fail));
  CHECK(id_success != id_fail);
}
