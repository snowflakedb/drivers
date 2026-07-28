#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>
#include <catch2/generators/catch_generators.hpp>

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
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);
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
  SKIP_OLD_DRIVER("SNOW-3235552",
                  "Old driver does not substitute unsupported concurrency modes; new driver substitutes with "
                  "SQL_CONCUR_READ_ONLY and warns 01S02");
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

TEST_CASE("should accept SQL_ATTR_CURSOR_SCROLLABLE SQL_NONSCROLLABLE directly",
          "[odbc-api][stmt_attr][cursor_scrollable]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_CURSOR_SCROLLABLE is set to SQL_NONSCROLLABLE
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_CURSOR_SCROLLABLE, reinterpret_cast<SQLPOINTER>(SQL_NONSCROLLABLE), 0);

  // Then It should succeed without warnings
  REQUIRE(ret == SQL_SUCCESS);

  SQLULEN scrollable = 99;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_CURSOR_SCROLLABLE, &scrollable, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(scrollable == SQL_NONSCROLLABLE);
}

TEST_CASE("should substitute SQL_ATTR_CURSOR_SCROLLABLE SQL_SCROLLABLE with 01S02",
          "[odbc-api][stmt_attr][cursor_scrollable][warning]") {
  SKIP_OLD_DRIVER(
      "SNOW-3235552",
      "Old driver does not substitute SQL_SCROLLABLE; new driver substitutes with SQL_NONSCROLLABLE and warns 01S02");
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

TEST_CASE("should accept SQL_ATTR_CURSOR_SENSITIVITY SQL_UNSPECIFIED directly",
          "[odbc-api][stmt_attr][cursor_sensitivity]") {
  SKIP_OLD_DRIVER("SNOW-3235552",
                  "Old driver may not support SQL_UNSPECIFIED cursor sensitivity; new driver accepts it directly");
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_CURSOR_SENSITIVITY is set to SQL_UNSPECIFIED
  SQLRETURN ret =
      SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_CURSOR_SENSITIVITY, reinterpret_cast<SQLPOINTER>(SQL_UNSPECIFIED), 0);

  // Then It should succeed without warnings
  REQUIRE(ret == SQL_SUCCESS);

  SQLULEN sensitivity = 99;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_CURSOR_SENSITIVITY, &sensitivity, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(sensitivity == SQL_UNSPECIFIED);
}

TEST_CASE("should substitute SQL_ATTR_CURSOR_SENSITIVITY SQL_INSENSITIVE with 01S02",
          "[odbc-api][stmt_attr][cursor_sensitivity][warning]") {
  SKIP_OLD_DRIVER("SNOW-3235552",
                  "Old driver does not substitute unsupported sensitivity values; new driver substitutes with "
                  "SQL_UNSPECIFIED and warns 01S02");
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
  SKIP_OLD_DRIVER("SNOW-3235552",
                  "Old driver does not substitute unsupported sensitivity values; new driver substitutes with "
                  "SQL_UNSPECIFIED and warns 01S02");
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
  SKIP_OLD_DRIVER(
      "SNOW-3235552",
      "Old driver may return a different default for SQL_ATTR_ENABLE_AUTO_IPD; new driver always returns SQL_FALSE");
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
  SKIP_OLD_DRIVER("SNOW-3235552", "Old driver does not return HYC00 for SQL_TRUE; new driver explicitly rejects it");
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
  SKIP_OLD_DRIVER("SNOW-3235552", "Old driver does not expose SQL_ATTR_KEYSET_SIZE; new driver supports get/set");
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
  SKIP_OLD_DRIVER("SNOW-3235552", "Old driver does not expose SQL_ATTR_KEYSET_SIZE; new driver supports get/set");
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
  SKIP_OLD_DRIVER("SNOW-3235552",
                  "Old driver may not support SQL_ATTR_SIMULATE_CURSOR; new driver defaults to SQL_SC_NON_UNIQUE");
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
  SKIP_OLD_DRIVER("SNOW-3235552",
                  "Old driver does not substitute unsupported simulate cursor values; new driver substitutes with "
                  "SQL_SC_NON_UNIQUE and warns 01S02");
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
  SKIP_OLD_DRIVER("SNOW-3235552",
                  "Old driver does not substitute unsupported simulate cursor values; new driver substitutes with "
                  "SQL_SC_NON_UNIQUE and warns 01S02");
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

TEST_CASE("should return 24000 when setting SQL_ATTR_SIMULATE_CURSOR with open cursor",
          "[odbc-api][stmt_attr][simulate_cursor][error]") {
  // Given A statement with an open cursor
  Connection conn;
  auto stmt = conn.createStatement();
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // When SQL_ATTR_SIMULATE_CURSOR is set while cursor is open
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_SIMULATE_CURSOR, reinterpret_cast<SQLPOINTER>(SQL_SC_NON_UNIQUE), 0);

  // Then It should return SQL_ERROR with SQLSTATE 24000
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(SQL_HANDLE_STMT, stmt.getHandle());
  REQUIRE(!records.empty());
  CHECK(records[0].sqlState == "24000");
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

TEST_CASE("should return HY024 for invalid SQL_ATTR_RETRIEVE_DATA value",
          "[odbc-api][stmt_attr][retrieve_data][error]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_RETRIEVE_DATA is set to an invalid value (not 0 or 1)
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_RETRIEVE_DATA, reinterpret_cast<SQLPOINTER>(99), 0);

  // Then It should return SQL_ERROR with SQLSTATE HY024
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(SQL_HANDLE_STMT, stmt.getHandle());
  REQUIRE(!records.empty());
  CHECK(records[0].sqlState == "HY024");
}

