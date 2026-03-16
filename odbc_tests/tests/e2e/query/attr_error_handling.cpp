#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <catch2/catch_test_macros.hpp>

#include "Connection.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "sf_odbc.h"

TEST_CASE("SQLGetStmtAttr with negative buffer length returns HY090.") {
  SKIP_OLD_DRIVER("SNOW-3235549", "Negative buffer length validation is new driver only");

  // Given Snowflake client is logged in
  Connection conn;
  auto stmt = conn.createStatement();

  // When SQLGetStmtAttr is called with buffer_length = -1 for a string attribute
  char buf[256] = {};
  SQLINTEGER len = 0;
  SQLRETURN ret = SQLGetStmtAttr(stmt.getHandle(), SQL_SF_STMT_ATTR_LAST_QUERY_ID, buf, -1, &len);

  // Then it should return SQL_ERROR with SQLSTATE HY090
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(stmt) == "HY090");
}

TEST_CASE("SQLGetStmtAttr string attribute truncation returns SQL_SUCCESS_WITH_INFO.") {
  SKIP_OLD_DRIVER("SNOW-3235549", "String truncation warning is new driver only");

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
  SKIP_OLD_DRIVER("SNOW-3235549", "HY092 for unknown attributes is new driver only");

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
  SKIP_OLD_DRIVER("SNOW-3235549", "Negative buffer length validation is new driver only");

  // Given Snowflake client is logged in
  Connection conn;

  // When SQLGetConnectAttr is called with buffer_length = -1 for a string attribute
  char buf[256] = {};
  SQLINTEGER len = 0;
  SQLRETURN ret = SQLGetConnectAttr(conn.handleWrapper().getHandle(), SQL_ATTR_CURRENT_CATALOG, buf, -1, &len);

  // Then it should return SQL_ERROR with SQLSTATE HY090
  REQUIRE(ret == SQL_ERROR);
  CHECK(get_sqlstate(conn.handleWrapper()) == "HY090");
}
