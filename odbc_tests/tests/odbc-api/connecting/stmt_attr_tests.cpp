#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "ODBCFixtures.hpp"
#include "get_diag_rec.hpp"
#include "test_macros.hpp"

// ============================================================================
// SQL_ATTR_METADATA_ID (10014) — statement level
// ============================================================================

TEST_CASE("should get SQL_ATTR_METADATA_ID default as SQL_FALSE on statement", "[odbc-api][stmt_attr][metadata_id]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_METADATA_ID is queried without being set
  SQLULEN metadata_id = 99;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_METADATA_ID, &metadata_id, 0, nullptr);

  // Then It should return SQL_FALSE (0) by default
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(metadata_id == SQL_FALSE);
}

TEST_CASE("should set and get SQL_ATTR_METADATA_ID with SQL_TRUE on statement", "[odbc-api][stmt_attr][metadata_id]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_METADATA_ID is set to SQL_TRUE
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then Getting the attribute should return SQL_TRUE
  SQLULEN metadata_id = 99;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_METADATA_ID, &metadata_id, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(metadata_id == SQL_TRUE);
}

TEST_CASE("should set and get SQL_ATTR_METADATA_ID with SQL_FALSE on statement", "[odbc-api][stmt_attr][metadata_id]") {
  // Given A connected statement with METADATA_ID set to SQL_TRUE
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQL_ATTR_METADATA_ID is set back to SQL_FALSE
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(SQL_FALSE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then Getting the attribute should return SQL_FALSE
  SQLULEN metadata_id = 99;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_METADATA_ID, &metadata_id, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(metadata_id == SQL_FALSE);
}

TEST_CASE("should return HY024 for invalid SQL_ATTR_METADATA_ID value on statement",
          "[odbc-api][stmt_attr][metadata_id][error]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_METADATA_ID is set to an invalid value
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(99), 0);

  // Then It should return SQL_ERROR with SQLSTATE HY024
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(SQL_HANDLE_STMT, stmt.getHandle());
  REQUIRE(!records.empty());
  CHECK(records[0].sqlState == "HY024");
}

TEST_CASE("statement should inherit SQL_ATTR_METADATA_ID from connection",
          "[odbc-api][stmt_attr][metadata_id][inheritance]") {
  // Given A connection with SQL_ATTR_METADATA_ID set to SQL_TRUE
  Connection conn;
  SQLRETURN ret = SQLSetConnectAttr(conn.handleWrapper().getHandle(), SQL_ATTR_METADATA_ID,
                                    reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When A new statement is allocated
  auto stmt = conn.createStatement();

  // Then The statement should inherit SQL_TRUE from the connection
  SQLULEN metadata_id = 99;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_METADATA_ID, &metadata_id, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(metadata_id == SQL_TRUE);
}

TEST_CASE("statement SQL_ATTR_METADATA_ID is independent from connection after allocation",
          "[odbc-api][stmt_attr][metadata_id][independence]") {
  // Given A connection with SQL_ATTR_METADATA_ID SQL_FALSE and an allocated statement
  Connection conn;
  auto stmt = conn.createStatement();

  // When The connection attribute is changed to SQL_TRUE after statement allocation
  SQLRETURN ret = SQLSetConnectAttr(conn.handleWrapper().getHandle(), SQL_ATTR_METADATA_ID,
                                    reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then The already-allocated statement should still have SQL_FALSE
  SQLULEN metadata_id = 99;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_METADATA_ID, &metadata_id, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(metadata_id == SQL_FALSE);
}
