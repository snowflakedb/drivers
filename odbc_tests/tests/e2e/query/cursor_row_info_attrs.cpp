#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <string>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"

// ============================================================================
// SQL_ATTR_ROW_NUMBER (14)
// ============================================================================

TEST_CASE("SQL_ATTR_ROW_NUMBER default value is 0 on a fresh statement.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_ROW_NUMBER is queried on a fresh statement
  SQLULEN value = 99;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_NUMBER, &value, 0, nullptr);

  // Then new driver returns SQL_SUCCESS with 0; old driver returns SQL_ERROR (BD#59)
  NEW_DRIVER_ONLY("BD#59") {
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(value == 0);
  }
  OLD_DRIVER_ONLY("BD#59") { REQUIRE(ret == SQL_ERROR); }
}

TEST_CASE("SQL_ATTR_ROW_NUMBER returns 0 before any fetch.") {
  // Given a prepared and executed query with rows
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQL_ATTR_ROW_NUMBER is queried before any SQLFetch
  SQLULEN value = 99;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_NUMBER, &value, 0, nullptr);

  // Then new driver returns SQL_SUCCESS with 0; old driver returns SQL_ERROR (BD#59)
  NEW_DRIVER_ONLY("BD#59") {
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(value == 0);
  }
  OLD_DRIVER_ONLY("BD#59") { REQUIRE(ret == SQL_ERROR); }
}

TEST_CASE("SQL_ATTR_ROW_NUMBER returns 1 after the first SQLFetch.") {
  // Given an executed query
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // When the first row is fetched
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);

  // Then SQL_ATTR_ROW_NUMBER should be 1
  SQLULEN value = 0;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_NUMBER, &value, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(value == 1);
}

TEST_CASE("SQL_ATTR_ROW_NUMBER increments with each SQLFetch.") {
  // Given an executed query
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // When three rows are fetched
  for (SQLULEN expected = 1; expected <= 3; ++expected) {
    ret = SQLFetch(stmt.getHandle());
    REQUIRE(ret == SQL_SUCCESS);

    // Then SQL_ATTR_ROW_NUMBER should match the fetch count
    SQLULEN value = 0;
    ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_NUMBER, &value, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(value == expected);
  }
}

TEST_CASE("SQL_ATTR_ROW_NUMBER returns 0 after SQL_NO_DATA.") {
  // Given an executed single-row query
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // When we fetch past the end of the result set
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);

  // Then new driver returns SQL_SUCCESS with 0; old driver returns SQL_ERROR (BD#59)
  SQLULEN value = 99;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_NUMBER, &value, 0, nullptr);
  NEW_DRIVER_ONLY("BD#59") {
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(value == 0);
  }
  OLD_DRIVER_ONLY("BD#59") { REQUIRE(ret == SQL_ERROR); }
}

TEST_CASE("SQL_ATTR_ROW_NUMBER set returns SQL_ERROR with state HY092.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_ROW_NUMBER is set (it is read-only)
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_NUMBER, (SQLPOINTER)1, 0);

  // Then it should return SQL_ERROR with SQLSTATE HY092
  REQUIRE(ret == SQL_ERROR);
  auto diag = get_diag_rec(SQL_HANDLE_STMT, stmt.getHandle());
  REQUIRE(!diag.empty());
  CHECK(diag[0].sqlState == "HY092");
}

// ============================================================================
// SQL_ATTR_ROW_OPERATION_PTR (24)
// ============================================================================

TEST_CASE("SQL_ATTR_ROW_OPERATION_PTR default value is null.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_ROW_OPERATION_PTR is queried on a fresh statement
  SQLUSMALLINT* value = reinterpret_cast<SQLUSMALLINT*>(0xdeadbeef);
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_OPERATION_PTR, &value, 0, nullptr);

  // Then it should return SQL_SUCCESS and the value nullptr
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(value == nullptr);
}

TEST_CASE("SQL_ATTR_ROW_OPERATION_PTR can be set and retrieved.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_ROW_OPERATION_PTR is set to a non-null pointer
  SQLUSMALLINT ops[4] = {SQL_ROW_PROCEED, SQL_ROW_IGNORE, SQL_ROW_PROCEED, SQL_ROW_PROCEED};
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_OPERATION_PTR, ops, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then get should return the same pointer
  SQLUSMALLINT* retrieved = nullptr;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_OPERATION_PTR, &retrieved, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(retrieved == ops);
}

// ============================================================================
// SQL_ATTR_ROW_NUMBER — block cursor (SQL_ATTR_ROW_ARRAY_SIZE > 1)
// ============================================================================

TEST_CASE("SQL_ATTR_ROW_NUMBER advances by rowset size with block cursor.") {
  // Given SQL_ATTR_ROW_ARRAY_SIZE is set to 3 and a 6-row result set
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)3, 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(
      stmt.getHandle(),
      sqlchar(
          "SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3 UNION ALL SELECT 4 UNION ALL SELECT 5 UNION ALL SELECT 6"),
      SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // When the first rowset (rows 1-3) is fetched
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);

  // Then SQL_ATTR_ROW_NUMBER should be 3 (last row of the first rowset)
  SQLULEN value = 0;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_NUMBER, &value, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(value == 3);

  // When the second rowset (rows 4-6) is fetched
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);

  // Then SQL_ATTR_ROW_NUMBER should be 6 (last row of the second rowset)
  value = 0;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_NUMBER, &value, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(value == 6);
}

TEST_CASE("SQL_ATTR_ROW_NUMBER resets to 0 after SQLCloseCursor.") {
  // Given a query has been executed and one row fetched
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT 1 UNION ALL SELECT 2"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLULEN value = 0;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_NUMBER, &value, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(value == 1);

  // When the cursor is closed
  ret = SQLCloseCursor(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);

  // Then SQL_ATTR_ROW_NUMBER should be reset to 0
  value = 99;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_NUMBER, &value, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(value == 0);
}
