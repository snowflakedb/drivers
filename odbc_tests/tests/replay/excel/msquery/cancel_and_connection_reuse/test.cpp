#include <algorithm>
#include <chrono>
#include <cstring>
#include <string>
#include <thread>
#include <vector>

#include <catch2/catch_test_macros.hpp>

#include "ODBCConfig.hpp"
#include "compatibility.hpp"
#include "odbc_cast.hpp"
#include "odbc_matchers.hpp"

TEST_CASE("Replay: excel msquery cancel_and_connection_reuse", "[excel][msquery]") {
  SKIP_IODBC("Excel MS Query replays use the Windows ODBC Driver Manager");
  auto config = DataSourceConfig::Snowflake().install();

  SQLHENV env0 = SQL_NULL_HENV;
  // SQLAllocHandle - SQLHENV
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env0);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(env0 != SQL_NULL_HENV);
  }

  // SQLSetEnvAttr - SQL_ATTR_ODBC_VERSION (synthetic; not in trace)
  {
    SQLRETURN ret = SQLSetEnvAttr(env0, SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC2, 0);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env0), OdbcMatchers::IsSuccess());
  }

  SQLHDBC dbc0 = SQL_NULL_HDBC;
  // SQLAllocHandle - SQLHDBC
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env0, &dbc0);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env0), OdbcMatchers::IsSuccess());
    REQUIRE(dbc0 != SQL_NULL_HDBC);
  }

  // SQLDriverConnect
  {
    SQLRETURN ret = SQLDriverConnect(dbc0, nullptr, sqlchar(config.connection_string().c_str()), SQL_NTS, nullptr, 0,
                                     nullptr, SQL_DRIVER_NOPROMPT);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::Succeeded());
  }

  // SQLGetInfo - SQL_DATA_SOURCE_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DATA_SOURCE_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt0 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc0, &stmt0);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    REQUIRE(stmt0 != SQL_NULL_HSTMT);
  }

  // SQLGetInfo - SQL_ACTIVE_STATEMENTS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_ACTIVE_STATEMENTS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x0u);
  }

  // SQLGetInfo - SQL_DATA_SOURCE_READ_ONLY
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DATA_SOURCE_READ_ONLY, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "N");
  }

  // SQLGetInfo - SQL_DRIVER_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DRIVER_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_SEARCH_PATTERN_ESCAPE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_SEARCH_PATTERN_ESCAPE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "\\");
  }

  // SQLGetInfo - SQL_CORRELATION_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CORRELATION_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x2u);
  }

  // SQLGetInfo - SQL_NON_NULLABLE_COLUMNS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_NON_NULLABLE_COLUMNS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x0u);
  }

  // SQLGetInfo - SQL_CATALOG_NAME_SEPARATOR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CATALOG_NAME_SEPARATOR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == ".");
  }

  // SQLGetInfo - SQL_FILE_USAGE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_FILE_USAGE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x0u);
  }

  // SQLGetInfo - SQL_SEARCH_PATTERN_ESCAPE
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_SEARCH_PATTERN_ESCAPE, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "\\");
  }

  // SQLGetInfo - SQL_CATALOG_TERM
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CATALOG_TERM, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "database");
  }

  // SQLGetInfo - SQL_DATABASE_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DATABASE_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLTables
  {
    SQLRETURN ret = SQLTables(stmt0, sqlchar("%"), -3, sqlchar(""), 0, sqlchar(""), 0, sqlchar(""), 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_0(1 * 65, 0);
  std::vector<SQLLEN> bind_ind_0(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 1, SQL_C_CHAR, bind_buf_0.data(), 65, bind_ind_0.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // catalog object enumeration: the row count and object names are a
  // property of the connected account and fixture, not the driver, so
  // drain the cursor structurally rather than pinning environment-specific
  // rows.
  {
    SQLRETURN ret;
    long long enum_rows = 0;
    while ((ret = SQLFetch(stmt0)) == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO) {
      ++enum_rows;
    }
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsNoData());
    CHECK(enum_rows > 0);
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt0, SQL_CLOSE);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt0, SQL_UNBIND);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_MAX_SCHEMA_NAME_LEN
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_MAX_SCHEMA_NAME_LEN, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0xFFu);
  }

  // SQLTables
  {
    SQLRETURN ret = SQLTables(stmt0, sqlchar(""), 0, sqlchar("%"), -3, sqlchar(""), 0, sqlchar(""), 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_1(1 * 65, 0);
  std::vector<SQLLEN> bind_ind_1(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 2, SQL_C_CHAR, bind_buf_1.data(), 65, bind_ind_1.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // catalog object enumeration: the row count and object names are a
  // property of the connected account and fixture, not the driver, so
  // drain the cursor structurally rather than pinning environment-specific
  // rows.
  {
    SQLRETURN ret;
    long long enum_rows = 0;
    while ((ret = SQLFetch(stmt0)) == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO) {
      ++enum_rows;
    }
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsNoData());
    CHECK(enum_rows > 0);
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt0, SQL_CLOSE);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt0, SQL_UNBIND);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLTables
  {
    SQLRETURN ret = SQLTables(stmt0, sqlchar("ODBCMETADATATESTDB"), 18, nullptr, -3, nullptr, -3,
                              sqlchar("'TABLE','VIEW','SYNONYM'"), 24);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  std::vector<char> bind_buf_2(1 * 129, 0);
  std::vector<SQLLEN> bind_ind_2(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 3, SQL_C_CHAR, bind_buf_2.data(), 129, bind_ind_2.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 4
  std::vector<char> bind_buf_3(1 * 65, 0);
  std::vector<SQLLEN> bind_ind_3(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 4, SQL_C_CHAR, bind_buf_3.data(), 65, bind_ind_3.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_4(1 * 65, 0);
  std::vector<SQLLEN> bind_ind_4(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 2, SQL_C_CHAR, bind_buf_4.data(), 65, bind_ind_4.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // catalog object enumeration: the row count and object names are a
  // property of the connected account and fixture, not the driver, so
  // drain the cursor structurally rather than pinning environment-specific
  // rows.
  {
    SQLRETURN ret;
    long long enum_rows = 0;
    while ((ret = SQLFetch(stmt0)) == SQL_SUCCESS || ret == SQL_SUCCESS_WITH_INFO) {
      ++enum_rows;
    }
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsNoData());
    CHECK(enum_rows > 0);
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt0, SQL_CLOSE);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt0, SQL_UNBIND);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_IDENTIFIER_QUOTE_CHAR
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_IDENTIFIER_QUOTE_CHAR, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "\"");
  }

  // SQLGetTypeInfo
  {
    SQLRETURN ret = SQLGetTypeInfo(stmt0, SQL_ALL_TYPES);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 2
  std::vector<char> bind_buf_5(1 * 2, 0);
  std::vector<SQLLEN> bind_ind_5(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 2, SQL_C_DEFAULT, bind_buf_5.data(), 2, bind_ind_5.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 4
  std::vector<char> bind_buf_6(1 * 128, 0);
  std::vector<SQLLEN> bind_ind_6(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 4, SQL_C_CHAR, bind_buf_6.data(), 128, bind_ind_6.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 5
  std::vector<char> bind_buf_7(1 * 128, 0);
  std::vector<SQLLEN> bind_ind_7(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 5, SQL_C_CHAR, bind_buf_7.data(), 128, bind_ind_7.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 9
  std::vector<char> bind_buf_8(1 * 2, 0);
  std::vector<SQLLEN> bind_ind_8(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 9, SQL_C_DEFAULT, bind_buf_8.data(), 2, bind_ind_8.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 3
  std::vector<char> bind_buf_9(1 * 8, 0);
  std::vector<SQLLEN> bind_ind_9(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 3, SQL_C_DEFAULT, bind_buf_9.data(), 8, bind_ind_9.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 15
  std::vector<char> bind_buf_10(1 * 2, 0);
  std::vector<SQLLEN> bind_ind_10(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 15, SQL_C_DEFAULT, bind_buf_10.data(), 2, bind_ind_10.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 7
  std::vector<char> bind_buf_11(1 * 2, 0);
  std::vector<SQLLEN> bind_ind_11(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt0, 7, SQL_C_DEFAULT, bind_buf_11.data(), 2, bind_ind_11.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt0);
    NEW_DRIVER_ONLY("BD#119") { CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess()); }
    OLD_DRIVER_ONLY("BD#119") { CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsNoData()); }
  }

  NEW_DRIVER_ONLY("BD#119") {
    SQLRETURN ret = SQLFetch(stmt0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsNoData());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt0, SQL_CLOSE);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt0, SQL_UNBIND);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_ACTIVE_STATEMENTS
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_ACTIVE_STATEMENTS, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    SQLUINTEGER numericValue = 0;
    std::memcpy(&numericValue, buf, sizeof(numericValue));
    CHECK(numericValue == 0x0u);
  }

  SQLHSTMT stmt1 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc0, &stmt1);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    REQUIRE(stmt1 != SQL_NULL_HSTMT);
  }

  // SQLGetInfo - SQL_DATABASE_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DATABASE_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_TABLE_TERM
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_TABLE_TERM, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "table");
  }

  // SQLGetInfo - SQL_OWNER_TERM
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_OWNER_TERM, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "schema");
  }

  // SQLGetInfo - SQL_CATALOG_TERM
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_CATALOG_TERM, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    CHECK(std::string(buf) == "database");
  }

  SQLHSTMT stmt2 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc0, &stmt2);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    REQUIRE(stmt2 != SQL_NULL_HSTMT);
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(stmt2, sqlchar("SELECT SYSTEM$WAIT(10, 'SECONDS') AS WAITED"), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt2, SQL_DROP);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt2), OdbcMatchers::IsSuccess());
  }

  SQLHSTMT stmt3 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc0, &stmt3);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
    REQUIRE(stmt3 != SQL_NULL_HSTMT);
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(stmt3, sqlchar("SELECT SYSTEM$WAIT(10, 'SECONDS') AS WAITED"), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt3, SQL_DROP);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt3), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DATABASE_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DATABASE_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt1, SQL_CLOSE);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt1, SQL_UNBIND);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt1, SQL_RESET_PARAMS);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLPrepare
  {
    SQLRETURN ret = SQLPrepare(stmt1, sqlchar("SELECT SYSTEM$WAIT(10, 'SECONDS') AS WAITED"), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLExecute
  {
    SQLRETURN ret = SQLExecute(stmt1);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt1, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numCols == 1);
  }

  // SQLDescribeCol col 1
  {
    char colName[257] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt1, 1, reinterpret_cast<SQLCHAR*>(colName), 256, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "WAITED");
    CHECK(dataType == 12);
    CHECK(colSize == 134217728);
    CHECK(scale == 0);
    CHECK(nullable == 0);
  }

  // SQLColAttribute - SQL_COLUMN_DISPLAY_SIZE
  {
    SQLUSMALLINT col = 1;
    SQLLEN numAttr = 0;
    SQLSMALLINT strLen = 0;
    SQLRETURN ret = SQLColAttribute(stmt1, col, SQL_COLUMN_DISPLAY_SIZE, nullptr, 0, &strLen, &numAttr);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
    CHECK(numAttr == 134217728);
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_12(1 * 256, 0);
  std::vector<SQLLEN> bind_ind_12(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt1, 1, SQL_C_CHAR, bind_buf_12.data(), 256, bind_ind_12.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt1);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> bind_buf_13(1 * 256, 0);
  std::vector<SQLLEN> bind_ind_13(1, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt1, 1, SQL_C_CHAR, bind_buf_13.data(), 256, bind_ind_13.data());
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLFetch
  {
    SQLRETURN ret = SQLFetch(stmt1);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsNoData());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt1, SQL_CLOSE);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt1, SQL_UNBIND);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt1, SQL_RESET_PARAMS);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLGetInfo - SQL_DATABASE_NAME
  {
    char buf[256] = {};
    SQLSMALLINT len = 0;
    SQLRETURN ret = SQLGetInfo(dbc0, SQL_DATABASE_NAME, buf, 255, &len);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc0), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt1, SQL_DROP);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt1), OdbcMatchers::IsSuccess());
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt0, SQL_DROP);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt0), OdbcMatchers::IsSuccess());
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

  SQLHENV env1 = SQL_NULL_HENV;
  // SQLAllocHandle - SQLHENV
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env1);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(env1 != SQL_NULL_HENV);
  }

  // SQLSetEnvAttr - SQL_ATTR_ODBC_VERSION (synthetic; not in trace)
  {
    SQLRETURN ret = SQLSetEnvAttr(env1, SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC2, 0);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env1), OdbcMatchers::IsSuccess());
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
    SQLRETURN ret = SQLSetConnectAttr(dbc1, SQL_ATTR_LOGIN_TIMEOUT, (SQLPOINTER)45, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
  }

  // SQLDriverConnect
  {
    SQLRETURN ret = SQLDriverConnect(dbc1, nullptr, sqlchar(config.connection_string().c_str()), SQL_NTS, nullptr, 0,
                                     nullptr, SQL_DRIVER_NOPROMPT);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::Succeeded());
  }

  SQLHSTMT stmt4 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc1, &stmt4);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc1), OdbcMatchers::IsSuccess());
    REQUIRE(stmt4 != SQL_NULL_HSTMT);
  }

  // SQLSetStmtAttr - SQL_ATTR_ASYNC_ENABLE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt4, SQL_ATTR_ASYNC_ENABLE, (SQLPOINTER)1, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLExecDirect
  {
    SQLRETURN ret = SQLExecDirect(stmt4, sqlchar("SELECT SYSTEM$WAIT(10, 'SECONDS') AS WAITED"), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsStillExecuting());
  }

  // SQLExecDirect
  {
    SQLRETURN ret = SQLExecDirect(stmt4, sqlchar("SELECT SYSTEM$WAIT(10, 'SECONDS') AS WAITED"), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsStillExecuting());
  }

  // SQLExecDirect
  {
    SQLRETURN ret = SQLExecDirect(stmt4, sqlchar("SELECT SYSTEM$WAIT(10, 'SECONDS') AS WAITED"), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsStillExecuting());
  }

  // SQLExecDirect
  {
    SQLRETURN ret = SQLExecDirect(stmt4, sqlchar("SELECT SYSTEM$WAIT(10, 'SECONDS') AS WAITED"), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsStillExecuting());
  }

  // SQLExecDirect
  {
    SQLRETURN ret = SQLExecDirect(stmt4, sqlchar("SELECT SYSTEM$WAIT(10, 'SECONDS') AS WAITED"), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsStillExecuting());
  }

  // SQLExecDirect
  {
    SQLRETURN ret = SQLExecDirect(stmt4, sqlchar("SELECT SYSTEM$WAIT(10, 'SECONDS') AS WAITED"), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsStillExecuting());
  }

  // SQLExecDirect
  {
    SQLRETURN ret = SQLExecDirect(stmt4, sqlchar("SELECT SYSTEM$WAIT(10, 'SECONDS') AS WAITED"), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsStillExecuting());
  }

  // SQLExecDirect
  {
    SQLRETURN ret = SQLExecDirect(stmt4, sqlchar("SELECT SYSTEM$WAIT(10, 'SECONDS') AS WAITED"), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsStillExecuting());
  }

  // SQLExecDirect
  {
    SQLRETURN ret = SQLExecDirect(stmt4, sqlchar("SELECT SYSTEM$WAIT(10, 'SECONDS') AS WAITED"), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsStillExecuting());
  }

  // SQLExecDirect
  {
    SQLRETURN ret = SQLExecDirect(stmt4, sqlchar("SELECT SYSTEM$WAIT(10, 'SECONDS') AS WAITED"), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsStillExecuting());
  }

  SQLHENV env2 = SQL_NULL_HENV;
  // SQLAllocHandle - SQLHENV
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env2);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(env2 != SQL_NULL_HENV);
  }

  // SQLSetEnvAttr - SQL_ATTR_ODBC_VERSION
  {
    SQLRETURN ret = SQLSetEnvAttr(env2, SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env2), OdbcMatchers::IsSuccess());
  }

  // SQLSetEnvAttr - SQL_ATTR_CONNECTION_POOLING
  {
    SQLRETURN ret = SQLSetEnvAttr(env2, SQL_ATTR_CONNECTION_POOLING, (SQLPOINTER)2, -6);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env2), OdbcMatchers::IsSuccess());
  }

  // async execution: the host re-issues the same statement until the
  // server stops reporting SQL_STILL_EXECUTING. The poll count is timing-
  // dependent, so loop rather than pin the trace's count, and sleep between
  // polls: hammering the execute in a tight loop starves the driver's
  // in-flight async work and it never reaches a terminal code. A statement
  // that still has not settled after the bound fails the test rather than
  // looping forever, so a regression in async completion is caught.
  {
    SQLRETURN ret;
    int async_polls = 0;
    do {
      if (async_polls++ >= 300) {
        FAIL("async execute did not reach a terminal code within the poll bound");
      }
      if (async_polls > 1) {
        std::this_thread::sleep_for(std::chrono::milliseconds(100));
      }
      ret = SQLExecDirect(stmt4, sqlchar("SELECT SYSTEM$WAIT(10, 'SECONDS') AS WAITED"), SQL_NTS);
    } while (ret == SQL_STILL_EXECUTING);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ASYNC_ENABLE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt4, SQL_ATTR_ASYNC_ENABLE, nullptr, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt4, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
    CHECK(numCols == 1);
  }

  // SQLDescribeCol col 1
  {
    char colName[258] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt4, 1, reinterpret_cast<SQLCHAR*>(colName), 257, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "WAITED");
    CHECK(dataType == 12);
    CHECK(colSize == 134217728);
    CHECK(scale == 0);
    CHECK(nullable == 0);
  }

  // SQLSetStmtAttr - SQL_ATTR_ROW_BIND_TYPE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt4, SQL_ATTR_ROW_BIND_TYPE, (SQLPOINTER)65540, -6);
    NEW_DRIVER_ONLY("BD#147") { CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess()); }
    OLD_DRIVER_ONLY("BD#147") {
      if (get_platform() == PLATFORM::PLATFORM_WINDOWS ||
          (get_platform() == PLATFORM::PLATFORM_LINUX && get_arch() == ARCH::ARCH_X86_64)) {
        CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
      } else {
        CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4),
                   OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("S1000"));
      }
    }
  }

  SQLULEN attr_ptr_14 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROW_BIND_OFFSET_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt4, SQL_ATTR_ROW_BIND_OFFSET_PTR, (SQLPOINTER)&attr_ptr_14, -4);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> row_buf_stmt4(1 * 65540, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt4, 1, SQL_C_WCHAR, row_buf_stmt4.data() + 8, 65532,
                               reinterpret_cast<SQLLEN*>(row_buf_stmt4.data() + 0));
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

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt4, SQL_DROP);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt4), OdbcMatchers::IsSuccess());
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

  SQLHENV env3 = SQL_NULL_HENV;
  // SQLAllocHandle - SQLHENV
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env3);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(env3 != SQL_NULL_HENV);
  }

  // SQLSetEnvAttr - SQL_ATTR_ODBC_VERSION (synthetic; not in trace)
  {
    SQLRETURN ret = SQLSetEnvAttr(env3, SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC2, 0);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env3), OdbcMatchers::IsSuccess());
  }

  SQLHDBC dbc2 = SQL_NULL_HDBC;
  // SQLAllocHandle - SQLHDBC
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env3, &dbc2);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env3), OdbcMatchers::IsSuccess());
    REQUIRE(dbc2 != SQL_NULL_HDBC);
  }

  // SQLSetConnectAttr - SQL_ATTR_LOGIN_TIMEOUT
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc2, SQL_ATTR_LOGIN_TIMEOUT, (SQLPOINTER)45, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
  }

  // SQLDriverConnect
  {
    SQLRETURN ret = SQLDriverConnect(dbc2, nullptr, sqlchar(config.connection_string().c_str()), SQL_NTS, nullptr, 0,
                                     nullptr, SQL_DRIVER_NOPROMPT);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::Succeeded());
  }

  SQLHSTMT stmt5 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc2, &stmt5);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
    REQUIRE(stmt5 != SQL_NULL_HSTMT);
  }

  // SQLSetStmtAttr - SQL_ATTR_ASYNC_ENABLE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt5, SQL_ATTR_ASYNC_ENABLE, (SQLPOINTER)1, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLExecDirect
  {
    SQLRETURN ret = SQLExecDirect(stmt5, sqlchar("SELECT SYSTEM$WAIT(10, 'SECONDS') AS WAITED"), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsStillExecuting());
  }

  // SQLExecDirect
  {
    SQLRETURN ret = SQLExecDirect(stmt5, sqlchar("SELECT SYSTEM$WAIT(10, 'SECONDS') AS WAITED"), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsStillExecuting());
  }

  // SQLExecDirect
  {
    SQLRETURN ret = SQLExecDirect(stmt5, sqlchar("SELECT SYSTEM$WAIT(10, 'SECONDS') AS WAITED"), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsStillExecuting());
  }

  // SQLExecDirect
  {
    SQLRETURN ret = SQLExecDirect(stmt5, sqlchar("SELECT SYSTEM$WAIT(10, 'SECONDS') AS WAITED"), SQL_NTS);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsStillExecuting());
  }

  // SQLCancel
  {
    SQLRETURN ret = SQLCancel(stmt5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // The reference driver's SQLCancel/SQLCancelHandle does not interrupt an
  // in-flight async op; it runs to natural completion. Poll (re-issuing the
  // same execute, paced) until it stops reporting SQL_STILL_EXECUTING so the
  // statement is idle before teardown — otherwise teardown races the worker
  // and the driver manager reports the connection busy. A statement that
  // still has not settled after the bound fails the test rather than falling
  // through to teardown.
  {
    SQLRETURN ret = SQL_STILL_EXECUTING;
    int settle_polls = 0;
    while (ret == SQL_STILL_EXECUTING && settle_polls++ < 300) {
      std::this_thread::sleep_for(std::chrono::milliseconds(100));
      ret = SQLExecDirect(stmt5, sqlchar("SELECT SYSTEM$WAIT(10, 'SECONDS') AS WAITED"), SQL_NTS);
    }
    if (ret == SQL_STILL_EXECUTING) {
      FAIL("cancelled async execute did not settle within the poll bound");
    }
  }

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt5, SQL_DROP);
    // The post-cancel settle drained stmt5 to idle, so it drops cleanly on every
    // driver manager. The raw trace recorded SQL_ERROR here because Excel freed
    // the statement while its cancelled async worker was still winding down.
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt5), OdbcMatchers::IsSuccess());
  }

  // SQLDisconnect
  {
    SQLRETURN ret = SQLDisconnect(dbc2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::Succeeded());
  }

  // SQLFreeHandle - SQLHDBC
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc2);
    // Clean teardown after the settle (see the SQLFreeStmt note above).
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc2), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHENV
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_ENV, env3);
    // Clean teardown after the settle (see the SQLFreeStmt note above).
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env3), OdbcMatchers::IsSuccess());
  }

  SQLHENV env4 = SQL_NULL_HENV;
  // SQLAllocHandle - SQLHENV
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &env4);
    REQUIRE(ret == SQL_SUCCESS);
    REQUIRE(env4 != SQL_NULL_HENV);
  }

  // SQLSetEnvAttr - SQL_ATTR_ODBC_VERSION (synthetic; not in trace)
  {
    SQLRETURN ret = SQLSetEnvAttr(env4, SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC2, 0);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env4), OdbcMatchers::IsSuccess());
  }

  SQLHDBC dbc3 = SQL_NULL_HDBC;
  // SQLAllocHandle - SQLHDBC
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_DBC, env4, &dbc3);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env4), OdbcMatchers::IsSuccess());
    REQUIRE(dbc3 != SQL_NULL_HDBC);
  }

  // SQLSetConnectAttr - SQL_ATTR_LOGIN_TIMEOUT
  {
    SQLRETURN ret = SQLSetConnectAttr(dbc3, SQL_ATTR_LOGIN_TIMEOUT, (SQLPOINTER)45, 0);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
  }

  // SQLDriverConnect
  {
    SQLRETURN ret = SQLDriverConnect(dbc3, nullptr, sqlchar(config.connection_string().c_str()), SQL_NTS, nullptr, 0,
                                     nullptr, SQL_DRIVER_NOPROMPT);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::Succeeded());
  }

  SQLHSTMT stmt6 = SQL_NULL_HSTMT;
  // SQLAllocHandle - SQLHSTMT
  {
    SQLRETURN ret = SQLAllocHandle(SQL_HANDLE_STMT, dbc3, &stmt6);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
    REQUIRE(stmt6 != SQL_NULL_HSTMT);
  }

  // SQLSetStmtAttr - SQL_ATTR_ASYNC_ENABLE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt6, SQL_ATTR_ASYNC_ENABLE, (SQLPOINTER)1, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // async execution: the host re-issues the same statement until the
  // server stops reporting SQL_STILL_EXECUTING. The poll count is timing-
  // dependent, so loop rather than pin the trace's count, and sleep between
  // polls: hammering the execute in a tight loop starves the driver's
  // in-flight async work and it never reaches a terminal code. A statement
  // that still has not settled after the bound fails the test rather than
  // looping forever, so a regression in async completion is caught.
  {
    SQLRETURN ret;
    int async_polls = 0;
    do {
      if (async_polls++ >= 300) {
        FAIL("async execute did not reach a terminal code within the poll bound");
      }
      if (async_polls > 1) {
        std::this_thread::sleep_for(std::chrono::milliseconds(100));
      }
      ret = SQLExecDirect(stmt6, sqlchar("SELECT SYSTEM$WAIT(10, 'SECONDS') AS WAITED"), SQL_NTS);
    } while (ret == SQL_STILL_EXECUTING);
    REQUIRE_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLSetStmtAttr - SQL_ATTR_ASYNC_ENABLE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt6, SQL_ATTR_ASYNC_ENABLE, nullptr, -5);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLNumResultCols
  {
    SQLSMALLINT numCols = 0;
    SQLRETURN ret = SQLNumResultCols(stmt6, &numCols);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
    CHECK(numCols == 1);
  }

  // SQLDescribeCol col 1
  {
    char colName[258] = {};
    SQLSMALLINT dataType = 0, scale = 0, nullable = 0;
    SQLULEN colSize = 0;
    SQLRETURN ret = SQLDescribeCol(stmt6, 1, reinterpret_cast<SQLCHAR*>(colName), 257, nullptr, &dataType, &colSize,
                                   &scale, &nullable);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
    CHECK(std::string(colName) == "WAITED");
    CHECK(dataType == 12);
    CHECK(colSize == 134217728);
    CHECK(scale == 0);
    CHECK(nullable == 0);
  }

  // SQLSetStmtAttr - SQL_ATTR_ROW_BIND_TYPE
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt6, SQL_ATTR_ROW_BIND_TYPE, (SQLPOINTER)65540, -6);
    NEW_DRIVER_ONLY("BD#147") { CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess()); }
    OLD_DRIVER_ONLY("BD#147") {
      if (get_platform() == PLATFORM::PLATFORM_WINDOWS ||
          (get_platform() == PLATFORM::PLATFORM_LINUX && get_arch() == ARCH::ARCH_X86_64)) {
        CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
      } else {
        CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6),
                   OdbcMatchers::IsError() && OdbcMatchers::HasSqlState("S1000"));
      }
    }
  }

  SQLULEN attr_ptr_15 = 0;
  // SQLSetStmtAttr - SQL_ATTR_ROW_BIND_OFFSET_PTR
  {
    SQLRETURN ret = SQLSetStmtAttr(stmt6, SQL_ATTR_ROW_BIND_OFFSET_PTR, (SQLPOINTER)&attr_ptr_15, -4);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLBindCol col 1
  std::vector<char> row_buf_stmt6(1 * 65540, 0);
  {
    SQLRETURN ret = SQLBindCol(stmt6, 1, SQL_C_WCHAR, row_buf_stmt6.data() + 8, 65532,
                               reinterpret_cast<SQLLEN*>(row_buf_stmt6.data() + 0));
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

  // SQLFreeStmt
  {
    SQLRETURN ret = SQLFreeStmt(stmt6, SQL_DROP);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_STMT, stmt6), OdbcMatchers::IsSuccess());
  }

  // SQLDisconnect
  {
    SQLRETURN ret = SQLDisconnect(dbc3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::Succeeded());
  }

  // SQLFreeHandle - SQLHDBC
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_DBC, dbc3);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_DBC, dbc3), OdbcMatchers::IsSuccess());
  }

  // SQLFreeHandle - SQLHENV
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_ENV, env4);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env4), OdbcMatchers::IsSuccess());
  }

  // --- Replay-only env cleanup (not present in the original trace) ---
  // ODBC-consuming hosts (Excel, Power Query, ...) deliberately leave their
  // SQL_HANDLE_ENV handles allocated at shutdown — the pool root for
  // `SQL_ATTR_CONNECTION_POOLING` is anchored on the env, and any teardown
  // done during DllMain(DLL_PROCESS_DETACH) is invisible to the trace logger.
  // Our replay binary runs many tests in one process, so we explicitly free
  // each leaked env here to avoid leaking pooled connections across tests.
  // SQLFreeHandle - SQLHENV (env2, replay-only)
  {
    SQLRETURN ret = SQLFreeHandle(SQL_HANDLE_ENV, env2);
    CHECK_THAT(OdbcResult(ret, SQL_HANDLE_ENV, env2), OdbcMatchers::IsSuccess());
  }
}
