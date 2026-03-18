#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
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

  // Then The already-allocated statement should still have SQL_FALSE.
  // The old driver reads SQL_ATTR_METADATA_ID dynamically from the connection rather than
  // storing an independent copy per statement, so this check is new-driver only.
  NEW_DRIVER_ONLY() {
    SQLULEN metadata_id = 99;
    ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_METADATA_ID, &metadata_id, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(metadata_id == SQL_FALSE);
  }
}

// ============================================================================
// SQL_ATTR_QUERY_TIMEOUT (0) — query timeout in seconds
// ============================================================================

TEST_CASE("should get SQL_ATTR_QUERY_TIMEOUT default as 0", "[odbc-api][stmt_attr][query_timeout]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_QUERY_TIMEOUT is queried without being set
  SQLULEN timeout = 99;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_QUERY_TIMEOUT, &timeout, 0, nullptr);

  // Then It should return 0 (no timeout) by default
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(timeout == 0);
}

TEST_CASE("should set and get SQL_ATTR_QUERY_TIMEOUT", "[odbc-api][stmt_attr][query_timeout]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_QUERY_TIMEOUT is set to 30 seconds
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_QUERY_TIMEOUT, reinterpret_cast<SQLPOINTER>(30), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then Getting the attribute should return 30
  SQLULEN timeout = 0;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_QUERY_TIMEOUT, &timeout, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(timeout == 30);
}

// ============================================================================
// SQL_ATTR_MAX_ROWS (1) — maximum rows returned
// ============================================================================

TEST_CASE("should get SQL_ATTR_MAX_ROWS default as 0", "[odbc-api][stmt_attr][max_rows]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_MAX_ROWS is queried without being set
  SQLULEN max_rows = 99;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_MAX_ROWS, &max_rows, 0, nullptr);

  // Then It should return 0 (no limit) by default
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(max_rows == 0);
}

TEST_CASE("should set and get SQL_ATTR_MAX_ROWS", "[odbc-api][stmt_attr][max_rows]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_MAX_ROWS is set to 100
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_MAX_ROWS, reinterpret_cast<SQLPOINTER>(100), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then Getting the attribute should return 100
  SQLULEN max_rows = 0;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_MAX_ROWS, &max_rows, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(max_rows == 100);
}

TEST_CASE("should limit fetched rows to SQL_ATTR_MAX_ROWS", "[odbc-api][stmt_attr][max_rows]") {
  // Given A statement with SQL_ATTR_MAX_ROWS set to 2
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_MAX_ROWS, reinterpret_cast<SQLPOINTER>(2), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When A query returning 3 rows is executed
  std::string query = "SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3";
  ret = SQLExecDirect(stmt.getHandle(), sqlchar(query.c_str()), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // Then Only 2 rows should be fetchable
  ret = SQLFetch(stmt.getHandle());
  CHECK(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt.getHandle());
  CHECK(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt.getHandle());
  CHECK(ret == SQL_NO_DATA);
}

// ============================================================================
// SQL_ATTR_NOSCAN (2) — whether to scan for ODBC escape sequences
// ============================================================================

TEST_CASE("should get SQL_ATTR_NOSCAN default as SQL_NOSCAN_OFF", "[odbc-api][stmt_attr][noscan]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_NOSCAN is queried without being set
  SQLULEN noscan = 99;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_NOSCAN, &noscan, 0, nullptr);

  // Then It should return SQL_NOSCAN_OFF (0) by default
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(noscan == SQL_NOSCAN_OFF);
}

TEST_CASE("should set and get SQL_ATTR_NOSCAN with SQL_NOSCAN_ON", "[odbc-api][stmt_attr][noscan]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_NOSCAN is set to SQL_NOSCAN_ON
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_NOSCAN, reinterpret_cast<SQLPOINTER>(SQL_NOSCAN_ON), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then Getting the attribute should return SQL_NOSCAN_ON
  SQLULEN noscan = 0;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_NOSCAN, &noscan, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(noscan == SQL_NOSCAN_ON);
}