// ============================================================================
// SQL_ATTR_MAX_ROWS — prepared execute and toggle
// ============================================================================

TEST_CASE("should limit rows via SQL_ATTR_MAX_ROWS with SQLPrepare + SQLExecute", "[odbc-api][stmt_attr][max_rows]") {
  // Given A statement prepared with a query returning 3 rows, MAX_ROWS set to 2
  Connection conn;
  auto stmt = conn.createStatement();

  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_MAX_ROWS, reinterpret_cast<SQLPOINTER>(2), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // When The prepared statement is executed
  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then Only 2 rows should be fetchable
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt.getHandle());
  CHECK(ret == SQL_NO_DATA);
}

TEST_CASE("should remove row limit when SQL_ATTR_MAX_ROWS is toggled to 0 between executions",
          "[odbc-api][stmt_attr][max_rows]") {
  // Given A prepared statement with MAX_ROWS=2 that has been executed and closed
  Connection conn;
  auto stmt = conn.createStatement();

  SQLRETURN ret = SQLPrepare(stmt.getHandle(), sqlchar("SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3"), SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_MAX_ROWS, reinterpret_cast<SQLPOINTER>(2), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Drain and close the cursor
  while (SQLFetch(stmt.getHandle()) == SQL_SUCCESS) {
  }
  ret = SQLFreeStmt(stmt.getHandle(), SQL_CLOSE);
  REQUIRE(ret == SQL_SUCCESS);

  // When MAX_ROWS is cleared and the statement is re-executed
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_MAX_ROWS, reinterpret_cast<SQLPOINTER>(0), 0);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt.getHandle());
  REQUIRE_ODBC(ret, stmt);

  // Then All 3 rows should be fetchable
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLFetch(stmt.getHandle());
  CHECK(ret == SQL_NO_DATA);
}

