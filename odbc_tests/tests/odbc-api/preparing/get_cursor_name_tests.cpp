#include <sql.h>
#include <sqltypes.h>

#include <cstring>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// ============================================================================
// SQLGetCursorName
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetCursorName: Auto-generated cursor name starts with SQL_CUR",
                 "[odbc-api][cursorname][preparing]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLCHAR cursor_name[128] = {};
  SQLSMALLINT name_len = 0;
  SQLRETURN ret = SQLGetCursorName(stmt_handle(), cursor_name, sizeof(cursor_name), &name_len);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(name_len > 0);

  std::string name(reinterpret_cast<char*>(cursor_name));
  REQUIRE(name.length() == static_cast<size_t>(name_len));
  REQUIRE(name.substr(0, 7) == "SQL_CUR");
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetCursorName: Different statements have different auto-generated names",
                 "[odbc-api][cursorname][preparing]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(dsn_name().c_str())), SQL_NTS,
                             nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHSTMT stmt1 = SQL_NULL_HSTMT, stmt2 = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt1);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt2);
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR name1[128] = {}, name2[128] = {};
  SQLSMALLINT len1 = 0, len2 = 0;

  ret = SQLGetCursorName(stmt1, name1, sizeof(name1), &len1);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLGetCursorName(stmt2, name2, sizeof(name2), &len2);
  REQUIRE(ret == SQL_SUCCESS);

  std::string sname1(reinterpret_cast<char*>(name1));
  std::string sname2(reinterpret_cast<char*>(name2));
  REQUIRE(sname1 != sname2);
  REQUIRE(sname1.length() == static_cast<size_t>(len1));
  REQUIRE(sname2.length() == static_cast<size_t>(len2));
  REQUIRE(sname1.substr(0, 7) == "SQL_CUR");
  REQUIRE(sname2.substr(0, 7) == "SQL_CUR");

  SQLFreeHandle(SQL_HANDLE_STMT, stmt1);
  SQLFreeHandle(SQL_HANDLE_STMT, stmt2);
  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetCursorName: Returns exact name set by SQLSetCursorName",
                 "[odbc-api][cursorname][preparing]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLSetCursorName(stmt_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("MyCursor")), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR cursor_name[128] = {};
  SQLSMALLINT name_len = 0;
  ret = SQLGetCursorName(stmt_handle(), cursor_name, sizeof(cursor_name), &name_len);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(name_len == 8);
  REQUIRE(std::string(reinterpret_cast<char*>(cursor_name)) == "MyCursor");
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetCursorName: Cursor name persists after SQLPrepare",
                 "[odbc-api][cursorname][preparing]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLSetCursorName(stmt_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("PrepCursor")), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLPrepare(stmt_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 1")), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR cursor_name[128] = {};
  SQLSMALLINT name_len = 0;
  ret = SQLGetCursorName(stmt_handle(), cursor_name, sizeof(cursor_name), &name_len);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(name_len == 10);
  REQUIRE(std::string(reinterpret_cast<char*>(cursor_name)) == "PrepCursor");
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetCursorName: Cursor name persists after SQLCloseCursor",
                 "[odbc-api][cursorname][preparing]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret =
      SQLSetCursorName(stmt_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("CloseCursor")), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("SELECT 1")), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR cursor_name[128] = {};
  SQLSMALLINT name_len = 0;
  ret = SQLGetCursorName(stmt_handle(), cursor_name, sizeof(cursor_name), &name_len);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(name_len == 11);
  REQUIRE(std::string(reinterpret_cast<char*>(cursor_name)) == "CloseCursor");
}

TEST_CASE_METHOD(StmtDefaultDSNFixture,
                 "SQLGetCursorName: 01004 truncation returns correct partial name and full length",
                 "[odbc-api][cursorname][preparing]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret =
      SQLSetCursorName(stmt_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("LongCursorName")), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // Buffer of 5 bytes = 4 chars + null terminator
  SQLCHAR cursor_name[5] = {};
  SQLSMALLINT name_len = 0;
  // 01004: String data, right truncated
  ret = SQLGetCursorName(stmt_handle(), cursor_name, sizeof(cursor_name), &name_len);
  REQUIRE_EXPECTED_WARNING(ret, "01004", stmt_handle(), SQL_HANDLE_STMT);
  REQUIRE(name_len == 14);
  REQUIRE(std::string(reinterpret_cast<char*>(cursor_name)) == "Long");
}

