#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"

// ============================================================================
// SQL_ATTR_ROW_NUMBER
// ============================================================================

TEST_CASE("SQL_ATTR_ROW_NUMBER default value is 0.") {
  SKIP_OLD_DRIVER("SNOW-3235556", "Cursor statement attributes are new driver feature");

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_ROW_NUMBER is queried on a fresh statement
  SQLULEN value = 99;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_NUMBER, &value, 0, nullptr);

  // Then it should return SQL_SUCCESS and the value 0
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(value == 0);
}

TEST_CASE("SQL_ATTR_ROW_NUMBER is read-only.") {
  SKIP_OLD_DRIVER("SNOW-3235556", "Cursor statement attributes are new driver feature");

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_ROW_NUMBER is set on a statement
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_NUMBER, (SQLPOINTER)1, 0);

  // Then it should return SQL_ERROR with HY092
  REQUIRE(ret == SQL_ERROR);
}

TEST_CASE("SQL_ATTR_ROW_NUMBER increments on each SQLFetch call.") {
  SKIP_OLD_DRIVER("SNOW-3235556", "Cursor statement attributes are new driver feature");

  // Given Snowflake client is logged in
  Connection conn;

  // When SQLFetch is called repeatedly on a result set
  auto stmt = conn.execute("SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3");

  SQLULEN row_num = 0;

  SQLFetch(stmt.getHandle());
  SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_NUMBER, &row_num, 0, nullptr);
  CHECK(row_num == 1);

  SQLFetch(stmt.getHandle());
  SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_NUMBER, &row_num, 0, nullptr);
  CHECK(row_num == 2);

  // Then SQL_ATTR_ROW_NUMBER should increment by 1 on each call
  SQLFetch(stmt.getHandle());
  SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_NUMBER, &row_num, 0, nullptr);
  CHECK(row_num == 3);
}

TEST_CASE("SQL_ATTR_ROW_NUMBER resets to 0 after all rows are fetched.") {
  SKIP_OLD_DRIVER("SNOW-3235556", "Cursor statement attributes are new driver feature");

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.execute("SELECT 1");

  // Fetch the only row
  SQLFetch(stmt.getHandle());

  // When all rows have been fetched from a result set
  SQLRETURN ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);

  // Then SQL_ATTR_ROW_NUMBER should be 0
  SQLULEN row_num = 99;
  SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_NUMBER, &row_num, 0, nullptr);
  CHECK(row_num == 0);
}

// ============================================================================
// SQL_ATTR_ROW_OPERATION_PTR
// ============================================================================

TEST_CASE("SQL_ATTR_ROW_OPERATION_PTR default value is NULL.") {
  SKIP_OLD_DRIVER("SNOW-3235556", "Cursor statement attributes are new driver feature");

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_ROW_OPERATION_PTR is queried on a fresh statement
  SQLUSMALLINT* ptr = reinterpret_cast<SQLUSMALLINT*>(0xdeadbeef);
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_OPERATION_PTR, &ptr, 0, nullptr);

  // Then it should return SQL_SUCCESS and the value NULL
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(ptr == nullptr);
}

TEST_CASE("SQL_ATTR_ROW_OPERATION_PTR can be set and retrieved.") {
  SKIP_OLD_DRIVER("SNOW-3235556", "Cursor statement attributes are new driver feature");

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_ROW_OPERATION_PTR is set to a pointer
  SQLUSMALLINT ops[5] = {};
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_OPERATION_PTR, ops, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then it should return SQL_SUCCESS and the retrieved pointer should match
  SQLUSMALLINT* retrieved = nullptr;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_OPERATION_PTR, &retrieved, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(retrieved == ops);
}