TEST_CASE("should enforce SQL_ATTR_MAX_ROWS across multiple block fetches", "[odbc-api][stmt_attr][max_rows]") {
  // Given A statement with SQL_ATTR_ROW_ARRAY_SIZE=2 and SQL_ATTR_MAX_ROWS=5
  // returning 10 rows; 5 % 2 = 1 so the limit splits the third rowset, which
  // exercises both the in-batch quota clamp and the post-quota SQL_NO_DATA path.
  Connection conn;
  auto stmt = conn.createStatement();

  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_ARRAY_SIZE, reinterpret_cast<SQLPOINTER>(2), 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_MAX_ROWS, reinterpret_cast<SQLPOINTER>(5), 0);
  REQUIRE(ret == SQL_SUCCESS);

  constexpr SQLULEN array_size = 2;
  SQLULEN rows_fetched = 0;
  SQLUSMALLINT row_status[array_size] = {SQL_ROW_NOROW, SQL_ROW_NOROW};
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROWS_FETCHED_PTR, &rows_fetched, 0);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_STATUS_PTR, row_status, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLINTEGER value[array_size] = {0, 0};
  SQLLEN value_ind[array_size] = {0, 0};
  ret = SQLBindCol(stmt.getHandle(), 1, SQL_C_LONG, value, sizeof(SQLINTEGER), value_ind);
  REQUIRE(ret == SQL_SUCCESS);

  // When A query returning 10 rows is executed
  ret = SQLExecDirect(stmt.getHandle(),
                      sqlchar("SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3 UNION ALL SELECT 4 UNION ALL SELECT 5 "
                              "UNION ALL SELECT 6 UNION ALL SELECT 7 UNION ALL SELECT 8 UNION ALL SELECT 9 UNION ALL "
                              "SELECT 10"),
                      SQL_NTS);
  REQUIRE_ODBC(ret, stmt);

  // Then The first two fetches return 2 rows each
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(rows_fetched == 2);
  CHECK(row_status[0] == SQL_ROW_SUCCESS);
  CHECK(row_status[1] == SQL_ROW_SUCCESS);

  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(rows_fetched == 2);
  CHECK(row_status[0] == SQL_ROW_SUCCESS);
  CHECK(row_status[1] == SQL_ROW_SUCCESS);

  // And The third fetch returns 1 row (clamped by the remaining quota).
  // Pre-clear row_status because the old driver does not overwrite slots
  // beyond rows_fetched; the contract is `rows_fetched` rows are valid.
  row_status[0] = SQL_ROW_NOROW;
  row_status[1] = SQL_ROW_NOROW;
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(rows_fetched == 1);
  CHECK(row_status[0] == SQL_ROW_SUCCESS);
  CHECK(row_status[1] == SQL_ROW_NOROW);

  // And A fourth fetch returns SQL_NO_DATA
  ret = SQLFetch(stmt.getHandle());
  CHECK(ret == SQL_NO_DATA);
}

// ============================================================================
// SQL_ATTR_CONCURRENCY — cursor attribute rejected in Done state
// ============================================================================

TEST_CASE("should return 24000 when setting SQL_ATTR_CONCURRENCY after all rows fetched (Done state)",
          "[odbc-api][stmt_attr][concurrency][error]") {
  // Given A statement whose cursor has reached the Done state (all rows fetched)
  Connection conn;
  auto stmt = conn.createStatement();

  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // Fetch the single row to advance into Fetching/Done
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_SUCCESS);

  // Fetch again to exhaust the result set and enter Done state
  ret = SQLFetch(stmt.getHandle());
  REQUIRE(ret == SQL_NO_DATA);

  // When SQL_ATTR_CONCURRENCY is set while cursor is in Done state
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_CONCURRENCY, reinterpret_cast<SQLPOINTER>(SQL_CONCUR_READ_ONLY), 0);

  // Then It should return SQL_ERROR with SQLSTATE 24000
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(SQL_HANDLE_STMT, stmt.getHandle());
  REQUIRE(!records.empty());
  CHECK(records[0].sqlState == "24000");
}

// ============================================================================
// SQL_ATTR_ROW_NUMBER (14) — read-only current row position (SNOW-3235555)
// ============================================================================

TEST_CASE("should return HY092 when setting read-only SQL_ATTR_ROW_NUMBER",
          "[odbc-api][stmt_attr][row_number][error]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When the read-only SQL_ATTR_ROW_NUMBER is set
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_NUMBER, reinterpret_cast<SQLPOINTER>(5), 0);

  // Then It should be rejected with SQLSTATE HY092 (read-only attribute).
  // Both the new and old drivers reject the read-only set identically, so this
  // runs on both (verified against the reference driver).
  REQUIRE(ret == SQL_ERROR);
  auto records = get_diag_rec(SQL_HANDLE_STMT, stmt.getHandle());
  REQUIRE(!records.empty());
  CHECK(records[0].sqlState == "HY092");
}