TEST_CASE("should return HY024 for invalid SQL_ATTR_NOSCAN value", "[odbc-api][stmt_attr][noscan][error]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_NOSCAN is set to an invalid value
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_NOSCAN, reinterpret_cast<SQLPOINTER>(99), 0);

  // Then It should return SQL_ERROR with SQLSTATE HY024
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(SQL_HANDLE_STMT, stmt.getHandle());
  REQUIRE(!records.empty());
  CHECK(records[0].sqlState == "HY024");
}

// ============================================================================
// SQL_ATTR_CONCURRENCY (7) — cursor concurrency
// ============================================================================

TEST_CASE("should get SQL_ATTR_CONCURRENCY default as SQL_CONCUR_READ_ONLY", "[odbc-api][stmt_attr][concurrency]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_CONCURRENCY is queried without being set
  SQLULEN concurrency = 99;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_CONCURRENCY, &concurrency, 0, nullptr);

  // Then It should return SQL_CONCUR_READ_ONLY (1) by default
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(concurrency == SQL_CONCUR_READ_ONLY);
}

TEST_CASE("should accept SQL_ATTR_CONCURRENCY SQL_CONCUR_READ_ONLY directly", "[odbc-api][stmt_attr][concurrency]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_CONCURRENCY is set to SQL_CONCUR_READ_ONLY
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_CONCURRENCY, reinterpret_cast<SQLPOINTER>(SQL_CONCUR_READ_ONLY), 0);

  // Then It should succeed without warnings
  REQUIRE(ret == SQL_SUCCESS);

  SQLULEN concurrency = 0;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_CONCURRENCY, &concurrency, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(concurrency == SQL_CONCUR_READ_ONLY);
}

TEST_CASE("should substitute SQL_ATTR_CONCURRENCY non-read-only values with 01S02",
          "[odbc-api][stmt_attr][concurrency][warning]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_CONCURRENCY is set to SQL_CONCUR_LOCK (Snowflake does not support locking)
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_CONCURRENCY, reinterpret_cast<SQLPOINTER>(SQL_CONCUR_LOCK), 0);

  // Then It should return SQL_SUCCESS_WITH_INFO with 01S02
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  auto records = get_diag_rec(SQL_HANDLE_STMT, stmt.getHandle());
  REQUIRE(!records.empty());
  CHECK(records[0].sqlState == "01S02");

  // And the stored value should be SQL_CONCUR_READ_ONLY
  SQLULEN concurrency = 0;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_CONCURRENCY, &concurrency, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(concurrency == SQL_CONCUR_READ_ONLY);
}

TEST_CASE("should return 24000 when setting SQL_ATTR_CONCURRENCY with open cursor",
          "[odbc-api][stmt_attr][concurrency][error]") {
  // Given A statement with an open cursor
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQL_ATTR_CONCURRENCY is set while cursor is open
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_CONCURRENCY, reinterpret_cast<SQLPOINTER>(SQL_CONCUR_READ_ONLY), 0);

  // Then It should return SQL_ERROR with SQLSTATE 24000
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(SQL_HANDLE_STMT, stmt.getHandle());
  REQUIRE(!records.empty());
  CHECK(records[0].sqlState == "24000");
}

TEST_CASE("should return HY024 for invalid SQL_ATTR_CONCURRENCY value", "[odbc-api][stmt_attr][concurrency][error]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_CONCURRENCY is set to an invalid value
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_CONCURRENCY, reinterpret_cast<SQLPOINTER>(99), 0);

  // Then It should return SQL_ERROR with SQLSTATE HY024
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(SQL_HANDLE_STMT, stmt.getHandle());
  REQUIRE(!records.empty());
  CHECK(records[0].sqlState == "HY024");
}

// ============================================================================
// SQL_ATTR_CURSOR_SCROLLABLE (-1) — cursor scrollability
// ============================================================================

TEST_CASE("should get SQL_ATTR_CURSOR_SCROLLABLE default as SQL_NONSCROLLABLE",
          "[odbc-api][stmt_attr][cursor_scrollable]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_CURSOR_SCROLLABLE is queried without being set
  SQLULEN scrollable = 99;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_CURSOR_SCROLLABLE, &scrollable, 0, nullptr);

  // Then It should return SQL_NONSCROLLABLE (0) by default
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(scrollable == SQL_NONSCROLLABLE);
}

