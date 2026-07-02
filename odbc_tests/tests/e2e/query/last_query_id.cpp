#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"
#include "query_helpers.hpp"

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

TEST_CASE("should return empty last query ID before any query is executed", "[query][last_query_id]") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLGetStmtAttr is called for SQL_SF_STMT_ATTR_LAST_QUERY_ID
  auto id = get_last_query_id(stmt);

  // Then the returned query ID should be empty
  CHECK(id.empty());
}

TEST_CASE("should set last query ID after successful SQLExecDirect", "[query][last_query_id]") {
  SKIP_IODBC("Depends on SQL_SF_STMT_ATTR_LAST_QUERY_ID; iODBC behavior covered in sf_stmt_attrs.cpp");
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLExecDirect is called with a valid query
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then the last query ID should be a valid UUID
  auto id = get_last_query_id(stmt);
  CHECK(is_valid_query_id(id));
}

TEST_CASE("should set last query ID after successful SQLPrepare + SQLExecute", "[query][last_query_id]") {
  SKIP_IODBC("Depends on SQL_SF_STMT_ATTR_LAST_QUERY_ID; iODBC behavior covered in sf_stmt_attrs.cpp");
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLPrepare and SQLExecute are called with a valid query
  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then the last query ID should be a valid UUID
  auto id = get_last_query_id(stmt);
  CHECK(is_valid_query_id(id));
}

TEST_CASE("should produce different last query IDs for successive queries", "[query][last_query_id]") {
  SKIP_IODBC("Depends on SQL_SF_STMT_ATTR_LAST_QUERY_ID; iODBC behavior covered in sf_stmt_attrs.cpp");
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When two different queries are executed sequentially
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  auto id1 = get_last_query_id(stmt);

  ret = SQLCloseCursor(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT 2"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  auto id2 = get_last_query_id(stmt);

  // Then the last query IDs should differ
  CHECK(is_valid_query_id(id1));
  CHECK(is_valid_query_id(id2));
  CHECK(id1 != id2);
}

// =============================================================================
// FAILED QUERIES
// =============================================================================

TEST_CASE("should set last query ID after failed query with syntax error", "[query][last_query_id]") {
  SKIP_IODBC("Depends on SQL_SF_STMT_ATTR_LAST_QUERY_ID; iODBC behavior covered in sf_stmt_attrs.cpp");
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLExecDirect is called with a syntax error
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT failed query syntax"), SQL_NTS);
  REQUIRE_ODBC_ERROR(ret, stmt);

  // Then the last query ID should be a valid UUID
  auto id = get_last_query_id(stmt);
  CHECK(is_valid_query_id(id));
}

TEST_CASE("should set last query ID after failed query referencing nonexistent table", "[query][last_query_id]") {
  SKIP_IODBC("Depends on SQL_SF_STMT_ATTR_LAST_QUERY_ID; iODBC behavior covered in sf_stmt_attrs.cpp");
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLExecDirect is called referencing a nonexistent table
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT col FROM non_existent_table_xyz_12345"), SQL_NTS);
  REQUIRE_ODBC_ERROR(ret, stmt);

  // Then the last query ID should be a valid UUID
  auto id = get_last_query_id(stmt);
  CHECK(is_valid_query_id(id));
}

TEST_CASE("should produce different last query IDs for successive failed queries", "[query][last_query_id]") {
  SKIP_IODBC("Depends on SQL_SF_STMT_ATTR_LAST_QUERY_ID; iODBC behavior covered in sf_stmt_attrs.cpp");
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When two different invalid queries are executed sequentially
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT failed query 1"), SQL_NTS);
  REQUIRE_ODBC_ERROR(ret, stmt);
  auto id1 = get_last_query_id(stmt);

  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT failed query 2"), SQL_NTS);
  REQUIRE_ODBC_ERROR(ret, stmt);
  auto id2 = get_last_query_id(stmt);

  // Then the last query IDs should differ
  CHECK(is_valid_query_id(id1));
  CHECK(is_valid_query_id(id2));
  CHECK(id1 != id2);
}

TEST_CASE("should update last query ID from successful to failed query", "[query][last_query_id]") {
  SKIP_IODBC("Depends on SQL_SF_STMT_ATTR_LAST_QUERY_ID; iODBC behavior covered in sf_stmt_attrs.cpp");
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When a successful query is followed by a failed query
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);
  auto id_success = get_last_query_id(stmt);

  ret = SQLCloseCursor(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT failed query"), SQL_NTS);
  REQUIRE_ODBC_ERROR(ret, stmt);
  auto id_fail = get_last_query_id(stmt);

  // Then the last query IDs should differ
  CHECK(is_valid_query_id(id_success));
  CHECK(is_valid_query_id(id_fail));
  CHECK(id_success != id_fail);
}
