#include <sql.h>
#include <sqltypes.h>

#include <cstring>
#include <string>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
#include "get_diag_rec.hpp"
#include "odbc_cast.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// ============================================================================
// SQLSetCursorName
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetCursorName: Renaming cursor replaces previous name",
                 "[odbc-api][cursorname][preparing]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLSetCursorName(stmt_handle(), sqlchar("CursorA"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetCursorName(stmt_handle(), sqlchar("CursorB"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR cursor_name[128] = {};
  SQLSMALLINT name_len = 0;
  ret = SQLGetCursorName(stmt_handle(), cursor_name, sizeof(cursor_name), &name_len);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(name_len == 7);
  REQUIRE(std::string(reinterpret_cast<char*>(cursor_name)) == "CursorB");
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetCursorName: Can rename in prepared state",
                 "[odbc-api][cursorname][preparing]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetCursorName(stmt_handle(), sqlchar("PreparedCur"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR cursor_name[128] = {};
  SQLSMALLINT name_len = 0;
  ret = SQLGetCursorName(stmt_handle(), cursor_name, sizeof(cursor_name), &name_len);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(name_len == 11);
  REQUIRE(std::string(reinterpret_cast<char*>(cursor_name)) == "PreparedCur");
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetCursorName: Can set after SQLCloseCursor",
                 "[odbc-api][cursorname][preparing]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetCursorName(stmt_handle(), sqlchar("AfterClose"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR cursor_name[128] = {};
  SQLSMALLINT name_len = 0;
  ret = SQLGetCursorName(stmt_handle(), cursor_name, sizeof(cursor_name), &name_len);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(name_len == 10);
  REQUIRE(std::string(reinterpret_cast<char*>(cursor_name)) == "AfterClose");
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetCursorName: With explicit name length instead of SQL_NTS",
                 "[odbc-api][cursorname][preparing]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLSetCursorName(stmt_handle(), sqlchar("ExplicitLen"), 11);
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR cursor_name[128] = {};
  SQLSMALLINT name_len = 0;
  ret = SQLGetCursorName(stmt_handle(), cursor_name, sizeof(cursor_name), &name_len);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(name_len == 11);
  REQUIRE(std::string(reinterpret_cast<char*>(cursor_name)) == "ExplicitLen");
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetCursorName: Explicit length shorter than string uses partial name",
                 "[odbc-api][cursorname][preparing]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // Pass length 4 for "LongName" -> should only use "Long"
  SQLRETURN ret = SQLSetCursorName(stmt_handle(), sqlchar("LongName"), 4);
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR cursor_name[128] = {};
  SQLSMALLINT name_len = 0;
  ret = SQLGetCursorName(stmt_handle(), cursor_name, sizeof(cursor_name), &name_len);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(name_len == 4);
  REQUIRE(std::string(reinterpret_cast<char*>(cursor_name)) == "Long");
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetCursorName: Empty cursor name succeeds",
                 "[odbc-api][cursorname][preparing]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLSetCursorName(stmt_handle(), sqlchar(""), SQL_NTS);
  WINDOWS_ONLY {
    // Windows DM rejects empty cursor name
    REQUIRE(ret == SQL_ERROR);
  }
  UNIX_ONLY {
    // Reference driver accepts empty cursor name
    REQUIRE(ret == SQL_SUCCESS);

    SQLCHAR cursor_name[128] = {};
    SQLSMALLINT name_len = 0;
    ret = SQLGetCursorName(stmt_handle(), cursor_name, sizeof(cursor_name), &name_len);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(name_len == 0);
    REQUIRE(cursor_name[0] == '\0');
  }
}

// ============================================================================
// SQLSetCursorName - Error Cases
// ============================================================================

TEST_CASE("SQLSetCursorName: SQL_INVALID_HANDLE for null statement handle",
          "[odbc-api][cursorname][preparing][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  const SQLRETURN ret = SQLSetCursorName(SQL_NULL_HSTMT, sqlchar("Test"), SQL_NTS);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLSetCursorName: 3C000 for duplicate cursor name on same connection",
                 "[odbc-api][cursorname][preparing][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHSTMT stmt1 = SQL_NULL_HSTMT, stmt2 = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt1);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt2);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLSetCursorName(stmt1, sqlchar("DupCursor"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // 3C000: Duplicate cursor name
  ret = SQLSetCursorName(stmt2, sqlchar("DupCursor"), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "3C000", stmt2, SQL_HANDLE_STMT);

  SQLFreeHandle(SQL_HANDLE_STMT, stmt1);
  SQLFreeHandle(SQL_HANDLE_STMT, stmt2);
  SQLDisconnect(dbc_handle());
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetCursorName: 34000 for cursor name starting with SQL_CUR prefix",
                 "[odbc-api][cursorname][preparing][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // 34000: Invalid cursor name (starting with reserved prefix "SQL_CUR")
  SQLRETURN ret = SQLSetCursorName(stmt_handle(), sqlchar("SQL_CUR_TEST"), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "34000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetCursorName: 34000 for cursor name starting with SQLCUR prefix",
                 "[odbc-api][cursorname][preparing][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // 34000: Invalid cursor name ("SQLCUR" prefix is also reserved)
  SQLRETURN ret = SQLSetCursorName(stmt_handle(), sqlchar("SQLCUR_TEST"), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "34000", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetCursorName: HY009 for null cursor name pointer",
                 "[odbc-api][cursorname][preparing][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  // HY009: Invalid use of null pointer
  SQLRETURN ret = SQLSetCursorName(stmt_handle(), nullptr, SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetCursorName: HY009 for negative NameLength",
                 "[odbc-api][cursorname][preparing][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLSetCursorName(stmt_handle(), sqlchar("Test"), -5);
  WINDOWS_ONLY {
    // Windows DM returns HY090 (Invalid string or buffer length) for negative NameLength
    REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
  }
  UNIX_ONLY {
    // Note: Reference driver returns HY009 instead of ODBC spec-defined HY090 for negative NameLength
    REQUIRE_EXPECTED_ERROR(ret, "HY009", stmt_handle(), SQL_HANDLE_STMT);
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLSetCursorName: 24000 when cursor is open",
                 "[odbc-api][cursorname][preparing][error]") {
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  SQLRETURN ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // 24000: Invalid cursor state (cursor is open)
  ret = SQLSetCursorName(stmt_handle(), sqlchar("AfterExec"), SQL_NTS);
  REQUIRE_EXPECTED_ERROR(ret, "24000", stmt_handle(), SQL_HANDLE_STMT);
}