TEST_CASE("should report 1-based SQL_ATTR_ROW_NUMBER during fetch and 0 when unpositioned",
          "[odbc-api][stmt_attr][row_number]") {
  // Given A connected statement with a 3-row result set
  Connection conn;
  auto stmt = conn.createStatement();
  std::string query = "SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3";
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), sqlchar(query.c_str()), SQL_NTS);
  REQUIRE(SQL_SUCCEEDED(ret));

  // Reads SQL_ATTR_ROW_NUMBER. When the cursor is not positioned on a row
  // (before the first fetch / after end-of-data) some driver managers
  // (e.g. unixODBC) reject the read with SQL_ERROR, whereas the driver
  // itself returns 0. Treat a rejected read as "no current row" so the test
  // is portable across DMs; assert the concrete value only when readable.
  // Runs on both drivers — the old driver supports SQL_ATTR_ROW_NUMBER
  // identically (verified against the reference driver).
  auto read_row_number = [&](SQLULEN& out) -> SQLRETURN {
    out = static_cast<SQLULEN>(-1);
    return SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_NUMBER, &out, 0, nullptr);
  };
  SQLULEN n = 0;

  // Before the first fetch the cursor is not positioned -> 0 (when readable)
  if (SQL_SUCCEEDED(read_row_number(n))) {
    CHECK(n == 0);
  }

  // Each successful fetch advances the 1-based row number
  for (SQLULEN expected = 1; expected <= 3; ++expected) {
    const SQLRETURN fetch_ret = SQLFetch(stmt.getHandle());
    REQUIRE(SQL_SUCCEEDED(fetch_ret));
    if (SQL_SUCCEEDED(read_row_number(n))) {
      CHECK(n == expected);
    }
  }

  // After end-of-data the cursor is no longer positioned -> 0 (when readable)
  REQUIRE(SQLFetch(stmt.getHandle()) == SQL_NO_DATA);
  if (SQL_SUCCEEDED(read_row_number(n))) {
    CHECK(n == 0);
  }
}

// ============================================================================
// SQL_ATTR_ROW_OPERATION_PTR (24) — ARD row-operation array pointer
// ============================================================================

TEST_CASE("should set and get SQL_ATTR_ROW_OPERATION_PTR", "[odbc-api][stmt_attr][row_operation_ptr]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // Default is a null pointer (pre-fill with a non-null sentinel to detect a write).
  // Runs on both drivers — the old driver supports SQL_ATTR_ROW_OPERATION_PTR
  // set/get identically (verified against the reference driver).
  SQLUSMALLINT sentinel = 0;
  SQLUSMALLINT* current = &sentinel;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_OPERATION_PTR, &current, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(current == nullptr);

  // When a row-operation array pointer is set
  SQLUSMALLINT row_ops[3] = {SQL_ROW_PROCEED, SQL_ROW_PROCEED, SQL_ROW_PROCEED};
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_OPERATION_PTR, row_ops, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then Getting it back returns the same pointer
  SQLUSMALLINT* got = nullptr;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_OPERATION_PTR, &got, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(got == row_ops);
}

// ============================================================================
// Null-handle error path (SQL_INVALID_HANDLE)
// ============================================================================