TEST_CASE_METHOD(StmtDefaultDSNFixture,
                 "SQLGetCursorName: 01004 with BufferLength of 1 returns empty string and full length",
                 "[odbc-api][cursorname][preparing]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLSetCursorName(stmt_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("TestName")), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // BufferLength of 1 = only null terminator fits, truncation occurs
  SQLCHAR cursor_name[1] = {};
  SQLSMALLINT name_len = 0;
  // 01004: String data, right truncated
  ret = SQLGetCursorName(stmt_handle(), cursor_name, 1, &name_len);
  REQUIRE_EXPECTED_WARNING(ret, "01004", stmt_handle(), SQL_HANDLE_STMT);
  REQUIRE(name_len == 8);
  REQUIRE(cursor_name[0] == '\0');
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetCursorName: NULL CursorName buffer returns length in NameLengthPtr",
                 "[odbc-api][cursorname][preparing]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret =
      SQLSetCursorName(stmt_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>("NullBufTest")), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // Per spec: "If CursorName is NULL, NameLengthPtr will still return the total number of characters"
  SQLSMALLINT name_len = 0;
  ret = SQLGetCursorName(stmt_handle(), nullptr, 0, &name_len);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(name_len == 11);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetCursorName: 01004 with BufferLength of 0 and non-NULL buffer",
                 "[odbc-api][cursorname][preparing]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), reinterpret_cast<SQLCHAR*>(const_cast<char*>(dsn_name().c_str())), SQL_NTS,
                             nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHSTMT stmt = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt);
  REQUIRE(ret == SQL_SUCCESS);

  // Known bug: cursor names shorter than 10 chars return incorrect name_len when
  // BufferLength=0 due to an interaction between unixODBC's ANSI-to-Wide shim and
  // the Simba SDK driver. The DM passes a miscast SQLCHAR* as SQLWCHAR* to the
  // driver's SQLGetCursorNameW, and the driver reads from the buffer despite
  // BufferLength=0. Using a name >= 10 chars avoids the bug.
  // See: https://github.com/snowflakedb/snowflake-sdks-drivers-issues-teamwork/issues/1371
  ret = SQLSetCursorName(stmt, reinterpret_cast<SQLCHAR*>(const_cast<char*>("ZeroBufCursor")), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR cursor_name[128] = {};
  SQLSMALLINT name_len = 0;
  // BufferLength 0 with non-NULL buffer: truncation since no chars can be written
  // 01004: String data, right truncated
  ret = SQLGetCursorName(stmt, cursor_name, 0, &name_len);
  REQUIRE_EXPECTED_WARNING(ret, "01004", stmt, SQL_HANDLE_STMT);
  REQUIRE(name_len == 13);

  SQLFreeHandle(SQL_HANDLE_STMT, stmt);
  SQLDisconnect(dbc_handle());
}

// ============================================================================
// SQLGetCursorName - Error Cases
// ============================================================================

TEST_CASE("SQLGetCursorName: SQL_INVALID_HANDLE for null statement handle",
          "[odbc-api][cursorname][preparing][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLCHAR cursor_name[128] = {};
  SQLSMALLINT name_len = 0;
  const SQLRETURN ret = SQLGetCursorName(SQL_NULL_HSTMT, cursor_name, sizeof(cursor_name), &name_len);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetCursorName: HY090 for negative BufferLength",
                 "[odbc-api][cursorname][preparing][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLCHAR cursor_name[128] = {};
  SQLSMALLINT name_len = 0;
  // HY090: Invalid string or buffer length (negative BufferLength)
  SQLRETURN ret = SQLGetCursorName(stmt_handle(), cursor_name, -1, &name_len);
  REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
}
