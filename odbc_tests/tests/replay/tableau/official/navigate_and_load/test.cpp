#include <algorithm>
#include <cstring>
#include <string>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "ReadOnlyDbFixture.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("Replay: tableau official navigate_and_load", "[tableau][official][navigate_and_load]") {
  // The new driver skips this replay while these are open:
  // SNOW-4283109 SQLTables returns no catalogs without a current database.
  // SNOW-4283110 SQLPrimaryKeys and SQLForeignKeys SMALLINT display size is 136.
  // SNOW-4283111 SQLTables SQL_C_WCHAR fetch posts 01004 when the values fit.
  SKIP_NEW_DRIVER_NOT_IMPLEMENTED();

  auto base = DataSourceConfig::Snowflake();
  auto config = base.install();
  // no_db and fixture_db copy base, so they use the driver install() registered.
  // dbc0 has no current database, so the catalog list is the navigator's
  // pre-selection probe. dbc1 names the fixture database and has no current
  // schema; qualified names resolve, and the replay also runs USE DATABASE.
  auto no_db = base;
  no_db.remove("DATABASE").remove("SCHEMA");
  auto fixture_db = base;
  fixture_db.set("DATABASE", READONLY_DB_NAME).remove("SCHEMA");

  SQLHENV env0 = SQL_NULL_HENV;
  // SQLAllocHandle - SQLHENV
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env0);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(env0 != SQL_NULL_HENV);
  }

  // SQLSetEnvAttr - SQL_ATTR_ODBC_VERSION
  {
    SQLRETURN ret = SQLSetEnvAttr(env0, SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env0), OdbcMatchers::IsSuccess());
  }

  SQLHDBC dbc0 = SQL_NULL_HDBC;
  // SQLAllocHandle - SQLHDBC
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env0, &dbc0);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env0), OdbcMatchers::IsSuccess());
    REQUIRE(dbc0 != SQL_NULL_HDBC);
  }

  // SQLSetConnectAttr - SQL_ATTR_LOGIN_TIMEOUT
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc0, SQL_ATTR_LOGIN_TIMEOUT, (SQLPOINTER)30, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLDriverConnect
  {
    SQLRETURN ret = SQLDriverConnect(dbc0, nullptr, sqlchar(no_db.connection_string().c_str()), SQL_NTS, nullptr, 0,
                                     nullptr, SQL_DRIVER_NOPROMPT);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::Succeeded());
  }

  // SQLSetConnectAttr - SQL_ATTR_QUIET_MODE
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc0, SQL_ATTR_QUIET_MODE, nullptr, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_TXN_CAPABLE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_TXN_CAPABLE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x3u);
  }

  // SQLGetInfo - SQL_CURSOR_COMMIT_BEHAVIOR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CURSOR_COMMIT_BEHAVIOR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1u);
  }

  // SQLGetInfo - SQL_DBMS_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DBMS_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DRIVER_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DRIVER_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_ODBC_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DRIVER_ODBC_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_GETDATA_EXTENSIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_GETDATA_EXTENSIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xBu);
  }

  // SQLGetInfo - SQL_ODBC_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_ODBC_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt0 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc0, &stmt0);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    REQUIRE(stmt0 != SQL_NULL_HSTMT);
  }

  // SQLSetStmtAttr - SQL_ATTR_ENABLE_AUTO_IPD
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt0, SQL_ATTR_ENABLE_AUTO_IPD, (SQLPOINTER)1, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_CURSOR_TYPE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt0, SQL_ATTR_CURSOR_TYPE, nullptr, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(stmt0, sqlchar("SHOW WAREHOUSES"), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLExecute
  {
    SQLRETURN ret = SQLExecute(stmt0);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SHOW WAREHOUSES column count and descriptor sizes follow the Snowflake
  // version, so this replay does not pin the captured layout.
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt0, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
    CHECK(numCols > 0);
  }

  {
    SQLRETURN ret;
    long long rows = 0;
    while ((ret = SQLFetch(stmt0)) == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO) {
      ++rows;
    }
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsNoData());
    CHECK(rows > 0);
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt1 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc0, &stmt1);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    REQUIRE(stmt1 != SQL_NULL_HSTMT);
  }

  // SQLGetInfo - SQL_SEARCH_PATTERN_ESCAPE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_SEARCH_PATTERN_ESCAPE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "\\");
  }

  // SQLTables
  {
    SQLRETURN ret = SQLTables(stmt1, sqlchar("%"), -3, sqlchar(""), -3, sqlchar(""), -3, nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_78(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_78(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt1, 1, SQL_C_WCHAR, bind_buf_78.data(), 1024, bind_ind_78.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_79(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_79(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt1, 2, SQL_C_WCHAR, bind_buf_79.data(), 1024, bind_ind_79.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  std::vector<char> bind_buf_80(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_80(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt1, 3, SQL_C_WCHAR, bind_buf_80.data(), 1024, bind_ind_80.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 4
  std::vector<char> bind_buf_81(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_81(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt1, 4, SQL_C_WCHAR, bind_buf_81.data(), 1024, bind_ind_81.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 5
  std::vector<char> bind_buf_82(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_82(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt1, 5, SQL_C_WCHAR, bind_buf_82.data(), 1024, bind_ind_82.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // catalog object enumeration: the row count and object names are a
  // property of the connected account and fixture, not the driver, so
  // drain the cursor structurally rather than pinning environment-specific
  // rows.
  {
    SQLRETURN ret;
    long long enum_rows = 0;
    while ((ret = SQLFetch(stmt1)) == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO) {
      ++enum_rows;
    }
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsNoData());
    CHECK(enum_rows > 0);
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt1);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  SQLHENV env1 = SQL_NULL_HENV;
  // SQLAllocHandle - SQLHENV
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env1);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(env1 != SQL_NULL_HENV);
  }

  // SQLSetEnvAttr - SQL_ATTR_ODBC_VERSION
  {
    SQLRETURN ret = SQLSetEnvAttr(env1, SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env1), OdbcMatchers::IsSuccess());
  }

  SQLHDBC dbc1 = SQL_NULL_HDBC;
  // SQLAllocHandle - SQLHDBC
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env1, &dbc1);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env1), OdbcMatchers::IsSuccess());
    REQUIRE(dbc1 != SQL_NULL_HDBC);
  }

  // SQLSetConnectAttr - SQL_ATTR_LOGIN_TIMEOUT
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc1, SQL_ATTR_LOGIN_TIMEOUT, (SQLPOINTER)30, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
  }

  // SQLDriverConnect
  {
    SQLRETURN ret = SQLDriverConnect(dbc1, nullptr, sqlchar(fixture_db.connection_string().c_str()), SQL_NTS, nullptr,
                                     0, nullptr, SQL_DRIVER_NOPROMPT);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::Succeeded());
  }

  // SQLSetConnectAttr - SQL_ATTR_QUIET_MODE
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc1, SQL_ATTR_QUIET_MODE, nullptr, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_TXN_CAPABLE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_TXN_CAPABLE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x3u);
  }

  // SQLGetInfo - SQL_CURSOR_COMMIT_BEHAVIOR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_CURSOR_COMMIT_BEHAVIOR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x1u);
  }

  // SQLGetInfo - SQL_DBMS_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_DBMS_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_DRIVER_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_DRIVER_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DRIVER_ODBC_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_DRIVER_ODBC_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_GETDATA_EXTENSIONS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_GETDATA_EXTENSIONS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xBu);
  }

  // SQLGetInfo - SQL_ODBC_VER
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_ODBC_VER, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt2 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc1, &stmt2);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    REQUIRE(stmt2 != SQL_NULL_HSTMT);
  }

  // SQLSetStmtAttr - SQL_ATTR_ENABLE_AUTO_IPD
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt2, SQL_ATTR_ENABLE_AUTO_IPD, (SQLPOINTER)1, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_CURSOR_TYPE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt2, SQL_ATTR_CURSOR_TYPE, nullptr, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(stmt2, sqlchar("USE DATABASE \"ODBCMETADATATESTDB\""), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLExecute
  {
    SQLRETURN ret = SQLExecute(stmt2);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt3 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc1, &stmt3);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    REQUIRE(stmt3 != SQL_NULL_HSTMT);
  }

  // SQLSetStmtAttr - SQL_ATTR_ENABLE_AUTO_IPD
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt3, SQL_ATTR_ENABLE_AUTO_IPD, (SQLPOINTER)1, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_CURSOR_TYPE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt3, SQL_ATTR_CURSOR_TYPE, nullptr, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(stmt3,
                               sqlchar("SELECT TYPE FROM \"ODBCMETADATATESTDB\".INFORMATION_SCHEMA.DATABASES WHERE "
                                       "DATABASE_NAME = 'ODBCMETADATATESTDB'"),
                               SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLExecute
  {
    SQLRETURN ret = SQLExecute(stmt3);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt3, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(numCols == 1);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 19);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt3, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROW_ARRAY_SIZE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt3, SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)26214, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  SQLULEN attr_ptr_83 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROWS_FETCHED_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt3, SQL_ATTR_ROWS_FETCHED_PTR, (SQLPOINTER)&attr_ptr_83, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  std::vector<SQLUSMALLINT> attr_ptr_84(26214, 0);
  // SQLSetStmtAttr - SQL_ATTR_ROW_STATUS_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt3, SQL_ATTR_ROW_STATUS_PTR, (SQLPOINTER)attr_ptr_84.data(), 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_85(26214 * 40, 0);
  std::vector<SQLLEN> bind_ind_85(26214, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt3, 1, SQL_C_WCHAR, bind_buf_85.data(), 40, bind_ind_85.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsNoData());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_86(26214 * 40, 0);
  std::vector<SQLLEN> bind_ind_86(26214, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt3, 1, SQL_C_WCHAR, bind_buf_86.data(), 40, bind_ind_86.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt4 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc1, &stmt4);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    REQUIRE(stmt4 != SQL_NULL_HSTMT);
  }

  // SQLSetStmtAttr - SQL_ATTR_ENABLE_AUTO_IPD
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt4, SQL_ATTR_ENABLE_AUTO_IPD, (SQLPOINTER)1, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_CURSOR_TYPE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt4, SQL_ATTR_CURSOR_TYPE, nullptr, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(stmt4, sqlchar("SELECT CURRENT_SCHEMA()"), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLExecute
  {
    SQLRETURN ret = SQLExecute(stmt4);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt4, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
    CHECK(numCols == 1);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 134217728);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt4, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROW_ARRAY_SIZE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt4, SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)16, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  SQLULEN attr_ptr_87 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROWS_FETCHED_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt4, SQL_ATTR_ROWS_FETCHED_PTR, (SQLPOINTER)&attr_ptr_87, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  std::vector<SQLUSMALLINT> attr_ptr_88(16, 0);
  // SQLSetStmtAttr - SQL_ATTR_ROW_STATUS_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt4, SQL_ATTR_ROW_STATUS_PTR, (SQLPOINTER)attr_ptr_88.data(), 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_89(16 * 1048576, 0);
  std::vector<SQLLEN> bind_ind_89(16, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt4, 1, SQL_C_WCHAR, bind_buf_89.data(), 1048576, bind_ind_89.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt4);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt4);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsNoData());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_90(16 * 1048576, 0);
  std::vector<SQLLEN> bind_ind_90(16, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt4, 1, SQL_C_WCHAR, bind_buf_90.data(), 1048576, bind_ind_90.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt4);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt5 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc1, &stmt5);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    REQUIRE(stmt5 != SQL_NULL_HSTMT);
  }

  // SQLGetInfo - SQL_SEARCH_PATTERN_ESCAPE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc1, SQL_SEARCH_PATTERN_ESCAPE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "\\");
  }

  // SQLTables
  {
    SQLRETURN ret = SQLTables(stmt5, sqlchar("ODBCMETADATATESTDB"), -3, nullptr, 0, sqlchar("%"), -3, nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_91(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_91(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt5, 1, SQL_C_WCHAR, bind_buf_91.data(), 1024, bind_ind_91.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_92(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_92(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt5, 2, SQL_C_WCHAR, bind_buf_92.data(), 1024, bind_ind_92.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  std::vector<char> bind_buf_93(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_93(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt5, 3, SQL_C_WCHAR, bind_buf_93.data(), 1024, bind_ind_93.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 4
  std::vector<char> bind_buf_94(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_94(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt5, 4, SQL_C_WCHAR, bind_buf_94.data(), 1024, bind_ind_94.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 5
  std::vector<char> bind_buf_95(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_95(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt5, 5, SQL_C_WCHAR, bind_buf_95.data(), 1024, bind_ind_95.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // catalog object enumeration: the row count and object names are a
  // property of the connected account and fixture, not the driver, so
  // drain the cursor structurally rather than pinning environment-specific
  // rows.
  {
    SQLRETURN ret;
    long long enum_rows = 0;
    while ((ret = SQLFetch(stmt5)) == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO) {
      ++enum_rows;
    }
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsNoData());
    CHECK(enum_rows > 0);
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt6 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc1, &stmt6);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    REQUIRE(stmt6 != SQL_NULL_HSTMT);
  }

  // SQLTables
  {
    SQLRETURN ret =
        SQLTables(stmt6, sqlchar("ODBCMETADATATESTDB"), -3, sqlchar("DATATYPETESTS"), -3, nullptr, 0, nullptr, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_96(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_96(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt6, 1, SQL_C_WCHAR, bind_buf_96.data(), 1024, bind_ind_96.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_97(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_97(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt6, 2, SQL_C_WCHAR, bind_buf_97.data(), 1024, bind_ind_97.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  std::vector<char> bind_buf_98(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_98(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt6, 3, SQL_C_WCHAR, bind_buf_98.data(), 1024, bind_ind_98.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 4
  std::vector<char> bind_buf_99(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_99(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt6, 4, SQL_C_WCHAR, bind_buf_99.data(), 1024, bind_ind_99.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 5
  std::vector<char> bind_buf_100(1 * 1024, 0);
  std::vector<SQLLEN> bind_ind_100(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt6, 5, SQL_C_WCHAR, bind_buf_100.data(), 1024, bind_ind_100.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsNoData());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt7 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc1, &stmt7);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    REQUIRE(stmt7 != SQL_NULL_HSTMT);
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(
        stmt7, sqlchar("SELECT *\nFROM \"ODBCMETADATATESTDB\".\"DATATYPETESTS\".\"ALLDATATYPESNAV\""), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt7, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numCols == 25);
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt7, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numCols == 25);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 16);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 136);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 136);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 136);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 136);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 136);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 136);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DOUBLE);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 24);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DOUBLE);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 24);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DOUBLE);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 24);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 256);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 16777216);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 10);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -2);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 32);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -2);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 16777216);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 15;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -7);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 1);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 16;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_TYPE_DATE);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 10);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 17;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_TYPE_TIME);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 18);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 18;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_TYPE_TIMESTAMP);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 29);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 19;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_TYPE_TIMESTAMP);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 29);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 20;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_TYPE_TIMESTAMP);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 29);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 21;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 134217728);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 22;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 134217728);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 23;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 134217728);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 24;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 134217728);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 25;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt7, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt7);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt7), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt8 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc1, &stmt8);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    REQUIRE(stmt8 != SQL_NULL_HSTMT);
  }

  // SQLForeignKeys
  {
    SQLRETURN ret = SQLForeignKeys(stmt8, nullptr, 0, nullptr, 0, nullptr, 0, sqlchar("ODBCMETADATATESTDB"), -3,
                                   sqlchar("DATATYPETESTS"), -3, sqlchar("ALLDATATYPESNAV"), -3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt8, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numCols == 14);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 255);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 255);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 255);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 255);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 255);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 255);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 255);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 7;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 255);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 8;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 6);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 9;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 6);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 10;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 6);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 11;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 255);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 12;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 255);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 13;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 6);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 14;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt8, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROW_ARRAY_SIZE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt8, SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)102, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  SQLULEN attr_ptr_101 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROWS_FETCHED_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt8, SQL_ATTR_ROWS_FETCHED_PTR, (SQLPOINTER)&attr_ptr_101, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  std::vector<SQLUSMALLINT> attr_ptr_102(102, 0);
  // SQLSetStmtAttr - SQL_ATTR_ROW_STATUS_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt8, SQL_ATTR_ROW_STATUS_PTR, (SQLPOINTER)attr_ptr_102.data(), 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_103(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_103(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 1, SQL_C_WCHAR, bind_buf_103.data(), 1022, bind_ind_103.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_104(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_104(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 2, SQL_C_WCHAR, bind_buf_104.data(), 1022, bind_ind_104.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  std::vector<char> bind_buf_105(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_105(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 3, SQL_C_WCHAR, bind_buf_105.data(), 1022, bind_ind_105.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 4
  std::vector<char> bind_buf_106(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_106(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 4, SQL_C_WCHAR, bind_buf_106.data(), 1022, bind_ind_106.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 5
  std::vector<char> bind_buf_107(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_107(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 5, SQL_C_WCHAR, bind_buf_107.data(), 1022, bind_ind_107.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 6
  std::vector<char> bind_buf_108(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_108(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 6, SQL_C_WCHAR, bind_buf_108.data(), 1022, bind_ind_108.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 7
  std::vector<char> bind_buf_109(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_109(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 7, SQL_C_WCHAR, bind_buf_109.data(), 1022, bind_ind_109.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 8
  std::vector<char> bind_buf_110(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_110(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 8, SQL_C_WCHAR, bind_buf_110.data(), 1022, bind_ind_110.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 9
  std::vector<char> bind_buf_111(102 * 2, 0);
  std::vector<SQLLEN> bind_ind_111(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 9, SQL_C_SSHORT, bind_buf_111.data(), 2, bind_ind_111.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 10
  std::vector<char> bind_buf_112(102 * 2, 0);
  std::vector<SQLLEN> bind_ind_112(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 10, SQL_C_SSHORT, bind_buf_112.data(), 2, bind_ind_112.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 11
  std::vector<char> bind_buf_113(102 * 2, 0);
  std::vector<SQLLEN> bind_ind_113(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 11, SQL_C_SSHORT, bind_buf_113.data(), 2, bind_ind_113.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 12
  std::vector<char> bind_buf_114(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_114(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 12, SQL_C_WCHAR, bind_buf_114.data(), 1022, bind_ind_114.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 13
  std::vector<char> bind_buf_115(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_115(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 13, SQL_C_WCHAR, bind_buf_115.data(), 1022, bind_ind_115.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 14
  std::vector<char> bind_buf_116(102 * 2, 0);
  std::vector<SQLLEN> bind_ind_116(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 14, SQL_C_SSHORT, bind_buf_116.data(), 2, bind_ind_116.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt8);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsNoData());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_117(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_117(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 1, SQL_C_WCHAR, bind_buf_117.data(), 1022, bind_ind_117.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_118(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_118(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 2, SQL_C_WCHAR, bind_buf_118.data(), 1022, bind_ind_118.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  std::vector<char> bind_buf_119(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_119(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 3, SQL_C_WCHAR, bind_buf_119.data(), 1022, bind_ind_119.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 4
  std::vector<char> bind_buf_120(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_120(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 4, SQL_C_WCHAR, bind_buf_120.data(), 1022, bind_ind_120.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 5
  std::vector<char> bind_buf_121(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_121(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 5, SQL_C_WCHAR, bind_buf_121.data(), 1022, bind_ind_121.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 6
  std::vector<char> bind_buf_122(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_122(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 6, SQL_C_WCHAR, bind_buf_122.data(), 1022, bind_ind_122.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 7
  std::vector<char> bind_buf_123(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_123(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 7, SQL_C_WCHAR, bind_buf_123.data(), 1022, bind_ind_123.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 8
  std::vector<char> bind_buf_124(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_124(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 8, SQL_C_WCHAR, bind_buf_124.data(), 1022, bind_ind_124.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 9
  std::vector<char> bind_buf_125(102 * 2, 0);
  std::vector<SQLLEN> bind_ind_125(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 9, SQL_C_SSHORT, bind_buf_125.data(), 2, bind_ind_125.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 10
  std::vector<char> bind_buf_126(102 * 2, 0);
  std::vector<SQLLEN> bind_ind_126(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 10, SQL_C_SSHORT, bind_buf_126.data(), 2, bind_ind_126.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 11
  std::vector<char> bind_buf_127(102 * 2, 0);
  std::vector<SQLLEN> bind_ind_127(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 11, SQL_C_SSHORT, bind_buf_127.data(), 2, bind_ind_127.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 12
  std::vector<char> bind_buf_128(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_128(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 12, SQL_C_WCHAR, bind_buf_128.data(), 1022, bind_ind_128.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 13
  std::vector<char> bind_buf_129(102 * 1022, 0);
  std::vector<SQLLEN> bind_ind_129(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 13, SQL_C_WCHAR, bind_buf_129.data(), 1022, bind_ind_129.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 14
  std::vector<char> bind_buf_130(102 * 2, 0);
  std::vector<SQLLEN> bind_ind_130(102, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt8, 14, SQL_C_SSHORT, bind_buf_130.data(), 2, bind_ind_130.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt8);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt8), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt9 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc1, &stmt9);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    REQUIRE(stmt9 != SQL_NULL_HSTMT);
  }

  // SQLPrimaryKeys
  {
    SQLRETURN ret = SQLPrimaryKeys(stmt9, sqlchar("ODBCMETADATATESTDB"), -3, sqlchar("DATATYPETESTS"), -3,
                                   sqlchar("ALLDATATYPESNAV"), -3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt9, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
    CHECK(numCols == 6);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 255);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 255);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 255);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 3;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 255);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 4;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_SMALLINT);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 6);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 5;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
    CHECK(numAttr == -9);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 255);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 6;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt9, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROW_ARRAY_SIZE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt9, SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)205, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  SQLULEN attr_ptr_131 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROWS_FETCHED_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt9, SQL_ATTR_ROWS_FETCHED_PTR, (SQLPOINTER)&attr_ptr_131, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  std::vector<SQLUSMALLINT> attr_ptr_132(205, 0);
  // SQLSetStmtAttr - SQL_ATTR_ROW_STATUS_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt9, SQL_ATTR_ROW_STATUS_PTR, (SQLPOINTER)attr_ptr_132.data(), 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_133(205 * 1022, 0);
  std::vector<SQLLEN> bind_ind_133(205, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt9, 1, SQL_C_WCHAR, bind_buf_133.data(), 1022, bind_ind_133.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_134(205 * 1022, 0);
  std::vector<SQLLEN> bind_ind_134(205, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt9, 2, SQL_C_WCHAR, bind_buf_134.data(), 1022, bind_ind_134.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  std::vector<char> bind_buf_135(205 * 1022, 0);
  std::vector<SQLLEN> bind_ind_135(205, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt9, 3, SQL_C_WCHAR, bind_buf_135.data(), 1022, bind_ind_135.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 4
  std::vector<char> bind_buf_136(205 * 1022, 0);
  std::vector<SQLLEN> bind_ind_136(205, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt9, 4, SQL_C_WCHAR, bind_buf_136.data(), 1022, bind_ind_136.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 5
  std::vector<char> bind_buf_137(205 * 2, 0);
  std::vector<SQLLEN> bind_ind_137(205, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt9, 5, SQL_C_SSHORT, bind_buf_137.data(), 2, bind_ind_137.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 6
  std::vector<char> bind_buf_138(205 * 1022, 0);
  std::vector<SQLLEN> bind_ind_138(205, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt9, 6, SQL_C_WCHAR, bind_buf_138.data(), 1022, bind_ind_138.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt9);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsNoData());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_139(205 * 1022, 0);
  std::vector<SQLLEN> bind_ind_139(205, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt9, 1, SQL_C_WCHAR, bind_buf_139.data(), 1022, bind_ind_139.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_140(205 * 1022, 0);
  std::vector<SQLLEN> bind_ind_140(205, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt9, 2, SQL_C_WCHAR, bind_buf_140.data(), 1022, bind_ind_140.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  std::vector<char> bind_buf_141(205 * 1022, 0);
  std::vector<SQLLEN> bind_ind_141(205, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt9, 3, SQL_C_WCHAR, bind_buf_141.data(), 1022, bind_ind_141.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 4
  std::vector<char> bind_buf_142(205 * 1022, 0);
  std::vector<SQLLEN> bind_ind_142(205, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt9, 4, SQL_C_WCHAR, bind_buf_142.data(), 1022, bind_ind_142.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 5
  std::vector<char> bind_buf_143(205 * 2, 0);
  std::vector<SQLLEN> bind_ind_143(205, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt9, 5, SQL_C_SSHORT, bind_buf_143.data(), 2, bind_ind_143.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 6
  std::vector<char> bind_buf_144(205 * 1022, 0);
  std::vector<SQLLEN> bind_ind_144(205, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt9, 6, SQL_C_WCHAR, bind_buf_144.data(), 1022, bind_ind_144.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt9);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt9), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt10 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc1, &stmt10);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    REQUIRE(stmt10 != SQL_NULL_HSTMT);
  }

  // SQLSetStmtAttr - SQL_ATTR_ENABLE_AUTO_IPD
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt10, SQL_ATTR_ENABLE_AUTO_IPD, (SQLPOINTER)1, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_CURSOR_TYPE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt10, SQL_ATTR_CURSOR_TYPE, nullptr, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(
        stmt10,
        sqlchar("SELECT \"ALLDATATYPESNAV\".\"ROWKIND\" AS \"ROWKIND\"\nFROM "
                "\"ODBCMETADATATESTDB\".\"DATATYPETESTS\".\"ALLDATATYPESNAV\" \"ALLDATATYPESNAV\"\nGROUP BY 1"),
        SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLExecute
  {
    SQLRETURN ret = SQLExecute(stmt10);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt10, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
    CHECK(numCols == 1);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt10, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt10, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt10, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt10, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt10, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt10, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt10, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt10, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt10, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 16);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt10, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt10, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt10, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt10, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt10, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROW_ARRAY_SIZE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt10, SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)30840, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  SQLULEN attr_ptr_145 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROWS_FETCHED_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt10, SQL_ATTR_ROWS_FETCHED_PTR, (SQLPOINTER)&attr_ptr_145, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  std::vector<SQLUSMALLINT> attr_ptr_146(30840, 0);
  // SQLSetStmtAttr - SQL_ATTR_ROW_STATUS_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt10, SQL_ATTR_ROW_STATUS_PTR, (SQLPOINTER)attr_ptr_146.data(), 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_147(30840 * 34, 0);
  std::vector<SQLLEN> bind_ind_147(30840, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt10, 1, SQL_C_WCHAR, bind_buf_147.data(), 34, bind_ind_147.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // BufferLength 34 holds 16 characters plus a NUL when SQLWCHAR is 2 bytes,
  // and 7 characters plus a NUL under iODBC, where SQLWCHAR is 4 bytes.
  // BOUNDARY is 8 characters, so that row truncates.
  {
    SQLRETURN ret = SQLFetch(stmt10);
    NON_IODBC { CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess()); }
    IODBC_ONLY {
      CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10),
                 OdbcMatchers::IsSuccessWithInfo() && OdbcMatchers::HasSqlState("01004"));
    }
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt10);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsNoData());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_148(30840 * 34, 0);
  std::vector<SQLLEN> bind_ind_148(30840, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt10, 1, SQL_C_WCHAR, bind_buf_148.data(), 34, bind_ind_148.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt10);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt10), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt11 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc1, &stmt11);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    REQUIRE(stmt11 != SQL_NULL_HSTMT);
  }

  // SQLSetStmtAttr - SQL_ATTR_ENABLE_AUTO_IPD
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt11, SQL_ATTR_ENABLE_AUTO_IPD, (SQLPOINTER)1, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_CURSOR_TYPE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt11, SQL_ATTR_CURSOR_TYPE, nullptr, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(
        stmt11,
        sqlchar("SELECT \"ALLDATATYPESNAV\".\"ROWKIND\" AS \"ROWKIND\",\n  SUM(\"ALLDATATYPESNAV\".\"INTVAL\") AS "
                "\"sum:INTVAL:ok\"\nFROM \"ODBCMETADATATESTDB\".\"DATATYPETESTS\".\"ALLDATATYPESNAV\" "
                "\"ALLDATATYPESNAV\"\nGROUP BY 1"),
        SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLExecute
  {
    SQLRETURN ret = SQLExecute(stmt11);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt11, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
    CHECK(numCols == 2);
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt11, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_VARCHAR);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt11, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt11, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt11, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt11, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt11, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt11, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt11, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt11, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 16);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt11, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt11, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt11, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt11, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt11, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_CONCISE_TYPE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt11, col, SQL_DESC_CONCISE_TYPE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
    CHECK(numAttr == SQL_DECIMAL);
  }

  // SQLColAttribute - SQL_DESC_UNSIGNED
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt11, col, SQL_DESC_UNSIGNED, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt11, col, SQL_DESC_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_BASE_COLUMN_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt11, col, SQL_DESC_BASE_COLUMN_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LABEL
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt11, col, SQL_DESC_LABEL, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_TYPE_NAME
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    char buf[1024] = {};
    SQLRETURN ret = SQLColAttribute(stmt11, col, SQL_DESC_TYPE_NAME, buf, 1024, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_OCTET_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt11, col, SQL_DESC_OCTET_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_LENGTH
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt11, col, SQL_DESC_LENGTH, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt11, col, SQL_DESC_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 136);
  }

  // SQLColAttribute - SQL_DESC_CASE_SENSITIVE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt11, col, SQL_DESC_CASE_SENSITIVE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_FIXED_PREC_SCALE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt11, col, SQL_DESC_FIXED_PREC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_NULLABLE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt11, col, SQL_DESC_NULLABLE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_PRECISION
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt11, col, SQL_DESC_PRECISION, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLColAttribute - SQL_DESC_SCALE
  {
    SQLUSMALLINT col = 2;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt11, col, SQL_DESC_SCALE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ROW_ARRAY_SIZE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt11, SQL_ATTR_ROW_ARRAY_SIZE, (SQLPOINTER)24966, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  SQLULEN attr_ptr_149 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROWS_FETCHED_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt11, SQL_ATTR_ROWS_FETCHED_PTR, (SQLPOINTER)&attr_ptr_149, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  std::vector<SQLUSMALLINT> attr_ptr_150(24966, 0);
  // SQLSetStmtAttr - SQL_ATTR_ROW_STATUS_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt11, SQL_ATTR_ROW_STATUS_PTR, (SQLPOINTER)attr_ptr_150.data(), 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_151(24966 * 34, 0);
  std::vector<SQLLEN> bind_ind_151(24966, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt11, 1, SQL_C_WCHAR, bind_buf_151.data(), 34, bind_ind_151.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_152(24966 * 8, 0);
  std::vector<SQLLEN> bind_ind_152(24966, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt11, 2, SQL_C_SBIGINT, bind_buf_152.data(), 8, bind_ind_152.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // BufferLength 34 holds 16 characters plus a NUL when SQLWCHAR is 2 bytes,
  // and 7 characters plus a NUL under iODBC, where SQLWCHAR is 4 bytes.
  // BOUNDARY is 8 characters, so that row truncates.
  {
    SQLRETURN ret = SQLFetch(stmt11);
    NON_IODBC { CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess()); }
    IODBC_ONLY {
      CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11),
                 OdbcMatchers::IsSuccessWithInfo() && OdbcMatchers::HasSqlState("01004"));
    }
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt11);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsNoData());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_153(24966 * 34, 0);
  std::vector<SQLLEN> bind_ind_153(24966, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt11, 1, SQL_C_WCHAR, bind_buf_153.data(), 34, bind_ind_153.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_154(24966 * 8, 0);
  std::vector<SQLLEN> bind_ind_154(24966, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt11, 2, SQL_C_SBIGINT, bind_buf_154.data(), 8, bind_ind_154.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_STMT, stmt11);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt11), OdbcMatchers::IsSuccess());
  }

  // SQLDisconnect
  {
    SQLRETURN ret = SQLDisconnect(dbc0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::Succeeded());
  }

  // SQLFreeHandle - SQLHDBC
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHENV
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_ENV, env0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env0), OdbcMatchers::IsSuccess());
  }

  // SQLDisconnect
  {
    SQLRETURN ret = SQLDisconnect(dbc1);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::Succeeded());
  }

  // SQLFreeHandle - SQLHDBC
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc1);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHENV
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_ENV, env1);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env1), OdbcMatchers::IsSuccess());
  }

  // skipped 88 SQLColAttribute call(s) with undocumented field id
}