TEST_CASE("should substitute SQL_ATTR_CURSOR_SCROLLABLE SQL_SCROLLABLE with 01S02",
          "[odbc-api][stmt_attr][cursor_scrollable][warning]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_CURSOR_SCROLLABLE is set to SQL_SCROLLABLE
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_CURSOR_SCROLLABLE, reinterpret_cast<SQLPOINTER>(SQL_SCROLLABLE), 0);

  // Then It should return SQL_SUCCESS_WITH_INFO with 01S02
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  auto records = get_diag_rec(SQL_HANDLE_STMT, stmt.getHandle());
  REQUIRE(!records.empty());
  CHECK(records[0].sqlState == "01S02");

  // And the stored value should be SQL_NONSCROLLABLE
  SQLULEN scrollable = 99;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_CURSOR_SCROLLABLE, &scrollable, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(scrollable == SQL_NONSCROLLABLE);
}

TEST_CASE("should return HY024 for invalid SQL_ATTR_CURSOR_SCROLLABLE value",
          "[odbc-api][stmt_attr][cursor_scrollable][error]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_CURSOR_SCROLLABLE is set to an invalid value
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_CURSOR_SCROLLABLE, reinterpret_cast<SQLPOINTER>(99), 0);

  // Then It should return SQL_ERROR with SQLSTATE HY024
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(SQL_HANDLE_STMT, stmt.getHandle());
  REQUIRE(!records.empty());
  CHECK(records[0].sqlState == "HY024");
}

TEST_CASE("should return 24000 when setting SQL_ATTR_CURSOR_SCROLLABLE with open cursor",
          "[odbc-api][stmt_attr][cursor_scrollable][error]") {
  // Given A statement with an open cursor
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQL_ATTR_CURSOR_SCROLLABLE is set while cursor is open
  ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_CURSOR_SCROLLABLE, reinterpret_cast<SQLPOINTER>(SQL_NONSCROLLABLE), 0);

  // Then It should return SQL_ERROR with SQLSTATE 24000
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(SQL_HANDLE_STMT, stmt.getHandle());
  REQUIRE(!records.empty());
  CHECK(records[0].sqlState == "24000");
}

// ============================================================================
// SQL_ATTR_CURSOR_SENSITIVITY (-2) — cursor sensitivity
// ============================================================================

TEST_CASE("should get SQL_ATTR_CURSOR_SENSITIVITY default as SQL_UNSPECIFIED",
          "[odbc-api][stmt_attr][cursor_sensitivity]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_CURSOR_SENSITIVITY is queried without being set
  SQLULEN sensitivity = 99;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_CURSOR_SENSITIVITY, &sensitivity, 0, nullptr);

  // Then It should return SQL_UNSPECIFIED (0) by default
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(sensitivity == SQL_UNSPECIFIED);
}

TEST_CASE("should substitute SQL_ATTR_CURSOR_SENSITIVITY SQL_INSENSITIVE with 01S02",
          "[odbc-api][stmt_attr][cursor_sensitivity][warning]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_CURSOR_SENSITIVITY is set to SQL_INSENSITIVE
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_CURSOR_SENSITIVITY, reinterpret_cast<SQLPOINTER>(SQL_INSENSITIVE), 0);

  // Then It should return SQL_SUCCESS_WITH_INFO with 01S02
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  auto records = get_diag_rec(SQL_HANDLE_STMT, stmt.getHandle());
  REQUIRE(!records.empty());
  CHECK(records[0].sqlState == "01S02");

  // And the stored value should be SQL_UNSPECIFIED
  SQLULEN sensitivity = 99;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_CURSOR_SENSITIVITY, &sensitivity, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(sensitivity == SQL_UNSPECIFIED);
}

