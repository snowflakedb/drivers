#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>
#include <catch2/generators/catch_generators.hpp>

#include "Connection.hpp"
#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
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
  SQLULEN metadata_id = static_cast<SQLULEN>(-1);
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_METADATA_ID, &metadata_id, 0, nullptr);

  // Then It should return SQL_FALSE (0) by default
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(metadata_id == SQL_FALSE);
}

TEST_CASE("should set and get SQL_ATTR_METADATA_ID on statement", "[odbc-api][stmt_attr][metadata_id]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_METADATA_ID is set to each supported value
  const SQLULEN value = GENERATE(SQL_TRUE, SQL_FALSE);
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(value), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then Getting the attribute should return the same value
  SQLULEN metadata_id = static_cast<SQLULEN>(-1);
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_METADATA_ID, &metadata_id, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(metadata_id == value);
}

TEST_CASE("should treat any non-zero value as SQL_TRUE for SQL_ATTR_METADATA_ID on statement",
          "[odbc-api][stmt_attr][metadata_id]") {
  // The old driver enforces strict 0/1 validation; the new driver accepts any non-zero as SQL_TRUE.
  // The ODBC spec defines SQL_ATTR_METADATA_ID as accepting only SQL_TRUE/SQL_FALSE.
  // The Microsoft DM returns HY024 for values outside that set (per SQLSetStmtAttr docs:
  // "The Driver Manager returns this SQLSTATE only for ... attributes that accept a discrete
  // set of values"). unixODBC/iODBC pass the value through to the driver without validation.
#ifdef _WIN32
  SKIP("Windows DM rejects non-standard SQL_ATTR_METADATA_ID values with HY024");
#endif
  NEW_DRIVER_ONLY() {
    // Given A connected statement handle
    Connection conn;
    auto stmt = conn.createStatement();

    // When SQL_ATTR_METADATA_ID is set to a truthy non-1 value
    SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_METADATA_ID, reinterpret_cast<SQLPOINTER>(2), 0);
    REQUIRE(ret == SQL_SUCCESS);

    // Then Getting the attribute should return SQL_TRUE
    SQLULEN metadata_id = static_cast<SQLULEN>(-1);
    ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_METADATA_ID, &metadata_id, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(metadata_id == SQL_TRUE);
  }
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
  SQLULEN metadata_id = static_cast<SQLULEN>(-1);
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

  // Then The already-allocated statement should still have SQL_FALSE.
  // The old driver reads SQL_ATTR_METADATA_ID dynamically from the connection rather than
  // storing an independent copy per statement, so this check is new-driver only.
  NEW_DRIVER_ONLY() {
    SQLULEN metadata_id = static_cast<SQLULEN>(-1);
    ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_METADATA_ID, &metadata_id, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(metadata_id == SQL_FALSE);
  }
}