TEST_CASE("SQLSetStmtAttr: SQL_INVALID_HANDLE for null statement handle", "[odbc-api][stmt_attr][error]") {
  // The Driver Manager rejects a null statement handle before dispatch,
  // independent of the attribute; one function-level test suffices.
  const SQLRETURN ret = SQLSetStmtAttr(SQL_NULL_HSTMT, SQL_ATTR_ROW_ARRAY_SIZE, reinterpret_cast<SQLPOINTER>(1), 0);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE("SQLGetStmtAttr: SQL_INVALID_HANDLE for null statement handle", "[odbc-api][stmt_attr][error]") {
  SQLULEN value = 0;
  const SQLRETURN ret = SQLGetStmtAttr(SQL_NULL_HSTMT, SQL_ATTR_ROW_NUMBER, &value, 0, nullptr);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

// ============================================================================
// SQL_ATTR_PARAM_OPERATION_PTR (19) — APD per-set operation array (SNOW-3235553)
// ============================================================================

TEST_CASE("should set and get SQL_ATTR_PARAM_OPERATION_PTR", "[odbc-api][stmt_attr][param_operation_ptr]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // Default is a null pointer (pre-fill with a non-null sentinel to detect a write)
  SQLUSMALLINT sentinel = 0;
  SQLUSMALLINT* current = &sentinel;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_OPERATION_PTR, &current, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(current == nullptr);

  // When a per-set operation array pointer is set
  SQLUSMALLINT param_ops[3] = {SQL_PARAM_PROCEED, SQL_PARAM_IGNORE, SQL_PARAM_PROCEED};
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_OPERATION_PTR, param_ops, 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then Getting it back returns the same pointer
  SQLUSMALLINT* got = nullptr;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_OPERATION_PTR, &got, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(got == param_ops);
}

// SNOW-3235553: SQL_ATTR_PARAM_OPERATION_PTR must be routed to the *effective*
// APD, so an explicit SQL_ATTR_APP_PARAM_DESC is honored (not the implicit APD,
// which the array-binding path never consults when an explicit descriptor is
// assigned). Consistent with SQL_ATTR_PARAM_BIND_TYPE / PARAM_BIND_OFFSET_PTR.
TEST_CASE("should route SQL_ATTR_PARAM_OPERATION_PTR to an explicit APP_PARAM_DESC",
          "[odbc-api][stmt_attr][param_operation_ptr]") {
  // Given A connected statement with an explicit application parameter descriptor
  Connection conn;
  auto stmt = conn.createStatement();

  // RAII: the descriptor is freed on scope exit even if an assertion below
  // throws. Per the ODBC spec, freeing an explicit descriptor reverts the
  // statement to its implicit APD, so no manual reset/free is needed.
  HandleWrapper explicit_apd(conn.handleWrapper().getHandle(), SQL_HANDLE_DESC);

  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_PARAM_DESC, explicit_apd.getHandle(), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Direction 1: set via the statement attribute -> the value must land on the
  // explicit descriptor (read it back independently via SQLGetDescField on the
  // explicit handle's SQL_DESC_ARRAY_STATUS_PTR). Before the fix, the set wrote
  // the now-inactive implicit APD, so the explicit descriptor stayed null.
  SQLUSMALLINT param_ops[3] = {SQL_PARAM_PROCEED, SQL_PARAM_IGNORE, SQL_PARAM_PROCEED};
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_OPERATION_PTR, param_ops, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT* desc_ptr = nullptr;
  ret = SQLGetDescField(explicit_apd.getHandle(), 0, SQL_DESC_ARRAY_STATUS_PTR, &desc_ptr, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(desc_ptr == param_ops);

  // Direction 2: set directly on the explicit descriptor -> the statement
  // attribute get must read it back from the effective (explicit) APD. Before
  // the fix, the get read the inactive implicit APD and returned the wrong ptr.
  SQLUSMALLINT other_ops[2] = {SQL_PARAM_IGNORE, SQL_PARAM_PROCEED};
  ret = SQLSetDescField(explicit_apd.getHandle(), 0, SQL_DESC_ARRAY_STATUS_PTR, other_ops, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLUSMALLINT* got = nullptr;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_OPERATION_PTR, &got, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(got == other_ops);
}

// ============================================================================
// SQL_ATTR_CURSOR_TYPE (6) — Snowflake supports forward-only only (SNOW-3235558)
// ============================================================================

TEST_CASE("should get SQL_ATTR_CURSOR_TYPE default as SQL_CURSOR_FORWARD_ONLY", "[odbc-api][stmt_attr][cursor_type]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_CURSOR_TYPE is queried without being set
  SQLULEN cursor_type = 99;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_CURSOR_TYPE, &cursor_type, 0, nullptr);

  // Then It should default to SQL_CURSOR_FORWARD_ONLY
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(cursor_type == SQL_CURSOR_FORWARD_ONLY);
}

TEST_CASE("should substitute non-forward-only SQL_ATTR_CURSOR_TYPE with 01S02",
          "[odbc-api][stmt_attr][cursor_type][warning]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_CURSOR_TYPE is set to a scrollable type Snowflake does not support
  const SQLULEN requested = GENERATE(SQL_CURSOR_STATIC, SQL_CURSOR_DYNAMIC, SQL_CURSOR_KEYSET_DRIVEN);
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_CURSOR_TYPE, reinterpret_cast<SQLPOINTER>(requested), 0);

  OLD_DRIVER_ONLY("BD#96") {
    // Old driver silently accepts without substitution or warning.
    REQUIRE(SQL_SUCCEEDED(ret));
  }
  NEW_DRIVER_ONLY("BD#96") {
    // New driver substitutes with SQL_CURSOR_FORWARD_ONLY and warns 01S02.
    REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
    auto records = get_diag_rec(SQL_HANDLE_STMT, stmt.getHandle());
    REQUIRE(!records.empty());
    CHECK(records[0].sqlState == "01S02");

    // And the stored value should be substituted to SQL_CURSOR_FORWARD_ONLY
    SQLULEN cursor_type = 99;
    ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_CURSOR_TYPE, &cursor_type, 0, nullptr);
    REQUIRE(ret == SQL_SUCCESS);
    CHECK(cursor_type == SQL_CURSOR_FORWARD_ONLY);
  }
}

// ============================================================================
// SQL_ATTR_ROW_ARRAY_SIZE (27) (SNOW-3235558)
// ============================================================================

TEST_CASE("should set and get SQL_ATTR_ROW_ARRAY_SIZE", "[odbc-api][stmt_attr][row_array_size]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_ROW_ARRAY_SIZE is set
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_ARRAY_SIZE, reinterpret_cast<SQLPOINTER>(10), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then Getting the attribute should return the value that was set
  SQLULEN size = 0;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_ROW_ARRAY_SIZE, &size, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(size == 10);
}

// ============================================================================
// Implicit descriptor handles: SQL_ATTR_APP_ROW_DESC / SQL_ATTR_IMP_ROW_DESC (SNOW-3235558)
// ============================================================================

TEST_CASE("should return automatic descriptor handles for SQL_ATTR_APP_ROW_DESC / SQL_ATTR_IMP_ROW_DESC",
          "[odbc-api][stmt_attr][descriptor_handles]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When the automatically-allocated row descriptor handles are queried
  SQLHDESC ard = nullptr;
  SQLHDESC ird = nullptr;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_APP_ROW_DESC, &ard, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_IMP_ROW_DESC, &ird, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);

  // Then Both handles are non-null and distinct
  CHECK(ard != nullptr);
  CHECK(ird != nullptr);
  CHECK(ard != ird);
}

