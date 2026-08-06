#include <sql.h>
#include <sqlext.h>
#include <sqltypes.h>

#include <chrono>
#include <string>
#include <thread>

#include <catch2/catch_test_macros.hpp>

#include "ODBCFixtures.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"
#include "test_macros.hpp"
#include "test_setup.hpp"

// ============================================================================
// SQLGetCursorName
// ============================================================================

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetCursorName: Auto-generated cursor name starts with SQL_CUR",
                 "[odbc-api][cursorname][preparing]") {
  SQLCHAR cursor_name[128] = {};
  SQLSMALLINT name_len = 0;
  SQLRETURN ret = SQLGetCursorName(stmt_handle(), cursor_name, sizeof(cursor_name), &name_len);

  IODBC_ONLY {
    // Under iODBC neither driver exposes an auto-generated cursor name: the
    //   first SQLGetCursorName on an unprepared statement returns SQL_ERROR.
    //   This is an iODBC Driver Manager behavior — the DM serves cursor names
    //   from its own state and never sees the driver-internal SQL_CUR default —
    //   so it holds for both drivers (BD#66 previously attributed this to the
    //   old driver alone; it also holds for the new driver under iODBC). On
    //   unixODBC / Windows the driver's SQL_CUR default shows through.
    REQUIRE(ret == SQL_ERROR);
    return;
  }
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(name_len > 0);

  std::string name(reinterpret_cast<char*>(cursor_name));
  REQUIRE(name.length() == static_cast<size_t>(name_len));
  REQUIRE(name.substr(0, 7) == "SQL_CUR");
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetCursorName: Different statements have different auto-generated names",
                 "[odbc-api][cursorname][preparing]") {
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
  REQUIRE(ret == SQL_SUCCESS);

  SQLHSTMT stmt1 = SQL_NULL_HSTMT, stmt2 = SQL_NULL_HSTMT;
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt1);
  REQUIRE(ret == SQL_SUCCESS);
  ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc_handle(), &stmt2);
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR name1[128] = {}, name2[128] = {};
  SQLSMALLINT len1 = 0, len2 = 0;

  ret = SQLGetCursorName(stmt1, name1, sizeof(name1), &len1);
  IODBC_ONLY {
    // Under iODBC neither driver synthesizes a per-statement auto cursor name;
    //   SQLGetCursorName on a fresh statement returns SQL_ERROR. This is an
    //   iODBC Driver Manager behavior, identical on both drivers (BD#66
    //   previously attributed this to the old driver alone; it also holds for
    //   the new driver under iODBC).
    REQUIRE(ret == SQL_ERROR);
    CHECK(SQLFreeHandle(SQL_HANDLE_STMT, stmt1) == SQL_SUCCESS);
    CHECK(SQLFreeHandle(SQL_HANDLE_STMT, stmt2) == SQL_SUCCESS);
    CHECK(SQLDisconnect(dbc_handle()) == SQL_SUCCESS);
    return;
  }
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
  SQLRETURN ret = SQLSetCursorName(stmt_handle(), sqlchar("MyCursor"), SQL_NTS);
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
  SQLRETURN ret = SQLSetCursorName(stmt_handle(), sqlchar("PrepCursor"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLPrepare(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
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
  SQLRETURN ret = SQLSetCursorName(stmt_handle(), sqlchar("CloseCursor"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT 1"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLCloseCursor(stmt_handle());
  REQUIRE(ret == SQL_SUCCESS);

  SQLCHAR cursor_name[128] = {};
  SQLSMALLINT name_len = 0;
  ret = SQLGetCursorName(stmt_handle(), cursor_name, sizeof(cursor_name), &name_len);
  IODBC_ONLY {
    // Under iODBC the cursor name is dropped once SQLCloseCursor releases the
    //   underlying cursor: SQLGetCursorName then returns SQL_ERROR instead of
    //   replaying the user-supplied name. This is an iODBC Driver Manager
    //   behavior, identical on both drivers (BD#66 previously attributed this
    //   to the old driver alone; it also holds for the new driver under
    //   iODBC). On unixODBC / Windows the name stays attached to the statement
    //   handle until another SQLSetCursorName overrides it.
    REQUIRE(ret == SQL_ERROR);
  }
  else {
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(name_len == 11);
    REQUIRE(std::string(reinterpret_cast<char*>(cursor_name)) == "CloseCursor");
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture,
                 "SQLGetCursorName: 01004 truncation returns correct partial name and full length",
                 "[odbc-api][cursorname][preparing]") {
  SQLRETURN ret = SQLSetCursorName(stmt_handle(), sqlchar("LongCursorName"), SQL_NTS);
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
  SQLRETURN ret = SQLSetCursorName(stmt_handle(), sqlchar("TestName"), SQL_NTS);
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
  SQLRETURN ret = SQLSetCursorName(stmt_handle(), sqlchar("NullBufTest"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // Per spec: "If CursorName is NULL, NameLengthPtr will still return the total number of characters"
  SQLSMALLINT name_len = 0;
  ret = SQLGetCursorName(stmt_handle(), nullptr, 0, &name_len);
  IODBC_ONLY {
    // Under iODBC the ANSI entry point flags the NULL buffer as an implicit
    //   string truncation and returns SQL_SUCCESS_WITH_INFO instead of plain
    //   SQL_SUCCESS (same pattern as SQLGetInfo with NULL InfoValuePtr). This
    //   is an iODBC Driver Manager behavior, identical on both drivers (BD#61
    //   previously attributed this to the old driver alone; it also holds for
    //   the new driver under iODBC).
    REQUIRE(ret == SQL_SUCCESS_WITH_INFO);
  }
  else {
    REQUIRE(ret == SQL_SUCCESS);
  }
  REQUIRE(name_len == 11);
}

TEST_CASE_METHOD(DbcDefaultDSNFixture, "SQLGetCursorName: 01004 with BufferLength of 0 and non-NULL buffer",
                 "[odbc-api][cursorname][preparing]") {
  SQLRETURN ret = SQLConnect(dbc_handle(), sqlchar(dsn_name().c_str()), SQL_NTS, nullptr, 0, nullptr, 0);
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
  ret = SQLSetCursorName(stmt, sqlchar("ZeroBufCursor"), SQL_NTS);
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

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetCursorName: exact-fit buffer returns full name without truncation",
                 "[odbc-api][cursorname][preparing]") {
  SQLRETURN ret = SQLSetCursorName(stmt_handle(), sqlchar("ExactFit"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  // BufferLength exactly name length + 1 (8 chars + null terminator): the
  //   boundary just below the truncation cases. Must return SQL_SUCCESS with
  //   the full name and no 01004 warning.
  SQLCHAR cursor_name[9] = {};
  SQLSMALLINT name_len = 0;
  ret = SQLGetCursorName(stmt_handle(), cursor_name, sizeof(cursor_name), &name_len);
  REQUIRE(ret == SQL_SUCCESS);
  REQUIRE(name_len == 8);
  REQUIRE(std::string(reinterpret_cast<char*>(cursor_name)) == "ExactFit");
}

// ============================================================================
// SQLGetCursorName - Error Cases
// ============================================================================

TEST_CASE("SQLGetCursorName: SQL_INVALID_HANDLE for null statement handle",
          "[odbc-api][cursorname][preparing][error]") {
  SQLCHAR cursor_name[128] = {};
  SQLSMALLINT name_len = 0;
  const SQLRETURN ret = SQLGetCursorName(SQL_NULL_HSTMT, cursor_name, sizeof(cursor_name), &name_len);
  REQUIRE(ret == SQL_INVALID_HANDLE);
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetCursorName: HY090 for negative BufferLength",
                 "[odbc-api][cursorname][preparing][error]") {
  SQLCHAR cursor_name[128] = {};
  SQLSMALLINT name_len = 0;
  // HY090: Invalid string or buffer length (negative BufferLength)
  SQLRETURN ret = SQLGetCursorName(stmt_handle(), cursor_name, -1, &name_len);
  IODBC_ONLY {
    // iODBC's DM validates BufferLength<0 itself and surfaces the ODBC 2.x
    //   alias "S1090" before the call ever reaches the driver, so both the
    //   old and new drivers report "S1090" here (BD#70).
    REQUIRE_EXPECTED_ERROR(ret, "S1090", stmt_handle(), SQL_HANDLE_STMT);
  }
  else {
    REQUIRE_EXPECTED_ERROR(ret, "HY090", stmt_handle(), SQL_HANDLE_STMT);
  }
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetCursorName: HY010 during SQL_NEED_DATA",
                 "[odbc-api][getcursorname][preparing][error]") {
  SQLRETURN ret = SQLPrepare(stmt_handle(), sqlchar("SELECT ?"), SQL_NTS);
  REQUIRE(ret == SQL_SUCCESS);

  SQLLEN dae_ind = SQL_DATA_AT_EXEC;
  ret = SQLBindParameter(stmt_handle(), 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR, 100, 0,
                         reinterpret_cast<SQLPOINTER>(1), 0, &dae_ind);
  REQUIRE(ret == SQL_SUCCESS);

  ret = SQLExecute(stmt_handle());
  REQUIRE(ret == SQL_NEED_DATA);

  SQLCHAR name[64] = {};
  SQLSMALLINT name_len = 0;
  ret = SQLGetCursorName(stmt_handle(), name, sizeof(name), &name_len);
  OLD_IODBC_ONLY("BD#70") {
    // iODBC's DM catches SQLGetCursorName during SQL_NEED_DATA as a function
    //   sequence error and surfaces the ODBC 2.x alias "S1010" before the old
    //   driver can map it to "HY010".
    REQUIRE_EXPECTED_ERROR(ret, "S1010", stmt_handle(), SQL_HANDLE_STMT);
  }
  else {
    REQUIRE_EXPECTED_ERROR(ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);
  }

  SQLCancel(stmt_handle());
}

TEST_CASE_METHOD(StmtDefaultDSNFixture, "SQLGetCursorName: returns HY010 during async execution (S11)",
                 "[odbc-api][cursorname][preparing][error][async]") {
  SKIP_OLD_DRIVER("BD#32", "Old driver does not cancel async ops; SQLFreeHandle on an in-flight statement segfaults.");
  SQLRETURN ret =
      SQLSetStmtAttr(stmt_handle(), SQL_ATTR_ASYNC_ENABLE, reinterpret_cast<SQLPOINTER>(SQL_ASYNC_ENABLE_ON), 0);
  REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt_handle()), OdbcMatchers::Succeeded());

  ret = SQLExecDirect(stmt_handle(), sqlchar("SELECT COUNT(*) FROM TABLE(GENERATOR(TIMELIMIT => 30))"), SQL_NTS);
  REQUIRE(ret == SQL_STILL_EXECUTING);

  // S11: GetCursorName must return HY010 (FunctionSequenceError).
  SQLCHAR name[64] = {};
  SQLSMALLINT name_len = 0;
  const SQLRETURN cn_ret = SQLGetCursorName(stmt_handle(), name, sizeof(name), &name_len);
  OLD_IODBC_ONLY("BD#70") { REQUIRE_EXPECTED_ERROR(cn_ret, "S1010", stmt_handle(), SQL_HANDLE_STMT); }
  else {
    REQUIRE_EXPECTED_ERROR(cn_ret, "HY010", stmt_handle(), SQL_HANDLE_STMT);
  }

  SQLCancel(stmt_handle());
}