TEST_CASE("should substitute SQL_ATTR_CURSOR_SENSITIVITY SQL_SENSITIVE with 01S02",
          "[odbc-api][stmt_attr][cursor_sensitivity][warning]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_CURSOR_SENSITIVITY is set to SQL_SENSITIVE
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_CURSOR_SENSITIVITY, reinterpret_cast<SQLPOINTER>(SQL_SENSITIVE), 0);

  // Then It should return SQL_SUCCESS_WITH_INFO with 01S02
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  auto records = get_diag_rec(SQL_HANDLE_STMT, stmt.getHandle());
  REQUIRE(!records.empty());
  CHECK(records[0].sqlState == "01S02");

  // And the stored value should be SQL_UNSPECIFIED
  SQLULEN sensitivity = 99;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_CURSOR_SENSITIVITY, &sensitivity, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(sensitivity == SQL_UNSPECIFIED);
}

TEST_CASE("should return HY024 for invalid SQL_ATTR_CURSOR_SENSITIVITY value",
          "[odbc-api][stmt_attr][cursor_sensitivity][error]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_CURSOR_SENSITIVITY is set to an invalid value
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_CURSOR_SENSITIVITY, reinterpret_cast<SQLPOINTER>(99), 0);

  // Then It should return SQL_ERROR with SQLSTATE HY024
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(SQL_HANDLE_STMT, stmt.getHandle());
  REQUIRE(!records.empty());
  CHECK(records[0].sqlState == "HY024");
}

TEST_CASE("should return 24000 when setting SQL_ATTR_CURSOR_SENSITIVITY with open cursor",
          "[odbc-api][stmt_attr][cursor_sensitivity][error]") {
  // Given A statement with an open cursor
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQL_ATTR_CURSOR_SENSITIVITY is set while cursor is open
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_CURSOR_SENSITIVITY, reinterpret_cast<SQLPOINTER>(SQL_UNSPECIFIED), 0);

  // Then It should return SQL_ERROR with SQLSTATE 24000
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(SQL_HANDLE_STMT, stmt.getHandle());
  REQUIRE(!records.empty());
  CHECK(records[0].sqlState == "24000");
}

// ============================================================================
// SQL_ATTR_ENABLE_AUTO_IPD (15) — automatic population of IPD
// ============================================================================

TEST_CASE("should get SQL_ATTR_ENABLE_AUTO_IPD as SQL_FALSE", "[odbc-api][stmt_attr][enable_auto_ipd]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_ENABLE_AUTO_IPD is queried
  SQLULEN auto_ipd = 99;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_ENABLE_AUTO_IPD, &auto_ipd, 0, nullptr);

  // Then It should always return SQL_FALSE (0)
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(auto_ipd == SQL_FALSE);
}

TEST_CASE("should accept SQL_ATTR_ENABLE_AUTO_IPD SQL_FALSE", "[odbc-api][stmt_attr][enable_auto_ipd]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_ENABLE_AUTO_IPD is set to SQL_FALSE
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ENABLE_AUTO_IPD, reinterpret_cast<SQLPOINTER>(SQL_FALSE), 0);

  // Then It should succeed
  REQUIRE(ret == SQL_SUCCESS);
}

TEST_CASE("should return HYC00 for SQL_ATTR_ENABLE_AUTO_IPD SQL_TRUE",
          "[odbc-api][stmt_attr][enable_auto_ipd][error]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_ENABLE_AUTO_IPD is set to SQL_TRUE (not supported)
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ENABLE_AUTO_IPD, reinterpret_cast<SQLPOINTER>(SQL_TRUE), 0);

  // Then It should return SQL_ERROR with SQLSTATE HYC00
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(SQL_HANDLE_STMT, stmt.getHandle());
  REQUIRE(!records.empty());
  CHECK(records[0].sqlState == "HYC00");
}

// ============================================================================
// SQL_ATTR_KEYSET_SIZE (8) — keyset size
// ============================================================================

TEST_CASE("should get SQL_ATTR_KEYSET_SIZE default as 0", "[odbc-api][stmt_attr][keyset_size]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_KEYSET_SIZE is queried without being set
  SQLULEN keyset_size = 99;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_KEYSET_SIZE, &keyset_size, 0, nullptr);

  // Then It should return 0 by default
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(keyset_size == 0);
}