// ============================================================================
// Parameter array attrs: SQL_ATTR_PARAMSET_SIZE / SQL_ATTR_PARAM_BIND_TYPE (SNOW-3235558)
// ============================================================================

TEST_CASE("should set and get SQL_ATTR_PARAMSET_SIZE and SQL_ATTR_PARAM_BIND_TYPE",
          "[odbc-api][stmt_attr][param_array]") {
  // Given A connected statement handle
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQL_ATTR_PARAMSET_SIZE is set
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, reinterpret_cast<SQLPOINTER>(5), 0);
  REQUIRE(ret == SQL_SUCCESS);
  // And SQL_ATTR_PARAM_BIND_TYPE is set to column-wise binding
  ret = SQLSetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_BIND_TYPE,
                       reinterpret_cast<SQLPOINTER>(SQL_PARAM_BIND_BY_COLUMN), 0);
  REQUIRE(ret == SQL_SUCCESS);

  // Then Both round-trip on GET
  SQLULEN paramset_size = 0;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAMSET_SIZE, &paramset_size, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(paramset_size == 5);

  SQLULEN bind_type = 99;
  ret = SQLGetStmtAttr(stmt.getHandle(), SQL_ATTR_PARAM_BIND_TYPE, &bind_type, 0, nullptr);
  REQUIRE(ret == SQL_SUCCESS);
  CHECK(bind_type == static_cast<SQLULEN>(SQL_PARAM_BIND_BY_COLUMN));
}
