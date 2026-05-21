#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "sf_odbc.h"

TEST_CASE("SQLGetStmtAttr with negative buffer length returns HY090.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLGetStmtAttr is called with buffer_length = -1 for a string attribute
  char buf[256] = {};
  SQLINTEGER len = 0;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_LAST_QUERY_ID, buf, -1, &len);

  // Then it should return SQL_ERROR; new driver returns HY090, old driver may return HY000 (BD#53)
  REQUIRE(ret == SQL_ERROR);
  NEW_DRIVER_ONLY("BD#53") { CHECK(get_sqlstate(stmt) == "HY090"); }
}

TEST_CASE("SQLGetStmtAttr string attribute truncation returns SQL_SUCCESS_WITH_INFO.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLExecDirect is called and SQLGetStmtAttr is called with an insufficient buffer
  SQLRETURN ret = SQLExecDirect(stmt.getHandle(), (SQLCHAR*)"SELECT 1", SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // Use SQLGetStmtAttrW directly with a small wide-char buffer.
  // On Windows only the W variant is exported; calling the ANSI SQLGetStmtAttr
  // routes through the Driver Manager which may swallow the truncation warning
  // for vendor-specific attributes it does not recognise. Calling W directly
  // exercises the same driver code path on every platform without DM interference.
  SQLWCHAR wbuf[4] = {};
  SQLINTEGER len = 0;
  ret = SQLGetStmtAttrW(stmt.getHandle(), SQL_SF_STMT_ATTR_LAST_QUERY_ID, wbuf, sizeof(wbuf), &len);

  // Then it should return SQL_SUCCESS_WITH_INFO with SQLSTATE 01004
  REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  CHECK(get_sqlstate(stmt) == "01004");
  CHECK(len > static_cast<SQLINTEGER>(sizeof(wbuf)));
}

TEST_CASE("SQLGetStmtAttr with invalid attribute identifier returns HY092.") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLGetStmtAttr is called with an invalid attribute identifier
  char buf[256] = {};
  SQLINTEGER len = 0;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), 99999, buf, sizeof(buf), &len);

  // Then it should return SQL_ERROR with SQLSTATE HY092
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "HY092");
}

TEST_CASE("SQLGetConnectAttr with negative buffer length returns HY090.") {
  // Given Snowflake client is logged in
  Connection conn;

  // When SQLGetConnectAttr is called with buffer_length = -1 for a string attribute
  char buf[256] = {};
  SQLINTEGER len = 0;
  SQLRETURN ret = SQLGetConnectAttr(conn.handleWrapper().getHandle(), SQL_ATTR_CURRENT_CATALOG, buf, -1, &len);

  // Then it should return SQL_ERROR; new driver returns HY090, old driver may return HY000 (BD#53)
  REQUIRE(ret == SQL_ERROR);
  NEW_DRIVER_ONLY("BD#53") { CHECK(get_sqlstate(conn.handleWrapper()) == "HY090"); }
}

// ============================================================================
// Unknown attribute handling (BD#61, BD#62)
// ============================================================================

TEST_CASE("SQLSetConnectAttr with unknown ODBC-range attribute returns error (BD#61).") {
  // Given Snowflake client is logged in
  Connection conn;

  // When SQLSetConnectAttr is called with SQL_ATTR_TRACE (114) — a valid ODBC attr not supported by the driver
  SQLRETURN ret = SQLSetConnectAttr(conn.handleWrapper().getHandle(), 114 /* SQL_ATTR_TRACE */, nullptr, 0);

  // Then new driver returns SQL_ERROR with HYC00; old driver silently ignores it (BD#61)
  NEW_DRIVER_ONLY("BD#61") {
    REQUIRE(ret == SQL_ERROR);
    CHECK(get_sqlstate(conn.handleWrapper()) == "HYC00");
  }
  OLD_DRIVER_ONLY("BD#61") { REQUIRE(ret == SQL_SUCCESS); }
}

TEST_CASE("SQLGetConnectAttr with unknown ODBC-range attribute returns error (BD#61).") {
  // Given Snowflake client is logged in
  Connection conn;

  // When SQLGetConnectAttr is called with SQL_ATTR_TRACE (114)
  SQLINTEGER value = 0;
  SQLRETURN ret = SQLGetConnectAttr(conn.handleWrapper().getHandle(), 114 /* SQL_ATTR_TRACE */, &value, 0, nullptr);

  // Then both drivers return SQL_ERROR; new driver returns HYC00, old driver returns HY092 (BD#61)
  REQUIRE(ret == SQL_ERROR);
  NEW_DRIVER_ONLY("BD#61") { CHECK(get_sqlstate(conn.handleWrapper()) == "HYC00"); }
  OLD_DRIVER_ONLY("BD#61") { CHECK(get_sqlstate(conn.handleWrapper()) == "HY092"); }
}