TEST_CASE("should set and get SQL_ATTR_KEYSET_SIZE", "[odbc-api][stmt_attr][keyset_size]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_KEYSET_SIZE is set to 10
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_KEYSET_SIZE, reinterpret_cast<SQLPOINTER>(10), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then Getting the attribute should return 10
  SQLULEN keyset_size = 0;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_KEYSET_SIZE, &keyset_size, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(keyset_size == 10);
}

// ============================================================================
// SQL_ATTR_SIMULATE_CURSOR (10) — simulate positioned update/delete
// ============================================================================

TEST_CASE("should get SQL_ATTR_SIMULATE_CURSOR default as SQL_SC_NON_UNIQUE",
          "[odbc-api][stmt_attr][simulate_cursor]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_SIMULATE_CURSOR is queried without being set
  SQLULEN simulate = 99;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_SIMULATE_CURSOR, &simulate, 0, nullptr);

  // Then It should return SQL_SC_NON_UNIQUE (0) by default
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(simulate == SQL_SC_NON_UNIQUE);
}

TEST_CASE("should substitute SQL_ATTR_SIMULATE_CURSOR non-unique values with 01S02",
          "[odbc-api][stmt_attr][simulate_cursor][warning]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_SIMULATE_CURSOR is set to SQL_SC_UNIQUE
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_SIMULATE_CURSOR, reinterpret_cast<SQLPOINTER>(SQL_SC_UNIQUE), 0);

  // Then It should return SQL_SUCCESS_WITH_INFO with 01S02
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  auto records = get_diag_rec(SQL_HANDLE_STMT, stmt.getHandle());
  REQUIRE(!records.empty());
  CHECK(records[0].sqlState == "01S02");

  // And the stored value should be SQL_SC_NON_UNIQUE
  SQLULEN simulate = 99;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_SIMULATE_CURSOR, &simulate, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(simulate == SQL_SC_NON_UNIQUE);
}

TEST_CASE("should substitute SQL_ATTR_SIMULATE_CURSOR SQL_SC_TRY_UNIQUE with 01S02",
          "[odbc-api][stmt_attr][simulate_cursor][warning]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_SIMULATE_CURSOR is set to SQL_SC_TRY_UNIQUE
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_SIMULATE_CURSOR, reinterpret_cast<SQLPOINTER>(SQL_SC_TRY_UNIQUE), 0);

  // Then It should return SQL_SUCCESS_WITH_INFO with 01S02
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  auto records = get_diag_rec(SQL_HANDLE_STMT, stmt.getHandle());
  REQUIRE(!records.empty());
  CHECK(records[0].sqlState == "01S02");

  // And the stored value should be SQL_SC_NON_UNIQUE
  SQLULEN simulate = 99;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_SIMULATE_CURSOR, &simulate, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(simulate == SQL_SC_NON_UNIQUE);
}

// ============================================================================
// SQL_ATTR_RETRIEVE_DATA (11) — whether to retrieve data after positioned update
// ============================================================================

TEST_CASE("should get SQL_ATTR_RETRIEVE_DATA default as SQL_RD_ON", "[odbc-api][stmt_attr][retrieve_data]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_RETRIEVE_DATA is queried without being set
  SQLULEN retrieve = 0;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_RETRIEVE_DATA, &retrieve, 0, nullptr);

  // Then It should return SQL_RD_ON (1) by default
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(retrieve == SQL_RD_ON);
}

TEST_CASE("should set and get SQL_ATTR_RETRIEVE_DATA SQL_RD_OFF", "[odbc-api][stmt_attr][retrieve_data]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_RETRIEVE_DATA is set to SQL_RD_OFF
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_RETRIEVE_DATA, reinterpret_cast<SQLPOINTER>(SQL_RD_OFF), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then Getting the attribute should return SQL_RD_OFF
  SQLULEN retrieve = 99;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_RETRIEVE_DATA, &retrieve, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(retrieve == SQL_RD_OFF);
}