TEST_CASE("SQLSetStmtAttr with unimplemented ODBC-range attribute returns HYC00 (BD#62).") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLSetStmtAttr is called with SQL_ATTR_FETCH_BOOKMARK_PTR (16) — valid ODBC attr not implemented
  SQLRETURN ret = SQLSetStmtAttr(stmt.getHandle(), 16 /* SQL_ATTR_FETCH_BOOKMARK_PTR */, nullptr, 0);

  // Then new driver returns SQL_ERROR with HYC00; old driver returns HY092 (BD#62)
  REQUIRE(ret == SQL_ERROR);
  NEW_DRIVER_ONLY("BD#62") { CHECK(get_sqlstate(stmt) == "HYC00"); }
  OLD_DRIVER_ONLY("BD#62") { CHECK(get_sqlstate(stmt) == "HY092"); }
}

TEST_CASE("SQLGetStmtAttr with unimplemented ODBC-range attribute returns HYC00 (BD#62).") {
  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLGetStmtAttr is called with SQL_ATTR_FETCH_BOOKMARK_PTR (16)
  SQLPOINTER value = nullptr;
  SQLINTEGER len = 0;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), 16 /* SQL_ATTR_FETCH_BOOKMARK_PTR */, &value, 0, &len);

  // Then new driver returns SQL_ERROR with HYC00; old driver returns HY092 (BD#62)
  REQUIRE(ret == SQL_ERROR);
  NEW_DRIVER_ONLY("BD#62") { CHECK(get_sqlstate(stmt) == "HYC00"); }
  OLD_DRIVER_ONLY("BD#62") { CHECK(get_sqlstate(stmt) == "HY092"); }
}

// ============================================================================
// Read-only connection attributes on set
// ============================================================================

TEST_CASE("SQLSetConnectAttr on read-only SQL_ATTR_CONNECTION_DEAD returns HY092.") {
  // Given Snowflake client is logged in
  Connection conn;

  // When SQLSetConnectAttr is called for SQL_ATTR_CONNECTION_DEAD (1209), which is read-only
  SQLRETURN ret = SQLSetConnectAttr(conn.handleWrapper().getHandle(), SQL_ATTR_CONNECTION_DEAD,
                                    reinterpret_cast<SQLPOINTER>(SQL_CD_FALSE), 0);

  // Then it should return SQL_ERROR with HY092
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(conn.handleWrapper()) == "HY092");
}

TEST_CASE("SQLSetConnectAttr on read-only SQL_ATTR_AUTO_IPD returns HY092.") {
  // Given Snowflake client is logged in
  Connection conn;

  // When SQLSetConnectAttr is called for SQL_ATTR_AUTO_IPD (10001), which is read-only
  SQLRETURN ret = SQLSetConnectAttr(conn.handleWrapper().getHandle(), SQL_ATTR_AUTO_IPD,
                                    reinterpret_cast<SQLPOINTER>(SQL_FALSE), 0);

  // Then it should return SQL_ERROR with HY092
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(conn.handleWrapper()) == "HY092");
}

// ============================================================================
// Connection-state-dependent attributes (HY011)
// ============================================================================

TEST_CASE("SQLSetConnectAttr for SQL_ATTR_LOGIN_TIMEOUT after connect returns HY011.") {
  // Given Snowflake client is logged in (connected state)
  Connection conn;

  // When SQL_ATTR_LOGIN_TIMEOUT is set while already connected
  SQLRETURN ret =
      SQLSetConnectAttr(conn.handleWrapper().getHandle(), SQL_ATTR_LOGIN_TIMEOUT, reinterpret_cast<SQLPOINTER>(30), 0);

  // Then it should return SQL_ERROR with HY011 (attribute cannot be set now)
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(conn.handleWrapper()) == "HY011");
}

TEST_CASE("SQLSetConnectAttr for SQL_ATTR_PACKET_SIZE after connect returns HY011.") {
  // Given Snowflake client is logged in (connected state)
  Connection conn;

  // When SQL_ATTR_PACKET_SIZE is set while already connected
  SQLRETURN ret =
      SQLSetConnectAttr(conn.handleWrapper().getHandle(), SQL_ATTR_PACKET_SIZE, reinterpret_cast<SQLPOINTER>(4096), 0);

  // Then it should return SQL_ERROR with HY011 (attribute cannot be set now)
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(conn.handleWrapper()) == "HY011");
}

// ============================================================================
// Negative buffer length on non-catalog string connection attributes
// ============================================================================

TEST_CASE("SQLGetConnectAttr for SF private key attribute with negative buffer returns HY090.") {
  // Given Snowflake client is logged in
  Connection conn;

  // When SQLGetConnectAttr is called with buffer_length = -1 for a Snowflake string attribute
  char buf[256] = {};
  SQLINTEGER len = 0;
  SQLRETURN ret = SQLGetConnectAttr(conn.handleWrapper().getHandle(), SQL_SF_CONN_ATTR_PRIV_KEY_CONTENT, buf, -1, &len);

  // Then new driver returns SQL_ERROR with HY090; old driver may return HY000 (BD#53)
  REQUIRE(ret == SQL_ERROR);
  NEW_DRIVER_ONLY("BD#53") { CHECK(get_sqlstate(conn.handleWrapper()) == "HY090"); }
}
